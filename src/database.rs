#[cfg(feature = "database")]
use anyhow::Result;
#[cfg(feature = "database")]
use sqlx::{migrate::MigrateDatabase, Row, SqlitePool};
#[cfg(feature = "database")]
use tracing::info;

#[cfg(feature = "database")]
/// Database manager for persistent state storage
pub struct DatabaseManager {
    pool: SqlitePool,
}

#[cfg(feature = "database")]
impl DatabaseManager {
    /// Initialize database with automatic migrations
    pub async fn new(database_url: &str, auto_migrate: bool) -> Result<Self> {
        // Create database if it doesn't exist
        if !sqlx::Sqlite::database_exists(database_url).await? {
            info!("Creating database at {}", database_url);
            sqlx::Sqlite::create_database(database_url).await?;
        }

        // Connect to database
        let pool = SqlitePool::connect(database_url).await?;

        // Run migrations if enabled
        if auto_migrate {
            info!("Running database migrations...");
            sqlx::migrate!("./migrations").run(&pool).await?;
            info!("Database migrations completed");
        }

        Ok(Self { pool })
    }

    /// Get database pool for queries
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Store agent coordination state
    pub async fn store_agent_state(
        &self,
        agent_id: &str,
        issue_number: u64,
        state: &str,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO agent_states (agent_id, issue_number, state, updated_at)
            VALUES (?1, ?2, ?3, datetime('now'))
            "#,
        )
        .bind(agent_id)
        .bind(issue_number as i64)
        .bind(state)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Retrieve agent coordination state
    pub async fn get_agent_state(&self, agent_id: &str) -> Result<Option<AgentState>> {
        let row = sqlx::query(
            r#"
            SELECT agent_id, issue_number, state, updated_at
            FROM agent_states
            WHERE agent_id = ?1
            ORDER BY updated_at DESC
            LIMIT 1
            "#,
        )
        .bind(agent_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let agent_id: String = row.get("agent_id");
            let issue_number: i64 = row.get("issue_number");
            let state: String = row.get("state");
            let updated_at: String = row.get("updated_at");

            Ok(Some(AgentState {
                agent_id,
                issue_number: issue_number as u64,
                state,
                updated_at,
            }))
        } else {
            Ok(None)
        }
    }

    /// Store bundle processing state
    pub async fn store_bundle_state(
        &self,
        bundle_id: &str,
        state: &str,
        metadata: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO bundle_states (bundle_id, state, metadata, updated_at)
            VALUES (?1, ?2, ?3, datetime('now'))
            "#,
        )
        .bind(bundle_id)
        .bind(state)
        .bind(metadata)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get all active bundle states
    pub async fn get_active_bundles(&self) -> Result<Vec<BundleState>> {
        let rows = sqlx::query(
            r#"
            SELECT bundle_id, state, metadata, updated_at
            FROM bundle_states
            WHERE state != 'completed' AND state != 'failed'
            ORDER BY updated_at ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let bundles = rows
            .into_iter()
            .map(|row| BundleState {
                bundle_id: row.get("bundle_id"),
                state: row.get("state"),
                metadata: row.get("metadata"),
                updated_at: row.get("updated_at"),
            })
            .collect();

        Ok(bundles)
    }

    /// Clean up old completed states
    pub async fn cleanup_old_states(&self, days_to_keep: i64) -> Result<()> {
        let deleted_agents = sqlx::query(
            r#"
            DELETE FROM agent_states
            WHERE updated_at < datetime('now', '-' || ?1 || ' days')
            "#,
        )
        .bind(days_to_keep)
        .execute(&self.pool)
        .await?;

        let deleted_bundles = sqlx::query(
            r#"
            DELETE FROM bundle_states
            WHERE state IN ('completed', 'failed')
            AND updated_at < datetime('now', '-' || ?1 || ' days')
            "#,
        )
        .bind(days_to_keep)
        .execute(&self.pool)
        .await?;

        info!(
            "Cleaned up {} old agent states and {} old bundle states",
            deleted_agents.rows_affected(),
            deleted_bundles.rows_affected()
        );

        Ok(())
    }

    /// Close database connections gracefully
    pub async fn shutdown(&self) {
        info!("Shutting down database connections...");
        self.pool.close().await;
        info!("Database connections closed");
    }
}

#[cfg(feature = "database")]
#[derive(Debug, Clone)]
pub struct AgentState {
    pub agent_id: String,
    pub issue_number: u64,
    pub state: String,
    pub updated_at: String,
}

#[cfg(feature = "database")]
#[derive(Debug, Clone)]
pub struct BundleState {
    pub bundle_id: String,
    pub state: String,
    pub metadata: Option<String>,
    pub updated_at: String,
}

#[cfg(feature = "database")]
static DB_MANAGER: std::sync::LazyLock<
    std::sync::Arc<tokio::sync::RwLock<Option<DatabaseManager>>>,
> = std::sync::LazyLock::new(|| std::sync::Arc::new(tokio::sync::RwLock::new(None)));

#[cfg(feature = "database")]
/// Initialize database manager
pub async fn init_database() -> Result<()> {
    let config = crate::config::config()?;

    if let Some(db_config) = &config.database {
        info!("Initializing database at {}", db_config.url);

        let manager = DatabaseManager::new(&db_config.url, db_config.auto_migrate).await?;

        let mut db_guard = DB_MANAGER.write().await;
        *db_guard = Some(manager);

        info!("Database manager initialized successfully");
    } else {
        info!("Database not configured, skipping initialization");
    }

    Ok(())
}

#[cfg(feature = "database")]
/// Get database manager
pub async fn database() -> Option<std::sync::Arc<tokio::sync::RwLock<Option<DatabaseManager>>>> {
    Some(DB_MANAGER.clone())
}

#[cfg(feature = "database")]
/// Shutdown database connections
pub async fn shutdown_database() {
    let db_guard = DB_MANAGER.read().await;
    if let Some(ref manager) = *db_guard {
        manager.shutdown().await;
    }
}

// Stub implementations for when database feature is not enabled
#[cfg(not(feature = "database"))]
pub async fn init_database() -> anyhow::Result<()> {
    tracing::info!("Database feature not enabled, skipping database initialization");
    Ok(())
}

#[cfg(not(feature = "database"))]
pub async fn shutdown_database() {
    tracing::info!("Database feature not enabled, no database to shutdown");
}
