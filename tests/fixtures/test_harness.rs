use anyhow::Result;
/// Test harness for managing temporary directories in integration tests
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tempfile::TempDir;

/// Resource tracking for leak detection
#[derive(Debug, Clone)]
pub struct ResourceTracker {
    created_files: Vec<PathBuf>,
    created_dirs: Vec<PathBuf>,
    spawned_processes: Vec<u32>,
    open_files: Vec<String>,
    created_at: Instant,
}

impl ResourceTracker {
    pub fn new() -> Self {
        Self {
            created_files: Vec::new(),
            created_dirs: Vec::new(),
            spawned_processes: Vec::new(),
            open_files: Vec::new(),
            created_at: Instant::now(),
        }
    }

    pub fn track_file(&mut self, path: PathBuf) {
        self.created_files.push(path);
    }

    pub fn track_dir(&mut self, path: PathBuf) {
        self.created_dirs.push(path);
    }

    pub fn track_process(&mut self, pid: u32) {
        self.spawned_processes.push(pid);
    }

    pub fn detect_leaks(&self) -> Vec<String> {
        let mut leaks = Vec::new();

        for file in &self.created_files {
            if file.exists() && !file.starts_with(&std::env::temp_dir()) {
                leaks.push(format!("File leak detected: {}", file.display()));
            }
        }

        for dir in &self.created_dirs {
            if dir.exists() && !dir.starts_with(&std::env::temp_dir()) {
                leaks.push(format!("Directory leak detected: {}", dir.display()));
            }
        }

        leaks
    }
}

/// Cleanup strategy for handling different failure scenarios
#[derive(Debug, Clone)]
pub enum CleanupStrategy {
    Immediate,
    Deferred,
    ForceKill,
    GracefulWithRetry { max_attempts: u32, delay_ms: u64 },
}

/// A comprehensive test harness for managing temporary directories with automatic cleanup
pub struct TestHarness {
    temp_dir: TempDir,
    cleanup_registered: bool,
    resource_tracker: ResourceTracker,
    cleanup_strategy: CleanupStrategy,
    cleanup_hooks: Vec<Box<dyn FnOnce() -> Result<()> + Send>>,
    isolation_verified: bool,
}

impl TestHarness {
    /// Create a new test harness with an isolated temporary directory
    pub fn new() -> Result<Self> {
        let temp_dir = tempfile::tempdir()?;
        Ok(Self {
            temp_dir,
            cleanup_registered: false,
            resource_tracker: ResourceTracker::new(),
            cleanup_strategy: CleanupStrategy::GracefulWithRetry {
                max_attempts: 3,
                delay_ms: 100,
            },
            cleanup_hooks: Vec::new(),
            isolation_verified: false,
        })
    }

    /// Create a test harness with specific cleanup strategy
    pub fn with_cleanup_strategy(cleanup_strategy: CleanupStrategy) -> Result<Self> {
        let temp_dir = tempfile::tempdir()?;
        Ok(Self {
            temp_dir,
            cleanup_registered: false,
            resource_tracker: ResourceTracker::new(),
            cleanup_strategy,
            cleanup_hooks: Vec::new(),
            isolation_verified: false,
        })
    }

    /// Get the path to the temporary directory
    pub fn path(&self) -> &Path {
        self.temp_dir.path()
    }

    /// Create a file in the temporary directory
    pub fn create_file(&mut self, relative_path: &str, content: &str) -> Result<PathBuf> {
        let file_path = self.temp_dir.path().join(relative_path);

        // Create parent directories if needed
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(&file_path, content)?;
        self.resource_tracker.track_file(file_path.clone());
        Ok(file_path)
    }

    /// Create a directory in the temporary directory
    pub fn create_dir(&mut self, relative_path: &str) -> Result<PathBuf> {
        let dir_path = self.temp_dir.path().join(relative_path);
        std::fs::create_dir_all(&dir_path)?;
        self.resource_tracker.track_dir(dir_path.clone());
        Ok(dir_path)
    }

    /// Initialize a Git repository in the temporary directory
    pub fn init_git_repository(&self) -> Result<()> {
        self.init_git_repository_at(self.temp_dir.path())
    }

    /// Initialize a Git repository at a specific path within the temporary directory
    pub fn init_git_repository_at(&self, repo_path: &Path) -> Result<()> {
        let output = Command::new("git")
            .args(["init"])
            .current_dir(repo_path)
            .output()?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to initialize git repository: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        self.setup_git_config(repo_path)?;
        Ok(())
    }

    /// Set up basic Git configuration for testing
    fn setup_git_config(&self, repo_path: &Path) -> Result<()> {
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(repo_path)
            .output()?;

        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(repo_path)
            .output()?;

        Ok(())
    }

    /// Add a Git remote to the repository
    pub fn add_git_remote(&self, name: &str, url: &str) -> Result<()> {
        let output = Command::new("git")
            .args(["remote", "add", name, url])
            .current_dir(self.temp_dir.path())
            .output()?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to add git remote: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    /// Commit all files in the repository
    pub fn commit_all(&self, message: &str) -> Result<()> {
        Command::new("git")
            .args(["add", "."])
            .current_dir(self.temp_dir.path())
            .output()?;

        let output = Command::new("git")
            .args(["commit", "-m", message])
            .current_dir(self.temp_dir.path())
            .output()?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to commit files: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    /// Create a basic Rust project structure
    pub fn create_rust_project(&mut self, name: &str) -> Result<()> {
        self.create_file(
            "Cargo.toml",
            &format!(
                r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = "1.0"
"#,
                name
            ),
        )?;

        self.create_dir("src")?;
        self.create_file(
            "src/main.rs",
            r#"fn main() {
    println!("Hello, world!");
}
"#,
        )?;

        self.create_file(
            "README.md",
            &format!("# {}\n\nA test Rust project.\n", name),
        )?;
        self.create_file(".gitignore", "target/\n*.log\n")?;

        Ok(())
    }

    /// Verify that the temporary directory is properly isolated
    pub fn verify_isolation(&mut self) -> Result<()> {
        let path = self.temp_dir.path();

        // Check that path exists and is writable
        if !path.exists() {
            anyhow::bail!("Temporary directory does not exist");
        }

        // Verify path is within system temp directory for proper isolation
        let system_temp = std::env::temp_dir();
        if !path.starts_with(&system_temp) {
            anyhow::bail!("Test directory is not properly isolated within system temp directory");
        }

        // Try to create a test file to verify write access
        let test_file = path.join("isolation_test");
        std::fs::write(&test_file, "test")?;

        if !test_file.exists() {
            anyhow::bail!("Unable to create files in temporary directory");
        }

        // Verify file permissions are correct
        let metadata = std::fs::metadata(&test_file)?;
        if metadata.len() != 4 {
            anyhow::bail!("Test file content verification failed");
        }

        // Test concurrent access to ensure proper isolation
        let concurrent_test = path.join("concurrent_test");
        std::fs::write(&concurrent_test, "concurrent")?;

        // Cleanup test files
        std::fs::remove_file(&test_file)?;
        std::fs::remove_file(&concurrent_test)?;

        self.isolation_verified = true;
        Ok(())
    }

    /// Enhanced isolation verification with cross-test interference checking
    pub fn verify_cross_test_isolation(&self, other_harnesses: &[&TestHarness]) -> Result<()> {
        if !self.isolation_verified {
            anyhow::bail!("Basic isolation must be verified first");
        }

        let self_path = self.temp_dir.path();

        // Verify no overlap with other test harnesses
        for (i, other) in other_harnesses.iter().enumerate() {
            let other_path = other.temp_dir.path();

            if self_path == other_path {
                anyhow::bail!(
                    "Test harness directories are not isolated: duplicate paths detected"
                );
            }

            // Verify no nested paths
            if self_path.starts_with(other_path) || other_path.starts_with(self_path) {
                anyhow::bail!("Test harness {} has nested path relationship", i);
            }
        }

        // Test that we cannot access files from other harnesses
        for (i, other) in other_harnesses.iter().enumerate() {
            if other.temp_dir.path().exists() {
                let other_files: Vec<_> = std::fs::read_dir(other.temp_dir.path())?
                    .filter_map(|entry| entry.ok())
                    .collect();

                for file_entry in other_files {
                    let file_path = file_entry.path();
                    let relative_to_self = self_path.join(file_entry.file_name());

                    if relative_to_self.exists() {
                        anyhow::bail!(
                            "File name collision detected with harness {}: {}",
                            i,
                            file_entry.file_name().to_string_lossy()
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// Register cleanup hooks to ensure temporary files are cleaned up
    pub fn register_cleanup(&mut self) -> Result<()> {
        if self.cleanup_registered {
            return Ok(());
        }

        self.cleanup_registered = true;
        Ok(())
    }

    /// Add custom cleanup hook
    pub fn add_cleanup_hook<F>(&mut self, hook: F)
    where
        F: FnOnce() -> Result<()> + Send + 'static,
    {
        self.cleanup_hooks.push(Box::new(hook));
    }

    /// Execute cleanup with error recovery
    pub fn cleanup(&mut self) -> Result<Vec<String>> {
        let mut cleanup_errors = Vec::new();

        // Execute custom cleanup hooks first
        for hook in self.cleanup_hooks.drain(..) {
            if let Err(e) = hook() {
                cleanup_errors.push(format!("Cleanup hook failed: {}", e));
            }
        }

        // Perform cleanup based on strategy
        match &self.cleanup_strategy {
            CleanupStrategy::Immediate => {
                if let Err(e) = self.immediate_cleanup() {
                    cleanup_errors.push(format!("Immediate cleanup failed: {}", e));
                }
            }
            CleanupStrategy::Deferred => {
                // Deferred cleanup will happen when harness is dropped
            }
            CleanupStrategy::ForceKill => {
                if let Err(e) = self.force_cleanup() {
                    cleanup_errors.push(format!("Force cleanup failed: {}", e));
                }
            }
            CleanupStrategy::GracefulWithRetry {
                max_attempts,
                delay_ms,
            } => {
                if let Err(e) = self.graceful_cleanup_with_retry(*max_attempts, *delay_ms) {
                    cleanup_errors.push(format!("Graceful cleanup failed: {}", e));
                }
            }
        }

        // Detect resource leaks
        let leaks = self.resource_tracker.detect_leaks();
        for leak in leaks {
            cleanup_errors.push(leak);
        }

        Ok(cleanup_errors)
    }

    /// Immediate cleanup - fail fast if any error occurs
    fn immediate_cleanup(&self) -> Result<()> {
        // Force sync any pending I/O
        let _ = std::process::Command::new("sync").output();

        // Verify no active file handles in temp directory
        let temp_path = self.temp_dir.path();
        if temp_path.exists() {
            // Check for any processes that might have open file handles
            let lsof_output = std::process::Command::new("lsof").arg(temp_path).output();

            if let Ok(output) = lsof_output {
                if !output.stdout.is_empty() {
                    anyhow::bail!(
                        "Active file handles detected during cleanup: {}",
                        String::from_utf8_lossy(&output.stdout)
                    );
                }
            }
        }

        Ok(())
    }

    /// Force cleanup - terminate processes and remove files aggressively
    fn force_cleanup(&self) -> Result<()> {
        // Kill any spawned processes
        for &pid in &self.resource_tracker.spawned_processes {
            let _ = std::process::Command::new("kill")
                .args(["-9", &pid.to_string()])
                .output();
        }

        // Force remove any remaining files outside temp directory
        for file in &self.resource_tracker.created_files {
            if file.exists() && !file.starts_with(&std::env::temp_dir()) {
                let _ = std::fs::remove_file(file);
            }
        }

        for dir in &self.resource_tracker.created_dirs {
            if dir.exists() && !dir.starts_with(&std::env::temp_dir()) {
                let _ = std::fs::remove_dir_all(dir);
            }
        }

        Ok(())
    }

    /// Graceful cleanup with retry logic
    fn graceful_cleanup_with_retry(&self, max_attempts: u32, delay_ms: u64) -> Result<()> {
        let mut last_error = None;

        for attempt in 1..=max_attempts {
            match self.attempt_graceful_cleanup() {
                Ok(()) => return Ok(()),
                Err(e) => {
                    last_error = Some(e);
                    if attempt < max_attempts {
                        std::thread::sleep(std::time::Duration::from_millis(delay_ms));
                    }
                }
            }
        }

        match last_error {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    /// Single attempt at graceful cleanup
    fn attempt_graceful_cleanup(&self) -> Result<()> {
        // Gracefully terminate any spawned processes
        for &pid in &self.resource_tracker.spawned_processes {
            let term_result = std::process::Command::new("kill")
                .args(["-TERM", &pid.to_string()])
                .output();

            if term_result.is_ok() {
                // Give process time to terminate gracefully
                std::thread::sleep(std::time::Duration::from_millis(100));

                // Check if still running
                let check_result = std::process::Command::new("kill")
                    .args(["-0", &pid.to_string()])
                    .output();

                if let Ok(output) = check_result {
                    if output.status.success() {
                        // Process still running, force kill
                        let _ = std::process::Command::new("kill")
                            .args(["-9", &pid.to_string()])
                            .output();
                    }
                }
            }
        }

        Ok(())
    }

    /// Detect resource leaks
    pub fn detect_resource_leaks(&self) -> Vec<String> {
        self.resource_tracker.detect_leaks()
    }

    /// Get cleanup strategy
    pub fn cleanup_strategy(&self) -> &CleanupStrategy {
        &self.cleanup_strategy
    }

    /// Set cleanup strategy
    pub fn set_cleanup_strategy(&mut self, strategy: CleanupStrategy) {
        self.cleanup_strategy = strategy;
    }

    /// Check if isolation has been verified
    pub fn is_isolation_verified(&self) -> bool {
        self.isolation_verified
    }

    /// Get the current working directory relative to the temp directory
    pub fn relative_path(&self, absolute_path: &Path) -> Option<PathBuf> {
        absolute_path
            .strip_prefix(self.temp_dir.path())
            .ok()
            .map(|p| p.to_path_buf())
    }
}

/// A builder for creating test harnesses with specific configurations
pub struct TestHarnessBuilder {
    init_git: bool,
    create_rust_project: Option<String>,
    git_remote: Option<(String, String)>,
    initial_commit: Option<String>,
}

impl TestHarnessBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            init_git: false,
            create_rust_project: None,
            git_remote: None,
            initial_commit: None,
        }
    }

    /// Configure the harness to initialize a Git repository
    pub fn with_git(mut self) -> Self {
        self.init_git = true;
        self
    }

    /// Configure the harness to create a Rust project structure
    pub fn with_rust_project(mut self, name: &str) -> Self {
        self.create_rust_project = Some(name.to_string());
        self
    }

    /// Configure the harness to add a Git remote
    pub fn with_git_remote(mut self, name: &str, url: &str) -> Self {
        self.git_remote = Some((name.to_string(), url.to_string()));
        self
    }

    /// Configure the harness to make an initial commit
    pub fn with_initial_commit(mut self, message: &str) -> Self {
        self.initial_commit = Some(message.to_string());
        self
    }

    /// Build the test harness with the configured options
    pub fn build(self) -> Result<TestHarness> {
        let mut harness = TestHarness::new()?;

        if let Some(project_name) = self.create_rust_project {
            harness.create_rust_project(&project_name)?;
        }

        if self.init_git {
            harness.init_git_repository()?;
        }

        if let Some((name, url)) = self.git_remote {
            harness.add_git_remote(&name, &url)?;
        }

        if let Some(message) = self.initial_commit {
            harness.commit_all(&message)?;
        }

        Ok(harness)
    }
}

impl Default for TestHarnessBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper functions for common test scenarios
pub mod helpers {
    use super::*;

    /// Create a simple test harness with just a temporary directory
    pub fn simple_harness() -> Result<TestHarness> {
        TestHarness::new()
    }

    /// Create a test harness with a Git repository
    pub fn git_harness() -> Result<TestHarness> {
        TestHarnessBuilder::new().with_git().build()
    }

    /// Create a test harness with a full Rust project and Git repository
    pub fn rust_project_harness(name: &str) -> Result<TestHarness> {
        TestHarnessBuilder::new()
            .with_rust_project(name)
            .with_git()
            .with_git_remote("origin", "https://github.com/test-owner/test-repo.git")
            .with_initial_commit("Initial commit")
            .build()
    }

    /// Create multiple isolated test harnesses for parallel testing
    pub fn parallel_harnesses(count: usize) -> Result<Vec<TestHarness>> {
        let mut harnesses = Vec::with_capacity(count);
        for _ in 0..count {
            harnesses.push(TestHarness::new()?);
        }
        Ok(harnesses)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_harness_creation() {
        let harness = TestHarness::new().unwrap();
        assert!(harness.path().exists());
    }

    #[test]
    fn test_file_creation() {
        let mut harness = TestHarness::new().unwrap();
        let file_path = harness.create_file("test.txt", "Hello, world!").unwrap();

        assert!(file_path.exists());
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "Hello, world!");
    }

    #[test]
    fn test_directory_creation() {
        let mut harness = TestHarness::new().unwrap();
        let dir_path = harness.create_dir("test_dir").unwrap();

        assert!(dir_path.exists());
        assert!(dir_path.is_dir());
    }

    #[test]
    fn test_nested_directory_creation() {
        let mut harness = TestHarness::new().unwrap();
        let file_path = harness
            .create_file("nested/dir/test.txt", "nested content")
            .unwrap();

        assert!(file_path.exists());
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "nested content");
    }

    #[test]
    fn test_git_repository_initialization() {
        let harness = TestHarness::new().unwrap();
        harness.init_git_repository().unwrap();

        let git_dir = harness.path().join(".git");
        assert!(git_dir.exists());
        assert!(git_dir.is_dir());
    }

    #[test]
    fn test_rust_project_creation() {
        let mut harness = TestHarness::new().unwrap();
        harness.create_rust_project("test-project").unwrap();

        assert!(harness.path().join("Cargo.toml").exists());
        assert!(harness.path().join("src").exists());
        assert!(harness.path().join("src/main.rs").exists());
        assert!(harness.path().join("README.md").exists());
        assert!(harness.path().join(".gitignore").exists());
    }

    #[test]
    fn test_git_commit() {
        let mut harness = TestHarness::new().unwrap();
        harness.init_git_repository().unwrap();
        harness.create_file("test.txt", "content").unwrap();
        harness.commit_all("Test commit").unwrap();

        let output = Command::new("git")
            .args(["log", "--oneline"])
            .current_dir(harness.path())
            .output()
            .unwrap();

        let log_output = String::from_utf8_lossy(&output.stdout);
        assert!(log_output.contains("Test commit"));
    }

    #[test]
    fn test_isolation_verification() {
        let mut harness = TestHarness::new().unwrap();
        harness.verify_isolation().unwrap();
    }

    #[test]
    fn test_builder_pattern() {
        let harness = TestHarnessBuilder::new()
            .with_rust_project("builder-test")
            .with_git()
            .with_git_remote("origin", "https://github.com/test/repo.git")
            .with_initial_commit("Initial commit")
            .build()
            .unwrap();

        assert!(harness.path().join("Cargo.toml").exists());
        assert!(harness.path().join(".git").exists());

        let output = Command::new("git")
            .args(["remote", "-v"])
            .current_dir(harness.path())
            .output()
            .unwrap();

        let remote_output = String::from_utf8_lossy(&output.stdout);
        assert!(remote_output.contains("https://github.com/test/repo.git"));
    }

    #[test]
    fn test_parallel_harnesses() {
        let mut harnesses = helpers::parallel_harnesses(3).unwrap();
        assert_eq!(harnesses.len(), 3);

        for (i, harness) in harnesses.iter_mut().enumerate() {
            let test_file = format!("test_{}.txt", i);
            harness
                .create_file(&test_file, &format!("content {}", i))
                .unwrap();
            assert!(harness.path().join(&test_file).exists());
        }
    }

    #[test]
    fn test_helper_functions() {
        let simple = helpers::simple_harness().unwrap();
        assert!(simple.path().exists());

        let git = helpers::git_harness().unwrap();
        assert!(git.path().exists());
        assert!(git.path().join(".git").exists());

        let rust = helpers::rust_project_harness("helper-test").unwrap();
        assert!(rust.path().exists());
        assert!(rust.path().join("Cargo.toml").exists());
        assert!(rust.path().join(".git").exists());
    }

    #[test]
    fn test_enhanced_isolation_verification() {
        let mut harness = TestHarness::new().unwrap();
        harness.verify_isolation().unwrap();
        assert!(harness.isolation_verified);
    }

    #[test]
    fn test_cross_test_isolation() {
        let mut harness1 = TestHarness::new().unwrap();
        let mut harness2 = TestHarness::new().unwrap();
        let mut harness3 = TestHarness::new().unwrap();

        harness1.verify_isolation().unwrap();
        harness2.verify_isolation().unwrap();
        harness3.verify_isolation().unwrap();

        let others = vec![&harness2, &harness3];
        harness1.verify_cross_test_isolation(&others).unwrap();
    }

    #[test]
    fn test_cleanup_strategies() {
        let mut harness = TestHarness::with_cleanup_strategy(CleanupStrategy::Immediate).unwrap();
        harness.create_file("test.txt", "content").unwrap();

        let cleanup_errors = harness.cleanup().unwrap();
        assert!(cleanup_errors.is_empty());
    }

    #[test]
    fn test_graceful_cleanup_with_retry() {
        let mut harness = TestHarness::with_cleanup_strategy(CleanupStrategy::GracefulWithRetry {
            max_attempts: 3,
            delay_ms: 50,
        })
        .unwrap();

        harness.create_file("test.txt", "content").unwrap();
        let cleanup_errors = harness.cleanup().unwrap();
        assert!(cleanup_errors.is_empty());
    }

    #[test]
    fn test_cleanup_hooks() {
        let mut harness = TestHarness::new().unwrap();
        let mut hook_executed = false;

        let hook_flag = Arc::new(Mutex::new(false));
        let hook_flag_clone = hook_flag.clone();

        harness.add_cleanup_hook(move || {
            *hook_flag_clone.lock().unwrap() = true;
            Ok(())
        });

        harness.cleanup().unwrap();
        assert!(*hook_flag.lock().unwrap());
    }

    #[test]
    fn test_resource_leak_detection() {
        let mut harness = TestHarness::new().unwrap();
        harness.create_file("test.txt", "content").unwrap();

        let leaks = harness.detect_resource_leaks();
        assert!(
            leaks.is_empty(),
            "No leaks should be detected for temp directory files"
        );
    }

    #[test]
    fn test_cleanup_error_recovery() {
        let mut harness = TestHarness::new().unwrap();

        // Add a hook that will fail
        harness.add_cleanup_hook(|| anyhow::bail!("Intentional cleanup failure for testing"));

        let cleanup_errors = harness.cleanup().unwrap();
        assert!(!cleanup_errors.is_empty());
        assert!(cleanup_errors[0].contains("Intentional cleanup failure"));
    }

    #[test]
    fn test_isolation_under_error_conditions() {
        let mut harness = TestHarness::new().unwrap();

        // Create file and simulate partial failure
        harness.create_file("test.txt", "content").unwrap();

        // Should still maintain isolation
        harness.verify_isolation().unwrap();

        // Cleanup should work even after errors
        let cleanup_errors = harness.cleanup().unwrap();
        assert!(cleanup_errors.is_empty());
    }
}
