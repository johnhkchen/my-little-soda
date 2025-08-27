use anyhow::Result;
use config::{Config, Environment, File};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Main configuration structure for My Little Soda
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MyLittleSodaConfig {
    /// GitHub configuration
    pub github: GitHubConfig,
    /// Observability settings
    pub observability: ObservabilityConfig,
    /// Agent coordination settings
    pub agents: AgentConfig,
    /// Database settings (optional)
    pub database: Option<DatabaseConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GitHubConfig {
    /// GitHub API token (can be set via env var)
    pub token: Option<String>,
    /// Repository owner
    pub owner: String,
    /// Repository name
    pub repo: String,
    /// Rate limiting settings
    pub rate_limit: RateLimitConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RateLimitConfig {
    /// Requests per hour limit
    pub requests_per_hour: u32,
    /// Burst capacity
    pub burst_capacity: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ObservabilityConfig {
    /// Enable OpenTelemetry tracing
    pub tracing_enabled: bool,
    /// OTLP endpoint for traces (if different from stdout)
    pub otlp_endpoint: Option<String>,
    /// Log level
    pub log_level: String,
    /// Enable metrics collection
    pub metrics_enabled: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentConfig {
    /// Agent coordination timeout
    pub coordination_timeout_seconds: u64,
    /// Bundle queue processing settings
    pub bundle_processing: BundleConfig,
    /// Agent process management settings
    pub process_management: AgentProcessConfig,
    /// CI/CD mode optimizations
    pub ci_mode: CIModeConfig,
    /// Work continuity and restart recovery settings
    pub work_continuity: WorkContinuityConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CIModeConfig {
    /// Enable CI-optimized mode by default
    pub enabled: bool,
    /// Artifact handling strategy (standard, optimized, enhanced)
    pub artifact_handling: String,
    /// GitHub token optimization strategy
    pub github_token_strategy: String,
    /// Enable workflow state persistence
    pub workflow_state_persistence: bool,
    /// CI-specific timeout adjustments in seconds
    pub ci_timeout_adjustment: u64,
    /// Enable enhanced error reporting for CI environments
    pub enhanced_error_reporting: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentProcessConfig {
    /// Path to Claude Code binary for real agent processes
    pub claude_code_path: String,
    /// Timeout for agent processes in minutes
    pub timeout_minutes: u32,
    /// Enable automatic cleanup of failed processes
    pub cleanup_on_failure: bool,
    /// Working directory prefix for agent isolation
    pub work_dir_prefix: String,
    /// Enable real agent process spawning (vs mocks)
    pub enable_real_agents: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BundleConfig {
    /// Maximum bundles in queue
    pub max_queue_size: u32,
    /// Bundle processing timeout
    pub processing_timeout_seconds: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    /// Database URL (SQLite file path or connection string)
    pub url: String,
    /// Maximum connections in pool
    pub max_connections: u32,
    /// Enable automatic migrations
    pub auto_migrate: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WorkContinuityConfig {
    /// Enable work continuity across process restarts
    pub enable_continuity: bool,
    /// Path to agent state file
    pub state_file_path: String,
    /// Backup interval in minutes
    pub backup_interval_minutes: u32,
    /// Maximum recovery attempts
    pub max_recovery_attempts: u32,
    /// Validation timeout in seconds
    pub validation_timeout_seconds: u32,
    /// Force fresh start after this many hours
    pub force_fresh_start_after_hours: u32,
    /// Preserve partial work during recovery
    pub preserve_partial_work: bool,
}

impl Default for WorkContinuityConfig {
    fn default() -> Self {
        Self {
            enable_continuity: true,
            state_file_path: ".my-little-soda/agent-state.json".to_string(),
            backup_interval_minutes: 5,
            max_recovery_attempts: 3,
            validation_timeout_seconds: 30,
            force_fresh_start_after_hours: 24,
            preserve_partial_work: true,
        }
    }
}

impl Default for MyLittleSodaConfig {
    fn default() -> Self {
        Self {
            github: GitHubConfig {
                token: None, // Will be read from env var or .my-little-soda-rc
                owner: "johnhkchen".to_string(),
                repo: "my-little-soda".to_string(),
                rate_limit: RateLimitConfig {
                    requests_per_hour: 5000,
                    burst_capacity: 100,
                },
            },
            observability: ObservabilityConfig {
                tracing_enabled: true,
                otlp_endpoint: None, // Defaults to stdout
                log_level: "info".to_string(),
                metrics_enabled: true,
            },
            agents: AgentConfig {
                coordination_timeout_seconds: 300, // 5 minutes
                bundle_processing: BundleConfig {
                    max_queue_size: 50,
                    processing_timeout_seconds: 1800, // 30 minutes
                },
                process_management: AgentProcessConfig {
                    claude_code_path: "claude-code".to_string(),
                    timeout_minutes: 30,
                    cleanup_on_failure: true,
                    work_dir_prefix: ".my-little-soda/agents".to_string(),
                    enable_real_agents: false, // Start with mocks by default for safety
                },
                ci_mode: CIModeConfig {
                    enabled: false, // Disabled by default, enabled via --ci-mode flag
                    artifact_handling: "standard".to_string(),
                    github_token_strategy: "standard".to_string(),
                    workflow_state_persistence: true,
                    ci_timeout_adjustment: 300, // Additional 5 minutes for CI environments
                    enhanced_error_reporting: true,
                },
                work_continuity: WorkContinuityConfig {
                    enable_continuity: true,
                    state_file_path: ".my-little-soda/agent-state.json".to_string(),
                    backup_interval_minutes: 5,
                    max_recovery_attempts: 3,
                    validation_timeout_seconds: 30,
                    force_fresh_start_after_hours: 24,
                    preserve_partial_work: true,
                },
            },
            database: Some(DatabaseConfig {
                url: ".my-little-soda/my-little-soda.db".to_string(),
                max_connections: 10,
                auto_migrate: true,
            }),
        }
    }
}

impl MyLittleSodaConfig {
    /// Load configuration from multiple sources with precedence:
    /// 1. Default values
    /// 2. Configuration files (my-little-soda.toml, .my-little-soda-rc)
    /// 3. Environment variables (prefixed with MY_LITTLE_SODA_)
    pub fn load() -> Result<Self> {
        // Start with default configuration
        let mut builder = Config::builder();

        // Try to load from configuration files
        if Path::new("my-little-soda.toml").exists() {
            builder = builder.add_source(File::with_name("my-little-soda"));
        }

        if Path::new(".my-little-soda-rc").exists() {
            builder = builder.add_source(File::with_name(".my-little-soda-rc"));
        }

        // Override with environment variables
        builder = builder.add_source(
            Environment::with_prefix("MY_LITTLE_SODA")
                .separator("_")
                .try_parsing(true),
        );

        let config = builder.build()?;

        // Deserialize into our config struct
        let mut my_little_soda_config: MyLittleSodaConfig = config.try_deserialize()?;

        // Special handling for GitHub token - check multiple sources
        if my_little_soda_config.github.token.is_none() {
            // Try environment variable
            if let Ok(token) = std::env::var("GITHUB_TOKEN") {
                my_little_soda_config.github.token = Some(token);
            } else if let Ok(token) = std::env::var("MY_LITTLE_SODA_GITHUB_TOKEN") {
                my_little_soda_config.github.token = Some(token);
            }
        }

        Ok(my_little_soda_config)
    }

    /// Save configuration to file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let toml_content = toml::to_string_pretty(self)?;
        std::fs::write(path, toml_content)?;
        Ok(())
    }

    /// Load .env file if it exists
    pub fn load_env_file() -> Result<()> {
        if Path::new(".env").exists() {
            dotenvy::dotenv()?;
            tracing::info!("Loaded environment variables from .env file");
        }
        Ok(())
    }
}

/// Global configuration instance
static CONFIG: std::sync::LazyLock<Result<MyLittleSodaConfig, anyhow::Error>> =
    std::sync::LazyLock::new(|| {
        // Load .env file first
        let _ = MyLittleSodaConfig::load_env_file();
        MyLittleSodaConfig::load()
    });

/// Get the global configuration
pub fn config() -> Result<&'static MyLittleSodaConfig> {
    CONFIG
        .as_ref()
        .map_err(|e| anyhow::anyhow!("Failed to load configuration: {}", e))
}

/// Initialize configuration (called at startup)
pub fn init_config() -> Result<()> {
    let _config = config()?;
    tracing::info!("Configuration loaded successfully");
    Ok(())
}
