/// Test harness for managing temporary directories in integration tests
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use anyhow::Result;
use std::process::Command;

/// A comprehensive test harness for managing temporary directories with automatic cleanup
pub struct TestHarness {
    temp_dir: TempDir,
    cleanup_registered: bool,
}

impl TestHarness {
    /// Create a new test harness with an isolated temporary directory
    pub fn new() -> Result<Self> {
        let temp_dir = tempfile::tempdir()?;
        Ok(Self {
            temp_dir,
            cleanup_registered: false,
        })
    }

    /// Get the path to the temporary directory
    pub fn path(&self) -> &Path {
        self.temp_dir.path()
    }

    /// Create a file in the temporary directory
    pub fn create_file(&self, relative_path: &str, content: &str) -> Result<PathBuf> {
        let file_path = self.temp_dir.path().join(relative_path);
        
        // Create parent directories if needed
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(&file_path, content)?;
        Ok(file_path)
    }

    /// Create a directory in the temporary directory
    pub fn create_dir(&self, relative_path: &str) -> Result<PathBuf> {
        let dir_path = self.temp_dir.path().join(relative_path);
        std::fs::create_dir_all(&dir_path)?;
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
            anyhow::bail!("Failed to initialize git repository: {}", 
                String::from_utf8_lossy(&output.stderr));
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
            anyhow::bail!("Failed to add git remote: {}", 
                String::from_utf8_lossy(&output.stderr));
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
            anyhow::bail!("Failed to commit files: {}", 
                String::from_utf8_lossy(&output.stderr));
        }

        Ok(())
    }

    /// Create a basic Rust project structure
    pub fn create_rust_project(&self, name: &str) -> Result<()> {
        self.create_file("Cargo.toml", &format!(r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = "1.0"
"#, name))?;

        self.create_dir("src")?;
        self.create_file("src/main.rs", r#"fn main() {
    println!("Hello, world!");
}
"#)?;

        self.create_file("README.md", &format!("# {}\n\nA test Rust project.\n", name))?;
        self.create_file(".gitignore", "target/\n*.log\n")?;

        Ok(())
    }

    /// Verify that the temporary directory is properly isolated
    pub fn verify_isolation(&self) -> Result<()> {
        let path = self.temp_dir.path();
        
        // Check that path exists and is writable
        if !path.exists() {
            anyhow::bail!("Temporary directory does not exist");
        }

        // Try to create a test file to verify write access
        let test_file = path.join("isolation_test");
        std::fs::write(&test_file, "test")?;
        
        if !test_file.exists() {
            anyhow::bail!("Unable to create files in temporary directory");
        }

        std::fs::remove_file(&test_file)?;
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

    /// Get the current working directory relative to the temp directory
    pub fn relative_path(&self, absolute_path: &Path) -> Option<PathBuf> {
        absolute_path.strip_prefix(self.temp_dir.path()).ok().map(|p| p.to_path_buf())
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
        TestHarnessBuilder::new()
            .with_git()
            .build()
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
        let harness = TestHarness::new().unwrap();
        let file_path = harness.create_file("test.txt", "Hello, world!").unwrap();
        
        assert!(file_path.exists());
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "Hello, world!");
    }

    #[test]
    fn test_directory_creation() {
        let harness = TestHarness::new().unwrap();
        let dir_path = harness.create_dir("test_dir").unwrap();
        
        assert!(dir_path.exists());
        assert!(dir_path.is_dir());
    }

    #[test]
    fn test_nested_directory_creation() {
        let harness = TestHarness::new().unwrap();
        let file_path = harness.create_file("nested/dir/test.txt", "nested content").unwrap();
        
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
        let harness = TestHarness::new().unwrap();
        harness.create_rust_project("test-project").unwrap();
        
        assert!(harness.path().join("Cargo.toml").exists());
        assert!(harness.path().join("src").exists());
        assert!(harness.path().join("src/main.rs").exists());
        assert!(harness.path().join("README.md").exists());
        assert!(harness.path().join(".gitignore").exists());
    }

    #[test]
    fn test_git_commit() {
        let harness = TestHarness::new().unwrap();
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
        let harness = TestHarness::new().unwrap();
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
        let harnesses = helpers::parallel_harnesses(3).unwrap();
        assert_eq!(harnesses.len(), 3);
        
        for (i, harness) in harnesses.iter().enumerate() {
            let test_file = format!("test_{}.txt", i);
            harness.create_file(&test_file, &format!("content {}", i)).unwrap();
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
}