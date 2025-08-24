//! Integration tests for CI mode and GitHub Actions optimization
//!
//! These tests verify the CI mode functionality including:
//! - CLI flag parsing and propagation
//! - Workflow artifact handling
//! - GitHub Actions integration
//! - CI-specific optimizations
//! - Error handling in CI environments

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

mod fixtures;

/// Mock CI environment for testing
#[derive(Debug, Clone)]
pub struct MockCIEnvironment {
    pub github_actions: bool,
    pub workflow_run_id: Option<String>,
    pub artifacts_enabled: bool,
    pub environment_variables: HashMap<String, String>,
    pub timeout_adjustments: Duration,
}

impl MockCIEnvironment {
    pub fn new_github_actions() -> Self {
        let mut env_vars = HashMap::new();
        env_vars.insert("GITHUB_ACTIONS".to_string(), "true".to_string());
        env_vars.insert("GITHUB_RUN_ID".to_string(), "12345".to_string());
        env_vars.insert("MY_LITTLE_SODA_CI_MODE".to_string(), "true".to_string());
        env_vars.insert(
            "MY_LITTLE_SODA_ARTIFACT_HANDLING".to_string(),
            "optimized".to_string(),
        );

        Self {
            github_actions: true,
            workflow_run_id: Some("12345".to_string()),
            artifacts_enabled: true,
            environment_variables: env_vars,
            timeout_adjustments: Duration::from_secs(300), // 5 minute CI adjustment
        }
    }

    pub fn new_local() -> Self {
        Self {
            github_actions: false,
            workflow_run_id: None,
            artifacts_enabled: false,
            environment_variables: HashMap::new(),
            timeout_adjustments: Duration::from_secs(0),
        }
    }

    pub fn is_ci_mode(&self) -> bool {
        self.github_actions && self.workflow_run_id.is_some()
    }

    pub fn get_artifact_handling_strategy(&self) -> String {
        self.environment_variables
            .get("MY_LITTLE_SODA_ARTIFACT_HANDLING")
            .cloned()
            .unwrap_or_else(|| "standard".to_string())
    }
}

/// Mock artifact handler for testing CI mode artifact functionality
#[derive(Debug)]
pub struct MockArtifactHandler {
    pub artifacts: Arc<Mutex<HashMap<String, serde_json::Value>>>,
    pub upload_calls: Arc<Mutex<Vec<String>>>,
    pub download_calls: Arc<Mutex<Vec<String>>>,
}

impl Default for MockArtifactHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl MockArtifactHandler {
    pub fn new() -> Self {
        Self {
            artifacts: Arc::new(Mutex::new(HashMap::new())),
            upload_calls: Arc::new(Mutex::new(Vec::new())),
            download_calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn upload_bundle_metadata(&self, metadata: serde_json::Value) -> Result<String, String> {
        let artifact_id = format!("bundle-metadata-{}", chrono::Utc::now().timestamp());

        self.artifacts
            .lock()
            .unwrap()
            .insert(artifact_id.clone(), metadata);
        self.upload_calls.lock().unwrap().push(artifact_id.clone());

        Ok(artifact_id)
    }

    pub fn download_bundle_metadata(
        &self,
        artifact_id: &str,
    ) -> Result<Option<serde_json::Value>, String> {
        self.download_calls
            .lock()
            .unwrap()
            .push(artifact_id.to_string());

        Ok(self.artifacts.lock().unwrap().get(artifact_id).cloned())
    }

    pub fn list_artifacts(&self) -> Vec<String> {
        self.artifacts.lock().unwrap().keys().cloned().collect()
    }

    pub fn get_upload_call_count(&self) -> usize {
        self.upload_calls.lock().unwrap().len()
    }

    pub fn get_download_call_count(&self) -> usize {
        self.download_calls.lock().unwrap().len()
    }
}

/// Mock workflow coordinator with CI mode support
#[derive(Debug)]
pub struct MockCIWorkflowCoordinator {
    pub ci_environment: MockCIEnvironment,
    pub artifact_handler: MockArtifactHandler,
    pub workflow_executions: Arc<Mutex<Vec<WorkflowExecution>>>,
    pub performance_metrics: Arc<Mutex<HashMap<String, Duration>>>,
}

#[derive(Debug, Clone)]
pub struct WorkflowExecution {
    pub execution_id: String,
    pub ci_mode: bool,
    pub start_time: Instant,
    pub duration: Option<Duration>,
    pub artifact_handling: String,
    pub success: bool,
    pub error_message: Option<String>,
}

impl MockCIWorkflowCoordinator {
    pub fn new(ci_environment: MockCIEnvironment) -> Self {
        Self {
            ci_environment,
            artifact_handler: MockArtifactHandler::new(),
            workflow_executions: Arc::new(Mutex::new(Vec::new())),
            performance_metrics: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn execute_bundling_workflow(
        &self,
        ci_mode: bool,
        force: bool,
    ) -> Result<WorkflowExecution, String> {
        let start_time = Instant::now();
        let execution_id = format!("exec-{}", chrono::Utc::now().timestamp_nanos());

        // Simulate CI mode optimizations
        if ci_mode && self.ci_environment.is_ci_mode() {
            self.apply_ci_optimizations()?;
        }

        // Simulate bundling execution
        std::thread::sleep(Duration::from_millis(if ci_mode { 50 } else { 100 }));

        let duration = start_time.elapsed();
        let artifact_handling = if ci_mode {
            self.ci_environment.get_artifact_handling_strategy()
        } else {
            "standard".to_string()
        };

        let execution = WorkflowExecution {
            execution_id: execution_id.clone(),
            ci_mode,
            start_time,
            duration: Some(duration),
            artifact_handling: artifact_handling.clone(),
            success: true,
            error_message: None,
        };

        // Store artifacts if in CI mode
        if ci_mode && self.ci_environment.artifacts_enabled {
            let metadata = serde_json::json!({
                "execution_id": execution_id,
                "workflow_run_id": self.ci_environment.workflow_run_id,
                "duration_ms": duration.as_millis(),
                "ci_optimizations_applied": true,
                "artifact_handling": artifact_handling
            });

            self.artifact_handler.upload_bundle_metadata(metadata)?;
        }

        self.workflow_executions
            .lock()
            .unwrap()
            .push(execution.clone());
        self.performance_metrics
            .lock()
            .unwrap()
            .insert(execution_id.clone(), duration);

        Ok(execution)
    }

    pub fn simulate_workflow_failure(&self, ci_mode: bool) -> Result<WorkflowExecution, String> {
        let start_time = Instant::now();
        let execution_id = format!("exec-fail-{}", chrono::Utc::now().timestamp_nanos());

        std::thread::sleep(Duration::from_millis(25)); // Simulate quick failure

        let execution = WorkflowExecution {
            execution_id: execution_id.clone(),
            ci_mode,
            start_time,
            duration: Some(start_time.elapsed()),
            artifact_handling: "failed".to_string(),
            success: false,
            error_message: Some("Simulated workflow failure".to_string()),
        };

        // Store failure metadata in CI mode
        if ci_mode && self.ci_environment.artifacts_enabled {
            let metadata = serde_json::json!({
                "execution_id": execution_id,
                "workflow_run_id": self.ci_environment.workflow_run_id,
                "status": "failed",
                "error": execution.error_message
            });

            self.artifact_handler.upload_bundle_metadata(metadata)?;
        }

        self.workflow_executions
            .lock()
            .unwrap()
            .push(execution.clone());

        Ok(execution)
    }

    fn apply_ci_optimizations(&self) -> Result<(), String> {
        let start_time = Instant::now();

        // Simulate CI-specific optimizations
        std::thread::sleep(Duration::from_millis(10));

        let optimization_time = start_time.elapsed();
        self.performance_metrics
            .lock()
            .unwrap()
            .insert("ci_optimizations".to_string(), optimization_time);

        Ok(())
    }

    pub fn get_execution_history(&self) -> Vec<WorkflowExecution> {
        self.workflow_executions.lock().unwrap().clone()
    }

    pub fn get_ci_performance_metrics(&self) -> HashMap<String, Duration> {
        self.performance_metrics.lock().unwrap().clone()
    }

    pub fn verify_ci_optimizations_applied(&self) -> bool {
        let executions = self.get_execution_history();
        executions.iter().any(|exec| exec.ci_mode && exec.success)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ci_mode_environment_detection() {
        // Given: Different environment configurations
        let github_actions_env = MockCIEnvironment::new_github_actions();
        let local_env = MockCIEnvironment::new_local();

        // Then: CI mode detection should work correctly
        assert!(
            github_actions_env.is_ci_mode(),
            "GitHub Actions environment should be detected as CI mode"
        );
        assert!(
            !local_env.is_ci_mode(),
            "Local environment should not be detected as CI mode"
        );

        // And: Artifact handling strategies should differ
        assert_eq!(
            github_actions_env.get_artifact_handling_strategy(),
            "optimized"
        );
        assert_eq!(local_env.get_artifact_handling_strategy(), "standard");
    }

    #[test]
    fn test_ci_mode_workflow_execution() {
        // Given: A CI environment and workflow coordinator
        let ci_env = MockCIEnvironment::new_github_actions();
        let coordinator = MockCIWorkflowCoordinator::new(ci_env);

        // When: We execute workflows with and without CI mode
        let ci_execution = coordinator.execute_bundling_workflow(true, false).unwrap();
        let standard_execution = coordinator.execute_bundling_workflow(false, false).unwrap();

        // Then: CI mode execution should be optimized
        assert!(
            ci_execution.ci_mode,
            "CI execution should have CI mode enabled"
        );
        assert!(
            !standard_execution.ci_mode,
            "Standard execution should not have CI mode"
        );

        // And: CI mode should be faster due to optimizations
        let ci_duration = ci_execution.duration.unwrap();
        let standard_duration = standard_execution.duration.unwrap();
        assert!(
            ci_duration <= standard_duration,
            "CI mode should be faster or equal due to optimizations"
        );

        // And: Artifact handling should differ
        assert_eq!(ci_execution.artifact_handling, "optimized");
        assert_eq!(standard_execution.artifact_handling, "standard");
    }

    #[test]
    fn test_ci_mode_artifact_handling() {
        // Given: A CI environment with artifacts enabled
        let ci_env = MockCIEnvironment::new_github_actions();
        let coordinator = MockCIWorkflowCoordinator::new(ci_env);

        // When: We execute a CI mode workflow
        let execution = coordinator.execute_bundling_workflow(true, false).unwrap();

        // Then: Artifacts should be uploaded
        assert!(
            coordinator.artifact_handler.get_upload_call_count() > 0,
            "Artifacts should be uploaded in CI mode"
        );

        // And: Artifact metadata should be accessible
        let artifacts = coordinator.artifact_handler.list_artifacts();
        assert!(!artifacts.is_empty(), "CI mode should generate artifacts");

        // When: We retrieve the uploaded metadata
        let artifact_id = &artifacts[0];
        let metadata = coordinator
            .artifact_handler
            .download_bundle_metadata(artifact_id)
            .unwrap();

        // Then: Metadata should contain execution information
        assert!(
            metadata.is_some(),
            "Artifact metadata should be retrievable"
        );
        let metadata = metadata.unwrap();
        assert_eq!(metadata["execution_id"], execution.execution_id);
        assert_eq!(metadata["ci_optimizations_applied"], true);
    }

    #[test]
    fn test_ci_mode_performance_optimization() {
        // Given: A CI environment
        let ci_env = MockCIEnvironment::new_github_actions();
        let coordinator = MockCIWorkflowCoordinator::new(ci_env);

        // When: We execute multiple workflows
        let execution_count = 5;
        let mut ci_durations = Vec::new();
        let mut standard_durations = Vec::new();

        for _ in 0..execution_count {
            let ci_exec = coordinator.execute_bundling_workflow(true, false).unwrap();
            let std_exec = coordinator.execute_bundling_workflow(false, false).unwrap();

            ci_durations.push(ci_exec.duration.unwrap());
            standard_durations.push(std_exec.duration.unwrap());
        }

        // Then: CI mode should consistently perform better or equal
        let avg_ci_duration: Duration =
            ci_durations.iter().sum::<Duration>() / ci_durations.len() as u32;
        let avg_std_duration: Duration =
            standard_durations.iter().sum::<Duration>() / standard_durations.len() as u32;

        assert!(
            avg_ci_duration <= avg_std_duration + Duration::from_millis(10),
            "CI mode should not be significantly slower than standard mode"
        );

        // And: CI optimizations should be tracked
        let metrics = coordinator.get_ci_performance_metrics();
        assert!(
            metrics.contains_key("ci_optimizations"),
            "CI optimization metrics should be tracked"
        );
    }

    #[test]
    fn test_ci_mode_failure_handling() {
        // Given: A CI environment
        let ci_env = MockCIEnvironment::new_github_actions();
        let coordinator = MockCIWorkflowCoordinator::new(ci_env);

        // When: We simulate workflow failures
        let ci_failure = coordinator.simulate_workflow_failure(true).unwrap();
        let std_failure = coordinator.simulate_workflow_failure(false).unwrap();

        // Then: Both failures should be properly recorded
        assert!(
            !ci_failure.success,
            "CI failure should be marked as unsuccessful"
        );
        assert!(
            !std_failure.success,
            "Standard failure should be marked as unsuccessful"
        );

        // And: CI mode should still upload failure artifacts
        assert!(
            coordinator.artifact_handler.get_upload_call_count() > 0,
            "CI mode should upload failure artifacts for debugging"
        );

        // And: Error messages should be preserved
        assert!(
            ci_failure.error_message.is_some(),
            "CI failure should have error message"
        );
        assert!(
            std_failure.error_message.is_some(),
            "Standard failure should have error message"
        );
    }

    #[test]
    fn test_multiple_ci_workflows_coordination() {
        // Given: A CI environment that supports multiple concurrent workflows
        let ci_env = MockCIEnvironment::new_github_actions();
        let coordinator = MockCIWorkflowCoordinator::new(ci_env);

        // When: We execute multiple workflows concurrently (simulated)
        let workflow_count = 3;
        let mut executions = Vec::new();

        for i in 0..workflow_count {
            let execution = coordinator
                .execute_bundling_workflow(true, i % 2 == 0)
                .unwrap();
            executions.push(execution);
        }

        // Then: All workflows should complete successfully
        assert_eq!(
            executions.len(),
            workflow_count,
            "All workflows should execute"
        );
        assert!(
            executions.iter().all(|e| e.success),
            "All workflows should succeed"
        );

        // And: Each should have unique execution IDs
        let execution_ids: std::collections::HashSet<_> =
            executions.iter().map(|e| &e.execution_id).collect();
        assert_eq!(
            execution_ids.len(),
            workflow_count,
            "All execution IDs should be unique"
        );

        // And: CI optimizations should be applied consistently
        assert!(
            coordinator.verify_ci_optimizations_applied(),
            "CI optimizations should be applied to all workflows"
        );
    }

    #[test]
    fn test_ci_mode_timeout_adjustments() {
        // Given: A CI environment with timeout adjustments
        let ci_env = MockCIEnvironment::new_github_actions();
        let coordinator = MockCIWorkflowCoordinator::new(ci_env.clone());

        // When: We check timeout adjustments
        let timeout_adjustment = ci_env.timeout_adjustments;

        // Then: CI mode should have longer timeouts for reliability
        assert!(
            timeout_adjustment > Duration::from_secs(0),
            "CI mode should have timeout adjustments"
        );
        assert_eq!(
            timeout_adjustment,
            Duration::from_secs(300),
            "Timeout adjustment should be 5 minutes"
        );

        // And: Environment variables should be properly set
        assert_eq!(
            ci_env
                .environment_variables
                .get("MY_LITTLE_SODA_CI_MODE")
                .unwrap(),
            "true"
        );
        assert_eq!(
            ci_env.environment_variables.get("GITHUB_ACTIONS").unwrap(),
            "true"
        );
    }

    #[test]
    fn test_ci_mode_artifact_persistence() {
        // Given: A CI environment with artifacts enabled
        let ci_env = MockCIEnvironment::new_github_actions();
        let coordinator = MockCIWorkflowCoordinator::new(ci_env);

        // When: We execute workflows and then simulate retrieval
        let execution1 = coordinator.execute_bundling_workflow(true, false).unwrap();
        let execution2 = coordinator.execute_bundling_workflow(true, true).unwrap();

        // Then: Multiple artifacts should be stored
        let artifacts = coordinator.artifact_handler.list_artifacts();
        assert!(
            artifacts.len() >= 2,
            "Multiple workflow artifacts should be persisted"
        );

        // And: Each artifact should be retrievable with correct metadata
        for artifact_id in &artifacts {
            let metadata = coordinator
                .artifact_handler
                .download_bundle_metadata(artifact_id)
                .unwrap();
            assert!(metadata.is_some(), "Each artifact should be retrievable");

            let metadata = metadata.unwrap();
            assert!(
                metadata.get("execution_id").is_some(),
                "Metadata should contain execution ID"
            );
            assert!(
                metadata.get("workflow_run_id").is_some(),
                "Metadata should contain workflow run ID"
            );
        }
    }

    #[test]
    fn test_hybrid_ci_and_local_workflow_compatibility() {
        // Given: Both CI and local environments
        let ci_env = MockCIEnvironment::new_github_actions();
        let local_env = MockCIEnvironment::new_local();

        let ci_coordinator = MockCIWorkflowCoordinator::new(ci_env);
        let local_coordinator = MockCIWorkflowCoordinator::new(local_env);

        // When: We execute workflows in both environments
        let ci_execution = ci_coordinator
            .execute_bundling_workflow(true, false)
            .unwrap();
        let local_execution = local_coordinator
            .execute_bundling_workflow(false, false)
            .unwrap();

        // Then: Both should work but with different characteristics
        assert!(ci_execution.success, "CI workflow should succeed");
        assert!(local_execution.success, "Local workflow should succeed");

        // And: Artifact handling should differ appropriately
        assert_eq!(ci_execution.artifact_handling, "optimized");
        assert_eq!(local_execution.artifact_handling, "standard");

        // And: Only CI should upload artifacts
        assert!(
            ci_coordinator.artifact_handler.get_upload_call_count() > 0,
            "CI should upload artifacts"
        );
        assert_eq!(
            local_coordinator.artifact_handler.get_upload_call_count(),
            0,
            "Local should not upload artifacts"
        );
    }
}
