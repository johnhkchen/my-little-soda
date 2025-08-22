use clap::{Parser, Subcommand};

pub mod commands;

#[derive(Parser)]
#[command(name = "clambake")]
#[command(about = "GitHub-native multi-agent development orchestration")]
#[command(long_about = "Clambake orchestrates multiple AI coding agents using GitHub Issues as tasks, \
                       with automatic branch management and work coordination. Get started with 'clambake pop' \
                       to claim your next task.")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Route multiple tickets to available agents (admin command for multi-agent coordination)
    Route {
        /// Maximum number of agents to route tickets to
        #[arg(long, default_value = "3", help = "Limit the number of agents that get assigned tickets")]
        agents: u32,
    },
    /// Claim and start working on your next task (primary command for individual agents)
    Pop {
        /// Only consider tasks already assigned to you
        #[arg(long, help = "Restrict to tasks with your GitHub username as assignee")]
        mine: bool,
        /// Process overdue branches that are past their departure time (>10min)
        #[arg(long, help = "Interactive processing of overdue branches past departure time")]
        bundle_branches: bool,
        /// Auto-approve all prompts during bundling (non-interactive mode)
        #[arg(short = 'y', long, help = "Skip interactive prompts and auto-approve bundling operations")]
        yes: bool,
    },
    /// Display system status, agent utilization, and task queue overview
    Status,
    /// Initialize multi-agent development environment
    Init {
        /// Number of agents to configure
        #[arg(long, default_value = "3", help = "Number of agents to configure (1-12)")]
        agents: u32,
        /// Project template to use
        #[arg(long, help = "Project template: webapp, api, cli, microservices, library")]
        template: Option<String>,
        /// Force initialization even if .clambake exists
        #[arg(long, help = "Force initialization, overwriting existing configuration")]
        force: bool,
        /// Show what would be created without making changes
        #[arg(long, help = "Show what would be created without making changes")]
        dry_run: bool,
    },
    /// Reset all agents to idle state by removing agent labels from issues
    Reset,
    /// Complete agent lifecycle by detecting merged work and cleaning up issues
    Land {
        /// Only scan open issues (excludes auto-closed issues from GitHub PR merges)
        #[arg(long, help = "Only scan open issues, exclude recently closed issues")]
        open_only: bool,
        /// Number of days to look back for closed issues
        #[arg(long, default_value = "7", help = "Days to look back for closed issues when scanning")]
        days: u32,
        /// Show what would be cleaned without making changes
        #[arg(long, help = "Preview what would be cleaned without making changes")]
        dry_run: bool,
        /// Show detailed information about the scan process
        #[arg(long, short = 'v', help = "Show detailed scan information")]
        verbose: bool,
    },
    /// Bundle multiple completed branches into a single PR for efficient review
    Bundle {
        /// Force bundling outside of scheduled departure times
        #[arg(long, help = "Force bundling even when not at departure time")]
        force: bool,
        /// Show what would be bundled without making changes
        #[arg(long, help = "Preview what would be bundled without making changes")]
        dry_run: bool,
        /// Show detailed information about the bundling process
        #[arg(long, short = 'v', help = "Show detailed bundling information")]
        verbose: bool,
    },
    /// Preview the next task in queue without claiming it
    Peek,
    /// Display integration success metrics and performance analytics
    Metrics {
        /// Time window in hours to analyze (default: 24)
        #[arg(long, default_value = "24", help = "Hours of history to analyze for metrics")]
        hours: u64,
        /// Show detailed breakdown including recent attempts
        #[arg(long, help = "Include detailed breakdown of recent integration attempts")]
        detailed: bool,
    },
    /// Export metrics in JSON format for external monitoring systems
    ExportMetrics {
        /// Time window in hours to analyze (default: 24)
        #[arg(long, default_value = "24", help = "Hours of history to analyze for metrics")]
        hours: u64,
        /// Output file path (default: stdout)
        #[arg(long, help = "File path to write JSON metrics (prints to stdout if not specified)")]
        output: Option<String>,
    },
}