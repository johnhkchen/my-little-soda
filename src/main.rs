use clap::Parser;
use anyhow::Result;

mod github;
mod agent_lifecycle;
mod agents;
mod workflows;
mod priority;
mod train_schedule;
mod telemetry;
mod metrics;
mod git;
mod bundling;
mod cli;
mod observability;
mod config;
mod shutdown;
mod database;

use cli::{Cli, Commands};
use cli::commands::{
    show_how_to_get_work,
    pop::PopCommand,
    route::RouteCommand,
    land::LandCommand,
    bundle::BundleCommand,
    peek::PeekCommand,
    status::StatusCommand,
    init::InitCommand,
    reset::ResetCommand,
    metrics::{MetricsCommand, ExportMetricsCommand},
};
use telemetry::init_telemetry;
use config::init_config;
use database::init_database;
use shutdown::ShutdownCoordinator;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize configuration first
    if let Err(e) = init_config() {
        eprintln!("Warning: Failed to initialize configuration: {}", e);
    }

    // Initialize OpenTelemetry tracing
    if let Err(e) = init_telemetry() {
        eprintln!("Warning: Failed to initialize telemetry: {}", e);
    }

    // Initialize database (if enabled)
    if let Err(e) = init_database().await {
        eprintln!("Warning: Failed to initialize database: {}", e);
    }

    // Create shutdown coordinator for graceful shutdowns
    let mut shutdown_coordinator = ShutdownCoordinator::new();

    let cli = Cli::parse();
    
    let result = match cli.command {
        // Default behavior: cargo run (no subcommand) - explain how to get work
        None => {
            show_how_to_get_work().await
        },
        Some(Commands::Route { agents }) => {
            RouteCommand::new(agents).execute().await
        },
        Some(Commands::Pop { mine, bundle_branches, yes }) => {
            PopCommand::new(mine, bundle_branches, yes).execute().await
        },
        Some(Commands::Status) => {
            StatusCommand::new().execute().await
        },
        Some(Commands::Init { agents, template, force, dry_run }) => {
            InitCommand::new(agents, template, force, dry_run).execute().await
        }
        Some(Commands::Reset) => {
            ResetCommand::new().execute().await
        }
        Some(Commands::Land { open_only, days, dry_run, verbose }) => {
            LandCommand::new(!open_only, days, dry_run, verbose).execute().await
        }
        Some(Commands::Bundle { force, dry_run, verbose, diagnose }) => {
            BundleCommand::new(force, dry_run, verbose, diagnose).execute().await
        }
        Some(Commands::Peek) => {
            PeekCommand::new().execute().await
        }
        Some(Commands::Metrics { hours, detailed }) => {
            MetricsCommand::new(hours, detailed).execute().await
        }
        Some(Commands::ExportMetrics { hours, output }) => {
            ExportMetricsCommand::new(hours, output).execute().await
        }
    };

    // Shutdown database connections and telemetry
    database::shutdown_database().await;
    telemetry::shutdown_telemetry();

    result
}