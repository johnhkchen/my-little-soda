/// Integration utilities for connecting repository state fixtures with init command testing
/// 
/// This module provides helpers to test the init command against different repository states
/// using the fixtures created in repository_states.rs. It bridges the gap between the fixtures
/// and the actual init command implementation.

use super::repository_states::{RepositoryStateFixture, RepositoryFixtureLoader, InitBehaviorExpectation};
use super::test_harness::TestHarness;
use crate::cli::commands::init::InitCommand;
use crate::fs::{FileSystemOperations, RealFileSystemOperations};
use anyhow::Result;
use std::path::Path;
use std::sync::Arc;
use tempfile::TempDir;

/// Integration helper that sets up a complete test environment for init command testing
pub struct InitCommandTestEnvironment {
    pub temp_dir: TempDir,
    pub fixture: RepositoryStateFixture,
    pub expected_behavior: InitBehaviorExpectation,
}

impl InitCommandTestEnvironment {
    /// Create a new test environment using a specific fixture by name
    pub fn from_fixture_name(fixture_name: &str) -> Result<Self> {
        let fixture = RepositoryFixtureLoader::load_fixture(fixture_name)
            .ok_or_else(|| anyhow::anyhow!("Fixture '{}' not found", fixture_name))?;
        
        let temp_dir = fixture.create_temp_repository()?;
        let expected_behavior = fixture.expected_init_behavior();
        
        Ok(Self {
            temp_dir,
            fixture,
            expected_behavior,
        })
    }
    
    /// Create a new test environment using a fixture instance
    pub fn from_fixture(fixture: RepositoryStateFixture) -> Result<Self> {
        let temp_dir = fixture.create_temp_repository()?;
        let expected_behavior = fixture.expected_init_behavior();
        
        Ok(Self {
            temp_dir,
            fixture,
            expected_behavior,
        })
    }
    
    /// Get the path to the temporary repository
    pub fn path(&self) -> &Path {
        self.temp_dir.path()
    }
    
    /// Create an InitCommand configured for this test environment
    pub fn create_init_command(&self, agents: u32, force: bool, dry_run: bool) -> InitCommand {
        let fs_ops: Arc<dyn FileSystemOperations> = Arc::new(RealFileSystemOperations::new());
        InitCommand::new(agents, None, force, dry_run, fs_ops)
    }
    
    /// Run init command and validate the result matches expected behavior
    pub async fn run_and_validate_init(&self, agents: u32, force: bool, dry_run: bool) -> Result<InitCommandTestResult> {
        let init_command = self.create_init_command(agents, force, dry_run);
        
        // Change to the test repository directory for the duration of the test
        let original_dir = std::env::current_dir()?;
        std::env::set_current_dir(self.path())?;
        
        let result = init_command.execute().await;
        
        // Restore original directory
        std::env::set_current_dir(original_dir)?;
        
        let test_result = InitCommandTestResult {
            success: result.is_ok(),
            error_message: result.as_ref().err().map(|e| e.to_string()),
            expected_success: if force { true } else { self.expected_behavior.should_succeed_without_force },
        };
        
        Ok(test_result)
    }
    
    /// Validate that the repository state matches expectations after init
    pub fn validate_post_init_state(&self, dry_run: bool) -> Result<PostInitValidationResult> {
        let mut validation = PostInitValidationResult::new();
        
        // Check if config file was created (unless dry run)
        let config_exists = self.path().join("clambake.toml").exists();
        validation.config_created = config_exists;
        validation.expected_config_creation = self.expected_behavior.should_create_config && !dry_run;
        
        // Check if .clambake directory was created (unless dry run)
        let clambake_dir_exists = self.path().join(".clambake").exists();
        validation.directories_created = clambake_dir_exists;
        validation.expected_directory_creation = self.expected_behavior.should_create_directories && !dry_run;
        
        // Validate expectations match reality
        validation.config_expectation_met = validation.config_created == validation.expected_config_creation;
        validation.directory_expectation_met = validation.directories_created == validation.expected_directory_creation;
        
        Ok(validation)
    }
    
    /// Helper to check if the repository has specific files
    pub fn has_file(&self, relative_path: &str) -> bool {
        self.path().join(relative_path).exists()
    }
    
    /// Helper to read file contents from the test repository
    pub fn read_file(&self, relative_path: &str) -> Result<String> {
        let file_path = self.path().join(relative_path);
        std::fs::read_to_string(file_path).map_err(|e| anyhow::anyhow!("Failed to read {}: {}", relative_path, e))
    }
}

/// Result of running an init command test
#[derive(Debug, Clone)]
pub struct InitCommandTestResult {
    pub success: bool,
    pub error_message: Option<String>,
    pub expected_success: bool,
}

impl InitCommandTestResult {
    /// Check if the test result matches expectations
    pub fn matches_expectation(&self) -> bool {
        self.success == self.expected_success
    }
    
    /// Get a description of any expectation mismatch
    pub fn expectation_mismatch_description(&self) -> Option<String> {
        if self.matches_expectation() {
            None
        } else {
            Some(format!(
                "Expected success: {}, actual success: {}, error: {:?}",
                self.expected_success, self.success, self.error_message
            ))
        }
    }
}

/// Result of validating repository state after init command execution
#[derive(Debug, Clone)]
pub struct PostInitValidationResult {
    pub config_created: bool,
    pub expected_config_creation: bool,
    pub config_expectation_met: bool,
    
    pub directories_created: bool,
    pub expected_directory_creation: bool,
    pub directory_expectation_met: bool,
}

impl PostInitValidationResult {
    pub fn new() -> Self {
        Self {
            config_created: false,
            expected_config_creation: false,
            config_expectation_met: true,
            directories_created: false,
            expected_directory_creation: false,
            directory_expectation_met: true,
        }
    }
    
    /// Check if all validations passed
    pub fn all_expectations_met(&self) -> bool {
        self.config_expectation_met && self.directory_expectation_met
    }
    
    /// Get a summary of any failed validations
    pub fn validation_failures(&self) -> Vec<String> {
        let mut failures = Vec::new();
        
        if !self.config_expectation_met {
            failures.push(format!(
                "Config creation mismatch: expected {}, got {}",
                self.expected_config_creation, self.config_created
            ));
        }
        
        if !self.directory_expectation_met {
            failures.push(format!(
                "Directory creation mismatch: expected {}, got {}",
                self.expected_directory_creation, self.directories_created
            ));
        }
        
        failures
    }
}

/// Batch runner for testing init command against all fixtures
pub struct InitCommandBatchTester;

impl InitCommandBatchTester {
    /// Run init command tests against all available fixtures
    pub async fn test_all_fixtures(agents: u32, force: bool, dry_run: bool) -> Result<BatchTestResults> {
        let fixtures = RepositoryFixtureLoader::load_init_command_fixtures();
        let mut results = Vec::new();
        
        for fixture in fixtures {
            let env = InitCommandTestEnvironment::from_fixture(fixture)?;
            let test_result = env.run_and_validate_init(agents, force, dry_run).await?;
            let post_init_validation = env.validate_post_init_state(dry_run)?;
            
            results.push(FixtureTestResult {
                fixture_name: env.fixture.name.clone(),
                fixture_description: env.fixture.description.clone(),
                command_result: test_result,
                post_init_validation,
            });
        }
        
        Ok(BatchTestResults { results })
    }
    
    /// Test a specific scenario across all fixtures (e.g., force mode, dry run)
    pub async fn test_scenario_across_fixtures(scenario: TestScenario) -> Result<BatchTestResults> {
        Self::test_all_fixtures(scenario.agents, scenario.force, scenario.dry_run).await
    }
}

/// Configuration for a specific test scenario
#[derive(Debug, Clone)]
pub struct TestScenario {
    pub agents: u32,
    pub force: bool,
    pub dry_run: bool,
    pub description: String,
}

impl TestScenario {
    pub fn normal_init() -> Self {
        Self {
            agents: 1,
            force: false,
            dry_run: false,
            description: "Normal init without force".to_string(),
        }
    }
    
    pub fn force_init() -> Self {
        Self {
            agents: 1,
            force: true,
            dry_run: false,
            description: "Force init to overwrite existing config".to_string(),
        }
    }
    
    pub fn dry_run() -> Self {
        Self {
            agents: 1,
            force: false,
            dry_run: true,
            description: "Dry run to validate without changes".to_string(),
        }
    }
    
    pub fn multi_agent() -> Self {
        Self {
            agents: 4,
            force: false,
            dry_run: false,
            description: "Multi-agent setup".to_string(),
        }
    }
}

/// Results from testing a single fixture
#[derive(Debug)]
pub struct FixtureTestResult {
    pub fixture_name: String,
    pub fixture_description: String,
    pub command_result: InitCommandTestResult,
    pub post_init_validation: PostInitValidationResult,
}

impl FixtureTestResult {
    /// Check if this fixture test passed completely
    pub fn passed(&self) -> bool {
        self.command_result.matches_expectation() && 
        self.post_init_validation.all_expectations_met()
    }
    
    /// Get a summary of any test failures
    pub fn failure_summary(&self) -> Option<String> {
        if self.passed() {
            return None;
        }
        
        let mut issues = Vec::new();
        
        if let Some(cmd_issue) = self.command_result.expectation_mismatch_description() {
            issues.push(format!("Command execution: {}", cmd_issue));
        }
        
        for validation_failure in self.post_init_validation.validation_failures() {
            issues.push(format!("Post-init validation: {}", validation_failure));
        }
        
        Some(issues.join("; "))
    }
}

/// Results from testing multiple fixtures
pub struct BatchTestResults {
    pub results: Vec<FixtureTestResult>,
}

impl BatchTestResults {
    /// Get the number of fixtures that passed all tests
    pub fn passed_count(&self) -> usize {
        self.results.iter().filter(|r| r.passed()).count()
    }
    
    /// Get the number of fixtures that failed any tests
    pub fn failed_count(&self) -> usize {
        self.results.len() - self.passed_count()
    }
    
    /// Check if all fixture tests passed
    pub fn all_passed(&self) -> bool {
        self.failed_count() == 0
    }
    
    /// Get summaries of all failed tests
    pub fn failure_summaries(&self) -> Vec<(String, String)> {
        self.results
            .iter()
            .filter_map(|result| {
                result.failure_summary().map(|summary| {
                    (result.fixture_name.clone(), summary)
                })
            })
            .collect()
    }
}

/// Assertion helpers for common test patterns
pub mod assertions {
    use super::*;
    
    /// Assert that init command should succeed for clean repositories
    pub fn assert_init_succeeds_for_clean_repo(result: &InitCommandTestResult, fixture_name: &str) {
        assert!(
            result.success,
            "Init should succeed for clean repository fixture '{}', but got error: {:?}",
            fixture_name,
            result.error_message
        );
    }
    
    /// Assert that init command should fail for problematic repositories without force
    pub fn assert_init_fails_without_force(result: &InitCommandTestResult, fixture_name: &str) {
        assert!(
            !result.success,
            "Init should fail for problematic repository fixture '{}' without force flag",
            fixture_name
        );
    }
    
    /// Assert that init command should succeed with force flag
    pub fn assert_init_succeeds_with_force(result: &InitCommandTestResult, fixture_name: &str) {
        assert!(
            result.success,
            "Init should succeed for any repository fixture '{}' with force flag, but got error: {:?}",
            fixture_name,
            result.error_message
        );
    }
    
    /// Assert that the test result matches the fixture's expected behavior
    pub fn assert_result_matches_expectation(result: &InitCommandTestResult, fixture_name: &str) {
        assert!(
            result.matches_expectation(),
            "Result should match expectation for fixture '{}': {}",
            fixture_name,
            result.expectation_mismatch_description().unwrap_or_default()
        );
    }
    
    /// Assert that post-init validation passes
    pub fn assert_post_init_validation_passes(validation: &PostInitValidationResult, fixture_name: &str) {
        assert!(
            validation.all_expectations_met(),
            "Post-init validation should pass for fixture '{}': {:?}",
            fixture_name,
            validation.validation_failures()
        );
    }
    
    /// Assert that a fixture test result is completely successful
    pub fn assert_fixture_test_passes(result: &FixtureTestResult) {
        assert!(
            result.passed(),
            "Fixture test '{}' should pass completely: {}",
            result.fixture_name,
            result.failure_summary().unwrap_or_default()
        );
    }
    
    /// Assert that batch test results show all fixtures passing
    pub fn assert_all_fixtures_pass(results: &BatchTestResults, scenario_description: &str) {
        assert!(
            results.all_passed(),
            "All fixtures should pass for scenario '{}'. Failed: {}/{} fixtures. Failures: {:?}",
            scenario_description,
            results.failed_count(),
            results.results.len(),
            results.failure_summaries()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::MockFileSystemOperations;
    use mockall::predicate::*;
    use std::process::Output;
    
    fn create_successful_exit_status() -> std::process::ExitStatus {
        std::process::Command::new("true").status().unwrap()
    }
    
    #[tokio::test]
    async fn test_init_environment_creation_from_fixture_name() {
        let env = InitCommandTestEnvironment::from_fixture_name("empty_repository");
        assert!(env.is_ok());
        
        let env = env.unwrap();
        assert_eq!(env.fixture.name, "empty_repository");
        assert!(env.path().exists());
        assert!(env.has_file("README.md"));
    }
    
    #[tokio::test]
    async fn test_init_environment_creation_from_fixture() {
        let fixture = RepositoryStateFixture::empty_repository();
        let env = InitCommandTestEnvironment::from_fixture(fixture);
        assert!(env.is_ok());
        
        let env = env.unwrap();
        assert_eq!(env.fixture.name, "empty_repository");
        assert!(env.path().exists());
    }
    
    #[test]
    fn test_init_environment_file_helpers() {
        let env = InitCommandTestEnvironment::from_fixture_name("empty_repository").unwrap();
        
        // Test file existence check
        assert!(env.has_file("README.md"));
        assert!(env.has_file(".gitignore"));
        assert!(!env.has_file("nonexistent.txt"));
        
        // Test file reading
        let readme_content = env.read_file("README.md");
        assert!(readme_content.is_ok());
        assert!(readme_content.unwrap().contains("Test Repository"));
    }
    
    #[tokio::test]
    async fn test_batch_tester_scenarios() {
        let normal_scenario = TestScenario::normal_init();
        assert_eq!(normal_scenario.agents, 1);
        assert!(!normal_scenario.force);
        assert!(!normal_scenario.dry_run);
        
        let force_scenario = TestScenario::force_init();
        assert!(force_scenario.force);
        
        let dry_run_scenario = TestScenario::dry_run();
        assert!(dry_run_scenario.dry_run);
        
        let multi_agent_scenario = TestScenario::multi_agent();
        assert_eq!(multi_agent_scenario.agents, 4);
    }
    
    #[test]
    fn test_post_init_validation_result() {
        let mut validation = PostInitValidationResult::new();
        
        // Initially should pass (no expectations set)
        assert!(validation.all_expectations_met());
        assert!(validation.validation_failures().is_empty());
        
        // Set up a failure scenario
        validation.config_created = true;
        validation.expected_config_creation = false;
        validation.config_expectation_met = false;
        
        assert!(!validation.all_expectations_met());
        assert_eq!(validation.validation_failures().len(), 1);
        assert!(validation.validation_failures()[0].contains("Config creation mismatch"));
    }
    
    #[test]
    fn test_init_command_test_result() {
        let success_result = InitCommandTestResult {
            success: true,
            error_message: None,
            expected_success: true,
        };
        assert!(success_result.matches_expectation());
        assert!(success_result.expectation_mismatch_description().is_none());
        
        let failure_result = InitCommandTestResult {
            success: false,
            error_message: Some("Test error".to_string()),
            expected_success: true,
        };
        assert!(!failure_result.matches_expectation());
        assert!(failure_result.expectation_mismatch_description().is_some());
    }
    
    #[test]
    fn test_fixture_test_result() {
        let passing_result = FixtureTestResult {
            fixture_name: "test_fixture".to_string(),
            fixture_description: "Test description".to_string(),
            command_result: InitCommandTestResult {
                success: true,
                error_message: None,
                expected_success: true,
            },
            post_init_validation: PostInitValidationResult::new(),
        };
        
        assert!(passing_result.passed());
        assert!(passing_result.failure_summary().is_none());
    }
    
    #[test]
    fn test_batch_test_results() {
        let results = BatchTestResults {
            results: vec![
                FixtureTestResult {
                    fixture_name: "passing".to_string(),
                    fixture_description: "Passing fixture".to_string(),
                    command_result: InitCommandTestResult {
                        success: true,
                        error_message: None,
                        expected_success: true,
                    },
                    post_init_validation: PostInitValidationResult::new(),
                },
            ],
        };
        
        assert_eq!(results.passed_count(), 1);
        assert_eq!(results.failed_count(), 0);
        assert!(results.all_passed());
        assert!(results.failure_summaries().is_empty());
    }
}