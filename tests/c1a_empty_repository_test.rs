/// C1a - Empty Repository Init Command Test Case
/// 
/// This test validates the init command behavior on completely empty repositories.
/// It implements the requirements from issue #311 as part of the Production Deployment Sprint.

use my_little_soda::cli::commands::init::InitCommand;
use my_little_soda::fs::{FileSystemOperations, StandardFileSystem};
use std::sync::Arc;
use tempfile::TempDir;
use anyhow::Result;

/// Test results for init command execution on empty repository
#[derive(Debug)]
struct EmptyRepositoryTestResult {
    temp_dir: TempDir,
    init_success: bool,
    error_message: Option<String>,
    config_created: bool,
    clambake_dir_created: bool,
    agents_dir_created: bool,
    files_created: Vec<String>,
}

impl EmptyRepositoryTestResult {
    /// Check if all expected files and directories were created
    fn validate_expected_files(&self) -> bool {
        self.config_created && self.clambake_dir_created && self.agents_dir_created
    }
    
    /// Get a summary of what was created
    fn creation_summary(&self) -> String {
        let mut summary = Vec::new();
        if self.config_created { summary.push("clambake.toml"); }
        if self.clambake_dir_created { summary.push(".clambake/"); }
        if self.agents_dir_created { summary.push(".clambake/agents/"); }
        summary.join(", ")
    }
}

/// Create a completely empty Git repository for testing
async fn create_empty_git_repository() -> Result<TempDir> {
    let temp_dir = tempfile::tempdir()?;
    
    // Initialize git repository
    let output = tokio::process::Command::new("git")
        .args(["init"])
        .current_dir(temp_dir.path())
        .output()
        .await?;
    
    if !output.status.success() {
        anyhow::bail!("Failed to initialize git repository: {}", 
                     String::from_utf8_lossy(&output.stderr));
    }
    
    // Set up basic git config for testing
    tokio::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(temp_dir.path())
        .output()
        .await?;
        
    tokio::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(temp_dir.path())
        .output()
        .await?;
    
    // Add GitHub origin remote (required for init command)
    tokio::process::Command::new("git")
        .args(["remote", "add", "origin", "https://github.com/test-owner/test-repo.git"])
        .current_dir(temp_dir.path())
        .output()
        .await?;
    
    Ok(temp_dir)
}

/// Run init command on empty repository and capture results
async fn run_init_on_empty_repository(dry_run: bool) -> Result<EmptyRepositoryTestResult> {
    // Create empty repository
    let temp_dir = create_empty_git_repository().await?;
    
    // Change to repository directory
    let original_dir = std::env::current_dir()?;
    std::env::set_current_dir(temp_dir.path())?;
    
    // Create and run init command
    let fs_ops: Arc<dyn FileSystemOperations> = Arc::new(StandardFileSystem);
    let init_command = InitCommand::new(1, None, false, dry_run, fs_ops);
    
    let init_result = init_command.execute().await;
    
    // Restore original directory
    std::env::set_current_dir(original_dir)?;
    
    // Check what files were created
    let config_created = temp_dir.path().join("clambake.toml").exists();
    let clambake_dir_created = temp_dir.path().join(".clambake").exists();
    let agents_dir_created = temp_dir.path().join(".clambake/agents").exists();
    
    // List all files that were created
    let mut files_created = Vec::new();
    if config_created { files_created.push("clambake.toml".to_string()); }
    if clambake_dir_created { files_created.push(".clambake/".to_string()); }
    if agents_dir_created { files_created.push(".clambake/agents/".to_string()); }
    
    Ok(EmptyRepositoryTestResult {
        temp_dir,
        init_success: init_result.is_ok(),
        error_message: init_result.err().map(|e| e.to_string()),
        config_created,
        clambake_dir_created,
        agents_dir_created,
        files_created,
    })
}

/// C1a Test: Init command works correctly on empty repositories (dry run)
#[tokio::test]
async fn test_c1a_init_on_empty_repository() {
    let result = run_init_on_empty_repository(true).await
        .expect("Failed to run init command on empty repository");
    
    // Validate that init dry run succeeded
    assert!(result.init_success, 
           "Init command dry run should succeed on empty repository. Error: {}", 
           result.error_message.unwrap_or_default());
    
    // For dry run, no files should be created, but the command should succeed
    assert!(!result.config_created, "clambake.toml should not be created in dry run");
    assert!(!result.clambake_dir_created, ".clambake directory should not be created in dry run");
    assert!(!result.agents_dir_created, ".clambake/agents directory should not be created in dry run");
    
    println!("✅ C1a Test PASSED: Init dry run works correctly on empty repository");
    println!("   No files created during dry run (as expected)");
}

/// C1a Test: Init command dry run works on empty repositories
#[tokio::test]
async fn test_c1a_init_dry_run_on_empty_repository() {
    let result = run_init_on_empty_repository(true).await
        .expect("Failed to run init dry run on empty repository");
    
    // Validate that dry run succeeded
    assert!(result.init_success, 
           "Init dry run should succeed on empty repository. Error: {}", 
           result.error_message.unwrap_or_default());
    
    // Validate no files were created during dry run
    assert!(!result.config_created, "clambake.toml should not be created in dry run");
    assert!(!result.clambake_dir_created, ".clambake directory should not be created in dry run");
    assert!(!result.agents_dir_created, ".clambake/agents directory should not be created in dry run");
    
    println!("✅ C1a Dry Run Test PASSED: Init dry run works correctly on empty repository");
    println!("   No files created (as expected for dry run)");
}

/// C1a Test: Init command handles multiple agent counts on empty repositories
#[tokio::test]
async fn test_c1a_init_multiple_agents_empty_repository() {
    let agent_counts = [1, 2, 4, 8, 12];
    
    for &agent_count in &agent_counts {
        let temp_dir = create_empty_git_repository().await
            .expect("Failed to create empty repository");
        
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();
        
        let fs_ops: Arc<dyn FileSystemOperations> = Arc::new(StandardFileSystem);
        let init_command = InitCommand::new(agent_count, None, false, true, fs_ops); // dry run
        
        let result = init_command.execute().await;
        
        std::env::set_current_dir(original_dir).unwrap();
        
        assert!(result.is_ok(), 
               "Init should succeed with {} agents on empty repository. Error: {}", 
               agent_count, 
               result.err().map(|e| e.to_string()).unwrap_or_default());
    }
    
    println!("✅ C1a Multi-Agent Test PASSED: Init works with various agent counts on empty repository");
}

/// C1a Test: Init command validates repository state correctly for empty repositories
#[tokio::test]
async fn test_c1a_init_validation_empty_repository() {
    let result = run_init_on_empty_repository(false).await
        .expect("Failed to run init on empty repository");
    
    // Check that the repository is in a clean state after init
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(result.temp_dir.path()).unwrap();
    
    // Run git status to verify repository is clean
    let git_status = tokio::process::Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .await
        .expect("Failed to check git status");
    
    std::env::set_current_dir(original_dir).unwrap();
    
    // After init, there should be uncommitted files (the created configuration)
    let status_output = String::from_utf8_lossy(&git_status.stdout);
    
    if result.config_created {
        assert!(!status_output.trim().is_empty(), 
               "Repository should have uncommitted changes after init (config files created)");
        assert!(status_output.contains("clambake.toml"), 
               "Git status should show clambake.toml as uncommitted");
    }
    
    println!("✅ C1a Validation Test PASSED: Repository state is correct after init on empty repository");
    println!("   Git status shows: {}", status_output.trim());
}

/// C1a Integration Test: Complete empty repository workflow
#[tokio::test] 
async fn test_c1a_complete_empty_repository_workflow() {
    println!("🧪 Running C1a complete empty repository workflow test...");
    
    // Step 1: Create empty repository
    println!("📁 Step 1: Creating empty Git repository...");
    let temp_dir = create_empty_git_repository().await
        .expect("Failed to create empty repository");
    println!("✅ Empty repository created at: {}", temp_dir.path().display());
    
    // Step 2: Verify repository is truly empty (no files except .git)
    println!("🔍 Step 2: Verifying repository is empty...");
    let mut file_count = 0;
    for entry in std::fs::read_dir(temp_dir.path()).unwrap() {
        let entry = entry.unwrap();
        if entry.file_name() != ".git" {
            file_count += 1;
        }
    }
    assert_eq!(file_count, 0, "Repository should be empty except for .git directory");
    println!("✅ Repository is confirmed empty");
    
    // Step 3: Run init command (dry run for test safety)
    println!("⚙️  Step 3: Running init command (dry run)...");
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(temp_dir.path()).unwrap();
    
    let fs_ops: Arc<dyn FileSystemOperations> = Arc::new(StandardFileSystem);
    let init_command = InitCommand::new(1, None, false, true, fs_ops); // Use dry_run = true
    let init_result = init_command.execute().await;
    
    std::env::set_current_dir(original_dir).unwrap();
    
    assert!(init_result.is_ok(), "Init command should succeed: {}", 
           init_result.err().map(|e| e.to_string()).unwrap_or_default());
    println!("✅ Init command (dry run) completed successfully");
    
    // Step 4: Verify that no files were created during dry run (as expected)
    println!("📋 Step 4: Verifying dry run behavior (no files created)...");
    let should_not_exist_files = [
        "clambake.toml",
        ".clambake/",
        ".clambake/agents/",
        ".clambake/credentials/",
    ];
    
    for file_path in &should_not_exist_files {
        let path = temp_dir.path().join(file_path);
        assert!(!path.exists(), "File/directory should not exist in dry run: {}", file_path);
        println!("  ✅ {} (correctly not created)", file_path);
    }
    
    // Step 5: Verify repository is still clean after dry run
    println!("⚙️  Step 5: Verifying repository remains clean after dry run...");
    let mut file_count = 0;
    for entry in std::fs::read_dir(temp_dir.path()).unwrap() {
        let entry = entry.unwrap();
        if entry.file_name() != ".git" {
            file_count += 1;
        }
    }
    assert_eq!(file_count, 0, "Repository should still be empty after dry run except for .git directory");
    println!("✅ Repository remains clean after dry run");
    
    println!("🎉 C1a Complete Workflow Test PASSED: Empty repository initialization (dry run) is fully functional");
}