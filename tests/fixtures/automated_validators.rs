/// Automated validation checks for init command results across test scenarios
///
/// This module provides automated validation utilities that can verify init command
/// outputs including file existence, directory structure, file content validation,
/// and Git configuration validation with clear error reporting.
use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;

/// Automated file and directory existence validator
pub struct FileSystemValidator;

impl FileSystemValidator {
    /// Validate that all expected files and directories exist after init
    pub fn validate_file_existence(
        repo_path: &Path,
        expected_files: &[&str],
        expected_directories: &[&str],
    ) -> Result<FileSystemValidationReport> {
        let mut report = FileSystemValidationReport::new();

        // Check expected files
        for file_path in expected_files {
            let full_path = repo_path.join(file_path);
            if full_path.exists() && full_path.is_file() {
                report.files_found.push(file_path.to_string());
            } else {
                report.files_missing.push(file_path.to_string());
                report.add_error(&format!("Expected file not found: {}", file_path));
            }
        }

        // Check expected directories
        for dir_path in expected_directories {
            let full_path = repo_path.join(dir_path);
            if full_path.exists() && full_path.is_dir() {
                report.directories_found.push(dir_path.to_string());
            } else {
                report.directories_missing.push(dir_path.to_string());
                report.add_error(&format!("Expected directory not found: {}", dir_path));
            }
        }

        report.success = report.errors.is_empty();
        Ok(report)
    }

    /// Validate directory structure matches expected layout
    pub fn validate_directory_structure(
        repo_path: &Path,
        expected_structure: &DirectoryStructure,
    ) -> Result<StructureValidationReport> {
        let mut report = StructureValidationReport::new();

        Self::validate_directory_recursive(repo_path, &expected_structure.root, "", &mut report)?;

        report.success = report.errors.is_empty();
        Ok(report)
    }

    fn validate_directory_recursive(
        base_path: &Path,
        expected_dir: &DirectoryNode,
        current_path: &str,
        report: &mut StructureValidationReport,
    ) -> Result<()> {
        let dir_path = if current_path.is_empty() {
            base_path.to_path_buf()
        } else {
            base_path.join(current_path)
        };

        // Check directory exists
        if !dir_path.exists() || !dir_path.is_dir() {
            report.add_error(&format!("Directory not found: {}", current_path));
            return Ok(());
        }

        // Check expected files in this directory
        for expected_file in &expected_dir.files {
            let file_path = dir_path.join(expected_file);
            if file_path.exists() && file_path.is_file() {
                report.files_validated += 1;
            } else {
                report.add_error(&format!(
                    "File not found: {}/{}",
                    current_path, expected_file
                ));
            }
        }

        // Recursively check subdirectories
        for (subdir_name, subdir_node) in &expected_dir.subdirectories {
            let subdir_path = if current_path.is_empty() {
                subdir_name.clone()
            } else {
                format!("{}/{}", current_path, subdir_name)
            };

            Self::validate_directory_recursive(base_path, subdir_node, &subdir_path, report)?;
        }

        Ok(())
    }
}

/// File content validation utilities
pub struct ContentValidator;

impl ContentValidator {
    /// Validate file contents match expected patterns
    pub fn validate_file_contents(
        repo_path: &Path,
        content_expectations: &HashMap<String, ContentExpectation>,
    ) -> Result<ContentValidationReport> {
        let mut report = ContentValidationReport::new();

        for (file_path, expectation) in content_expectations {
            let full_path = repo_path.join(file_path);

            if !full_path.exists() {
                report.add_error(&format!(
                    "File not found for content validation: {}",
                    file_path
                ));
                continue;
            }

            let content = match std::fs::read_to_string(&full_path) {
                Ok(content) => content,
                Err(e) => {
                    report.add_error(&format!("Failed to read file {}: {}", file_path, e));
                    continue;
                }
            };

            let validation_result = Self::validate_single_file_content(&content, expectation)?;
            report
                .file_results
                .insert(file_path.clone(), validation_result.clone());

            if !validation_result.passed {
                report.failed_files.push(file_path.clone());
                for error in &validation_result.errors {
                    report.add_error(&format!(
                        "Content validation failed for {}: {}",
                        file_path, error
                    ));
                }
            } else {
                report.passed_files.push(file_path.clone());
            }
        }

        report.success = report.errors.is_empty();
        Ok(report)
    }

    fn validate_single_file_content(
        content: &str,
        expectation: &ContentExpectation,
    ) -> Result<FileContentValidationResult> {
        let mut result = FileContentValidationResult::new();

        // Check required patterns
        for pattern in &expectation.must_contain {
            if content.contains(pattern) {
                result.patterns_found.push(pattern.clone());
            } else {
                result.patterns_missing.push(pattern.clone());
                result.add_error(&format!("Required pattern not found: '{}'", pattern));
            }
        }

        // Check forbidden patterns
        for pattern in &expectation.must_not_contain {
            if content.contains(pattern) {
                result.forbidden_patterns_found.push(pattern.clone());
                result.add_error(&format!("Forbidden pattern found: '{}'", pattern));
            }
        }

        // Check regex patterns
        for regex_pattern in &expectation.regex_patterns {
            match regex::Regex::new(regex_pattern) {
                Ok(regex) => {
                    if regex.is_match(content) {
                        result.regex_patterns_matched.push(regex_pattern.clone());
                    } else {
                        result.regex_patterns_unmatched.push(regex_pattern.clone());
                        result
                            .add_error(&format!("Regex pattern not matched: '{}'", regex_pattern));
                    }
                }
                Err(e) => {
                    result.add_error(&format!("Invalid regex pattern '{}': {}", regex_pattern, e));
                }
            }
        }

        // Validate line count if specified
        if let Some(expected_min_lines) = expectation.min_lines {
            let line_count = content.lines().count();
            if line_count < expected_min_lines {
                result.add_error(&format!(
                    "File has {} lines, expected at least {}",
                    line_count, expected_min_lines
                ));
            }
        }

        result.passed = result.errors.is_empty();
        Ok(result)
    }
}

/// Git configuration validator
pub struct GitConfigValidator;

impl GitConfigValidator {
    /// Validate Git configuration matches expectations
    pub fn validate_git_configuration(
        repo_path: &Path,
        expectations: &GitConfigExpectations,
    ) -> Result<GitConfigValidationReport> {
        let mut report = GitConfigValidationReport::new();

        // Check if git repository exists
        let git_dir = repo_path.join(".git");
        if !git_dir.exists() {
            if expectations.should_be_git_repo {
                report.add_error(
                    "Repository should be a Git repository but .git directory not found",
                );
            } else {
                report.git_repo_exists = false;
            }
        } else {
            report.git_repo_exists = true;

            // Validate git configuration if expected
            if expectations.should_be_git_repo {
                Self::validate_git_repo_details(repo_path, expectations, &mut report)?;
            }
        }

        report.success = report.errors.is_empty();
        Ok(report)
    }

    fn validate_git_repo_details(
        repo_path: &Path,
        expectations: &GitConfigExpectations,
        report: &mut GitConfigValidationReport,
    ) -> Result<()> {
        use std::process::Command;

        let original_dir = std::env::current_dir()?;
        std::env::set_current_dir(repo_path)?;

        // Check current branch
        if let Some(expected_branch) = &expectations.expected_branch {
            let output = Command::new("git")
                .args(&["rev-parse", "--abbrev-ref", "HEAD"])
                .output()?;

            if output.status.success() {
                let current_branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if current_branch == *expected_branch {
                    report.branch_correct = true;
                } else {
                    report.add_error(&format!(
                        "Expected branch '{}', found '{}'",
                        expected_branch, current_branch
                    ));
                }
            } else {
                report.add_error("Failed to get current Git branch");
            }
        }

        // Check remote configuration
        if let Some(expected_remote) = &expectations.expected_remote_url {
            let output = Command::new("git")
                .args(&["remote", "get-url", "origin"])
                .output()?;

            if output.status.success() {
                let remote_url = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if remote_url == *expected_remote {
                    report.remote_correct = true;
                } else {
                    report.add_error(&format!(
                        "Expected remote '{}', found '{}'",
                        expected_remote, remote_url
                    ));
                }
            } else if expectations.should_have_remote {
                report.add_error("Expected remote 'origin' but none found");
            }
        }

        // Check working directory status
        if expectations.should_be_clean {
            let output = Command::new("git")
                .args(&["status", "--porcelain"])
                .output()?;

            if output.status.success() {
                if output.stdout.is_empty() {
                    report.working_directory_clean = true;
                } else {
                    report
                        .add_error("Working directory should be clean but has uncommitted changes");
                }
            } else {
                report.add_error("Failed to check Git status");
            }
        }

        std::env::set_current_dir(original_dir)?;
        Ok(())
    }
}

/// Comprehensive validation result reporter
pub struct ValidationResultReporter;

impl ValidationResultReporter {
    /// Generate comprehensive report from all validation results
    pub fn generate_comprehensive_report(
        filesystem_report: &FileSystemValidationReport,
        content_report: &ContentValidationReport,
        git_report: &GitConfigValidationReport,
        scenario_name: &str,
        fixture_name: &str,
    ) -> ValidationSummaryReport {
        let mut summary = ValidationSummaryReport {
            scenario_name: scenario_name.to_string(),
            fixture_name: fixture_name.to_string(),
            overall_success: true,
            filesystem_passed: filesystem_report.success,
            content_validation_passed: content_report.success,
            git_validation_passed: git_report.success,
            total_errors: 0,
            error_summary: Vec::new(),
        };

        // Collect all errors
        let mut all_errors = Vec::new();
        all_errors.extend(filesystem_report.errors.iter().cloned());
        all_errors.extend(content_report.errors.iter().cloned());
        all_errors.extend(git_report.errors.iter().cloned());

        summary.total_errors = all_errors.len();
        summary.error_summary = all_errors;
        summary.overall_success = summary.total_errors == 0;

        summary
    }

    /// Generate detailed failure analysis
    pub fn generate_failure_analysis(reports: &[ValidationSummaryReport]) -> FailureAnalysisReport {
        let mut analysis = FailureAnalysisReport::new();

        for report in reports {
            analysis.total_validations += 1;

            if report.overall_success {
                analysis.successful_validations += 1;
            } else {
                analysis.failed_validations.push(FailedValidation {
                    scenario: report.scenario_name.clone(),
                    fixture: report.fixture_name.clone(),
                    error_count: report.total_errors,
                    primary_errors: report.error_summary.clone(),
                });
            }
        }

        analysis.success_rate = if analysis.total_validations > 0 {
            analysis.successful_validations as f64 / analysis.total_validations as f64
        } else {
            0.0
        };

        analysis
    }
}

// Data structures for validation configuration and results

/// Expected directory structure definition
#[derive(Debug, Clone)]
pub struct DirectoryStructure {
    pub root: DirectoryNode,
}

#[derive(Debug, Clone)]
pub struct DirectoryNode {
    pub files: Vec<String>,
    pub subdirectories: HashMap<String, DirectoryNode>,
}

/// Content validation expectations
#[derive(Debug, Clone)]
pub struct ContentExpectation {
    pub must_contain: Vec<String>,
    pub must_not_contain: Vec<String>,
    pub regex_patterns: Vec<String>,
    pub min_lines: Option<usize>,
}

/// Git configuration expectations
#[derive(Debug, Clone)]
pub struct GitConfigExpectations {
    pub should_be_git_repo: bool,
    pub expected_branch: Option<String>,
    pub expected_remote_url: Option<String>,
    pub should_have_remote: bool,
    pub should_be_clean: bool,
}

// Validation report structures

/// Filesystem validation report
#[derive(Debug, Clone)]
pub struct FileSystemValidationReport {
    pub success: bool,
    pub files_found: Vec<String>,
    pub files_missing: Vec<String>,
    pub directories_found: Vec<String>,
    pub directories_missing: Vec<String>,
    pub errors: Vec<String>,
}

impl FileSystemValidationReport {
    fn new() -> Self {
        Self {
            success: false,
            files_found: Vec::new(),
            files_missing: Vec::new(),
            directories_found: Vec::new(),
            directories_missing: Vec::new(),
            errors: Vec::new(),
        }
    }

    fn add_error(&mut self, error: &str) {
        self.errors.push(error.to_string());
    }
}

/// Directory structure validation report
#[derive(Debug, Clone)]
pub struct StructureValidationReport {
    pub success: bool,
    pub files_validated: usize,
    pub errors: Vec<String>,
}

impl StructureValidationReport {
    fn new() -> Self {
        Self {
            success: false,
            files_validated: 0,
            errors: Vec::new(),
        }
    }

    fn add_error(&mut self, error: &str) {
        self.errors.push(error.to_string());
    }
}

/// Content validation report
#[derive(Debug, Clone)]
pub struct ContentValidationReport {
    pub success: bool,
    pub passed_files: Vec<String>,
    pub failed_files: Vec<String>,
    pub file_results: HashMap<String, FileContentValidationResult>,
    pub errors: Vec<String>,
}

impl ContentValidationReport {
    fn new() -> Self {
        Self {
            success: false,
            passed_files: Vec::new(),
            failed_files: Vec::new(),
            file_results: HashMap::new(),
            errors: Vec::new(),
        }
    }

    fn add_error(&mut self, error: &str) {
        self.errors.push(error.to_string());
    }
}

/// Single file content validation result
#[derive(Debug, Clone)]
pub struct FileContentValidationResult {
    pub passed: bool,
    pub patterns_found: Vec<String>,
    pub patterns_missing: Vec<String>,
    pub forbidden_patterns_found: Vec<String>,
    pub regex_patterns_matched: Vec<String>,
    pub regex_patterns_unmatched: Vec<String>,
    pub errors: Vec<String>,
}

impl FileContentValidationResult {
    fn new() -> Self {
        Self {
            passed: false,
            patterns_found: Vec::new(),
            patterns_missing: Vec::new(),
            forbidden_patterns_found: Vec::new(),
            regex_patterns_matched: Vec::new(),
            regex_patterns_unmatched: Vec::new(),
            errors: Vec::new(),
        }
    }

    fn add_error(&mut self, error: &str) {
        self.errors.push(error.to_string());
    }
}

/// Git configuration validation report
#[derive(Debug, Clone)]
pub struct GitConfigValidationReport {
    pub success: bool,
    pub git_repo_exists: bool,
    pub branch_correct: bool,
    pub remote_correct: bool,
    pub working_directory_clean: bool,
    pub errors: Vec<String>,
}

impl GitConfigValidationReport {
    fn new() -> Self {
        Self {
            success: false,
            git_repo_exists: false,
            branch_correct: false,
            remote_correct: false,
            working_directory_clean: false,
            errors: Vec::new(),
        }
    }

    fn add_error(&mut self, error: &str) {
        self.errors.push(error.to_string());
    }
}

/// Comprehensive validation summary
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ValidationSummaryReport {
    pub scenario_name: String,
    pub fixture_name: String,
    pub overall_success: bool,
    pub filesystem_passed: bool,
    pub content_validation_passed: bool,
    pub git_validation_passed: bool,
    pub total_errors: usize,
    pub error_summary: Vec<String>,
}

/// Failed validation details
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FailedValidation {
    pub scenario: String,
    pub fixture: String,
    pub error_count: usize,
    pub primary_errors: Vec<String>,
}

/// Analysis of validation failures across multiple scenarios
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FailureAnalysisReport {
    pub total_validations: usize,
    pub successful_validations: usize,
    pub failed_validations: Vec<FailedValidation>,
    pub success_rate: f64,
}

impl FailureAnalysisReport {
    fn new() -> Self {
        Self {
            total_validations: 0,
            successful_validations: 0,
            failed_validations: Vec::new(),
            success_rate: 0.0,
        }
    }
}

/// Utility function to create standard init command validation expectations
pub fn create_standard_init_expectations() -> (
    Vec<String>,
    Vec<String>,
    HashMap<String, ContentExpectation>,
    GitConfigExpectations,
) {
    let expected_files = vec!["my-little-soda.toml".to_string()];

    let expected_directories = vec![
        ".my-little-soda".to_string(),
        ".my-little-soda/credentials".to_string(),
        ".my-little-soda/agents".to_string(),
    ];

    let mut content_expectations = HashMap::new();
    content_expectations.insert(
        "my-little-soda.toml".to_string(),
        ContentExpectation {
            must_contain: vec![
                "[github]".to_string(),
                "[observability]".to_string(),
                "[agents]".to_string(),
                "max_agents".to_string(),
            ],
            must_not_contain: vec!["PLACEHOLDER".to_string(), "TODO".to_string()],
            regex_patterns: vec![
                r#"owner\s*=\s*"[^"]+""#.to_string(),
                r#"repo\s*=\s*"[^"]+""#.to_string(),
            ],
            min_lines: Some(10),
        },
    );

    let git_expectations = GitConfigExpectations {
        should_be_git_repo: true,
        expected_branch: Some("main".to_string()),
        expected_remote_url: None, // Will vary by repository
        should_have_remote: true,
        should_be_clean: false, // May have modifications during testing
    };

    (
        expected_files,
        expected_directories,
        content_expectations,
        git_expectations,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_filesystem_validator_success() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        // Create expected files
        std::fs::write(repo_path.join("test.txt"), "test content").unwrap();
        std::fs::create_dir(repo_path.join("test_dir")).unwrap();

        let expected_files = vec!["test.txt"];
        let expected_dirs = vec!["test_dir"];

        let report = FileSystemValidator::validate_file_existence(
            repo_path,
            &expected_files,
            &expected_dirs,
        )
        .unwrap();

        assert!(report.success);
        assert_eq!(report.files_found.len(), 1);
        assert_eq!(report.directories_found.len(), 1);
        assert!(report.errors.is_empty());
    }

    #[test]
    fn test_content_validator() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        std::fs::write(repo_path.join("config.toml"), "[section]\nkey = \"value\"").unwrap();

        let mut expectations = HashMap::new();
        expectations.insert(
            "config.toml".to_string(),
            ContentExpectation {
                must_contain: vec!["[section]".to_string(), "key".to_string()],
                must_not_contain: vec!["forbidden".to_string()],
                regex_patterns: vec![r#"key\s*=\s*"[^"]+""#.to_string()],
                min_lines: Some(1),
            },
        );

        let report = ContentValidator::validate_file_contents(repo_path, &expectations).unwrap();

        assert!(report.success);
        assert_eq!(report.passed_files.len(), 1);
        assert_eq!(report.failed_files.len(), 0);
    }

    #[test]
    fn test_validation_result_reporter() {
        let fs_report = FileSystemValidationReport {
            success: true,
            files_found: vec!["test.txt".to_string()],
            files_missing: Vec::new(),
            directories_found: vec!["test_dir".to_string()],
            directories_missing: Vec::new(),
            errors: Vec::new(),
        };

        let content_report = ContentValidationReport {
            success: true,
            passed_files: vec!["test.txt".to_string()],
            failed_files: Vec::new(),
            file_results: HashMap::new(),
            errors: Vec::new(),
        };

        let git_report = GitConfigValidationReport {
            success: true,
            git_repo_exists: true,
            branch_correct: true,
            remote_correct: true,
            working_directory_clean: true,
            errors: Vec::new(),
        };

        let summary = ValidationResultReporter::generate_comprehensive_report(
            &fs_report,
            &content_report,
            &git_report,
            "test_scenario",
            "test_fixture",
        );

        assert!(summary.overall_success);
        assert!(summary.filesystem_passed);
        assert!(summary.content_validation_passed);
        assert!(summary.git_validation_passed);
        assert_eq!(summary.total_errors, 0);
    }
}
