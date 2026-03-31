use clap::Subcommand;
use oxidros_zenoh::{Context, GraphCache};
use std::collections::VecDeque;
use std::time::Instant;

#[derive(Subcommand)]
pub enum TopicCommand {
    /// List all topics
    List {
        /// Show topic types
        #[arg(short = 't', long)]
        show_types: bool,
        /// Show verbose info (publisher/subscriber counts)
        #[arg(short, long)]
        verbose: bool,
    },
    /// Show info about a specific topic
    Info {
        /// Topic name (e.g. /chatter)
        name: String,
    },
    /// Subscribe and print messages
    Echo {
        /// Topic name (e.g. /chatter)
        name: String,
        /// Print as JSON instead of YAML-like format
        #[arg(long)]
        json: bool,
        /// Print only one message and exit
        #[arg(long)]
        once: bool,
        /// Maximum number of messages to print (0 = unlimited)
        #[arg(short = 'n', long, default_value = "0")]
        max_count: usize,
    },
    /// Print message frequency
    Hz {
        /// Topic name (e.g. /chatter)
        name: String,
        /// Window size for rolling average
        #[arg(short, long, default_value = "100")]
        window: usize,
    },
    /// Print topic bandwidth
    Bw {
        /// Topic name (e.g. /chatter)
        name: String,
        /// Window size for rolling average
        #[arg(short, long, default_value = "100")]
        window: usize,
    },
}

pub async fn run(cmd: TopicCommand, ctx: &Context) -> Result<(), Box<dyn std::error::Error>> {
    let graph = ctx.graph_cache();
    match cmd {
        TopicCommand::List {
            show_types,
            verbose,
        } => list(&graph, show_types, verbose),
        TopicCommand::Info { name } => info(&graph, &name),
        TopicCommand::Echo {
            name,
            json,
            once,
            max_count,
        } => {
            let limit = if once { 1 } else { max_count };
            echo(ctx, &graph, &name, json, limit).await
        }
        TopicCommand::Hz { name, window } => hz(ctx, &graph, &name, window).await,
        TopicCommand::Bw { name, window } => bw(ctx, &graph, &name, window).await,
    }
}

fn list(
    graph: &GraphCache,
    show_types: bool,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let topics = graph.get_topic_names_and_types();
    if topics.is_empty() {
        eprintln!("No topics found.");
        return Ok(());
    }
    for (name, type_name) in &topics {
        if verbose {
            let pub_count = graph.count_publishers(name);
            let sub_count = graph.count_subscribers(name);
            println!("{name} [type: {type_name}, pub: {pub_count}, sub: {sub_count}]");
        } else if show_types {
            println!("{name} [type: {type_name}]");
        } else {
            println!("{name}");
        }
    }
    Ok(())
}

fn info(graph: &GraphCache, topic_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let pubs = graph.get_publishers_info(topic_name);
    let subs = graph.get_subscribers_info(topic_name);

    if pubs.is_empty() && subs.is_empty() {
        eprintln!("Topic '{topic_name}' not found.");
        return Ok(());
    }

    // Print type from first available entity
    let type_name = pubs
        .first()
        .or(subs.first())
        .and_then(|e| e.type_name.as_deref())
        .unwrap_or("unknown");

    // Deduplicate by node fully-qualified name
    let dedup = |entities: &[&oxidros_zenoh::EntityInfo]| -> Vec<String> {
        let mut seen = std::collections::BTreeSet::new();
        for e in entities {
            seen.insert(format_node_fqn(&e.namespace, &e.node_name));
        }
        seen.into_iter().collect()
    };

    let pub_nodes = dedup(&pubs);
    let sub_nodes = dedup(&subs);

    println!("Type: {type_name}");
    println!("Publisher count: {}", pub_nodes.len());
    println!("Subscriber count: {}", sub_nodes.len());

    if !pub_nodes.is_empty() {
        println!("\nPublishers:");
        for n in &pub_nodes {
            println!("  {n}");
        }
    }
    if !sub_nodes.is_empty() {
        println!("\nSubscribers:");
        for n in &sub_nodes {
            println!("  {n}");
        }
    }

    Ok(())
}

fn format_node_fqn(namespace: &str, name: &str) -> String {
    if namespace == "/" || namespace.is_empty() {
        format!("/{name}")
    } else {
        format!("{namespace}/{name}")
    }
}

// ============================================================================
// Helper: resolve topic type info from the graph cache
// ============================================================================

/// Find the DDS type name and type hash for a topic from the graph cache.
/// Prefers publishers, falls back to subscribers.
fn find_topic_type(graph: &GraphCache, topic: &str) -> Option<(String, String)> {
    let entities = graph.get_publishers_info(topic);
    let entity = if entities.is_empty() {
        graph.get_subscribers_info(topic).into_iter().next()
    } else {
        entities.into_iter().next()
    };
    let e = entity?;
    Some((e.type_name.clone()?, e.type_hash.clone()?))
}

/// Build the Zenoh key expression for a topic.
fn topic_key_expr(domain_id: u32, topic: &str, type_name: &str, type_hash: &str) -> String {
    let name = topic.strip_prefix('/').unwrap_or(topic);
    format!("{domain_id}/{name}/{type_name}/{type_hash}")
}

// ============================================================================
// topic echo
// ============================================================================

async fn echo(
    ctx: &Context,
    graph: &GraphCache,
    topic: &str,
    json_mode: bool,
    max_count: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let (dds_type, type_hash) = find_topic_type(graph, topic)
        .ok_or_else(|| format!("Topic '{topic}' not found or has no type information"))?;

    // Resolve the type description (registry or service call fallback)
    let type_desc = crate::type_resolve::resolve(&dds_type, ctx)
        .await
        .ok_or_else(|| format!("Cannot resolve type description for '{dds_type}'"))?;

    // Subscribe to the topic
    let key_expr = topic_key_expr(ctx.domain_id(), topic, &dds_type, &type_hash);
    let subscriber = ctx
        .session()
        .declare_subscriber(&key_expr)
        .await
        .map_err(|e| format!("subscribe failed: {e}"))?;

    let mut count = 0usize;
    loop {
        let sample = subscriber
            .recv_async()
            .await
            .map_err(|e| format!("recv failed: {e}"))?;
        let payload = sample.payload().to_bytes();

        match oxidros_dynamic::decode_cdr(&payload, &type_desc) {
            Ok(value) => {
                if json_mode {
                    println!("{}", serde_json::to_string_pretty(&value)?);
                } else {
                    print_yaml(&value, 0);
                }
                println!("---");
            }
            Err(e) => {
                eprintln!("decode error: {e}");
            }
        }

        count += 1;
        if max_count > 0 && count >= max_count {
            break;
        }
    }

    Ok(())
}

/// Print a serde_json::Value in YAML-like format (matching `ros2 topic echo` style).
fn print_yaml(value: &serde_json::Value, indent: usize) {
    let prefix = "  ".repeat(indent);
    match value {
        serde_json::Value::Object(map) => {
            for (key, val) in map {
                match val {
                    serde_json::Value::Object(_) => {
                        println!("{prefix}{key}:");
                        print_yaml(val, indent + 1);
                    }
                    serde_json::Value::Array(arr) => {
                        if arr.is_empty() {
                            println!("{prefix}{key}: []");
                        } else if is_scalar_array(arr) {
                            // Print small scalar arrays inline
                            let items: Vec<String> = arr.iter().map(format_scalar).collect();
                            println!("{prefix}{key}: [{}]", items.join(", "));
                        } else {
                            println!("{prefix}{key}:");
                            for item in arr {
                                if item.is_object() {
                                    println!("{prefix}- ");
                                    print_yaml(item, indent + 1);
                                } else {
                                    println!("{prefix}- {}", format_scalar(item));
                                }
                            }
                        }
                    }
                    _ => {
                        println!("{prefix}{key}: {}", format_scalar(val));
                    }
                }
            }
        }
        _ => {
            println!("{prefix}{}", format_scalar(value));
        }
    }
}

fn is_scalar_array(arr: &[serde_json::Value]) -> bool {
    arr.iter()
        .all(|v| matches!(v, serde_json::Value::Number(_) | serde_json::Value::Bool(_)))
}

fn format_scalar(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => format!("'{s}'"),
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(b) => if *b { "true" } else { "false" }.to_string(),
        serde_json::Value::Number(n) => {
            // Print floats without trailing zeros like ROS2 does
            if let Some(f) = n.as_f64()
                && n.is_f64()
            {
                return format!("{f}");
            }
            n.to_string()
        }
        serde_json::Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(format_scalar).collect();
            format!("[{}]", items.join(", "))
        }
        serde_json::Value::Object(_) => "{...}".to_string(),
    }
}

// ============================================================================
// topic hz
// ============================================================================

async fn hz(
    ctx: &Context,
    graph: &GraphCache,
    topic: &str,
    window: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let (dds_type, type_hash) = find_topic_type(graph, topic)
        .ok_or_else(|| format!("Topic '{topic}' not found or has no type information"))?;

    let key_expr = topic_key_expr(ctx.domain_id(), topic, &dds_type, &type_hash);
    let subscriber = ctx
        .session()
        .declare_subscriber(&key_expr)
        .await
        .map_err(|e| format!("subscribe failed: {e}"))?;

    let mut timestamps: VecDeque<Instant> = VecDeque::with_capacity(window + 1);

    println!("Subscribed to [{topic}]");
    loop {
        let _sample = subscriber
            .recv_async()
            .await
            .map_err(|e| format!("recv failed: {e}"))?;
        let now = Instant::now();
        timestamps.push_back(now);
        if timestamps.len() > window {
            timestamps.pop_front();
        }

        if timestamps.len() >= 2 {
            let dt = timestamps
                .back()
                .unwrap()
                .duration_since(*timestamps.front().unwrap());
            let n = timestamps.len() - 1;
            let avg_hz = n as f64 / dt.as_secs_f64();
            let avg_period = dt.as_secs_f64() / n as f64;
            // Min/max period from consecutive deltas
            let mut min_dt = f64::MAX;
            let mut max_dt = 0.0f64;
            let ts_vec: Vec<Instant> = timestamps.iter().copied().collect();
            for pair in ts_vec.windows(2) {
                let d = pair[1].duration_since(pair[0]).as_secs_f64();
                min_dt = min_dt.min(d);
                max_dt = max_dt.max(d);
            }
            println!(
                "average rate: {avg_hz:.3}\n\
                 \tmin: {min_dt:.6}s max: {max_dt:.6}s std dev: {:.6}s window: {n}",
                std_dev(&timestamps, avg_period),
            );
        }
    }
}

fn std_dev(timestamps: &VecDeque<Instant>, mean_period: f64) -> f64 {
    if timestamps.len() < 3 {
        return 0.0;
    }
    let ts_vec: Vec<Instant> = timestamps.iter().copied().collect();
    let periods: Vec<f64> = ts_vec
        .windows(2)
        .map(|w| w[1].duration_since(w[0]).as_secs_f64())
        .collect();
    let variance = periods
        .iter()
        .map(|p| (p - mean_period).powi(2))
        .sum::<f64>()
        / periods.len() as f64;
    variance.sqrt()
}

// ============================================================================
// topic bw
// ============================================================================

async fn bw(
    ctx: &Context,
    graph: &GraphCache,
    topic: &str,
    window: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let (dds_type, type_hash) = find_topic_type(graph, topic)
        .ok_or_else(|| format!("Topic '{topic}' not found or has no type information"))?;

    let key_expr = topic_key_expr(ctx.domain_id(), topic, &dds_type, &type_hash);
    let subscriber = ctx
        .session()
        .declare_subscriber(&key_expr)
        .await
        .map_err(|e| format!("subscribe failed: {e}"))?;

    let mut samples: VecDeque<(Instant, usize)> = VecDeque::with_capacity(window + 1);

    println!("Subscribed to [{topic}]");
    loop {
        let sample = subscriber
            .recv_async()
            .await
            .map_err(|e| format!("recv failed: {e}"))?;
        let size = sample.payload().len();
        let now = Instant::now();
        samples.push_back((now, size));
        if samples.len() > window {
            samples.pop_front();
        }

        if samples.len() >= 2 {
            let dt = samples
                .back()
                .unwrap()
                .0
                .duration_since(samples.front().unwrap().0);
            let total_bytes: usize = samples.iter().map(|(_, s)| s).sum();
            let n = samples.len();
            let bw = total_bytes as f64 / dt.as_secs_f64();
            let avg_size = total_bytes as f64 / n as f64;
            let (bw_val, bw_unit) = human_bandwidth(bw);
            let (sz_val, sz_unit) = human_size(avg_size);
            println!(
                "average: {bw_val:.2} {bw_unit}\n\
                 \tmean: {sz_val:.2} {sz_unit} min: {min_sz} max: {max_sz} window: {n}",
                min_sz = samples.iter().map(|(_, s)| s).min().unwrap(),
                max_sz = samples.iter().map(|(_, s)| s).max().unwrap(),
            );
        }
    }
}

fn human_bandwidth(bytes_per_sec: f64) -> (f64, &'static str) {
    if bytes_per_sec >= 1_000_000.0 {
        (bytes_per_sec / 1_000_000.0, "MB/s")
    } else if bytes_per_sec >= 1_000.0 {
        (bytes_per_sec / 1_000.0, "KB/s")
    } else {
        (bytes_per_sec, "B/s")
    }
}

fn human_size(bytes: f64) -> (f64, &'static str) {
    if bytes >= 1_000_000.0 {
        (bytes / 1_000_000.0, "MB")
    } else if bytes >= 1_000.0 {
        (bytes / 1_000.0, "KB")
    } else {
        (bytes, "B")
    }
}
