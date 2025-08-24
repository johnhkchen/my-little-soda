use clap::{Parser, Subcommand};

pub mod commands;

#[derive(Parser)]
#[command(name = "my-little-soda")]
#[command(about = "GitHub-native multi-agent development orchestration")]
#[command(
    long_about = "My Little Soda orchestrates multiple AI coding agents using GitHub Issues as tasks, \
                       with automatic branch management and work coordination. Get started with 'my-little-soda pop' \
                       to claim your next task."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Enable CI-optimized mode for GitHub Actions environments
    #[arg(
        long,
        global = true,
        help = "Optimize operations for CI/CD environments with enhanced artifact handling"
    )]
    pub ci_mode: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Route multiple tickets to available agents (admin command for multi-agent coordination)
    Route {
        /// Maximum number of agents to route tickets to
        #[arg(
            long,
            default_value = "3",
            help = "Limit the number of agents that get assigned tickets"
        )]
        agents: u32,
    },
    /// Claim and start working on your next task (primary command for individual agents)
    Pop {
        /// Only consider tasks already assigned to you
        #[arg(long, help = "Restrict to tasks with your GitHub username as assignee")]
        mine: bool,
        /// Process overdue branches that are past their departure time (>10min)
        #[arg(
            long,
            help = "Interactive processing of overdue branches past departure time"
        )]
        bundle_branches: bool,
        /// Auto-approve all prompts during bundling (non-interactive mode)
        #[arg(
            short = 'y',
            long,
            help = "Skip interactive prompts and auto-approve bundling operations"
        )]
        yes: bool,
        /// Show detailed diagnostic information
        #[arg(
            long,
            short = 'v',
            help = "Show detailed diagnostic information during task assignment"
        )]
        verbose: bool,
    },
    /// Display system status, agent utilization, and task queue overview
    Status,
    /// Initialize multi-agent development environment
    Init {
        /// Number of agents to configure
        #[arg(
            long,
            default_value = "3",
            help = "Number of agents to configure (1-12)"
        )]
        agents: u32,
        /// Project template to use
        #[arg(
            long,
            help = "Project template: webapp, api, cli, microservices, library"
        )]
        template: Option<String>,
        /// Force initialization even if .clambake exists
        #[arg(
            long,
            help = "Force initialization, overwriting existing configuration"
        )]
        force: bool,
        /// Show what would be created without making changes
        #[arg(long, help = "Show what would be created without making changes")]
        dry_run: bool,
    },
    /// Reset all agents to idle state by removing agent labels from issues
    Reset,
    /// Complete work and bundle into a PR (replaces the old land functionality)
    Bottle {
        /// Only scan open issues (excludes auto-closed issues from GitHub PR merges)
        #[arg(long, help = "Only scan open issues, exclude recently closed issues")]
        open_only: bool,
        /// Number of days to look back for closed issues
        #[arg(
            long,
            default_value = "7",
            help = "Days to look back for closed issues when scanning"
        )]
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
        /// Show bundling system status and diagnostics
        #[arg(
            long,
            help = "Display bundling system diagnostics and troubleshooting information"
        )]
        diagnose: bool,
    },
    /// Preview the next task in queue without claiming it
    Peek,
    /// Display integration success metrics and performance analytics
    Metrics {
        /// Time window in hours to analyze (default: 24)
        #[arg(
            long,
            default_value = "24",
            help = "Hours of history to analyze for metrics"
        )]
        hours: u64,
        /// Show detailed breakdown including recent attempts
        #[arg(
            long,
            help = "Include detailed breakdown of recent integration attempts"
        )]
        detailed: bool,
    },
    /// Export metrics in JSON format for external monitoring systems
    ExportMetrics {
        /// Time window in hours to analyze (default: 24)
        #[arg(
            long,
            default_value = "24",
            help = "Hours of history to analyze for metrics"
        )]
        hours: u64,
        /// Output file path (default: stdout)
        #[arg(
            long,
            help = "File path to write JSON metrics (prints to stdout if not specified)"
        )]
        output: Option<String>,
    },
    /// Manage GitHub Actions integration for automated bundling workflows
    Actions {
        /// Trigger the bundling workflow manually
        #[arg(long, help = "Manually trigger the GitHub Actions bundling workflow")]
        trigger_bundle: bool,
        /// Show recent workflow run status
        #[arg(long, help = "Display status of recent bundling workflow runs")]
        status: bool,
        /// Show workflow logs for a specific run
        #[arg(
            long,
            help = "Display logs for a specific workflow run (requires --run-id)"
        )]
        logs: bool,
        /// Workflow run ID for log viewing
        #[arg(long, help = "Workflow run ID to fetch logs for")]
        run_id: Option<u64>,
        /// Force bundling outside of scheduled times
        #[arg(
            long,
            help = "Force bundling even when not at departure time (use with --trigger-bundle)"
        )]
        force: bool,
        /// Perform dry run without creating PRs
        #[arg(
            long,
            help = "Preview what would be bundled without making changes (use with --trigger-bundle)"
        )]
        dry_run: bool,
        /// Enable verbose output
        #[arg(long, short = 'v', help = "Show detailed workflow information")]
        verbose: bool,
    },
    /// Agent state management and diagnostic commands
    Agent {
        #[command(subcommand)]
        command: AgentCommands,
    },
}

#[derive(Subcommand)]
pub enum AgentCommands {
    /// Show agent status and current work
    Status {
        /// Specific agent to show status for
        #[arg(long, help = "Show status for specific agent (e.g., agent001)")]
        agent: Option<String>,
    },
    /// Diagnose agent state and validate consistency
    Diagnose {
        /// Specific agent to diagnose
        #[arg(long, help = "Diagnose specific agent (e.g., agent001)")]
        agent: Option<String>,
        /// Diagnose all agents
        #[arg(long, help = "Diagnose all agents")]
        all: bool,
    },
    /// Recover agents from stuck states
    Recover {
        /// Specific agent to recover
        #[arg(long, help = "Recover specific agent (e.g., agent001)")]
        agent: Option<String>,
        /// Recover all agents with issues
        #[arg(long, help = "Recover all agents with detected issues")]
        all: bool,
        /// Show what would be recovered without making changes
        #[arg(long, help = "Preview recovery actions without making changes")]
        dry_run: bool,
    },
    /// Force reset agent to idle state
    ForceReset {
        /// Agent to reset
        #[arg(long, help = "Agent to reset (e.g., agent001)", required = true)]
        agent: String,
        /// Preserve current work when possible
        #[arg(long, help = "Attempt to preserve current work when resetting")]
        preserve_work: bool,
    },
    /// Validate agent states against external reality
    Validate {
        /// Specific agent to validate
        #[arg(long, help = "Validate specific agent (e.g., agent001)")]
        agent: Option<String>,
        /// Validate all agents
        #[arg(long, help = "Validate all agents")]
        all: bool,
    },
}
