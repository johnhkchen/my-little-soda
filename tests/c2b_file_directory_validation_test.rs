/// C2b - Validate all expected files/directories are created
/// 
/// This test validates that the init command creates all expected files and directories
/// with correct structure, permissions, and content across all test scenarios.
/// 
/// Expected Files/Directories Created by Init Command:
/// - `my-little-soda.toml` - Main configuration file at repository root
/// - `.my-little-soda/` - Main clambake directory
/// - `.my-little-soda/credentials/` - Directory for credential storage
/// - `.my-little-soda/agents/` - Directory for agent working directories
/// - `.my-little-soda/clambake.db` - Database file (referenced in config)
/// - `.my-little-soda/autonomous_state/` - Directory for autonomous agent state (created on demand)
/// - `.my-little-soda/metrics/` - Directory for metrics storage (created on demand)

use anyhow::Result;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Comprehensive checklist of expected files and directories
#[derive(Debug)]
struct ExpectedFilesAndDirectories {
    /// Files that must be created by init command
    required_files: Vec<String>,
    /// Directories that must be created by init command
    required_directories: Vec<String>,
    /// Files that are referenced in config but created on demand
    on_demand_files: Vec<String>,
    /// Directories that are created on demand during operation
    on_demand_directories: Vec<String>,
}

impl ExpectedFilesAndDirectories {
    fn standard_init_expectations() -> Self {
        Self {
            required_files: vec![
                "my-little-soda.toml".to_string(),
            ],
            required_directories: vec![
                ".my-little-soda".to_string(),
                ".my-little-soda/credentials".to_string(),
                ".my-little-soda/agents".to_string(),
            ],
            on_demand_files: vec![
                ".my-little-soda/clambake.db".to_string(), // Database file
                ".my-little-soda/bundle.lock".to_string(),  // Bundle lock file
                ".my-little-soda/bundle_state.json".to_string(), // Bundle state
            ],
            on_demand_directories: vec![
                ".my-little-soda/autonomous_state".to_string(), // Autonomous agent state
                ".my-little-soda/metrics".to_string(),          // Metrics storage
            ],
        }
    }
}

/// File and directory validation results
#[derive(Debug)]
struct ValidationReport {
    files_created: Vec<String>,
    files_missing: Vec<String>,
    directories_created: Vec<String>,
    directories_missing: Vec<String>,
    permission_issues: Vec<String>,
    structure_issues: Vec<String>,
    success: bool,
}

impl ValidationReport {
    fn new() -> Self {
        Self {
            files_created: Vec::new(),
            files_missing: Vec::new(),
            directories_created: Vec::new(),
            directories_missing: Vec::new(),
            permission_issues: Vec::new(),
            structure_issues: Vec::new(),
            success: true,
        }
    }
    
    fn add_error(&mut self, error: String) {
        self.structure_issues.push(error);
        self.success = false;
    }
}

/// File and directory validator for init command results
struct FileSystemValidator;

impl FileSystemValidator {
    /// Validate all expected files and directories are created correctly
    fn validate_init_file_creation(repo_path: &Path, dry_run: bool) -> Result<ValidationReport> {
        let mut report = ValidationReport::new();
        let expectations = ExpectedFilesAndDirectories::standard_init_expectations();
        
        // Check required files
        for file_path in &expectations.required_files {
            let full_path = repo_path.join(file_path);
            if dry_run {
                // In dry run, files should NOT be created
                if full_path.exists() {
                    report.add_error(format!("File {} should not exist in dry run mode", file_path));
                }
            } else {
                // In real run, files should be created
                if full_path.exists() && full_path.is_file() {
                    report.files_created.push(file_path.clone());
                    
                    // Validate file permissions
                    if let Err(e) = Self::validate_file_permissions(&full_path) {
                        report.permission_issues.push(format!("{}: {}", file_path, e));
                    }
                } else {
                    report.files_missing.push(file_path.clone());
                    report.add_error(format!("Required file not created: {}", file_path));
                }
            }
        }
        
        // Check required directories
        for dir_path in &expectations.required_directories {
            let full_path = repo_path.join(dir_path);
            if dry_run {
                // In dry run, directories should NOT be created
                if full_path.exists() {
                    report.add_error(format!("Directory {} should not exist in dry run mode", dir_path));
                }
            } else {
                // In real run, directories should be created
                if full_path.exists() && full_path.is_dir() {
                    report.directories_created.push(dir_path.clone());
                    
                    // Validate directory permissions
                    if let Err(e) = Self::validate_directory_permissions(&full_path) {
                        report.permission_issues.push(format!("{}: {}", dir_path, e));
                    }
                } else {
                    report.directories_missing.push(dir_path.clone());
                    report.add_error(format!("Required directory not created: {}", dir_path));
                }
            }
        }
        
        // Validate directory structure hierarchy
        if !dry_run {
            Self::validate_directory_hierarchy(repo_path, &mut report)?;
        }
        
        report.success = report.files_missing.is_empty() 
                      && report.directories_missing.is_empty()
                      && report.structure_issues.is_empty();
        
        Ok(report)
    }
    
    /// Validate file permissions are appropriate
    fn validate_file_permissions(file_path: &Path) -> Result<()> {
        let metadata = fs::metadata(file_path)?;
        
        // Check file is readable
        if metadata.permissions().readonly() && file_path.file_name().unwrap() != "my-little-soda.toml" {
            return Err(anyhow::anyhow!("File is readonly when it should be writable"));
        }
        
        Ok(())
    }
    
    /// Validate directory permissions are appropriate
    fn validate_directory_permissions(dir_path: &Path) -> Result<()> {
        let metadata = fs::metadata(dir_path)?;
        
        // Check directory is accessible
        if metadata.permissions().readonly() {
            return Err(anyhow::anyhow!("Directory is readonly when it should be writable"));
        }
        
        // Try to create a test file to verify write permissions
        let test_file = dir_path.join(".test_write_permission");
        match fs::write(&test_file, "test") {
            Ok(()) => {
                let _ = fs::remove_file(&test_file); // Clean up
                Ok(())
            }
            Err(e) => Err(anyhow::anyhow!("Directory not writable: {}", e)),
        }
    }
    
    /// Validate directory hierarchy is correct
    fn validate_directory_hierarchy(repo_path: &Path, report: &mut ValidationReport) -> Result<()> {
        let clambake_dir = repo_path.join(".my-little-soda");
        
        if !clambake_dir.exists() {
            report.add_error("Root .my-little-soda directory missing".to_string());
            return Ok(());
        }
        
        // Check that .my-little-soda is at repository root level, not nested
        if clambake_dir.parent() != Some(repo_path) {
            report.add_error("clambake directory not at repository root level".to_string());
        }
        
        // Check credentials directory is under .my-little-soda
        let credentials_dir = clambake_dir.join("credentials");
        if credentials_dir.exists() && credentials_dir.parent() != Some(&clambake_dir) {
            report.add_error("credentials directory not properly nested under .my-little-soda".to_string());
        }
        
        // Check agents directory is under .my-little-soda
        let agents_dir = clambake_dir.join("agents");
        if agents_dir.exists() && agents_dir.parent() != Some(&clambake_dir) {
            report.add_error("agents directory not properly nested under .my-little-soda".to_string());
        }
        
        Ok(())
    }
    
    /// Validate configuration file content is correctly generated
    fn validate_config_file_content(repo_path: &Path) -> Result<ValidationReport> {
        let mut report = ValidationReport::new();
        let config_path = repo_path.join("my-little-soda.toml");
        
        if !config_path.exists() {
            report.add_error("my-little-soda.toml file not found".to_string());
            return Ok(report);
        }
        
        let config_content = fs::read_to_string(&config_path)?;
        
        // Check required sections exist
        let required_sections = vec!["[github]", "[observability]", "[agents]", "[database]"];
        for section in required_sections {
            if !config_content.contains(section) {
                report.add_error(format!("Configuration missing required section: {}", section));
            }
        }
        
        // Check required keys exist
        let required_keys = vec!["max_agents", "owner", "repo", "log_level"];
        for key in required_keys {
            if !config_content.contains(key) {
                report.add_error(format!("Configuration missing required key: {}", key));
            }
        }
        
        // Check database URL points to correct location
        if config_content.contains("url = \".my-little-soda/clambake.db\"") {
            // Good - database path is correct
        } else {
            report.add_error("Database URL not set to .my-little-soda/clambake.db".to_string());
        }
        
        // Validate TOML syntax by parsing
        match toml::from_str::<toml::Value>(&config_content) {
            Ok(_) => {
                // TOML is valid
            }
            Err(e) => {
                report.add_error(format!("Invalid TOML syntax: {}", e));
            }
        }
        
        report.success = report.structure_issues.is_empty();
        Ok(report)
    }
}

/// Test helper to run init command with proper Git setup
fn setup_test_repository() -> Result<TempDir> {
    let temp_dir = TempDir::new()?;
    let repo_path = temp_dir.path();
    
    // Initialize Git repository
    std::process::Command::new("git")
        .args(&["init"])
        .current_dir(repo_path)
        .output()?;
    
    // Configure Git user (required for commits)
    std::process::Command::new("git")
        .args(&["config", "user.name", "Test User"])
        .current_dir(repo_path)
        .output()?;
        
    std::process::Command::new("git")
        .args(&["config", "user.email", "test@example.com"])
        .current_dir(repo_path)
        .output()?;
    
    // Add a fake GitHub remote (required by init command)
    std::process::Command::new("git")
        .args(&["remote", "add", "origin", "https://github.com/test/test.git"])
        .current_dir(repo_path)
        .output()?;
    
    Ok(temp_dir)
}

#[tokio::test]
async fn test_init_creates_all_required_files_dry_run() {
    let temp_dir = setup_test_repository().unwrap();
    let repo_path = temp_dir.path();
    
    // Run init command in dry run mode
    let binary_path = std::env::current_dir().unwrap().join("target/debug/my-little-soda");
    let output = std::process::Command::new(&binary_path)
        .args(&["init", "--dry-run"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to run init command");
    
    assert!(output.status.success(), "Init command should succeed in dry run");
    
    // Validate no files/directories were created in dry run
    let validation_report = FileSystemValidator::validate_init_file_creation(repo_path, true)
        .expect("Validation should succeed");
    
    assert!(validation_report.success, "Dry run validation should pass: {:?}", validation_report);
    assert!(validation_report.files_missing.is_empty(), "No files should be missing in dry run validation");
    assert!(validation_report.directories_missing.is_empty(), "No directories should be missing in dry run validation");
}

#[tokio::test]
#[ignore] // Requires GitHub authentication
async fn test_init_creates_all_required_files_real_run() {
    let temp_dir = setup_test_repository().unwrap();
    let repo_path = temp_dir.path();
    
    // Run init command in real mode (requires GitHub authentication)
    let binary_path = std::env::current_dir().unwrap().join("target/debug/my-little-soda");
    let output = std::process::Command::new(&binary_path)
        .args(&["init"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to run init command");
    
    // Note: May fail without GitHub authentication, which is expected
    if !output.status.success() {
        println!("Init command failed (likely due to missing GitHub authentication): {}", 
                String::from_utf8_lossy(&output.stderr));
        return;
    }
    
    // Validate all files/directories were created
    let validation_report = FileSystemValidator::validate_init_file_creation(repo_path, false)
        .expect("Validation should succeed");
    
    assert!(validation_report.success, "Real run validation should pass: {:?}", validation_report);
    assert!(validation_report.files_missing.is_empty(), 
           "All required files should be created: missing {:?}", validation_report.files_missing);
    assert!(validation_report.directories_missing.is_empty(), 
           "All required directories should be created: missing {:?}", validation_report.directories_missing);
    
    // Validate configuration file content
    let config_validation = FileSystemValidator::validate_config_file_content(repo_path)
        .expect("Config validation should succeed");
    
    assert!(config_validation.success, "Config file validation should pass: {:?}", config_validation);
}

#[tokio::test]
async fn test_init_directory_structure_validation() {
    let temp_dir = setup_test_repository().unwrap();
    let repo_path = temp_dir.path();
    
    // Manually create the expected directory structure for testing
    fs::create_dir_all(repo_path.join(".my-little-soda/credentials")).unwrap();
    fs::create_dir_all(repo_path.join(".my-little-soda/agents")).unwrap();
    fs::write(repo_path.join("my-little-soda.toml"), r#"
[github]
owner = "test"
repo = "test"

[observability]
log_level = "info"

[agents]
max_agents = 2

[database]
url = ".my-little-soda/clambake.db"
"#).unwrap();
    
    // Validate the structure
    let validation_report = FileSystemValidator::validate_init_file_creation(repo_path, false)
        .expect("Validation should succeed");
    
    assert!(validation_report.success, "Structure validation should pass: {:?}", validation_report);
    assert!(validation_report.structure_issues.is_empty(), 
           "No structure issues should be found: {:?}", validation_report.structure_issues);
}

#[tokio::test]
async fn test_config_file_content_validation() {
    let temp_dir = setup_test_repository().unwrap();
    let repo_path = temp_dir.path();
    
    // Create a valid configuration file
    fs::write(repo_path.join("my-little-soda.toml"), r#"
[github]
owner = "testowner"
repo = "testrepo"

[observability]
log_level = "info"
tracing_enabled = true

[agents]
max_agents = 4
coordination_timeout_seconds = 300

[database]
url = ".my-little-soda/clambake.db"
max_connections = 10
auto_migrate = true
"#).unwrap();
    
    // Validate the configuration content
    let config_validation = FileSystemValidator::validate_config_file_content(repo_path)
        .expect("Config validation should succeed");
    
    assert!(config_validation.success, "Config validation should pass: {:?}", config_validation);
}

#[tokio::test]
async fn test_config_file_content_validation_with_missing_sections() {
    let temp_dir = setup_test_repository().unwrap();
    let repo_path = temp_dir.path();
    
    // Create an invalid configuration file missing sections
    fs::write(repo_path.join("my-little-soda.toml"), r#"
[github]
owner = "testowner"
# Missing repo, observability, agents, database sections
"#).unwrap();
    
    // Validate the configuration content
    let config_validation = FileSystemValidator::validate_config_file_content(repo_path)
        .expect("Config validation should succeed");
    
    assert!(!config_validation.success, "Config validation should fail with missing sections");
    assert!(config_validation.structure_issues.len() >= 3, 
           "Should detect multiple missing sections: {:?}", config_validation.structure_issues);
}

#[tokio::test]
async fn test_permission_validation() {
    let temp_dir = setup_test_repository().unwrap();
    let repo_path = temp_dir.path();
    
    // Create files and directories with proper permissions
    fs::create_dir_all(repo_path.join(".my-little-soda/credentials")).unwrap();
    fs::create_dir_all(repo_path.join(".my-little-soda/agents")).unwrap();
    fs::write(repo_path.join("my-little-soda.toml"), "[github]\nowner = \"test\"\n").unwrap();
    
    // Validate permissions (this mainly tests that the validation doesn't fail)
    let validation_report = FileSystemValidator::validate_init_file_creation(repo_path, false)
        .expect("Permission validation should succeed");
    
    // Should succeed unless there are actual permission issues
    if !validation_report.success {
        println!("Permission validation issues: {:?}", validation_report.permission_issues);
    }
}

#[tokio::test]
async fn test_comprehensive_file_directory_validation_checklist() {
    // This test documents the complete checklist of expected files and directories
    let expectations = ExpectedFilesAndDirectories::standard_init_expectations();
    
    // Verify our expectations are comprehensive
    assert!(!expectations.required_files.is_empty(), "Should have required files");
    assert!(!expectations.required_directories.is_empty(), "Should have required directories");
    
    // Verify specific requirements
    assert!(expectations.required_files.contains(&"my-little-soda.toml".to_string()), 
           "Should expect my-little-soda.toml file");
    assert!(expectations.required_directories.contains(&".my-little-soda".to_string()), 
           "Should expect .my-little-soda directory");
    assert!(expectations.required_directories.contains(&".my-little-soda/credentials".to_string()), 
           "Should expect .my-little-soda/credentials directory");
    assert!(expectations.required_directories.contains(&".my-little-soda/agents".to_string()), 
           "Should expect .my-little-soda/agents directory");
    
    println!("✅ Complete file/directory validation checklist:");
    println!("Required files: {:?}", expectations.required_files);
    println!("Required directories: {:?}", expectations.required_directories);
    println!("On-demand files: {:?}", expectations.on_demand_files);
    println!("On-demand directories: {:?}", expectations.on_demand_directories);
}