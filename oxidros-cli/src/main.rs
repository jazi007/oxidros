use clap::{Parser, Subcommand};

mod bag;
mod node;
mod param;
mod service;
mod topic;
pub mod type_registry;
pub mod type_resolve;
mod yaml;

/// ROS2 command-line tool powered by Zenoh.
///
/// Operates over the Zenoh middleware — no ROS2 installation required.
#[derive(Parser)]
#[command(name = "ros2", version, about)]
struct Cli {
    /// ROS domain ID (overrides ROS_DOMAIN_ID env)
    #[arg(long, env = "ROS_DOMAIN_ID", default_value = "0")]
    domain_id: u32,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Node introspection commands
    Node {
        #[command(subcommand)]
        action: node::NodeCommand,
    },
    /// Topic introspection commands
    Topic {
        #[command(subcommand)]
        action: topic::TopicCommand,
    },
    /// Service introspection commands
    Service {
        #[command(subcommand)]
        action: service::ServiceCommand,
    },
    /// Parameter commands
    Param {
        #[command(subcommand)]
        action: param::ParamCommand,
    },
    /// Bag recording and playback commands
    Bag {
        #[command(subcommand)]
        action: bag::BagCommand,
    },
}

/// Wait briefly for graph discovery to populate.
async fn wait_for_discovery() {
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();

    let cli = Cli::parse();

    // Create Zenoh context
    let ctx = if cli.domain_id != 0 {
        oxidros_zenoh::Context::with_domain_id(cli.domain_id)?
    } else {
        oxidros_zenoh::Context::new()?
    };

    // Wait for graph discovery
    wait_for_discovery().await;

    let graph = ctx.graph_cache();

    match cli.command {
        Commands::Node { action } => node::run(action, &graph),
        Commands::Topic { action } => topic::run(action, &ctx).await,
        Commands::Service { action } => service::run(action, &ctx).await,
        Commands::Param { action } => param::run(action, &ctx).await,
        Commands::Bag { action } => bag::run(action, &ctx).await,
    }
}
