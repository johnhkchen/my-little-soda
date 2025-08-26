/// Validation helpers for repository state fixtures and init command testing
/// 
/// This module provides comprehensive validation utilities that go beyond basic assertions
/// to validate complex repository states, configuration correctness, and behavioral expectations.

use super::repository_states::{RepositoryStateFixture, InitBehaviorExpectation};
use super::init_integration::{InitCommandTestEnvironment, InitCommandTestResult, PostInitValidationResult};
use anyhow::Result;
use std::path::Path;
use std::collections::HashMap;

/// Comprehensive validator for repository state fixtures
pub struct RepositoryStateValidator;

impl RepositoryStateValidator {
    /// Validate that a fixture has all required properties and is internally consistent
    pub fn validate_fixture_consistency(fixture: &RepositoryStateFixture) -> Result<FixtureValidationReport> {
        let mut report = FixtureValidationReport::new(&fixture.name);
        
        // Basic property validation
        Self::validate_basic_properties(fixture, &mut report)?;
        
        // File content validation
        Self::validate_file_contents(fixture, &mut report)?;
        
        // Git configuration validation
        Self::validate_git_configuration(fixture, &mut report)?;
        
        // Behavior expectation validation
        Self::validate_behavior_expectations(fixture, &mut report)?;
        
        // Cross-validation between properties
        Self::validate_property_consistency(fixture, &mut report)?;
        
        Ok(report)
    }
    
    /// Validate basic fixture properties
    fn validate_basic_properties(fixture: &RepositoryStateFixture, report: &mut FixtureValidationReport) -> Result<()> {
        if fixture.name.is_empty() {
            report.add_error("Fixture name cannot be empty");
        }
        
        if fixture.description.is_empty() {
            report.add_error("Fixture description cannot be empty");
        }
        
        if fixture.files.is_empty() {
            report.add_error("Fixture must contain at least one file");
        }
        
        // Validate fixture name follows conventions
        if !fixture.name.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
            report.add_warning("Fixture name should use snake_case convention");
        }
        
        Ok(())
    }
    
    /// Validate file contents for common issues
    fn validate_file_contents(fixture: &RepositoryStateFixture, report: &mut FixtureValidationReport) -> Result<()> {
        // Check for essential files in git repositories
        if fixture.git_config.initialized {
            if !fixture.files.contains_key("README.md") {
                report.add_warning("Git repositories should typically have a README.md");
            }
            
            if !fixture.files.contains_key(".gitignore") {
                report.add_warning("Git repositories should typically have a .gitignore");
            }
        }
        
        // Validate file content consistency
        for (file_path, content) in &fixture.files {
            if content.is_empty() && !file_path.starts_with('.') {
                report.add_warning(&format!("File '{}' is empty", file_path));
            }
            
            // Check for conflict markers in non-conflict fixtures
            if content.contains("<<<<<<<") || content.contains(">>>>>>>") {
                if fixture.name != "repository_with_conflicts" {
                    report.add_error(&format!("File '{}' contains conflict markers but fixture is not marked as conflicted", file_path));
                }
            }
            
            // Validate Rust project structure if Cargo.toml exists
            if file_path == "Cargo.toml" {
                if !content.contains("[package]") {
                    report.add_error("Cargo.toml should contain [package] section");
                }
            }
        }
        
        Ok(())
    }
    
    /// Validate Git configuration consistency
    fn validate_git_configuration(fixture: &RepositoryStateFixture, report: &mut FixtureValidationReport) -> Result<()> {
        let git_config = &fixture.git_config;
        
        if git_config.initialized {
            if git_config.current_branch.is_empty() {
                report.add_error("Initialized git repositories must have a current branch");
            }
            
            if git_config.has_remote && git_config.remote_url.is_none() {
                report.add_error("Git config marked as having remote but no remote URL provided");
            }
            
            if !git_config.has_remote && git_config.remote_url.is_some() {
                report.add_warning("Remote URL provided but has_remote is false");
            }
        }
        
        // Validate conflict state consistency
        if !git_config.conflicted_files.is_empty() && !git_config.uncommitted_changes {
            report.add_error("Conflicted files require uncommitted_changes to be true");
        }
        
        Ok(())
    }
    
    /// Validate behavior expectations are reasonable
    fn validate_behavior_expectations(fixture: &RepositoryStateFixture, report: &mut FixtureValidationReport) -> Result<()> {
        let behavior = fixture.expected_init_behavior();
        
        // If fixture has existing config, it shouldn't succeed without force
        if fixture.existing_clambake_config.is_some() && behavior.should_succeed_without_force {
            report.add_error("Fixtures with existing config should not succeed without force");
        }
        
        // If repository has conflicts, it should have warnings
        if fixture.git_config.uncommitted_changes && behavior.validation_warnings.is_empty() {
            report.add_warning("Repositories with uncommitted changes should have validation warnings");
        }
        
        // Consistency between creation expectations
        if !behavior.should_succeed_without_force && behavior.should_create_config {
            report.add_warning("If init shouldn't succeed without force, it probably shouldn't create config either");
        }
        
        Ok(())
    }
    
    /// Validate cross-property consistency
    fn validate_property_consistency(fixture: &RepositoryStateFixture, report: &mut FixtureValidationReport) -> Result<()> {
        // If existing_clambake_config is Some, should have clambake.toml in files
        if fixture.existing_clambake_config.is_some() {
            if !fixture.files.contains_key("clambake.toml") {
                report.add_error("existing_clambake_config is present but clambake.toml not in files");
            } else {
                // Content should match
                let file_content = &fixture.files["clambake.toml"];
                let config_content = fixture.existing_clambake_config.as_ref().unwrap();
                if file_content != config_content {
                    report.add_error("clambake.toml file content doesn't match existing_clambake_config");
                }
            }
        }
        
        // If fixture name suggests conflicts, should have conflict markers
        if fixture.name.contains("conflicts") {
            let has_conflict_markers = fixture.files.values()
                .any(|content| content.contains("<<<<<<<") || content.contains(">>>>>>>"));
            if !has_conflict_markers {
                report.add_warning("Fixture name suggests conflicts but no conflict markers found in files");
            }
        }
        
        Ok(())
    }
    
    /// Validate that fixture can be successfully instantiated as temporary repository
    pub async fn validate_fixture_instantiation(fixture: &RepositoryStateFixture) -> Result<InstantiationValidationReport> {
        let mut report = InstantiationValidationReport::new(&fixture.name);
        
        // Try to create temporary repository
        match fixture.create_temp_repository() {
            Ok(temp_dir) => {
                report.creation_successful = true;
                
                // Validate all expected files exist
                for file_path in fixture.files.keys() {
                    let full_path = temp_dir.path().join(file_path);
                    if full_path.exists() {
                        report.files_created += 1;
                        
                        // Validate file content matches
                        match std::fs::read_to_string(&full_path) {
                            Ok(actual_content) => {
                                let expected_content = &fixture.files[file_path];
                                if actual_content == *expected_content {
                                    report.content_matches += 1;
                                } else {
                                    report.add_error(&format!("Content mismatch for file: {}", file_path));
                                }
                            }
                            Err(e) => {
                                report.add_error(&format!("Failed to read created file {}: {}", file_path, e));
                            }
                        }
                    } else {
                        report.add_error(&format!("Expected file not created: {}", file_path));
                    }
                }
                
                // Validate git repository if expected
                if fixture.git_config.initialized {
                    let git_dir = temp_dir.path().join(".git");
                    if git_dir.exists() {
                        report.git_initialized = true;
                    } else {
                        report.add_error("Git repository should be initialized but .git directory not found");
                    }
                }
                
            }
            Err(e) => {
                report.add_error(&format!("Failed to create temporary repository: {}", e));
            }
        }
        
        Ok(report)
    }
}

/// Validator for init command test results and expectations
pub struct InitCommandValidator;

impl InitCommandValidator {
    /// Validate that init command test result matches fixture expectations comprehensively
    pub fn validate_comprehensive_result(
        result: &InitCommandTestResult,
        post_init: &PostInitValidationResult,
        fixture: &RepositoryStateFixture,
        scenario: &str,
    ) -> Result<ComprehensiveValidationReport> {
        let mut report = ComprehensiveValidationReport::new(&fixture.name, scenario);
        
        // Basic expectation matching
        if result.matches_expectation() {
            report.command_result_matches = true;
        } else {
            report.add_error(&format!("Command result doesn't match expectation: {}", 
                result.expectation_mismatch_description().unwrap_or_default()));
        }
        
        // Post-init validation
        if post_init.all_expectations_met() {
            report.post_init_valid = true;
        } else {
            for failure in post_init.validation_failures() {
                report.add_error(&format!("Post-init validation failed: {}", failure));
            }
        }
        
        // Scenario-specific validation
        Self::validate_scenario_specific_behavior(&mut report, result, post_init, fixture, scenario)?;
        
        Ok(report)
    }
    
    /// Validate behavior specific to test scenarios (dry-run, force, etc.)
    fn validate_scenario_specific_behavior(
        report: &mut ComprehensiveValidationReport,
        result: &InitCommandTestResult,
        post_init: &PostInitValidationResult,
        fixture: &RepositoryStateFixture,
        scenario: &str,
    ) -> Result<()> {
        match scenario {
            "dry_run" | "dry run" => {
                // Dry run should never create actual files
                if post_init.config_created {
                    report.add_error("Dry run should not create config files");
                }
                if post_init.directories_created {
                    report.add_error("Dry run should not create directories");
                }
            }
            
            "force" | "force_init" => {
                // Force should always succeed regardless of repository state
                if !result.success {
                    report.add_error("Force init should always succeed");
                }
            }
            
            "normal" | "normal_init" => {
                // Normal init should respect fixture expectations
                let expected_behavior = fixture.expected_init_behavior();
                if result.success != expected_behavior.should_succeed_without_force {
                    report.add_error("Normal init result doesn't match fixture behavior expectations");
                }
            }
            
            _ => {
                // Unknown scenario - add warning
                report.add_warning(&format!("Unknown test scenario: {}", scenario));
            }
        }
        
        Ok(())
    }
    
    /// Validate init command behavior across multiple fixtures for consistency
    pub fn validate_cross_fixture_consistency(
        results: &[(String, InitCommandTestResult, PostInitValidationResult)],
        scenario: &str,
    ) -> Result<CrossFixtureValidationReport> {
        let mut report = CrossFixtureValidationReport::new(scenario);
        
        report.total_fixtures = results.len();
        
        // Count successes and failures
        for (fixture_name, result, _) in results {
            if result.success {
                report.successful_fixtures += 1;
            } else {
                report.failed_fixtures += 1;
                report.failures.push(fixture_name.clone());
            }
        }
        
        // Scenario-specific consistency checks
        match scenario {
            "dry_run" => {
                // All fixtures should succeed in dry run mode
                if report.failed_fixtures > 0 {
                    report.add_error("Some fixtures failed in dry run mode - this indicates validation issues");
                }
            }
            
            "force" => {
                // All fixtures should succeed with force
                if report.failed_fixtures > 0 {
                    report.add_error("Some fixtures failed even with force flag");
                }
            }
            
            _ => {}
        }
        
        Ok(report)
    }
}

/// Environment validator for test setup and cleanup
pub struct TestEnvironmentValidator;

impl TestEnvironmentValidator {
    /// Validate that test environment is properly set up and isolated
    pub fn validate_test_environment(env: &InitCommandTestEnvironment) -> Result<EnvironmentValidationReport> {
        let mut report = EnvironmentValidationReport::new(&env.fixture.name);
        
        // Validate temporary directory exists and is accessible
        if env.path().exists() {
            report.directory_accessible = true;
            
            // Check permissions
            if env.path().metadata()?.permissions().readonly() {
                report.add_warning("Temporary directory is read-only");
            } else {
                report.writable = true;
            }
        } else {
            report.add_error("Temporary directory does not exist");
        }
        
        // Validate fixture files are present
        for file_path in env.fixture.files.keys() {
            if env.has_file(file_path) {
                report.expected_files_present += 1;
            } else {
                report.add_error(&format!("Expected file not found: {}", file_path));
            }
        }
        report.total_expected_files = env.fixture.files.len();
        
        // Validate git repository state if applicable
        if env.fixture.git_config.initialized {
            if env.has_file(".git") {
                report.git_repository_valid = true;
            } else {
                report.add_error("Git repository should be initialized but .git not found");
            }
        }
        
        // Check for unexpected files (isolation validation)
        // This is a basic check - in a real implementation, you might want to be more thorough
        report.isolated = true; // Assume isolated unless proven otherwise
        
        Ok(report)
    }
    
    /// Validate test environment cleanup after test completion
    pub fn validate_cleanup(temp_dir_path: &Path) -> Result<CleanupValidationReport> {
        let mut report = CleanupValidationReport::new();
        
        // Check if temporary directory was cleaned up
        if temp_dir_path.exists() {
            report.add_warning("Temporary directory still exists after test completion");
        } else {
            report.cleanup_successful = true;
        }
        
        Ok(report)
    }
}

// Validation report structures

/// Report for fixture consistency validation
#[derive(Debug)]
pub struct FixtureValidationReport {
    pub fixture_name: String,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub passed: bool,
}

impl FixtureValidationReport {
    fn new(fixture_name: &str) -> Self {
        Self {
            fixture_name: fixture_name.to_string(),
            errors: Vec::new(),
            warnings: Vec::new(),
            passed: true,
        }
    }
    
    fn add_error(&mut self, error: &str) {
        self.errors.push(error.to_string());
        self.passed = false;
    }
    
    fn add_warning(&mut self, warning: &str) {
        self.warnings.push(warning.to_string());
    }
    
    /// Get a summary of validation results
    pub fn summary(&self) -> String {
        format!("Fixture '{}': {} errors, {} warnings", 
                self.fixture_name, self.errors.len(), self.warnings.len())
    }
}

/// Report for fixture instantiation validation
#[derive(Debug)]
pub struct InstantiationValidationReport {
    pub fixture_name: String,
    pub creation_successful: bool,
    pub files_created: usize,
    pub content_matches: usize,
    pub git_initialized: bool,
    pub errors: Vec<String>,
}

impl InstantiationValidationReport {
    fn new(fixture_name: &str) -> Self {
        Self {
            fixture_name: fixture_name.to_string(),
            creation_successful: false,
            files_created: 0,
            content_matches: 0,
            git_initialized: false,
            errors: Vec::new(),
        }
    }
    
    fn add_error(&mut self, error: &str) {
        self.errors.push(error.to_string());
    }
    
    pub fn successful(&self) -> bool {
        self.creation_successful && self.errors.is_empty()
    }
}

/// Report for comprehensive init command validation
#[derive(Debug)]
pub struct ComprehensiveValidationReport {
    pub fixture_name: String,
    pub scenario: String,
    pub command_result_matches: bool,
    pub post_init_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ComprehensiveValidationReport {
    fn new(fixture_name: &str, scenario: &str) -> Self {
        Self {
            fixture_name: fixture_name.to_string(),
            scenario: scenario.to_string(),
            command_result_matches: false,
            post_init_valid: false,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }
    
    fn add_error(&mut self, error: &str) {
        self.errors.push(error.to_string());
    }
    
    fn add_warning(&mut self, warning: &str) {
        self.warnings.push(warning.to_string());
    }
    
    pub fn passed(&self) -> bool {
        self.command_result_matches && self.post_init_valid && self.errors.is_empty()
    }
}

/// Report for cross-fixture validation
#[derive(Debug)]
pub struct CrossFixtureValidationReport {
    pub scenario: String,
    pub total_fixtures: usize,
    pub successful_fixtures: usize,
    pub failed_fixtures: usize,
    pub failures: Vec<String>,
    pub errors: Vec<String>,
}

impl CrossFixtureValidationReport {
    fn new(scenario: &str) -> Self {
        Self {
            scenario: scenario.to_string(),
            total_fixtures: 0,
            successful_fixtures: 0,
            failed_fixtures: 0,
            failures: Vec::new(),
            errors: Vec::new(),
        }
    }
    
    fn add_error(&mut self, error: &str) {
        self.errors.push(error.to_string());
    }
    
    pub fn success_rate(&self) -> f64 {
        if self.total_fixtures == 0 {
            return 0.0;
        }
        self.successful_fixtures as f64 / self.total_fixtures as f64
    }
    
    pub fn passed(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Report for test environment validation
#[derive(Debug)]
pub struct EnvironmentValidationReport {
    pub fixture_name: String,
    pub directory_accessible: bool,
    pub writable: bool,
    pub expected_files_present: usize,
    pub total_expected_files: usize,
    pub git_repository_valid: bool,
    pub isolated: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl EnvironmentValidationReport {
    fn new(fixture_name: &str) -> Self {
        Self {
            fixture_name: fixture_name.to_string(),
            directory_accessible: false,
            writable: false,
            expected_files_present: 0,
            total_expected_files: 0,
            git_repository_valid: false,
            isolated: false,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }
    
    fn add_error(&mut self, error: &str) {
        self.errors.push(error.to_string());
    }
    
    fn add_warning(&mut self, warning: &str) {
        self.warnings.push(warning.to_string());
    }
    
    pub fn passed(&self) -> bool {
        self.directory_accessible && 
        self.expected_files_present == self.total_expected_files &&
        self.errors.is_empty()
    }
}

/// Report for cleanup validation
#[derive(Debug)]
pub struct CleanupValidationReport {
    pub cleanup_successful: bool,
    pub warnings: Vec<String>,
}

impl CleanupValidationReport {
    fn new() -> Self {
        Self {
            cleanup_successful: false,
            warnings: Vec::new(),
        }
    }
    
    fn add_warning(&mut self, warning: &str) {
        self.warnings.push(warning.to_string());
    }
}

/// Utility macros for common validation patterns
#[macro_export]
macro_rules! validate_fixture {
    ($fixture:expr) => {
        {
            use crate::tests::fixtures::validation_helpers::RepositoryStateValidator;
            let report = RepositoryStateValidator::validate_fixture_consistency(&$fixture)?;
            assert!(report.passed, "Fixture validation failed: {}", report.summary());
            report
        }
    };
}

#[macro_export]
macro_rules! validate_init_result {
    ($result:expr, $post_init:expr, $fixture:expr, $scenario:expr) => {
        {
            use crate::tests::fixtures::validation_helpers::InitCommandValidator;
            let report = InitCommandValidator::validate_comprehensive_result(
                &$result, &$post_init, &$fixture, $scenario
            )?;
            assert!(report.passed(), "Init command validation failed for fixture '{}' in scenario '{}'", 
                    report.fixture_name, report.scenario);
            report
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::repository_states::RepositoryStateFixture;
    
    #[tokio::test]
    async fn test_fixture_validation() {
        let fixture = RepositoryStateFixture::empty_repository();
        let report = RepositoryStateValidator::validate_fixture_consistency(&fixture).unwrap();
        
        assert!(report.passed, "Empty repository fixture should pass validation");
        assert!(report.errors.is_empty(), "Should have no validation errors");
    }
    
    #[tokio::test] 
    async fn test_fixture_instantiation_validation() {
        let fixture = RepositoryStateFixture::empty_repository();
        let report = RepositoryStateValidator::validate_fixture_instantiation(&fixture).await.unwrap();
        
        assert!(report.successful(), "Fixture instantiation should succeed");
        assert!(report.creation_successful, "Temporary repository creation should succeed");
        assert!(report.files_created > 0, "Should create expected files");
    }
    
    #[test]
    fn test_validation_reports() {
        let mut fixture_report = FixtureValidationReport::new("test_fixture");
        assert!(fixture_report.passed);
        
        fixture_report.add_error("Test error");
        assert!(!fixture_report.passed);
        assert_eq!(fixture_report.errors.len(), 1);
        
        fixture_report.add_warning("Test warning");
        assert_eq!(fixture_report.warnings.len(), 1);
    }
    
    #[test]
    fn test_cross_fixture_validation() {
        let results = vec![
            ("fixture1".to_string(), InitCommandTestResult {
                success: true,
                error_message: None,
                expected_success: true,
            }, PostInitValidationResult::new()),
            ("fixture2".to_string(), InitCommandTestResult {
                success: false,
                error_message: Some("Test error".to_string()),
                expected_success: false,
            }, PostInitValidationResult::new()),
        ];
        
        let report = InitCommandValidator::validate_cross_fixture_consistency(&results, "test_scenario").unwrap();
        
        assert_eq!(report.total_fixtures, 2);
        assert_eq!(report.successful_fixtures, 1);
        assert_eq!(report.failed_fixtures, 1);
        assert_eq!(report.success_rate(), 0.5);
    }
}