use anyhow::Result;
use config::{Config, File, Environment};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Main configuration structure for Clambake
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ClambakeConfig {
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
    /// Maximum number of concurrent agents
    pub max_agents: u32,
    /// Agent coordination timeout
    pub coordination_timeout_seconds: u64,
    /// Bundle queue processing settings
    pub bundle_processing: BundleConfig,
    /// Agent process management settings
    pub process_management: AgentProcessConfig,
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

impl Default for ClambakeConfig {
    fn default() -> Self {
        Self {
            github: GitHubConfig {
                token: None, // Will be read from env var or .clambakerc
                owner: "johnhkchen".to_string(),
                repo: "clambake".to_string(),
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
                max_agents: 4,
                coordination_timeout_seconds: 300, // 5 minutes
                bundle_processing: BundleConfig {
                    max_queue_size: 50,
                    processing_timeout_seconds: 1800, // 30 minutes
                },
                process_management: AgentProcessConfig {
                    claude_code_path: "claude-code".to_string(),
                    timeout_minutes: 30,
                    cleanup_on_failure: true,
                    work_dir_prefix: ".clambake/agents".to_string(),
                    enable_real_agents: false, // Start with mocks by default for safety
                },
            },
            database: Some(DatabaseConfig {
                url: ".clambake/clambake.db".to_string(),
                max_connections: 10,
                auto_migrate: true,
            }),
        }
    }
}

impl ClambakeConfig {
    /// Load configuration from multiple sources with precedence:
    /// 1. Default values
    /// 2. Configuration files (clambake.toml, .clambakerc)
    /// 3. Environment variables (prefixed with CLAMBAKE_)
    pub fn load() -> Result<Self> {
        // Start with default configuration
        let mut builder = Config::builder();

        // Try to load from configuration files
        if Path::new("clambake.toml").exists() {
            builder = builder.add_source(File::with_name("clambake"));
        }
        
        if Path::new(".clambakerc").exists() {
            builder = builder.add_source(File::with_name(".clambakerc"));
        }

        // Override with environment variables
        builder = builder.add_source(
            Environment::with_prefix("CLAMBAKE")
                .separator("_")
                .try_parsing(true)
        );

        let config = builder.build()?;
        
        // Deserialize into our config struct
        let mut clambake_config: ClambakeConfig = config.try_deserialize()?;

        // Special handling for GitHub token - check multiple sources
        if clambake_config.github.token.is_none() {
            // Try environment variable
            if let Ok(token) = std::env::var("GITHUB_TOKEN") {
                clambake_config.github.token = Some(token);
            } else if let Ok(token) = std::env::var("CLAMBAKE_GITHUB_TOKEN") {
                clambake_config.github.token = Some(token);
            }
        }

        Ok(clambake_config)
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
static CONFIG: std::sync::LazyLock<Result<ClambakeConfig, anyhow::Error>> = 
    std::sync::LazyLock::new(|| {
        // Load .env file first
        let _ = ClambakeConfig::load_env_file();
        ClambakeConfig::load()
    });

/// Get the global configuration
pub fn config() -> Result<&'static ClambakeConfig> {
    CONFIG.as_ref().map_err(|e| anyhow::anyhow!("Failed to load configuration: {}", e))
}

/// Initialize configuration (called at startup)
pub fn init_config() -> Result<()> {
    let _config = config()?;
    tracing::info!("Configuration loaded successfully");
    Ok(())
}