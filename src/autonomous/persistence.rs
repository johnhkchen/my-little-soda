use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncReadExt;
use chrono::{DateTime, Utc};
use thiserror::Error;
use tracing::{info, warn, error, debug};
use rand::Rng;

use super::{
    AutonomousWorkflowState,
    workflow_state_machine::StateTransitionRecord,
    error_recovery::AutonomousRecoveryAttempt,
};

/// Errors that can occur during state persistence operations
#[derive(Debug, Error)]
pub enum PersistenceError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    #[error("State corruption detected: {reason}")]
    StateCorruption { reason: String },
    
    #[error("Version mismatch: expected {expected}, found {found}")]
    VersionMismatch { expected: String, found: String },
    
    #[error("Lock acquisition failed: {reason}")]
    LockError { reason: String },
    
    #[error("Recovery failed: {reason}")]
    RecoveryFailed { reason: String },
}

/// Complete persistent state for autonomous workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistentWorkflowState {
    pub version: String,
    pub agent_id: String,
    pub current_state: Option<AutonomousWorkflowState>,
    pub start_time: Option<DateTime<Utc>>,
    pub max_work_hours: u8,
    pub state_history: Vec<StateTransitionRecord>,
    pub recovery_history: Vec<AutonomousRecoveryAttempt>,
    pub checkpoint_metadata: CheckpointMetadata,
    pub last_persisted: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointMetadata {
    pub checkpoint_id: String,
    pub creation_reason: CheckpointReason,
    pub integrity_hash: String,
    pub agent_pid: Option<u32>,
    pub hostname: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CheckpointReason {
    PeriodicSave,
    StateTransition,
    BeforeRecovery,
    BeforeShutdown,
    AfterError,
    UserRequested,
}

/// Configuration for state persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceConfig {
    pub enable_persistence: bool,
    pub persistence_directory: PathBuf,
    pub auto_save_interval_minutes: u32,
    pub max_state_history_entries: usize,
    pub max_recovery_history_entries: usize,
    pub compress_old_states: bool,
    pub backup_retention_days: u32,
    pub enable_integrity_checks: bool,
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            enable_persistence: true,
            persistence_directory: PathBuf::from(".clambake/autonomous_state"),
            auto_save_interval_minutes: 5,
            max_state_history_entries: 1000,
            max_recovery_history_entries: 500,
            compress_old_states: true,
            backup_retention_days: 7,
            enable_integrity_checks: true,
        }
    }
}

/// Trait for state persistence operations
#[async_trait]
pub trait StatePersistence {
    /// Save current state to persistent storage
    async fn save_state(
        &self,
        state: &PersistentWorkflowState,
        reason: CheckpointReason,
    ) -> Result<String, PersistenceError>; // Returns checkpoint ID
    
    /// Load state from persistent storage
    async fn load_state(&self, agent_id: &str) -> Result<Option<PersistentWorkflowState>, PersistenceError>;
    
    /// Create a checkpoint of current state
    async fn create_checkpoint(
        &self,
        state: &PersistentWorkflowState,
        reason: CheckpointReason,
    ) -> Result<String, PersistenceError>;
    
    /// Restore from a specific checkpoint
    async fn restore_from_checkpoint(
        &self,
        agent_id: &str,
        checkpoint_id: &str,
    ) -> Result<PersistentWorkflowState, PersistenceError>;
    
    /// List available checkpoints for an agent
    async fn list_checkpoints(&self, agent_id: &str) -> Result<Vec<CheckpointInfo>, PersistenceError>;
    
    /// Clean up old checkpoints and state files
    async fn cleanup_old_data(&self, agent_id: &str) -> Result<(), PersistenceError>;
    
    /// Verify state integrity
    async fn verify_integrity(&self, state: &PersistentWorkflowState) -> Result<bool, PersistenceError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointInfo {
    pub checkpoint_id: String,
    pub creation_time: DateTime<Utc>,
    pub reason: CheckpointReason,
    pub state_summary: StateSummary,
    pub file_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSummary {
    pub current_state_type: Option<String>,
    pub transitions_count: usize,
    pub recovery_attempts: usize,
    pub uptime_minutes: Option<i64>,
}

/// File system implementation of state persistence
pub struct FileSystemPersistence {
    config: PersistenceConfig,
}

impl FileSystemPersistence {
    pub fn new(config: PersistenceConfig) -> Self {
        Self { config }
    }
    
    /// Get the state file path for an agent
    fn get_state_file_path(&self, agent_id: &str) -> PathBuf {
        self.config.persistence_directory.join(format!("{}.state.json", agent_id))
    }
    
    /// Get the checkpoint directory for an agent
    fn get_checkpoint_dir(&self, agent_id: &str) -> PathBuf {
        self.config.persistence_directory.join(format!("{}_checkpoints", agent_id))
    }
    
    /// Get the checkpoint file path
    fn get_checkpoint_file_path(&self, agent_id: &str, checkpoint_id: &str) -> PathBuf {
        self.get_checkpoint_dir(agent_id).join(format!("{}.checkpoint.json", checkpoint_id))
    }
    
    /// Ensure directories exist
    async fn ensure_directories(&self, agent_id: &str) -> Result<(), PersistenceError> {
        fs::create_dir_all(&self.config.persistence_directory).await?;
        fs::create_dir_all(self.get_checkpoint_dir(agent_id)).await?;
        Ok(())
    }
    
    /// Calculate integrity hash for state
    fn calculate_integrity_hash(&self, state: &PersistentWorkflowState) -> String {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;
        
        let mut hasher = DefaultHasher::new();
        
        // Hash key fields that shouldn't change unexpectedly
        state.agent_id.hash(&mut hasher);
        state.version.hash(&mut hasher);
        state.state_history.len().hash(&mut hasher);
        state.recovery_history.len().hash(&mut hasher);
        
        if let Some(start_time) = state.start_time {
            start_time.timestamp().hash(&mut hasher);
        }
        
        format!("{:x}", hasher.finish())
    }
    
    /// Compress old state data if enabled
    async fn compress_if_needed(&self, _file_path: &Path) -> Result<(), PersistenceError> {
        // In a real implementation, this would compress old files
        // For now, just log that compression would happen
        if self.config.compress_old_states {
            debug!("Would compress old state file: {:?}", _file_path);
        }
        Ok(())
    }
    
    /// Prune state history to stay within limits
    fn prune_state_history(&self, state: &mut PersistentWorkflowState) {
        if state.state_history.len() > self.config.max_state_history_entries {
            let excess = state.state_history.len() - self.config.max_state_history_entries;
            state.state_history.drain(0..excess);
            info!(
                agent_id = %state.agent_id,
                pruned = %excess,
                remaining = %state.state_history.len(),
                "Pruned excess state history entries"
            );
        }
        
        if state.recovery_history.len() > self.config.max_recovery_history_entries {
            let excess = state.recovery_history.len() - self.config.max_recovery_history_entries;
            state.recovery_history.drain(0..excess);
            info!(
                agent_id = %state.agent_id,
                pruned = %excess,
                remaining = %state.recovery_history.len(),
                "Pruned excess recovery history entries"
            );
        }
    }
}

#[async_trait]
impl StatePersistence for FileSystemPersistence {
    async fn save_state(
        &self,
        state: &PersistentWorkflowState,
        reason: CheckpointReason,
    ) -> Result<String, PersistenceError> {
        if !self.config.enable_persistence {
            return Ok("persistence_disabled".to_string());
        }
        
        self.ensure_directories(&state.agent_id).await?;
        
        let mut state_to_save = state.clone();
        state_to_save.last_persisted = Utc::now();
        
        // Prune history if needed
        self.prune_state_history(&mut state_to_save);
        
        // Calculate integrity hash
        let integrity_hash = if self.config.enable_integrity_checks {
            self.calculate_integrity_hash(&state_to_save)
        } else {
            "integrity_disabled".to_string()
        };
        
        // Update metadata
        let checkpoint_id = format!("{}_{}", Utc::now().timestamp(), rand::rng().random::<u32>());
        state_to_save.checkpoint_metadata = CheckpointMetadata {
            checkpoint_id: checkpoint_id.clone(),
            creation_reason: reason.clone(),
            integrity_hash,
            agent_pid: std::process::id().into(),
            hostname: hostname::get()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
        };
        
        let state_file = self.get_state_file_path(&state.agent_id);
        let serialized = serde_json::to_string_pretty(&state_to_save)?;
        
        // Write to temporary file first, then rename (atomic operation)
        let temp_file = format!("{}.tmp", state_file.display());
        fs::write(&temp_file, serialized).await?;
        fs::rename(&temp_file, &state_file).await?;
        
        // Compress old data if needed
        self.compress_if_needed(&state_file).await?;
        
        info!(
            agent_id = %state.agent_id,
            checkpoint_id = %checkpoint_id,
            reason = ?reason,
            file = ?state_file,
            "State saved successfully"
        );
        
        Ok(checkpoint_id)
    }
    
    async fn load_state(&self, agent_id: &str) -> Result<Option<PersistentWorkflowState>, PersistenceError> {
        if !self.config.enable_persistence {
            return Ok(None);
        }
        
        let state_file = self.get_state_file_path(agent_id);
        
        if !state_file.exists() {
            info!(
                agent_id = %agent_id,
                file = ?state_file,
                "No existing state file found"
            );
            return Ok(None);
        }
        
        let mut file = fs::File::open(&state_file).await?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).await?;
        
        let state: PersistentWorkflowState = serde_json::from_str(&contents)?;
        
        // Verify integrity if enabled
        if self.config.enable_integrity_checks {
            if !self.verify_integrity(&state).await? {
                return Err(PersistenceError::StateCorruption {
                    reason: "Integrity check failed".to_string(),
                });
            }
        }
        
        info!(
            agent_id = %agent_id,
            checkpoint_id = %state.checkpoint_metadata.checkpoint_id,
            last_persisted = %state.last_persisted,
            "State loaded successfully"
        );
        
        Ok(Some(state))
    }
    
    async fn create_checkpoint(
        &self,
        state: &PersistentWorkflowState,
        reason: CheckpointReason,
    ) -> Result<String, PersistenceError> {
        if !self.config.enable_persistence {
            return Ok("persistence_disabled".to_string());
        }
        
        self.ensure_directories(&state.agent_id).await?;
        
        let checkpoint_id = format!("{}_{}", Utc::now().timestamp(), rand::rng().random::<u32>());
        let checkpoint_file = self.get_checkpoint_file_path(&state.agent_id, &checkpoint_id);
        
        let mut checkpoint_state = state.clone();
        checkpoint_state.checkpoint_metadata = CheckpointMetadata {
            checkpoint_id: checkpoint_id.clone(),
            creation_reason: reason.clone(),
            integrity_hash: self.calculate_integrity_hash(&checkpoint_state),
            agent_pid: std::process::id().into(),
            hostname: hostname::get()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
        };
        
        let serialized = serde_json::to_string_pretty(&checkpoint_state)?;
        fs::write(&checkpoint_file, serialized).await?;
        
        info!(
            agent_id = %state.agent_id,
            checkpoint_id = %checkpoint_id,
            reason = ?reason,
            file = ?checkpoint_file,
            "Checkpoint created successfully"
        );
        
        Ok(checkpoint_id)
    }
    
    async fn restore_from_checkpoint(
        &self,
        agent_id: &str,
        checkpoint_id: &str,
    ) -> Result<PersistentWorkflowState, PersistenceError> {
        let checkpoint_file = self.get_checkpoint_file_path(agent_id, checkpoint_id);
        
        if !checkpoint_file.exists() {
            return Err(PersistenceError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Checkpoint {} not found for agent {}", checkpoint_id, agent_id),
            )));
        }
        
        let mut file = fs::File::open(&checkpoint_file).await?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).await?;
        
        let state: PersistentWorkflowState = serde_json::from_str(&contents)?;
        
        if self.config.enable_integrity_checks && !self.verify_integrity(&state).await? {
            return Err(PersistenceError::StateCorruption {
                reason: "Checkpoint integrity check failed".to_string(),
            });
        }
        
        info!(
            agent_id = %agent_id,
            checkpoint_id = %checkpoint_id,
            "Restored from checkpoint successfully"
        );
        
        Ok(state)
    }
    
    async fn list_checkpoints(&self, agent_id: &str) -> Result<Vec<CheckpointInfo>, PersistenceError> {
        let checkpoint_dir = self.get_checkpoint_dir(agent_id);
        
        if !checkpoint_dir.exists() {
            return Ok(vec![]);
        }
        
        let mut checkpoints = Vec::new();
        let mut entries = fs::read_dir(&checkpoint_dir).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if stem.ends_with(".checkpoint") {
                        let checkpoint_id = stem.trim_end_matches(".checkpoint");
                        
                        // Load minimal info for the checkpoint
                        if let Ok(metadata) = fs::metadata(&path).await {
                            // For full implementation, would parse the file to get detailed info
                            let info = CheckpointInfo {
                                checkpoint_id: checkpoint_id.to_string(),
                                creation_time: metadata.created()
                                    .or_else(|_| metadata.modified())
                                    .map(|time| DateTime::<Utc>::from(time))
                                    .unwrap_or_else(|_| Utc::now()),
                                reason: CheckpointReason::PeriodicSave, // Would be loaded from file
                                state_summary: StateSummary {
                                    current_state_type: None,
                                    transitions_count: 0,
                                    recovery_attempts: 0,
                                    uptime_minutes: None,
                                },
                                file_size: metadata.len(),
                            };
                            checkpoints.push(info);
                        }
                    }
                }
            }
        }
        
        // Sort by creation time, newest first
        checkpoints.sort_by(|a, b| b.creation_time.cmp(&a.creation_time));
        
        Ok(checkpoints)
    }
    
    async fn cleanup_old_data(&self, agent_id: &str) -> Result<(), PersistenceError> {
        let retention_duration = chrono::Duration::days(self.config.backup_retention_days as i64);
        let cutoff_time = Utc::now() - retention_duration;
        
        let checkpoint_dir = self.get_checkpoint_dir(agent_id);
        if !checkpoint_dir.exists() {
            return Ok(());
        }
        
        let mut entries = fs::read_dir(&checkpoint_dir).await?;
        let mut cleaned_count = 0;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if let Ok(metadata) = fs::metadata(&path).await {
                let file_time = metadata.created()
                    .or_else(|_| metadata.modified())
                    .map(|time| DateTime::<Utc>::from(time))
                    .unwrap_or_else(|_| Utc::now());
                
                if file_time < cutoff_time {
                    if let Err(e) = fs::remove_file(&path).await {
                        warn!(
                            file = ?path,
                            error = %e,
                            "Failed to remove old checkpoint file"
                        );
                    } else {
                        cleaned_count += 1;
                        debug!(file = ?path, "Removed old checkpoint file");
                    }
                }
            }
        }
        
        if cleaned_count > 0 {
            info!(
                agent_id = %agent_id,
                cleaned_count = %cleaned_count,
                retention_days = %self.config.backup_retention_days,
                "Cleaned up old checkpoint files"
            );
        }
        
        Ok(())
    }
    
    async fn verify_integrity(&self, state: &PersistentWorkflowState) -> Result<bool, PersistenceError> {
        if !self.config.enable_integrity_checks {
            return Ok(true);
        }
        
        let expected_hash = &state.checkpoint_metadata.integrity_hash;
        let actual_hash = self.calculate_integrity_hash(state);
        
        let is_valid = expected_hash == &actual_hash;
        
        if !is_valid {
            warn!(
                agent_id = %state.agent_id,
                expected_hash = %expected_hash,
                actual_hash = %actual_hash,
                "State integrity check failed"
            );
        }
        
        Ok(is_valid)
    }
}

/// High-level state persistence manager
pub struct StatePersistenceManager {
    persistence: Box<dyn StatePersistence + Send + Sync>,
    config: PersistenceConfig,
    auto_save_handle: Option<tokio::task::JoinHandle<()>>,
}

impl StatePersistenceManager {
    /// Create new state persistence manager
    pub fn new(config: PersistenceConfig) -> Self {
        let persistence = Box::new(FileSystemPersistence::new(config.clone()));
        
        Self {
            persistence,
            config,
            auto_save_handle: None,
        }
    }
    
    /// Start automatic state saving with a channel-based approach
    pub async fn start_auto_save(&mut self, mut state_receiver: tokio::sync::mpsc::Receiver<PersistentWorkflowState>) -> Result<(), PersistenceError> {
        if self.auto_save_handle.is_some() {
            return Ok(()); // Already running
        }
        
        let interval_duration = tokio::time::Duration::from_secs(
            self.config.auto_save_interval_minutes as u64 * 60
        );
        
        // Create a shared reference to persistence that can be moved into the async task
        let persistence = Box::new(FileSystemPersistence::new(self.config.clone()));
        
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(interval_duration);
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // Periodic save - try to get latest state if available
                        if let Ok(state) = state_receiver.try_recv() {
                            match persistence.save_state(&state, CheckpointReason::PeriodicSave).await {
                                Ok(checkpoint_id) => {
                                    debug!(
                                        agent_id = %state.agent_id,
                                        checkpoint_id = %checkpoint_id,
                                        "Auto-save completed"
                                    );
                                }
                                Err(e) => {
                                    error!(
                                        agent_id = %state.agent_id,
                                        error = %e,
                                        "Auto-save failed"
                                    );
                                }
                            }
                        }
                    }
                    
                    Some(state) = state_receiver.recv() => {
                        // Immediate save when state is updated
                        match persistence.save_state(&state, CheckpointReason::StateTransition).await {
                            Ok(checkpoint_id) => {
                                debug!(
                                    agent_id = %state.agent_id,
                                    checkpoint_id = %checkpoint_id,
                                    "State transition auto-save completed"
                                );
                            }
                            Err(e) => {
                                error!(
                                    agent_id = %state.agent_id,
                                    error = %e,
                                    "State transition auto-save failed"
                                );
                            }
                        }
                    }
                    
                    else => {
                        // Channel closed, exit
                        break;
                    }
                }
            }
        });
        
        self.auto_save_handle = Some(handle);
        
        info!(
            interval_minutes = %self.config.auto_save_interval_minutes,
            "Started automatic state saving with channel-based updates"
        );
        
        Ok(())
    }
    
    /// Stop automatic state saving
    pub async fn stop_auto_save(&mut self) {
        if let Some(handle) = self.auto_save_handle.take() {
            handle.abort();
            info!("Stopped automatic state saving");
        }
    }
    
    /// Delegate to underlying persistence implementation
    pub async fn save_state(
        &self,
        state: &PersistentWorkflowState,
        reason: CheckpointReason,
    ) -> Result<String, PersistenceError> {
        self.persistence.save_state(state, reason).await
    }
    
    pub async fn load_state(&self, agent_id: &str) -> Result<Option<PersistentWorkflowState>, PersistenceError> {
        self.persistence.load_state(agent_id).await
    }
    
    pub async fn create_checkpoint(
        &self,
        state: &PersistentWorkflowState,
        reason: CheckpointReason,
    ) -> Result<String, PersistenceError> {
        self.persistence.create_checkpoint(state, reason).await
    }
    
    pub async fn restore_from_checkpoint(
        &self,
        agent_id: &str,
        checkpoint_id: &str,
    ) -> Result<PersistentWorkflowState, PersistenceError> {
        self.persistence.restore_from_checkpoint(agent_id, checkpoint_id).await
    }
    
    pub async fn list_checkpoints(&self, agent_id: &str) -> Result<Vec<CheckpointInfo>, PersistenceError> {
        self.persistence.list_checkpoints(agent_id).await
    }
    
    pub async fn cleanup_old_data(&self, agent_id: &str) -> Result<(), PersistenceError> {
        self.persistence.cleanup_old_data(agent_id).await
    }
    
    pub async fn verify_integrity(&self, state: &PersistentWorkflowState) -> Result<bool, PersistenceError> {
        self.persistence.verify_integrity(state).await
    }
}

impl Drop for StatePersistenceManager {
    fn drop(&mut self) {
        if let Some(handle) = self.auto_save_handle.take() {
            handle.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_state() -> PersistentWorkflowState {
        PersistentWorkflowState {
            version: "1.0.0".to_string(),
            agent_id: "test-agent".to_string(),
            current_state: None,
            start_time: Some(Utc::now()),
            max_work_hours: 8,
            state_history: vec![],
            recovery_history: vec![],
            checkpoint_metadata: CheckpointMetadata {
                checkpoint_id: "test".to_string(),
                creation_reason: CheckpointReason::PeriodicSave,
                integrity_hash: "test".to_string(),
                agent_pid: Some(1234),
                hostname: "test-host".to_string(),
            },
            last_persisted: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_filesystem_persistence_save_load() {
        let temp_dir = TempDir::new().unwrap();
        let config = PersistenceConfig {
            enable_persistence: true,
            persistence_directory: temp_dir.path().to_path_buf(),
            ..PersistenceConfig::default()
        };
        
        let persistence = FileSystemPersistence::new(config);
        let state = create_test_state();
        
        // Save state
        let checkpoint_id = persistence.save_state(&state, CheckpointReason::PeriodicSave).await.unwrap();
        assert!(!checkpoint_id.is_empty());
        
        // Load state
        let loaded_state = persistence.load_state(&state.agent_id).await.unwrap().unwrap();
        assert_eq!(loaded_state.agent_id, state.agent_id);
        assert_eq!(loaded_state.version, state.version);
    }
    
    #[tokio::test]
    async fn test_checkpoint_creation_and_restoration() {
        let temp_dir = TempDir::new().unwrap();
        let config = PersistenceConfig {
            enable_persistence: true,
            persistence_directory: temp_dir.path().to_path_buf(),
            ..PersistenceConfig::default()
        };
        
        let persistence = FileSystemPersistence::new(config);
        let state = create_test_state();
        
        // Create checkpoint
        let checkpoint_id = persistence.create_checkpoint(&state, CheckpointReason::BeforeRecovery).await.unwrap();
        assert!(!checkpoint_id.is_empty());
        
        // Restore from checkpoint
        let restored_state = persistence.restore_from_checkpoint(&state.agent_id, &checkpoint_id).await.unwrap();
        assert_eq!(restored_state.agent_id, state.agent_id);
    }
    
    #[tokio::test]
    async fn test_checkpoint_listing() {
        let temp_dir = TempDir::new().unwrap();
        let config = PersistenceConfig {
            enable_persistence: true,
            persistence_directory: temp_dir.path().to_path_buf(),
            ..PersistenceConfig::default()
        };
        
        let persistence = FileSystemPersistence::new(config);
        let state = create_test_state();
        
        // Create multiple checkpoints
        let _checkpoint1 = persistence.create_checkpoint(&state, CheckpointReason::PeriodicSave).await.unwrap();
        let _checkpoint2 = persistence.create_checkpoint(&state, CheckpointReason::BeforeRecovery).await.unwrap();
        
        // List checkpoints
        let checkpoints = persistence.list_checkpoints(&state.agent_id).await.unwrap();
        assert_eq!(checkpoints.len(), 2);
    }
    
    #[tokio::test]
    async fn test_state_persistence_manager() {
        let temp_dir = TempDir::new().unwrap();
        let config = PersistenceConfig {
            enable_persistence: true,
            persistence_directory: temp_dir.path().to_path_buf(),
            auto_save_interval_minutes: 1, // Short interval for testing
            ..PersistenceConfig::default()
        };
        
        let manager = StatePersistenceManager::new(config);
        let state = create_test_state();
        
        // Test save and load through manager
        let checkpoint_id = manager.save_state(&state, CheckpointReason::UserRequested).await.unwrap();
        assert!(!checkpoint_id.is_empty());
        
        let loaded_state = manager.load_state(&state.agent_id).await.unwrap().unwrap();
        assert_eq!(loaded_state.agent_id, state.agent_id);
    }
}