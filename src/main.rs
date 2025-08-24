use anyhow::Result;
use clap::Parser;

mod agent_lifecycle;
mod agents;
mod autonomous;
mod bundling;
mod cli;
mod config;
mod database;
mod git;
mod github;
mod metrics;
mod observability;
mod priority;
mod shutdown;
mod telemetry;
mod train_schedule;
mod workflows;

use cli::commands::{
    actions::ActionsCommand,
    agent::{
        AgentDiagnoseCommand, AgentForceResetCommand, AgentRecoverCommand, AgentStatusCommand,
        AgentValidateCommand,
    },
    bundle::BundleCommand,
    init::InitCommand,
    land::LandCommand,
    metrics::{ExportMetricsCommand, MetricsCommand},
    peek::PeekCommand,
    pop::PopCommand,
    reset::ResetCommand,
    route::RouteCommand,
    show_how_to_get_work,
    status::StatusCommand,
    Command,
};
use cli::{AgentCommands, Cli, Commands};
use config::init_config;
use database::init_database;
use shutdown::ShutdownCoordinator;
use telemetry::init_telemetry;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize configuration first
    if let Err(e) = init_config() {
        eprintln!("Warning: Failed to initialize configuration: {e}");
    }

    // Initialize OpenTelemetry tracing
    if let Err(e) = init_telemetry() {
        eprintln!("Warning: Failed to initialize telemetry: {e}");
    }

    // Initialize database (if enabled)
    if let Err(e) = init_database().await {
        eprintln!("Warning: Failed to initialize database: {e}");
    }

    // Create shutdown coordinator for graceful shutdowns
    let _shutdown_coordinator = ShutdownCoordinator::new();

    let cli = Cli::parse();

    let result = match cli.command {
        // Default behavior: cargo run (no subcommand) - explain how to get work
        None => show_how_to_get_work().await,
        Some(Commands::Route { agents }) => {
            RouteCommand::new(agents)
                .with_ci_mode(cli.ci_mode)
                .execute()
                .await
        }
        Some(Commands::Pop {
            mine,
            bundle_branches,
            yes,
            verbose,
        }) => {
            PopCommand::new(mine, bundle_branches, yes)
                .with_ci_mode(cli.ci_mode)
                .with_verbose(verbose)
                .execute()
                .await
        }
        Some(Commands::Status) => {
            StatusCommand::new()
                .with_ci_mode(cli.ci_mode)
                .execute()
                .await
        }
        Some(Commands::Init {
            agents,
            template,
            force,
            dry_run,
        }) => {
            InitCommand::new(agents, template, force, dry_run)
                .with_ci_mode(cli.ci_mode)
                .execute()
                .await
        }
        Some(Commands::Reset) => {
            ResetCommand::new()
                .with_ci_mode(cli.ci_mode)
                .execute()
                .await
        }
        Some(Commands::Bottle {
            open_only,
            days,
            dry_run,
            verbose,
        }) => {
            LandCommand::new(!open_only, days, dry_run, verbose)
                .with_ci_mode(cli.ci_mode)
                .execute()
                .await
        }
        Some(Commands::Bundle {
            force,
            dry_run,
            verbose,
            diagnose,
        }) => {
            BundleCommand::new(force, dry_run, verbose, diagnose)
                .with_ci_mode(cli.ci_mode)
                .execute()
                .await
        }
        Some(Commands::Peek) => PeekCommand::new().with_ci_mode(cli.ci_mode).execute().await,
        Some(Commands::Metrics { hours, detailed }) => {
            MetricsCommand::new(hours, detailed)
                .with_ci_mode(cli.ci_mode)
                .execute()
                .await
        }
        Some(Commands::ExportMetrics { hours, output }) => {
            ExportMetricsCommand::new(hours, output)
                .with_ci_mode(cli.ci_mode)
                .execute()
                .await
        }
        Some(Commands::Actions {
            trigger_bundle,
            status,
            logs,
            run_id,
            force,
            dry_run,
            verbose,
        }) => {
            ActionsCommand::new(
                trigger_bundle,
                status,
                logs,
                run_id,
                force,
                dry_run,
                verbose,
            )
            .with_ci_mode(cli.ci_mode)
            .execute()
            .await
        }
        Some(Commands::Agent { command }) => match command {
            AgentCommands::Status { agent } => {
                AgentStatusCommand::new(agent.clone())
                    .with_ci_mode(cli.ci_mode)
                    .execute()
                    .await
            }
            AgentCommands::Diagnose { agent, all } => {
                AgentDiagnoseCommand::new(agent.clone(), all)
                    .with_ci_mode(cli.ci_mode)
                    .execute()
                    .await
            }
            AgentCommands::Recover {
                agent,
                all,
                dry_run,
            } => {
                AgentRecoverCommand::new(agent.clone(), all, dry_run)
                    .with_ci_mode(cli.ci_mode)
                    .execute()
                    .await
            }
            AgentCommands::ForceReset {
                agent,
                preserve_work,
            } => {
                AgentForceResetCommand::new(agent.clone(), preserve_work)
                    .with_ci_mode(cli.ci_mode)
                    .execute()
                    .await
            }
            AgentCommands::Validate { agent, all } => {
                AgentValidateCommand::new(agent.clone(), all)
                    .with_ci_mode(cli.ci_mode)
                    .execute()
                    .await
            }
        },
    };

    // Shutdown database connections and telemetry
    database::shutdown_database().await;
    telemetry::shutdown_telemetry();

    result
}
