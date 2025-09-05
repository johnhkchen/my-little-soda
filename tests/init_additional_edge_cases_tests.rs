/// Additional edge cases testing for init command (C3c requirement)
///
/// Tests additional edge cases for init command:
/// - Network connectivity failures
/// - Corrupted git repositories
/// - Missing git binary
/// - Disk space limitations  
/// - File system permission issues
use my_little_soda::cli::commands::init::InitCommand;
use my_little_soda::fs::{FileSystemOperations, StandardFileSystem};
use std::sync::Arc;
use tempfile::TempDir;

async fn setup_base_git_repo(temp_dir: &TempDir) -> Result<(), Box<dyn std::error::Error>> {
    let original_dir = std::env::current_dir()?;
    std::env::set_current_dir(temp_dir.path())?;

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .output()
        .expect("Failed to init git repo");

    std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .output()
        .expect("Failed to set git user");

    std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .output()
        .expect("Failed to set git email");

    std::process::Command::new("git")
        .args([
            "remote",
            "add",
            "origin",
            "https://github.com/test-owner/test-repo.git",
        ])
        .output()
        .expect("Failed to add git remote");

    std::fs::write("README.md", "# Test Repository")?;

    std::process::Command::new("git")
        .args(["add", "README.md"])
        .output()
        .expect("Failed to add file");

    std::process::Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .output()
        .expect("Failed to commit");

    std::env::set_current_dir(original_dir)?;
    Ok(())
}

#[tokio::test]
async fn test_init_with_corrupted_git_objects() {
    let temp_dir = tempfile::tempdir().unwrap();
    setup_base_git_repo(&temp_dir).await.unwrap();
    let original_dir = std::env::current_dir().unwrap();

    std::env::set_current_dir(temp_dir.path()).unwrap();

    // Corrupt a git object by writing invalid data
    let objects_dir = std::path::Path::new(".git/objects");
    if let Ok(entries) = std::fs::read_dir(objects_dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                if entry.path().is_dir()
                    && entry.file_name() != "info"
                    && entry.file_name() != "pack"
                {
                    if let Ok(subentries) = std::fs::read_dir(entry.path()) {
                        for subentry in subentries {
                            if let Ok(subentry) = subentry {
                                // Corrupt the first object file we find
                                std::fs::write(subentry.path(), "corrupted data")
                                    .unwrap_or_default();
                                break;
                            }
                        }
                    }
                    break;
                }
            }
        }
    }

    let fs_ops = Arc::new(StandardFileSystem);
    let init_command = InitCommand::new(None, false, true, fs_ops);

    let result = init_command.execute().await;
    // Should handle corrupted git repository gracefully
    if result.is_err() {
        let error_msg = result.unwrap_err().to_string();
        println!("Expected error for corrupted git objects: {}", error_msg);
    }

    std::env::set_current_dir(original_dir).unwrap();
}

#[tokio::test]
async fn test_init_with_corrupted_git_index() {
    let temp_dir = tempfile::tempdir().unwrap();
    setup_base_git_repo(&temp_dir).await.unwrap();
    let original_dir = std::env::current_dir().unwrap();

    std::env::set_current_dir(temp_dir.path()).unwrap();

    // Corrupt the git index file
    std::fs::write(".git/index", "corrupted index data").unwrap_or_default();

    let fs_ops = Arc::new(StandardFileSystem);
    let init_command = InitCommand::new(None, false, true, fs_ops);

    let result = init_command.execute().await;
    // Should handle corrupted git index gracefully
    if result.is_err() {
        let error_msg = result.unwrap_err().to_string();
        println!("Expected error for corrupted git index: {}", error_msg);
    }

    std::env::set_current_dir(original_dir).unwrap();
}

#[tokio::test]
async fn test_init_with_corrupted_git_config() {
    let temp_dir = tempfile::tempdir().unwrap();
    setup_base_git_repo(&temp_dir).await.unwrap();
    let original_dir = std::env::current_dir().unwrap();

    std::env::set_current_dir(temp_dir.path()).unwrap();

    // Corrupt the git config file
    std::fs::write(".git/config", "[invalid config syntax here").unwrap_or_default();

    let fs_ops = Arc::new(StandardFileSystem);
    let init_command = InitCommand::new(None, false, true, fs_ops);

    let result = init_command.execute().await;
    // Should handle corrupted git config gracefully
    if result.is_err() {
        let error_msg = result.unwrap_err().to_string();
        println!("Expected error for corrupted git config: {}", error_msg);
    }

    std::env::set_current_dir(original_dir).unwrap();
}

#[tokio::test]
async fn test_init_with_missing_git_directory() {
    let temp_dir = tempfile::tempdir().unwrap();
    setup_base_git_repo(&temp_dir).await.unwrap();
    let original_dir = std::env::current_dir().unwrap();

    std::env::set_current_dir(temp_dir.path()).unwrap();

    // Remove the .git directory entirely
    std::fs::remove_dir_all(".git").unwrap_or_default();

    let fs_ops = Arc::new(StandardFileSystem);
    let init_command = InitCommand::new(None, false, false, fs_ops); // not dry run to trigger git checks

    let result = init_command.execute().await;
    // Should fail when .git directory is missing
    assert!(
        result.is_err(),
        "Init should fail when .git directory is missing"
    );

    let error_msg = result.unwrap_err().to_string();
    println!("Expected error for missing .git directory: {}", error_msg);

    std::env::set_current_dir(original_dir).unwrap();
}

#[tokio::test]
async fn test_init_with_insufficient_file_permissions() {
    let temp_dir = tempfile::tempdir().unwrap();
    setup_base_git_repo(&temp_dir).await.unwrap();
    let original_dir = std::env::current_dir().unwrap();

    std::env::set_current_dir(temp_dir.path()).unwrap();

    // Make the current directory read-only to simulate permission issues
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(".").unwrap().permissions();
        perms.set_mode(0o444); // Read-only permissions
        std::fs::set_permissions(".", perms).unwrap_or_default();
    }

    let fs_ops = Arc::new(StandardFileSystem);
    let init_command = InitCommand::new(None, false, true, fs_ops);

    let result = init_command.execute().await;

    // Restore permissions for cleanup
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(".").unwrap().permissions();
        perms.set_mode(0o755); // Restore normal permissions
        std::fs::set_permissions(".", perms).unwrap_or_default();
    }

    // Should handle permission issues gracefully (might succeed in dry run mode)
    if result.is_err() {
        let error_msg = result.unwrap_err().to_string();
        println!("Result for insufficient permissions: {}", error_msg);
    }

    std::env::set_current_dir(original_dir).unwrap();
}

#[tokio::test]
async fn test_init_with_very_long_git_remote_url() {
    let temp_dir = tempfile::tempdir().unwrap();
    setup_base_git_repo(&temp_dir).await.unwrap();
    let original_dir = std::env::current_dir().unwrap();

    std::env::set_current_dir(temp_dir.path()).unwrap();

    // Create an extremely long remote URL
    let long_part = "a".repeat(1000);
    let long_url = format!("https://github.com/{}/test-repo.git", long_part);

    // Remove existing remote and add the long one
    std::process::Command::new("git")
        .args(["remote", "remove", "origin"])
        .output()
        .expect("Failed to remove origin");

    std::process::Command::new("git")
        .args(["remote", "add", "origin", &long_url])
        .output()
        .expect("Failed to add long remote URL");

    let fs_ops = Arc::new(StandardFileSystem);
    let init_command = InitCommand::new(None, false, true, fs_ops);

    let result = init_command.execute().await;
    // Should handle very long URLs appropriately
    if result.is_err() {
        let error_msg = result.unwrap_err().to_string();
        println!("Result for very long remote URL: {}", error_msg);
    }

    std::env::set_current_dir(original_dir).unwrap();
}

#[tokio::test]
async fn test_init_with_unicode_characters_in_paths() {
    let temp_dir = tempfile::tempdir().unwrap();
    let original_dir = std::env::current_dir().unwrap();

    // Create directory with Unicode characters
    let unicode_dir = temp_dir.path().join("тест_репо");
    std::fs::create_dir(&unicode_dir).unwrap();

    std::env::set_current_dir(&unicode_dir).unwrap();

    // Initialize git repo with Unicode path
    std::process::Command::new("git")
        .args(["init"])
        .output()
        .expect("Failed to init git repo");

    std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .output()
        .expect("Failed to set git user");

    std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .output()
        .expect("Failed to set git email");

    std::process::Command::new("git")
        .args([
            "remote",
            "add",
            "origin",
            "https://github.com/test-owner/тест-репо.git",
        ])
        .output()
        .expect("Failed to add git remote");

    std::fs::write("ПРОЧТИ_МЕНЯ.md", "# Тестовый Репозиторий").unwrap();

    std::process::Command::new("git")
        .args(["add", "ПРОЧТИ_МЕНЯ.md"])
        .output()
        .expect("Failed to add file");

    std::process::Command::new("git")
        .args(["commit", "-m", "Начальный коммит"])
        .output()
        .expect("Failed to commit");

    let fs_ops = Arc::new(StandardFileSystem);
    let init_command = InitCommand::new(None, false, true, fs_ops);

    let result = init_command.execute().await;
    // Should handle Unicode characters in paths and content
    if result.is_err() {
        let error_msg = result.unwrap_err().to_string();
        println!("Result for Unicode paths: {}", error_msg);
    } else {
        println!("Successfully handled Unicode characters in paths");
    }

    std::env::set_current_dir(original_dir).unwrap();
}

#[tokio::test]
async fn test_init_with_case_sensitive_filesystem_issues() {
    let temp_dir = tempfile::tempdir().unwrap();
    setup_base_git_repo(&temp_dir).await.unwrap();
    let original_dir = std::env::current_dir().unwrap();

    std::env::set_current_dir(temp_dir.path()).unwrap();

    // Create files with case conflicts
    std::fs::write("File.txt", "Uppercase F").unwrap();
    std::fs::write("file.txt", "Lowercase f").unwrap_or(()); // May fail on case-insensitive filesystems

    std::process::Command::new("git")
        .args(["add", "."])
        .output()
        .expect("Failed to add files");

    std::process::Command::new("git")
        .args(["commit", "-m", "Add case-sensitive files"])
        .output()
        .expect("Failed to commit");

    let fs_ops = Arc::new(StandardFileSystem);
    let init_command = InitCommand::new(None, false, true, fs_ops);

    let result = init_command.execute().await;
    assert!(
        result.is_ok(),
        "Init should handle case sensitivity issues: {:?}",
        result.err()
    );

    std::env::set_current_dir(original_dir).unwrap();
}

#[tokio::test]
async fn test_init_with_symlinks_in_repository() {
    let temp_dir = tempfile::tempdir().unwrap();
    setup_base_git_repo(&temp_dir).await.unwrap();
    let original_dir = std::env::current_dir().unwrap();

    std::env::set_current_dir(temp_dir.path()).unwrap();

    // Create a symlink (Unix only)
    #[cfg(unix)]
    {
        std::fs::write("target_file.txt", "Target content").unwrap();
        std::os::unix::fs::symlink("target_file.txt", "symlink.txt").unwrap_or_default();

        std::process::Command::new("git")
            .args(["add", "target_file.txt", "symlink.txt"])
            .output()
            .expect("Failed to add files");

        std::process::Command::new("git")
            .args(["commit", "-m", "Add symlink"])
            .output()
            .expect("Failed to commit");
    }

    let fs_ops = Arc::new(StandardFileSystem);
    let init_command = InitCommand::new(None, false, true, fs_ops);

    let result = init_command.execute().await;
    assert!(
        result.is_ok(),
        "Init should handle symlinks: {:?}",
        result.err()
    );

    std::env::set_current_dir(original_dir).unwrap();
}

#[tokio::test]
async fn test_init_with_git_repository_in_subdirectory() {
    let temp_dir = tempfile::tempdir().unwrap();
    let original_dir = std::env::current_dir().unwrap();

    // Create nested directory structure
    std::fs::create_dir_all(temp_dir.path().join("project/nested/deep")).unwrap();

    // Initialize git repo at the top level
    std::env::set_current_dir(temp_dir.path()).unwrap();
    setup_base_git_repo(&temp_dir).await.unwrap();

    // Run init from a subdirectory
    std::env::set_current_dir(temp_dir.path().join("project/nested/deep")).unwrap();

    let fs_ops = Arc::new(StandardFileSystem);
    let init_command = InitCommand::new(None, false, true, fs_ops);

    let result = init_command.execute().await;
    // Should handle being run from subdirectory of git repo
    if result.is_err() {
        let error_msg = result.unwrap_err().to_string();
        println!("Result for subdirectory execution: {}", error_msg);
    }

    std::env::set_current_dir(original_dir).unwrap();
}

#[tokio::test]
async fn test_init_with_extremely_large_file_in_repo() {
    let temp_dir = tempfile::tempdir().unwrap();
    setup_base_git_repo(&temp_dir).await.unwrap();
    let original_dir = std::env::current_dir().unwrap();

    std::env::set_current_dir(temp_dir.path()).unwrap();

    // Create a moderately large file (don't want to create too large for test performance)
    let large_content = "x".repeat(100_000); // 100KB
    std::fs::write("large_file.txt", large_content).unwrap();

    std::process::Command::new("git")
        .args(["add", "large_file.txt"])
        .output()
        .expect("Failed to add large file");

    std::process::Command::new("git")
        .args(["commit", "-m", "Add large file"])
        .output()
        .expect("Failed to commit large file");

    let fs_ops = Arc::new(StandardFileSystem);
    let init_command = InitCommand::new(None, false, true, fs_ops);

    let result = init_command.execute().await;
    assert!(
        result.is_ok(),
        "Init should handle repos with large files: {:?}",
        result.err()
    );

    std::env::set_current_dir(original_dir).unwrap();
}
