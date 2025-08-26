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

    /// Create a fixture for a repository with complex directory structure
    pub fn repository_with_complex_directory_structure() -> Self {
        let mut files = HashMap::new();
        
        // Root level files
        files.insert("README.md".to_string(), "# Complex Project\n\nA project with nested directory structure.\n".to_string());
        files.insert(".gitignore".to_string(), "target/\n*.log\nnode_modules/\n.env\n".to_string());
        files.insert("Cargo.toml".to_string(), r#"[package]
name = "complex-project"
version = "0.2.0"
edition = "2021"

[workspace]
members = ["crates/*", "services/*"]

[dependencies]
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
"#.to_string());

        // Source directory structure
        files.insert("src/main.rs".to_string(), r#"mod config;
mod utils;

fn main() {
    println!("Complex project entry point");
}
"#.to_string());
        files.insert("src/config/mod.rs".to_string(), r#"pub mod database;
pub mod logging;

pub struct AppConfig {
    pub port: u16,
    pub debug: bool,
}
"#.to_string());
        files.insert("src/config/database.rs".to_string(), r#"use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub pool_size: u32,
}
"#.to_string());
        files.insert("src/config/logging.rs".to_string(), r#"pub struct LoggingConfig {
    pub level: String,
    pub file_path: Option<String>,
}
"#.to_string());
        files.insert("src/utils/mod.rs".to_string(), r#"pub mod helpers;
pub mod validation;
"#.to_string());
        files.insert("src/utils/helpers.rs".to_string(), r#"pub fn format_timestamp() -> String {
    "2024-01-01T00:00:00Z".to_string()
}
"#.to_string());
        files.insert("src/utils/validation.rs".to_string(), r#"pub fn is_valid_email(email: &str) -> bool {
    email.contains('@')
}
"#.to_string());

        // Workspace crates
        files.insert("crates/shared/Cargo.toml".to_string(), r#"[package]
name = "shared"
version = "0.1.0"
edition = "2021"
"#.to_string());
        files.insert("crates/shared/src/lib.rs".to_string(), r#"pub mod types;

pub use types::*;
"#.to_string());
        files.insert("crates/shared/src/types.rs".to_string(), r#"pub struct SharedType {
    pub id: u64,
    pub name: String,
}
"#.to_string());

        // Services directory
        files.insert("services/api/Cargo.toml".to_string(), r#"[package]
name = "api-service"
version = "0.1.0"
edition = "2021"

[dependencies]
shared = { path = "../../crates/shared" }
"#.to_string());
        files.insert("services/api/src/main.rs".to_string(), r#"use shared::SharedType;

fn main() {
    let item = SharedType {
        id: 1,
        name: "API Service".to_string(),
    };
    println!("Starting API service with: {:?}", item);
}
"#.to_string());

        // Tests and docs
        files.insert("tests/integration_tests.rs".to_string(), r#"#[tokio::test]
async fn test_complex_integration() {
    // Integration test for complex structure
    assert!(true);
}
"#.to_string());
        files.insert("docs/architecture.md".to_string(), "# Architecture\n\nThis describes the complex project architecture.\n".to_string());
        files.insert("docs/deployment/README.md".to_string(), "# Deployment Guide\n\nHow to deploy this complex project.\n".to_string());

        // Configuration files
        files.insert("config/development.toml".to_string(), r#"[database]
url = "postgresql://localhost/dev_db"
pool_size = 10

[server]
port = 8080
debug = true
"#.to_string());
        files.insert("config/production.toml".to_string(), r#"[database]
url = "postgresql://prod-server/prod_db"
pool_size = 20

[server]
port = 80
debug = false
"#.to_string());

        Self {
            name: "repository_with_complex_directory_structure".to_string(),
            description: "Repository with nested directories, workspace structure, and multiple modules".to_string(),
            files,
            git_config: GitConfig::default(),
            existing_clambake_config: None,
        }
    }

    /// Create a fixture for a repository with existing issue templates
    pub fn repository_with_existing_issue_templates() -> Self {
        let mut files = HashMap::new();
        
        // Basic project files
        files.insert("README.md".to_string(), "# Project with Issue Templates\n\nA repository with GitHub issue templates.\n".to_string());
        files.insert(".gitignore".to_string(), "target/\n*.log\n".to_string());
        files.insert("Cargo.toml".to_string(), r#"[package]
name = "issue-template-project"
version = "0.1.0"
edition = "2021"
"#.to_string());

        // Issue templates
        files.insert(".github/ISSUE_TEMPLATE/bug_report.md".to_string(), r#"---
name: Bug report
about: Create a report to help us improve
title: ''
labels: 'bug'
assignees: ''
---

**Describe the bug**
A clear and concise description of what the bug is.

**To Reproduce**
Steps to reproduce the behavior:
1. Go to '...'
2. Click on '....'
3. Scroll down to '....'
4. See error

**Expected behavior**
A clear and concise description of what you expected to happen.

**Additional context**
Add any other context about the problem here.
"#.to_string());

        files.insert(".github/ISSUE_TEMPLATE/feature_request.md".to_string(), r#"---
name: Feature request
about: Suggest an idea for this project
title: ''
labels: 'enhancement'
assignees: ''
---

**Is your feature request related to a problem? Please describe.**
A clear and concise description of what the problem is. Ex. I'm always frustrated when [...]

**Describe the solution you'd like**
A clear and concise description of what you want to happen.

**Describe alternatives you've considered**
A clear and concise description of any alternative solutions or features you've considered.

**Additional context**
Add any other context or screenshots about the feature request here.
"#.to_string());

        files.insert(".github/ISSUE_TEMPLATE/config.yml".to_string(), r#"blank_issues_enabled: false
contact_links:
  - name: Community Discord
    url: https://discord.gg/example
    about: Please ask and answer questions here.
"#.to_string());

        // Pull request template
        files.insert(".github/pull_request_template.md".to_string(), r#"## Description
Brief description of changes

## Type of change
- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Documentation update

## Testing
- [ ] Tests pass locally with my changes
- [ ] I have added tests that prove my fix is effective or that my feature works
- [ ] New and existing unit tests pass locally with my changes

## Checklist
- [ ] My code follows the style guidelines of this project
- [ ] I have performed a self-review of my own code
- [ ] I have commented my code, particularly in hard-to-understand areas
"#.to_string());

        // Contributing guide
        files.insert("CONTRIBUTING.md".to_string(), r#"# Contributing Guide

## How to Contribute

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

## Issue Templates

We have several issue templates to help you report bugs and request features:

- **Bug Report**: Use when something isn't working as expected
- **Feature Request**: Use when proposing new functionality
- **Custom Issue**: Use for other types of issues

## Code Style

Please follow the existing code style and run tests before submitting.
"#.to_string());

        files.insert("src/lib.rs".to_string(), r#"pub fn example_function() -> String {
    "This project uses issue templates".to_string()
}
"#.to_string());

        Self {
            name: "repository_with_existing_issue_templates".to_string(),
            description: "Repository with comprehensive GitHub issue templates and contribution guidelines".to_string(),
            files,
            git_config: GitConfig::default(),
            existing_clambake_config: None,
        }
    }

    /// Create a fixture for a repository with existing CI/CD files
    pub fn repository_with_existing_cicd_files() -> Self {
        let mut files = HashMap::new();
        
        // Basic project files
        files.insert("README.md".to_string(), "# Project with CI/CD\n\nA repository with comprehensive CI/CD setup.\n".to_string());
        files.insert(".gitignore".to_string(), "target/\n*.log\n.coverage/\n".to_string());
        files.insert("Cargo.toml".to_string(), r#"[package]
name = "cicd-project"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1.0", features = ["full"] }

[dev-dependencies]
proptest = "1.0"
"#.to_string());

        // GitHub Actions workflows
        files.insert(".github/workflows/ci.yml".to_string(), r#"name: CI

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main ]

env:
  CARGO_TERM_COLOR: always

jobs:
  test:
    name: Test
    runs-on: ubuntu-latest
    strategy:
      matrix:
        rust:
          - stable
          - beta
          - nightly
    
    steps:
    - uses: actions/checkout@v4
    
    - name: Install Rust
      uses: dtolnay/rust-toolchain@master
      with:
        toolchain: ${{ matrix.rust }}
        components: rustfmt, clippy
    
    - name: Cache dependencies
      uses: actions/cache@v3
      with:
        path: |
          ~/.cargo/registry
          ~/.cargo/git
          target
        key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
    
    - name: Run tests
      run: cargo test --verbose
    
    - name: Run clippy
      run: cargo clippy -- -D warnings
    
    - name: Check formatting
      run: cargo fmt -- --check

  coverage:
    name: Coverage
    runs-on: ubuntu-latest
    
    steps:
    - uses: actions/checkout@v4
    
    - name: Install Rust
      uses: dtolnay/rust-toolchain@stable
    
    - name: Install cargo-tarpaulin
      run: cargo install cargo-tarpaulin
    
    - name: Generate coverage report
      run: cargo tarpaulin --verbose --all-features --workspace --timeout 120 --out xml
    
    - name: Upload coverage reports
      uses: codecov/codecov-action@v3
      with:
        file: ./cobertura.xml
        flags: unittests
        name: codecov-umbrella
        fail_ci_if_error: true
"#.to_string());

        files.insert(".github/workflows/release.yml".to_string(), r#"name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  create-release:
    name: Create Release
    runs-on: ubuntu-latest
    
    steps:
    - uses: actions/checkout@v4
    
    - name: Create Release
      uses: actions/create-release@v1
      env:
        GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
      with:
        tag_name: ${{ github.ref }}
        release_name: Release ${{ github.ref }}
        body: |
          Changes in this Release
          - First Change
          - Second Change
        draft: false
        prerelease: false

  build:
    name: Build Release
    needs: create-release
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        build: [linux, macos, windows]
        include:
        - build: linux
          os: ubuntu-latest
          rust: stable
          target: x86_64-unknown-linux-gnu
        - build: macos
          os: macos-latest
          rust: stable
          target: x86_64-apple-darwin
        - build: windows
          os: windows-latest
          rust: stable
          target: x86_64-pc-windows-msvc
    
    steps:
    - name: Checkout
      uses: actions/checkout@v4
    
    - name: Install Rust
      uses: dtolnay/rust-toolchain@master
      with:
        toolchain: ${{ matrix.rust }}
        target: ${{ matrix.target }}
    
    - name: Build
      run: cargo build --release --target ${{ matrix.target }}
"#.to_string());

        files.insert(".github/workflows/security.yml".to_string(), r#"name: Security audit

on:
  push:
    paths:
      - '**/Cargo.toml'
      - '**/Cargo.lock'
  pull_request:
    paths:
      - '**/Cargo.toml'
      - '**/Cargo.lock'
  schedule:
    - cron: '0 2 * * *'

jobs:
  security_audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: rustsec/audit-check@v1.2.0
        with:
          token: ${{ secrets.GITHUB_TOKEN }}
"#.to_string());

        // Docker setup
        files.insert("Dockerfile".to_string(), r#"FROM rust:1.75 as builder

WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/cicd-project /usr/local/bin/cicd-project

EXPOSE 8080

CMD ["cicd-project"]
"#.to_string());

        files.insert("docker-compose.yml".to_string(), r#"version: '3.8'

services:
  app:
    build: .
    ports:
      - "8080:8080"
    environment:
      - RUST_LOG=info
    depends_on:
      - postgres
      - redis

  postgres:
    image: postgres:15
    environment:
      POSTGRES_DB: cicd_project
      POSTGRES_USER: user
      POSTGRES_PASSWORD: password
    volumes:
      - postgres_data:/var/lib/postgresql/data
    ports:
      - "5432:5432"

  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"

volumes:
  postgres_data:
"#.to_string());

        // Additional CI/CD configuration files
        files.insert("codecov.yml".to_string(), r#"coverage:
  status:
    project:
      default:
        target: 80%
        threshold: 5%
    patch:
      default:
        target: 80%
        threshold: 5%
"#.to_string());

        files.insert(".pre-commit-config.yaml".to_string(), r#"repos:
  - repo: https://github.com/pre-commit/pre-commit-hooks
    rev: v4.4.0
    hooks:
      - id: trailing-whitespace
      - id: end-of-file-fixer
      - id: check-yaml
      - id: check-added-large-files

  - repo: local
    hooks:
      - id: cargo-fmt
        name: cargo fmt
        entry: cargo fmt
        language: system
        types: [rust]
        args: ["--", "--check"]

      - id: cargo-clippy
        name: cargo clippy
        entry: cargo clippy
        language: system
        types: [rust]
        args: ["--", "-D", "warnings"]
"#.to_string());

        files.insert("src/main.rs".to_string(), r#"use tokio;

#[tokio::main]
async fn main() {
    println!("CI/CD enabled project running!");
}
"#.to_string());

        files.insert("tests/integration_test.rs".to_string(), r#"#[tokio::test]
async fn test_basic_functionality() {
    assert!(true, "Integration tests work");
}
"#.to_string());

        Self {
            name: "repository_with_existing_cicd_files".to_string(),
            description: "Repository with comprehensive CI/CD setup including GitHub Actions, Docker, and security scanning".to_string(),
            files,
            git_config: GitConfig::default(),
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
            Self::repository_with_complex_directory_structure(),
            Self::repository_with_existing_issue_templates(),
            Self::repository_with_existing_cicd_files(),
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
            "repository_with_complex_directory_structure" => self.existing_clambake_config.is_none(),
            "repository_with_existing_issue_templates" => self.existing_clambake_config.is_none(),
            "repository_with_existing_cicd_files" => self.existing_clambake_config.is_none(),
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
            "repository_with_complex_directory_structure" => InitBehaviorExpectation {
                should_succeed_without_force: true,
                should_create_config: true,
                should_create_directories: true,
                should_create_labels: true,
                validation_warnings: Vec::new(),
            },
            "repository_with_existing_issue_templates" => InitBehaviorExpectation {
                should_succeed_without_force: true,
                should_create_config: true,
                should_create_directories: true,
                should_create_labels: true,
                validation_warnings: Vec::new(),
            },
            "repository_with_existing_cicd_files" => InitBehaviorExpectation {
                should_succeed_without_force: true,
                should_create_config: true,
                should_create_directories: true,
                should_create_labels: true,
                validation_warnings: Vec::new(),
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
        assert_eq!(fixtures.len(), 7);
        
        let names: Vec<String> = fixtures.iter().map(|f| f.name.clone()).collect();
        assert!(names.contains(&"empty_repository".to_string()));
        assert!(names.contains(&"repository_with_existing_files".to_string()));
        assert!(names.contains(&"repository_with_partial_initialization".to_string()));
        assert!(names.contains(&"repository_with_conflicts".to_string()));
        assert!(names.contains(&"repository_with_complex_directory_structure".to_string()));
        assert!(names.contains(&"repository_with_existing_issue_templates".to_string()));
        assert!(names.contains(&"repository_with_existing_cicd_files".to_string()));
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

    #[test]
    fn test_repository_with_complex_directory_structure_fixture() {
        let fixture = RepositoryStateFixture::repository_with_complex_directory_structure();
        assert_eq!(fixture.name, "repository_with_complex_directory_structure");
        assert!(fixture.is_valid_for_init_testing());
        assert!(fixture.existing_clambake_config.is_none());
        
        // Verify workspace structure
        assert!(fixture.files.contains_key("Cargo.toml"));
        assert!(fixture.files.get("Cargo.toml").unwrap().contains("[workspace]"));
        
        // Verify nested modules
        assert!(fixture.files.contains_key("src/config/mod.rs"));
        assert!(fixture.files.contains_key("src/config/database.rs"));
        assert!(fixture.files.contains_key("src/utils/helpers.rs"));
        
        // Verify workspace members
        assert!(fixture.files.contains_key("crates/shared/Cargo.toml"));
        assert!(fixture.files.contains_key("services/api/src/main.rs"));
        
        // Verify docs and configuration
        assert!(fixture.files.contains_key("docs/architecture.md"));
        assert!(fixture.files.contains_key("config/development.toml"));
        
        let behavior = fixture.expected_init_behavior();
        assert!(behavior.should_succeed_without_force);
    }

    #[test]
    fn test_repository_with_existing_issue_templates_fixture() {
        let fixture = RepositoryStateFixture::repository_with_existing_issue_templates();
        assert_eq!(fixture.name, "repository_with_existing_issue_templates");
        assert!(fixture.is_valid_for_init_testing());
        assert!(fixture.existing_clambake_config.is_none());
        
        // Verify issue templates
        assert!(fixture.files.contains_key(".github/ISSUE_TEMPLATE/bug_report.md"));
        assert!(fixture.files.contains_key(".github/ISSUE_TEMPLATE/feature_request.md"));
        assert!(fixture.files.contains_key(".github/ISSUE_TEMPLATE/config.yml"));
        
        // Verify pull request template
        assert!(fixture.files.contains_key(".github/pull_request_template.md"));
        
        // Verify contributing guide
        assert!(fixture.files.contains_key("CONTRIBUTING.md"));
        
        // Verify bug report template has proper frontmatter
        let bug_report = fixture.files.get(".github/ISSUE_TEMPLATE/bug_report.md").unwrap();
        assert!(bug_report.contains("name: Bug report"));
        assert!(bug_report.contains("labels: 'bug'"));
        
        let behavior = fixture.expected_init_behavior();
        assert!(behavior.should_succeed_without_force);
    }

    #[test]
    fn test_repository_with_existing_cicd_files_fixture() {
        let fixture = RepositoryStateFixture::repository_with_existing_cicd_files();
        assert_eq!(fixture.name, "repository_with_existing_cicd_files");
        assert!(fixture.is_valid_for_init_testing());
        assert!(fixture.existing_clambake_config.is_none());
        
        // Verify GitHub Actions workflows
        assert!(fixture.files.contains_key(".github/workflows/ci.yml"));
        assert!(fixture.files.contains_key(".github/workflows/release.yml"));
        assert!(fixture.files.contains_key(".github/workflows/security.yml"));
        
        // Verify CI workflow has proper structure
        let ci_workflow = fixture.files.get(".github/workflows/ci.yml").unwrap();
        assert!(ci_workflow.contains("name: CI"));
        assert!(ci_workflow.contains("cargo test --verbose"));
        assert!(ci_workflow.contains("cargo clippy -- -D warnings"));
        
        // Verify Docker setup
        assert!(fixture.files.contains_key("Dockerfile"));
        assert!(fixture.files.contains_key("docker-compose.yml"));
        
        // Verify additional CI/CD configuration
        assert!(fixture.files.contains_key("codecov.yml"));
        assert!(fixture.files.contains_key(".pre-commit-config.yaml"));
        
        // Verify Docker compose has services
        let docker_compose = fixture.files.get("docker-compose.yml").unwrap();
        assert!(docker_compose.contains("postgres:"));
        assert!(docker_compose.contains("redis:"));
        
        let behavior = fixture.expected_init_behavior();
        assert!(behavior.should_succeed_without_force);
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

    #[tokio::test]
    async fn test_complex_directory_structure_temp_repository_creation() {
        let fixture = RepositoryStateFixture::repository_with_complex_directory_structure();
        let temp_repo = fixture.create_temp_repository();
        
        assert!(temp_repo.is_ok());
        let temp_dir = temp_repo.unwrap();
        
        // Verify nested directory structure was created
        let config_dir = temp_dir.path().join("src/config");
        assert!(config_dir.exists());
        
        let database_rs = temp_dir.path().join("src/config/database.rs");
        assert!(database_rs.exists());
        
        let shared_crate = temp_dir.path().join("crates/shared/Cargo.toml");
        assert!(shared_crate.exists());
        
        let api_service = temp_dir.path().join("services/api/src/main.rs");
        assert!(api_service.exists());
        
        let docs_dir = temp_dir.path().join("docs/deployment");
        assert!(docs_dir.exists());
        
        // Verify git repository was initialized
        let git_dir = temp_dir.path().join(".git");
        assert!(git_dir.exists());
    }

    #[tokio::test]
    async fn test_issue_templates_temp_repository_creation() {
        let fixture = RepositoryStateFixture::repository_with_existing_issue_templates();
        let temp_repo = fixture.create_temp_repository();
        
        assert!(temp_repo.is_ok());
        let temp_dir = temp_repo.unwrap();
        
        // Verify GitHub directory structure was created
        let github_dir = temp_dir.path().join(".github/ISSUE_TEMPLATE");
        assert!(github_dir.exists());
        
        let bug_report = temp_dir.path().join(".github/ISSUE_TEMPLATE/bug_report.md");
        assert!(bug_report.exists());
        
        let feature_request = temp_dir.path().join(".github/ISSUE_TEMPLATE/feature_request.md");
        assert!(feature_request.exists());
        
        let pr_template = temp_dir.path().join(".github/pull_request_template.md");
        assert!(pr_template.exists());
        
        let contributing = temp_dir.path().join("CONTRIBUTING.md");
        assert!(contributing.exists());
        
        // Verify git repository was initialized
        let git_dir = temp_dir.path().join(".git");
        assert!(git_dir.exists());
    }
}