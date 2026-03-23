use clap::Subcommand;
use oxidros_zenoh::{EntityKind, GraphCache};
use std::collections::BTreeMap;

#[derive(Subcommand)]
pub enum NodeCommand {
    /// List all known nodes
    List,
    /// Show info about a specific node
    Info {
        /// Fully qualified node name (e.g. /my_node)
        name: String,
    },
}

pub fn run(cmd: NodeCommand, graph: &GraphCache) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        NodeCommand::List => list(graph),
        NodeCommand::Info { name } => info(graph, &name),
    }
}

fn list(graph: &GraphCache) -> Result<(), Box<dyn std::error::Error>> {
    let mut names = graph.get_node_names();
    names.sort();
    if names.is_empty() {
        eprintln!("No nodes found.");
    } else {
        for name in &names {
            println!("{name}");
        }
    }
    Ok(())
}

fn info(graph: &GraphCache, node_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut publishers: BTreeMap<String, String> = BTreeMap::new();
    let mut subscribers: BTreeMap<String, String> = BTreeMap::new();
    let mut service_servers: BTreeMap<String, String> = BTreeMap::new();
    let mut service_clients: BTreeMap<String, String> = BTreeMap::new();

    for entity in graph.get_all_entities() {
        let fqn = format_node_fqn(&entity.namespace, &entity.node_name);
        if fqn != node_name {
            continue;
        }
        let topic = match &entity.topic_name {
            Some(t) => t.clone(),
            None => continue,
        };
        let type_name = entity.type_name.as_deref().unwrap_or("unknown").to_string();

        match entity.kind {
            EntityKind::Publisher => {
                publishers.insert(topic, type_name);
            }
            EntityKind::Subscriber => {
                subscribers.insert(topic, type_name);
            }
            EntityKind::ServiceServer => {
                service_servers.insert(topic, type_name);
            }
            EntityKind::ServiceClient => {
                service_clients.insert(topic, type_name);
            }
            EntityKind::Node => {}
        }
    }

    println!("{node_name}");
    print_section("Publishers", &publishers);
    print_section("Subscribers", &subscribers);
    print_section("Service Servers", &service_servers);
    print_section("Service Clients", &service_clients);

    Ok(())
}

fn print_section(label: &str, items: &BTreeMap<String, String>) {
    println!("  {label}:");
    if items.is_empty() {
        println!("    (none)");
    } else {
        for (name, type_name) in items {
            println!("    {name}: {type_name}");
        }
    }
}

fn format_node_fqn(namespace: &str, name: &str) -> String {
    if namespace == "/" || namespace.is_empty() {
        format!("/{name}")
    } else {
        format!("{namespace}/{name}")
    }
}
