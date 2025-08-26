/// Test fixtures for different repository states used in init command testing
use std::collections::HashMap;
use std::path::Path;
use tempfile::TempDir;
use anyhow::Result;

/// Repository state fixture that can be loaded in tests
#[derive(Debug, Clone)]
pub struct RepositoryStateFixture {
    pub name: String,
    pub description: String,
    pub files: HashMap<String, String>,
    pub git_config: GitConfig,
    pub existing_clambake_config: Option<String>,
}

/// Git repository configuration for fixtures
#[derive(Debug, Clone)]
pub struct GitConfig {
    pub initialized: bool,
    pub has_remote: bool,
    pub remote_url: Option<String>,
    pub current_branch: String,
    pub uncommitted_changes: bool,
    pub conflicted_files: Vec<String>,
}

impl Default for GitConfig {
    fn default() -> Self {
        Self {
            initialized: true,
            has_remote: true,
            remote_url: Some("https://github.com/test-owner/test-repo.git".to_string()),
            current_branch: "main".to_string(),
            uncommitted_changes: false,
            conflicted_files: Vec::new(),
        }
    }
}

impl RepositoryStateFixture {
    /// Create a fixture for an empty repository (minimal files)
    pub fn empty_repository() -> Self {
        Self {
            name: "empty_repository".to_string(),
            description: "Empty repository with only basic git structure".to_string(),
            files: HashMap::from([
                ("README.md".to_string(), "# Test Repository\n\nThis is a test repository.\n".to_string()),
                (".gitignore".to_string(), "target/\n*.log\n".to_string()),
            ]),
            git_config: GitConfig::default(),
            existing_clambake_config: None,
        }
    }

    /// Create a fixture for a repository with existing files
    pub fn repository_with_existing_files() -> Self {
        let mut files = HashMap::new();
        files.insert("README.md".to_string(), "# Existing Project\n\nThis project already has content.\n".to_string());
        files.insert(".gitignore".to_string(), "target/\n*.log\n.env\n".to_string());
        files.insert("Cargo.toml".to_string(), r#"[package]
name = "existing-project"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = "1.0"
"#.to_string());
        files.insert("src/main.rs".to_string(), r#"fn main() {
    println!("Hello, existing world!");
}
"#.to_string());
        files.insert("src/lib.rs".to_string(), r#"pub fn existing_function() -> String {
    "This is an existing function".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_existing_function() {
        assert_eq!(existing_function(), "This is an existing function");
    }
}
"#.to_string());

        Self {
            name: "repository_with_existing_files".to_string(),
            description: "Repository with substantial existing codebase".to_string(),
            files,
            git_config: GitConfig::default(),
            existing_clambake_config: None,
        }
    }

    /// Create a fixture for a repository with partial initialization
    pub fn repository_with_partial_initialization() -> Self {
        let mut files = HashMap::new();
        files.insert("README.md".to_string(), "# Partially Initialized Project\n".to_string());
        files.insert("Cargo.toml".to_string(), r#"[package]
name = "partial-project"
version = "0.1.0"
edition = "2021"
"#.to_string());
        
        // Existing partial config that should conflict with init
        let partial_config = r#"[github]
owner = "old-owner"
repo = "old-repo"

[observability]
tracing_enabled = false
"#.to_string();

        files.insert("clambake.toml".to_string(), partial_config.clone());
        files.insert(".clambake/partial_setup".to_string(), "This indicates partial setup\n".to_string());

        Self {
            name: "repository_with_partial_initialization".to_string(),
            description: "Repository with incomplete clambake setup".to_string(),
            files,
            git_config: GitConfig::default(),
            existing_clambake_config: Some(partial_config),
        }
    }

    /// Create a fixture for a repository with conflicts
    pub fn repository_with_conflicts() -> Self {
        let mut files = HashMap::new();
        files.insert("README.md".to_string(), "# Conflicted Repository\n".to_string());
        files.insert("src/main.rs".to_string(), r#"fn main() {
<<<<<<< HEAD
    println!("Version from main branch");
=======
    println!("Version from feature branch");
>>>>>>> feature-branch
}
"#.to_string());
        files.insert("Cargo.toml".to_string(), r#"[package]
name = "conflicted-project"
version = "0.1.0"
edition = "2021"

<<<<<<< HEAD
[dependencies]
serde = "1.0"
=======
[dependencies] 
tokio = "1.0"
>>>>>>> feature-branch
"#.to_string());

        let git_config = GitConfig {
            initialized: true,
            has_remote: true,
            remote_url: Some("https://github.com/test-owner/conflicted-repo.git".to_string()),
            current_branch: "main".to_string(),
            uncommitted_changes: true,
            conflicted_files: vec!["src/main.rs".to_string(), "Cargo.toml".to_string()],
        };

        Self {
            name: "repository_with_conflicts".to_string(),
            description: "Repository with merge conflicts and uncommitted changes".to_string(),
            files,
            git_config,
            existing_clambake_config: None,
        }
    }

    /// Get all available repository state fixtures
    pub fn all_fixtures() -> Vec<Self> {
        vec![
            Self::empty_repository(),
            Self::repository_with_existing_files(),
            Self::repository_with_partial_initialization(),
            Self::repository_with_conflicts(),
        ]
    }

    /// Create a temporary directory with this fixture's file structure
    pub fn create_temp_repository(&self) -> Result<TempDir> {
        let temp_dir = tempfile::tempdir()?;

        for (file_path, content) in &self.files {
            let full_path = temp_dir.path().join(file_path);
            
            // Create parent directories if needed
            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            std::fs::write(full_path, content)?;
        }

        // Initialize git repository if configured
        if self.git_config.initialized {
            self.setup_git_repository(temp_dir.path())?;
        }

        Ok(temp_dir)
    }

    /// Setup git repository in the temporary directory
    fn setup_git_repository(&self, repo_path: &Path) -> Result<()> {
        use std::process::Command;

        // Initialize git repository
        let output = Command::new("git")
            .args(["init"])
            .current_dir(repo_path)
            .output()?;
        
        if !output.status.success() {
            anyhow::bail!("Failed to initialize git repository");
        }

        // Set up basic git config for testing
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(repo_path)
            .output()?;
            
        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(repo_path)
            .output()?;

        // Add remote if configured
        if self.git_config.has_remote {
            if let Some(remote_url) = &self.git_config.remote_url {
                Command::new("git")
                    .args(["remote", "add", "origin", remote_url])
                    .current_dir(repo_path)
                    .output()?;
            }
        }

        // Add and commit files (unless there should be uncommitted changes)
        if !self.git_config.uncommitted_changes {
            Command::new("git")
                .args(["add", "."])
                .current_dir(repo_path)
                .output()?;
                
            Command::new("git")
                .args(["commit", "-m", "Initial commit"])
                .current_dir(repo_path)
                .output()?;
        } else {
            // For repositories with uncommitted changes, commit some files but leave others
            let mut committed_files = false;
            for (file_path, _) in &self.files {
                if !self.git_config.conflicted_files.contains(file_path) {
                    Command::new("git")
                        .args(["add", file_path])
                        .current_dir(repo_path)
                        .output()?;
                    committed_files = true;
                }
            }
            
            if committed_files {
                Command::new("git")
                    .args(["commit", "-m", "Partial commit"])
                    .current_dir(repo_path)
                    .output()?;
            }
        }

        Ok(())
    }

    /// Check if this fixture represents a valid state for init command testing
    pub fn is_valid_for_init_testing(&self) -> bool {
        match self.name.as_str() {
            "empty_repository" => self.existing_clambake_config.is_none(),
            "repository_with_existing_files" => self.existing_clambake_config.is_none(),
            "repository_with_partial_initialization" => self.existing_clambake_config.is_some(),
            "repository_with_conflicts" => self.git_config.uncommitted_changes,
            _ => false,
        }
    }

    /// Get expected init command behavior for this fixture
    pub fn expected_init_behavior(&self) -> InitBehaviorExpectation {
        match self.name.as_str() {
            "empty_repository" => InitBehaviorExpectation {
                should_succeed_without_force: true,
                should_create_config: true,
                should_create_directories: true,
                should_create_labels: true,
                validation_warnings: Vec::new(),
            },
            "repository_with_existing_files" => InitBehaviorExpectation {
                should_succeed_without_force: true,
                should_create_config: true,
                should_create_directories: true,
                should_create_labels: true,
                validation_warnings: Vec::new(),
            },
            "repository_with_partial_initialization" => InitBehaviorExpectation {
                should_succeed_without_force: false, // Config exists
                should_create_config: false, // Would fail without --force
                should_create_directories: true,
                should_create_labels: true,
                validation_warnings: vec!["Configuration file already exists".to_string()],
            },
            "repository_with_conflicts" => InitBehaviorExpectation {
                should_succeed_without_force: false, // Uncommitted changes
                should_create_config: false, // Would fail without --force
                should_create_directories: false,
                should_create_labels: false,
                validation_warnings: vec!["Repository has uncommitted changes".to_string()],
            },
            _ => InitBehaviorExpectation::default(),
        }
    }
}

/// Expected behavior when running init command on a fixture
#[derive(Debug, Clone)]
pub struct InitBehaviorExpectation {
    pub should_succeed_without_force: bool,
    pub should_create_config: bool,
    pub should_create_directories: bool,
    pub should_create_labels: bool,
    pub validation_warnings: Vec<String>,
}

impl Default for InitBehaviorExpectation {
    fn default() -> Self {
        Self {
            should_succeed_without_force: true,
            should_create_config: true,
            should_create_directories: true,
            should_create_labels: true,
            validation_warnings: Vec::new(),
        }
    }
}

/// Utility functions for loading and using fixtures in tests
pub struct RepositoryFixtureLoader;

impl RepositoryFixtureLoader {
    /// Load a specific fixture by name
    pub fn load_fixture(name: &str) -> Option<RepositoryStateFixture> {
        RepositoryStateFixture::all_fixtures()
            .into_iter()
            .find(|f| f.name == name)
    }

    /// Load all fixtures for comprehensive testing
    pub fn load_all_fixtures() -> Vec<RepositoryStateFixture> {
        RepositoryStateFixture::all_fixtures()
    }

    /// Load fixtures suitable for specific test scenarios
    pub fn load_init_command_fixtures() -> Vec<RepositoryStateFixture> {
        RepositoryStateFixture::all_fixtures()
            .into_iter()
            .filter(|f| f.is_valid_for_init_testing())
            .collect()
    }

    /// Create a temporary repository from fixture name
    pub fn create_temp_repository_from_name(name: &str) -> Result<Option<TempDir>> {
        if let Some(fixture) = Self::load_fixture(name) {
            Ok(Some(fixture.create_temp_repository()?))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_repository_fixture() {
        let fixture = RepositoryStateFixture::empty_repository();
        assert_eq!(fixture.name, "empty_repository");
        assert!(fixture.is_valid_for_init_testing());
        assert!(fixture.existing_clambake_config.is_none());
        assert!(fixture.files.contains_key("README.md"));
        assert!(fixture.files.contains_key(".gitignore"));
    }

    #[test]
    fn test_repository_with_existing_files_fixture() {
        let fixture = RepositoryStateFixture::repository_with_existing_files();
        assert_eq!(fixture.name, "repository_with_existing_files");
        assert!(fixture.is_valid_for_init_testing());
        assert!(fixture.existing_clambake_config.is_none());
        assert!(fixture.files.contains_key("Cargo.toml"));
        assert!(fixture.files.contains_key("src/main.rs"));
    }

    #[test]
    fn test_repository_with_partial_initialization_fixture() {
        let fixture = RepositoryStateFixture::repository_with_partial_initialization();
        assert_eq!(fixture.name, "repository_with_partial_initialization");
        assert!(fixture.is_valid_for_init_testing());
        assert!(fixture.existing_clambake_config.is_some());
        assert!(fixture.files.contains_key("clambake.toml"));
        
        let behavior = fixture.expected_init_behavior();
        assert!(!behavior.should_succeed_without_force);
    }

    #[test]
    fn test_repository_with_conflicts_fixture() {
        let fixture = RepositoryStateFixture::repository_with_conflicts();
        assert_eq!(fixture.name, "repository_with_conflicts");
        assert!(fixture.is_valid_for_init_testing());
        assert!(fixture.git_config.uncommitted_changes);
        assert!(!fixture.git_config.conflicted_files.is_empty());
        
        let behavior = fixture.expected_init_behavior();
        assert!(!behavior.should_succeed_without_force);
    }

    #[test]
    fn test_all_fixtures_loaded() {
        let fixtures = RepositoryStateFixture::all_fixtures();
        assert_eq!(fixtures.len(), 4);
        
        let names: Vec<String> = fixtures.iter().map(|f| f.name.clone()).collect();
        assert!(names.contains(&"empty_repository".to_string()));
        assert!(names.contains(&"repository_with_existing_files".to_string()));
        assert!(names.contains(&"repository_with_partial_initialization".to_string()));
        assert!(names.contains(&"repository_with_conflicts".to_string()));
    }

    #[test]
    fn test_fixture_loader_functionality() {
        let fixture = RepositoryFixtureLoader::load_fixture("empty_repository");
        assert!(fixture.is_some());
        assert_eq!(fixture.unwrap().name, "empty_repository");
        
        let nonexistent = RepositoryFixtureLoader::load_fixture("nonexistent_fixture");
        assert!(nonexistent.is_none());
    }

    #[test]
    fn test_init_command_fixtures_filtering() {
        let fixtures = RepositoryFixtureLoader::load_init_command_fixtures();
        assert!(!fixtures.is_empty());
        
        for fixture in fixtures {
            assert!(fixture.is_valid_for_init_testing());
        }
    }

    #[tokio::test]
    async fn test_temp_repository_creation() {
        let fixture = RepositoryStateFixture::empty_repository();
        let temp_repo = fixture.create_temp_repository();
        
        assert!(temp_repo.is_ok());
        let temp_dir = temp_repo.unwrap();
        
        // Verify files were created
        let readme_path = temp_dir.path().join("README.md");
        assert!(readme_path.exists());
        
        let gitignore_path = temp_dir.path().join(".gitignore");
        assert!(gitignore_path.exists());
        
        // Verify git repository was initialized
        let git_dir = temp_dir.path().join(".git");
        assert!(git_dir.exists());
    }
}