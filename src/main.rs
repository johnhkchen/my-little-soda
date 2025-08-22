use clap::Parser;
use anyhow::Result;

mod github;
mod agents;
mod workflows;
mod priority;
mod train_schedule;
mod telemetry;
mod metrics;
mod git;
mod bundling;
mod cli;

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

fn main() -> Result<()> {
    // Initialize OpenTelemetry tracing
    if let Err(e) = init_telemetry() {
        eprintln!("Warning: Failed to initialize telemetry: {}", e);
    }

    let cli = Cli::parse();
    
    match cli.command {
        // Default behavior: cargo run (no subcommand) - explain how to get work
        None => {
            tokio::runtime::Runtime::new()?.block_on(async {
                show_how_to_get_work().await
            })
        },
        Some(Commands::Route { agents }) => {
            tokio::runtime::Runtime::new()?.block_on(async {
                RouteCommand::new(agents).execute().await
            })
        },
        Some(Commands::Pop { mine, bundle_branches, yes }) => {
            tokio::runtime::Runtime::new()?.block_on(async {
                PopCommand::new(mine, bundle_branches, yes).execute().await
            })
        },
        Some(Commands::Status) => {
            tokio::runtime::Runtime::new()?.block_on(async {
                StatusCommand::new().execute().await
            })
        },
        Some(Commands::Init { agents, template, force, dry_run }) => {
            tokio::runtime::Runtime::new()?.block_on(async {
                InitCommand::new(agents, template, force, dry_run).execute().await
            })
        }
        Some(Commands::Reset) => {
            tokio::runtime::Runtime::new()?.block_on(async {
                ResetCommand::new().execute().await
            })
        }
        Some(Commands::Land { open_only, days, dry_run, verbose }) => {
            tokio::runtime::Runtime::new()?.block_on(async {
                LandCommand::new(!open_only, days, dry_run, verbose).execute().await
            })
        }
        Some(Commands::Bundle { force, dry_run, verbose }) => {
            tokio::runtime::Runtime::new()?.block_on(async {
                BundleCommand::new(force, dry_run, verbose).execute().await
            })
        }
        Some(Commands::Peek) => {
            tokio::runtime::Runtime::new()?.block_on(async {
                PeekCommand::new().execute().await
            })
        }
        Some(Commands::Metrics { hours, detailed }) => {
            tokio::runtime::Runtime::new()?.block_on(async {
                MetricsCommand::new(hours, detailed).execute().await
            })
        }
        Some(Commands::ExportMetrics { hours, output }) => {
            tokio::runtime::Runtime::new()?.block_on(async {
                ExportMetricsCommand::new(hours, output).execute().await
            })
        }
    }
}