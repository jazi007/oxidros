use clap::Subcommand;
use mcap::WriteOptions;
use mcap::records::MessageHeader;
use oxidros_zenoh::{Context, GraphCache};
use std::collections::BTreeMap;
use std::io::BufWriter;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

#[derive(Subcommand)]
pub enum BagCommand {
    /// Record messages to an MCAP file
    Record {
        /// Topics to record (omit for all topics with -a)
        #[arg(trailing_var_arg = true)]
        topics: Vec<String>,
        /// Record all topics
        #[arg(short, long)]
        all: bool,
        /// Output file path
        #[arg(short, long, default_value = "rosbag2.mcap")]
        output: PathBuf,
        /// Compression: none, zstd, lz4
        #[arg(long, default_value = "zstd")]
        compression: String,
        /// Maximum recording duration (e.g. "30s", "5m")
        #[arg(long)]
        duration: Option<String>,
    },
    /// Play back messages from an MCAP bag file
    Play {
        /// Path to the MCAP file
        path: PathBuf,
        /// Playback rate multiplier (e.g. 2.0 = 2x speed)
        #[arg(short, long, default_value = "1.0")]
        rate: f64,
        /// Loop playback
        #[arg(short, long)]
        r#loop: bool,
        /// Only play these topics (comma-separated or repeated)
        #[arg(short, long)]
        topics: Vec<String>,
        /// Start offset (e.g. "5s", "1m")
        #[arg(long)]
        start_offset: Option<String>,
    },
    /// Show information about an MCAP bag file
    Info {
        /// Path to the MCAP file
        path: PathBuf,
    },
}

pub async fn run(cmd: BagCommand, ctx: &Context) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        BagCommand::Record {
            topics,
            all,
            output,
            compression,
            duration,
        } => record(ctx, topics, all, output, &compression, duration.as_deref()).await,
        BagCommand::Play {
            path,
            rate,
            r#loop,
            topics,
            start_offset,
        } => play(ctx, &path, rate, r#loop, &topics, start_offset.as_deref()).await,
        BagCommand::Info { path } => info(&path),
    }
}

// ============================================================================
// bag record
// ============================================================================

async fn record(
    ctx: &Context,
    topics: Vec<String>,
    all: bool,
    output: PathBuf,
    compression: &str,
    duration_str: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    if topics.is_empty() && !all {
        return Err("Specify topics to record, or use --all (-a) for all topics".into());
    }

    let graph = ctx.graph_cache();
    let max_duration = duration_str.map(parse_duration).transpose()?;

    // Discover topics to record
    let discovered = discover_topics(&graph, &topics, all)?;
    if discovered.is_empty() {
        return Err("No topics found to record".into());
    }

    eprintln!(
        "Recording {} topic(s) to {}",
        discovered.len(),
        output.display()
    );
    for (topic, dds_type, _hash) in &discovered {
        let ros_type =
            crate::type_resolve::dds_to_ros_type_name(dds_type).unwrap_or_else(|| dds_type.clone());
        eprintln!("  {topic} [{ros_type}]");
    }

    // Open MCAP writer
    let file = std::fs::File::create(&output)?;
    let buf_writer = BufWriter::new(file);

    let mcap_compression = match compression {
        "none" => None,
        "zstd" => Some(mcap::Compression::Zstd),
        "lz4" => Some(mcap::Compression::Lz4),
        other => {
            eprintln!("Unknown compression '{other}', using zstd");
            Some(mcap::Compression::Zstd)
        }
    };

    let opts = WriteOptions::new()
        .compression(mcap_compression)
        .profile("ros2".to_string())
        .library("oxidros-cli".to_string());

    let mut writer = opts.create(buf_writer)?;

    // Register schemas and channels for each topic, track channel IDs
    let mut channel_ids: Vec<u16> = Vec::new();

    for (topic_name, dds_type, type_hash) in &discovered {
        let ros_type =
            crate::type_resolve::dds_to_ros_type_name(dds_type).unwrap_or_else(|| dds_type.clone());

        // Try to resolve type description for schema
        let schema_data = match crate::type_resolve::resolve(dds_type, type_hash, ctx, &graph).await
        {
            Some(type_desc) => type_desc.to_msg_definition(),
            None => {
                eprintln!(
                    "  Warning: no type description for {dds_type}, recording without schema"
                );
                String::new()
            }
        };

        let schema_id = if schema_data.is_empty() {
            0u16
        } else {
            writer.add_schema(&ros_type, "ros2msg", schema_data.as_bytes())?
        };

        let mut metadata = BTreeMap::new();
        metadata.insert("dds_type".to_string(), dds_type.clone());
        metadata.insert("type_hash".to_string(), type_hash.clone());
        let channel_id = writer.add_channel(schema_id, topic_name, "cdr", &metadata)?;
        channel_ids.push(channel_id);
    }

    // Fan all subscribers into a single mpsc channel
    let (tx, mut rx) = tokio::sync::mpsc::channel::<(u16, Vec<u8>, i64)>(256);

    for (i, (topic_name, dds_type, type_hash)) in discovered.iter().enumerate() {
        let name = topic_name.strip_prefix('/').unwrap_or(topic_name);
        let key_expr = format!("{}/{}/{}/{}", ctx.domain_id(), name, dds_type, type_hash,);
        let sub = ctx
            .session()
            .declare_subscriber(&key_expr)
            .await
            .map_err(|e| format!("subscribe to {topic_name} failed: {e}"))?;

        let ch_id = channel_ids[i];
        let tx = tx.clone();

        tokio::spawn(async move {
            while let Ok(sample) = sub.recv_async().await {
                let payload = sample.payload().to_bytes().to_vec();
                let timestamp_ns = extract_timestamp(&sample);
                if tx.send((ch_id, payload, timestamp_ns)).await.is_err() {
                    break; // receiver dropped
                }
            }
        });
    }
    // Drop the original sender so rx closes when all spawn tasks end
    drop(tx);

    // Install Ctrl+C handler
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let shutdown_tx = std::sync::Mutex::new(Some(shutdown_tx));
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        if let Ok(mut guard) = shutdown_tx.lock()
            && let Some(tx) = guard.take()
        {
            let _ = tx.send(());
        }
    });

    let start_time = std::time::Instant::now();
    let mut message_count: u64 = 0;
    let mut sequence: u32 = 0;

    eprintln!("Recording... Press Ctrl+C to stop.");

    loop {
        tokio::select! {
            msg = rx.recv() => {
                match msg {
                    Some((channel_id, payload, timestamp_ns)) => {
                        let header = MessageHeader {
                            channel_id,
                            sequence,
                            log_time: timestamp_ns as u64,
                            publish_time: timestamp_ns as u64,
                        };
                        writer.write_to_known_channel(&header, &payload)?;
                        sequence = sequence.wrapping_add(1);
                        message_count += 1;
                    }
                    None => {
                        eprintln!("All subscribers closed");
                        break;
                    }
                }
            }
            _ = &mut shutdown_rx => {
                eprintln!("\nStopping recording...");
                break;
            }
            _ = async {
                match max_duration {
                    Some(d) => tokio::time::sleep(d.saturating_sub(start_time.elapsed())).await,
                    None => std::future::pending::<()>().await,
                }
            } => {
                eprintln!("\nDuration limit reached.");
                break;
            }
        }
    }

    // Finalize MCAP
    let _ = writer.finish()?;

    let elapsed = start_time.elapsed();
    eprintln!(
        "Recorded {message_count} messages in {:.1}s to {}",
        elapsed.as_secs_f64(),
        output.display()
    );

    Ok(())
}

/// Extract timestamp from a Zenoh sample's attachment (bytes 8..16 as i64 LE).
/// Falls back to current time if no attachment.
fn extract_timestamp(sample: &zenoh::sample::Sample) -> i64 {
    if let Some(att) = sample.attachment() {
        let bytes = att.to_bytes();
        if bytes.len() >= 16 {
            return i64::from_le_bytes(bytes[8..16].try_into().unwrap_or([0; 8]));
        }
    }
    // Fallback: current time
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as i64)
        .unwrap_or(0)
}

/// Discover topics to record from the graph cache.
#[allow(clippy::type_complexity)]
fn discover_topics(
    graph: &GraphCache,
    topic_filter: &[String],
    all: bool,
) -> Result<Vec<(String, String, String)>, Box<dyn std::error::Error>> {
    let mut result = Vec::new();

    if all {
        let topics = graph.get_topic_names_and_types();
        for (topic, _type_name) in topics {
            if let Some((dds_type, hash)) = find_topic_type(graph, &topic) {
                result.push((topic, dds_type, hash));
            }
        }
    } else {
        for topic in topic_filter {
            match find_topic_type(graph, topic) {
                Some((dds_type, hash)) => {
                    result.push((topic.clone(), dds_type, hash));
                }
                None => {
                    eprintln!("Warning: topic '{topic}' not found, skipping");
                }
            }
        }
    }

    Ok(result)
}

/// Find DDS type name and hash for a topic (prefers publishers).
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

/// Parse a duration string like "30s", "5m", "1h", "90" (seconds).
fn parse_duration(s: &str) -> Result<Duration, Box<dyn std::error::Error>> {
    let s = s.trim();
    if let Some(secs) = s.strip_suffix('s') {
        Ok(Duration::from_secs_f64(secs.parse()?))
    } else if let Some(mins) = s.strip_suffix('m') {
        Ok(Duration::from_secs_f64(mins.parse::<f64>()? * 60.0))
    } else if let Some(hours) = s.strip_suffix('h') {
        Ok(Duration::from_secs_f64(hours.parse::<f64>()? * 3600.0))
    } else {
        Ok(Duration::from_secs_f64(s.parse()?))
    }
}

// ============================================================================
// bag play
// ============================================================================

async fn play(
    ctx: &Context,
    path: &std::path::Path,
    rate: f64,
    do_loop: bool,
    topic_filter: &[String],
    start_offset_str: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    if rate <= 0.0 {
        return Err("Rate must be positive".into());
    }

    let start_offset = start_offset_str.map(parse_duration).transpose()?;
    let data = std::fs::read(path)?;

    eprintln!("Playing {} at {rate}x speed", path.display());

    // Ctrl+C handler
    let cancelled = Arc::new(AtomicBool::new(false));
    let cancelled_clone = cancelled.clone();
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        cancelled_clone.store(true, Ordering::Relaxed);
    });

    loop {
        let mut publishers: BTreeMap<u16, zenoh::pubsub::Publisher<'_>> = BTreeMap::new();
        let mut gids: BTreeMap<u16, [u8; 16]> = BTreeMap::new();
        let mut sequences: BTreeMap<u16, i64> = BTreeMap::new();

        // First pass: discover channels and create publishers
        let summary = mcap::Summary::read(&data)?.ok_or("MCAP file has no summary section")?;

        for channel in summary.channels.values() {
            let topic = &channel.topic;
            // Apply topic filter
            if !topic_filter.is_empty() && !topic_filter.iter().any(|f| f == topic) {
                continue;
            }
            // Get DDS type and hash from channel metadata, or derive from schema
            let (dds_type, type_hash) = resolve_channel_type(channel, ctx)?;
            let name = topic.strip_prefix('/').unwrap_or(topic);
            let key_expr_str = format!("{}/{}/{}/{}", ctx.domain_id(), name, dds_type, type_hash,);
            let key_expr = zenoh::key_expr::KeyExpr::try_from(key_expr_str)
                .map_err(|e| format!("invalid key expression for {topic}: {e}"))?;

            let publisher = ctx
                .session()
                .declare_publisher(key_expr)
                .await
                .map_err(|e| format!("Failed to create publisher for {topic}: {e}"))?;

            let gid: [u8; 16] = std::array::from_fn(|i| {
                // Deterministic but unique per channel
                ((channel.id as u8).wrapping_add(i as u8)).wrapping_mul(37)
            });

            let ros_type = channel
                .schema
                .as_ref()
                .map(|s| s.name.as_str())
                .unwrap_or("?");
            eprintln!("  {topic} [{ros_type}]");

            publishers.insert(channel.id, publisher);
            gids.insert(channel.id, gid);
            sequences.insert(channel.id, 0);
        }

        if publishers.is_empty() {
            return Err("No matching topics found in bag".into());
        }

        // Iterate messages
        let mut first_timestamp: Option<u64> = None;
        let mut last_wall_time: Option<std::time::Instant> = None;
        let mut last_msg_time: Option<u64> = None;
        let mut message_count: u64 = 0;

        let start_offset_ns = start_offset.map(|d| d.as_nanos() as u64).unwrap_or(0);

        for message in mcap::MessageStream::new(&data)? {
            if cancelled.load(Ordering::Relaxed) {
                eprintln!("\nPlayback stopped.");
                return Ok(());
            }

            let msg = message?;

            // Skip channels we're not publishing
            if !publishers.contains_key(&msg.channel.id) {
                continue;
            }

            // Track first timestamp for offset
            let first_ts = *first_timestamp.get_or_insert(msg.publish_time);

            // Apply start offset
            if msg.publish_time.saturating_sub(first_ts) < start_offset_ns {
                continue;
            }

            // Sleep to maintain timing
            if let (Some(last_wall), Some(last_ts)) = (last_wall_time, last_msg_time) {
                let msg_delta_ns = msg.publish_time.saturating_sub(last_ts);
                if msg_delta_ns > 0 {
                    let sleep_ns = (msg_delta_ns as f64 / rate) as u64;
                    let elapsed = last_wall.elapsed();
                    let target = Duration::from_nanos(sleep_ns);
                    if target > elapsed {
                        tokio::time::sleep(target - elapsed).await;
                        if cancelled.load(Ordering::Relaxed) {
                            eprintln!("\nPlayback stopped.");
                            return Ok(());
                        }
                    }
                }
            }

            last_wall_time = Some(std::time::Instant::now());
            last_msg_time = Some(msg.publish_time);

            // Build attachment
            let ch_id = msg.channel.id;
            let seq = sequences.get_mut(&ch_id).unwrap();
            let gid = gids[&ch_id];

            let mut attachment = [0u8; 33];
            attachment[0..8].copy_from_slice(&seq.to_le_bytes());
            attachment[8..16].copy_from_slice(&(msg.publish_time as i64).to_le_bytes());
            attachment[16] = 16;
            attachment[17..33].copy_from_slice(&gid);
            *seq += 1;

            // Publish
            let publisher = &publishers[&ch_id];
            publisher
                .put(msg.data.as_ref())
                .attachment(zenoh::bytes::ZBytes::from(attachment.to_vec()))
                .await
                .map_err(|e| format!("publish failed: {e}"))?;

            message_count += 1;
        }

        eprintln!("Played {message_count} messages.");

        if !do_loop || cancelled.load(Ordering::Relaxed) {
            break;
        }
        eprintln!("Looping...");
    }

    Ok(())
}

/// Resolve DDS type name and type hash for an MCAP channel.
/// Uses channel metadata if available, otherwise derives from schema name
/// and uses a placeholder hash.
fn resolve_channel_type(
    channel: &mcap::Channel,
    ctx: &Context,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    // Try channel metadata first (written by our recorder)
    if let (Some(dds_type), Some(type_hash)) = (
        channel.metadata.get("dds_type"),
        channel.metadata.get("type_hash"),
    ) {
        return Ok((dds_type.clone(), type_hash.clone()));
    }

    // Derive DDS type from schema name: "std_msgs/msg/String" → "std_msgs::msg::dds_::String_"
    if let Some(schema) = &channel.schema {
        let parts: Vec<&str> = schema.name.split('/').collect();
        if parts.len() == 3 {
            let dds_type = format!("{}::{}::dds_::{}_", parts[0], parts[1], parts[2]);

            // Try to find type hash from graph cache
            let graph = ctx.graph_cache();
            if let Some((_found_dds, hash)) = find_topic_type(&graph, &channel.topic) {
                return Ok((dds_type, hash));
            }

            // Use RIHS01 zero hash as fallback
            return Ok((dds_type, "RIHS01_0".to_string()));
        }
    }

    Err(format!(
        "Cannot determine type for channel '{}' — no metadata or schema",
        channel.topic
    )
    .into())
}

fn info(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let data = std::fs::read(path)?;
    let summary = mcap::Summary::read(&data)?.ok_or("MCAP file has no summary section")?;

    let file_size = data.len();

    println!("Files:             {}", path.display());
    println!("Size:              {}", format_size(file_size));

    if let Some(stats) = &summary.stats {
        let start_ns = stats.message_start_time;
        let end_ns = stats.message_end_time;
        let duration_ns = end_ns.saturating_sub(start_ns);
        let duration_secs = duration_ns as f64 / 1e9;

        println!("Duration:          {:.3}s", duration_secs);
        println!("Start:             {}", format_timestamp(start_ns));
        println!("End:               {}", format_timestamp(end_ns));
        println!("Messages:          {}", stats.message_count);
    }

    // Topics
    println!();
    println!("Topic information:");

    let mut topic_info: BTreeMap<String, (String, String, u64)> = BTreeMap::new();

    for (ch_id, channel) in &summary.channels {
        let schema_name = channel
            .schema
            .as_ref()
            .map(|s| s.name.clone())
            .unwrap_or_else(|| "unknown".to_string());
        let msg_count = summary
            .stats
            .as_ref()
            .and_then(|s| s.channel_message_counts.get(ch_id))
            .copied()
            .unwrap_or(0);

        topic_info.insert(
            channel.topic.clone(),
            (schema_name, channel.message_encoding.clone(), msg_count),
        );
    }

    let topic_width = topic_info.keys().map(|k| k.len()).max().unwrap_or(5).max(5);
    let type_width = topic_info
        .values()
        .map(|(t, _, _)| t.len())
        .max()
        .unwrap_or(4)
        .max(4);

    println!(
        "  {:topic_width$}  {:type_width$}  {:>8}  Encoding",
        "Topic", "Type", "Count"
    );
    for (topic, (type_name, encoding, count)) in &topic_info {
        println!(
            "  {:topic_width$}  {:type_width$}  {:>8}  {}",
            topic, type_name, count, encoding
        );
    }

    Ok(())
}

fn format_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

fn format_timestamp(ns: u64) -> String {
    let secs = (ns / 1_000_000_000) as i64;
    let nsec = (ns % 1_000_000_000) as u32;
    match chrono::DateTime::from_timestamp(secs, nsec) {
        Some(dt) => dt.format("%Y-%m-%d %H:%M:%S%.3f").to_string(),
        None => format!("{ns} ns"),
    }
}
