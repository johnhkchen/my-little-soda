use anyhow::{Context, Result};
use git2::{BranchType, Cred, Direction, Oid, PushOptions, RemoteCallbacks, Repository, Signature};
use std::path::Path;

/// Trait defining core git operations that replace shell commands
pub trait GitOperations {
    /// Checkout a branch (replaces `git checkout`)
    fn checkout_branch(&self, branch: &str) -> Result<()>;

    /// Create a new branch (replaces `git branch -b`)
    #[allow(dead_code)]
    fn create_branch(&self, name: &str, from: &str) -> Result<()>;

    /// Cherry-pick a commit with conflict detection (replaces `git cherry-pick`)
    #[allow(dead_code)]
    fn cherry_pick(&self, commit: Oid) -> Result<Option<Vec<String>>>;

    /// Push to remote (replaces `git push`)
    #[allow(dead_code)]
    fn push(&self, remote: &str, branch: &str) -> Result<()>;

    /// Get branch status (replaces `git status`)
    #[allow(dead_code)]
    fn get_status(&self) -> Result<Vec<String>>;

    /// Check if branch exists locally (replaces `git branch --list`)
    fn branch_exists(&self, branch: &str) -> Result<bool>;

    /// Check if remote branch exists (replaces `git ls-remote`)
    fn remote_branch_exists(&self, remote: &str, branch: &str) -> Result<bool>;

    /// Fetch from remote (replaces `git fetch`)
    fn fetch(&self, remote: &str) -> Result<()>;

    /// Get commit log (replaces `git log`)
    fn get_commits(&self, from: Option<&str>, to: Option<&str>) -> Result<Vec<CommitInfo>>;

    /// Delete branch (replaces `git branch -D`)
    #[allow(dead_code)]
    fn delete_branch(&self, branch: &str, force: bool) -> Result<()>;

    /// Get GitHub repository information from remote URL
    fn get_github_repo_info(&self, remote_name: Option<&str>) -> Result<Option<GitHubRepoInfo>>;
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CommitInfo {
    pub id: String,
    pub message: String,
    pub author: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone)]
pub struct GitHubRepoInfo {
    pub owner: String,
    pub repo: String,
}

/// Implementation of GitOperations using git2
pub struct Git2Operations {
    repo: Repository,
}

impl Git2Operations {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let repo = Repository::open(path).context("Failed to open git repository")?;
        Ok(Self { repo })
    }

    fn get_signature(&self) -> Result<Signature> {
        // Try to get signature from config, fall back to defaults
        match self.repo.signature() {
            Ok(sig) => Ok(sig),
            Err(_) => Signature::now("Clambake Agent", "noreply@clambake.dev")
                .context("Failed to create default signature"),
        }
    }
}

impl GitOperations for Git2Operations {
    fn checkout_branch(&self, branch: &str) -> Result<()> {
        // Find the branch reference
        let branch_ref = self
            .repo
            .find_branch(branch, BranchType::Local)
            .or_else(|_| self.repo.find_branch(branch, BranchType::Remote))
            .with_context(|| format!("Branch '{branch}' not found"))?;

        let reference = branch_ref.get();
        let target = reference.target().context("Branch has no target commit")?;

        // Get the commit and tree
        let commit = self.repo.find_commit(target)?;
        let tree = commit.tree()?;

        // Set HEAD to the branch
        self.repo.set_head(reference.name().unwrap())?;

        // Update working directory
        self.repo.checkout_tree(tree.as_object(), None)?;

        Ok(())
    }

    fn create_branch(&self, name: &str, from: &str) -> Result<()> {
        // Find the reference commit
        let from_commit = if from == "HEAD" {
            self.repo.head()?.peel_to_commit()?
        } else {
            // Try to find as branch first, then as commit
            if let Ok(branch) = self
                .repo
                .find_branch(from, BranchType::Local)
                .or_else(|_| self.repo.find_branch(from, BranchType::Remote))
            {
                branch.get().peel_to_commit()?
            } else {
                // Try as commit ID
                let oid = Oid::from_str(from)
                    .map_err(|e| anyhow::anyhow!("Invalid commit or branch '{}': {}", from, e))?;
                self.repo.find_commit(oid)?
            }
        };

        // Create the branch
        self.repo
            .branch(name, &from_commit, false)
            .with_context(|| format!("Failed to create branch '{name}'"))?;

        Ok(())
    }

    fn cherry_pick(&self, commit_oid: Oid) -> Result<Option<Vec<String>>> {
        let commit = self
            .repo
            .find_commit(commit_oid)
            .context("Failed to find commit for cherry-pick")?;

        // Perform the cherry-pick
        let mut cherrypick_options = git2::CherrypickOptions::new();
        self.repo
            .cherrypick(&commit, Some(&mut cherrypick_options))
            .context("Cherry-pick operation failed")?;

        // Check for conflicts
        let index = self.repo.index()?;
        if index.has_conflicts() {
            let mut conflicts = Vec::new();

            // Collect conflict information
            let conflicts_iter = index.conflicts()?;
            for conflict in conflicts_iter {
                let conflict = conflict?;
                if let Some(our) = conflict.our {
                    if let Ok(path) = std::str::from_utf8(&our.path) {
                        conflicts.push(path.to_string());
                    }
                }
            }

            return Ok(Some(conflicts));
        }

        // If no conflicts, commit the cherry-pick
        let signature = self.get_signature()?;
        let tree_id = self.repo.index()?.write_tree()?;
        let tree = self.repo.find_tree(tree_id)?;
        let parent = self.repo.head()?.peel_to_commit()?;

        self.repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            commit.message().unwrap_or("Cherry-picked commit"),
            &tree,
            &[&parent],
        )?;

        Ok(None)
    }

    fn push(&self, remote_name: &str, branch: &str) -> Result<()> {
        let mut remote = self
            .repo
            .find_remote(remote_name)
            .with_context(|| format!("Remote '{remote_name}' not found"))?;

        let refspec = format!("refs/heads/{branch}:refs/heads/{branch}");

        // Set up callbacks for authentication
        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(|_url, username_from_url, _allowed_types| {
            Cred::ssh_key(
                username_from_url.unwrap_or("git"),
                None,
                std::path::Path::new(&format!(
                    "{}/.ssh/id_rsa",
                    std::env::var("HOME").unwrap_or_default()
                )),
                None,
            )
        });

        let mut push_options = PushOptions::new();
        push_options.remote_callbacks(callbacks);

        remote
            .push(&[&refspec], Some(&mut push_options))
            .context("Failed to push to remote")?;

        Ok(())
    }

    fn get_status(&self) -> Result<Vec<String>> {
        let statuses = self.repo.statuses(None)?;
        let mut status_list = Vec::new();

        for entry in statuses.iter() {
            if let Some(path) = entry.path() {
                let status = entry.status();
                let mut status_str = String::new();

                if status.contains(git2::Status::WT_NEW) {
                    status_str.push_str("?? ");
                }
                if status.contains(git2::Status::WT_MODIFIED) {
                    status_str.push_str(" M ");
                }
                if status.contains(git2::Status::WT_DELETED) {
                    status_str.push_str(" D ");
                }
                if status.contains(git2::Status::INDEX_NEW) {
                    status_str.push_str("A  ");
                }
                if status.contains(git2::Status::INDEX_MODIFIED) {
                    status_str.push_str("M  ");
                }
                if status.contains(git2::Status::INDEX_DELETED) {
                    status_str.push_str("D  ");
                }

                status_list.push(format!("{status_str}{path}"));
            }
        }

        Ok(status_list)
    }

    fn branch_exists(&self, branch: &str) -> Result<bool> {
        match self.repo.find_branch(branch, BranchType::Local) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    fn remote_branch_exists(&self, remote: &str, branch: &str) -> Result<bool> {
        let remote_branch = format!("{remote}/{branch}");
        match self.repo.find_branch(&remote_branch, BranchType::Remote) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    fn fetch(&self, remote_name: &str) -> Result<()> {
        let mut remote = self
            .repo
            .find_remote(remote_name)
            .with_context(|| format!("Remote '{remote_name}' not found"))?;

        // Set up callbacks for authentication
        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(|_url, username_from_url, _allowed_types| {
            Cred::ssh_key(
                username_from_url.unwrap_or("git"),
                None,
                std::path::Path::new(&format!(
                    "{}/.ssh/id_rsa",
                    std::env::var("HOME").unwrap_or_default()
                )),
                None,
            )
        });

        remote.connect_auth(Direction::Fetch, Some(callbacks), None)?;
        remote.fetch(&[] as &[&str], None, None)?;

        Ok(())
    }

    fn get_commits(&self, from: Option<&str>, to: Option<&str>) -> Result<Vec<CommitInfo>> {
        let mut revwalk = self.repo.revwalk()?;

        // Set up the range
        if let Some(to_ref) = to {
            let to_oid = self.repo.revparse_single(to_ref)?.id();
            revwalk.push(to_oid)?;
        } else {
            revwalk.push_head()?;
        }

        if let Some(from_ref) = from {
            let from_oid = self.repo.revparse_single(from_ref)?.id();
            revwalk.hide(from_oid)?;
        }

        let mut commits = Vec::new();
        for oid in revwalk {
            let oid = oid?;
            let commit = self.repo.find_commit(oid)?;

            commits.push(CommitInfo {
                id: oid.to_string(),
                message: commit.message().unwrap_or("").to_string(),
                author: commit.author().name().unwrap_or("Unknown").to_string(),
                timestamp: commit.time().seconds(),
            });
        }

        Ok(commits)
    }

    fn delete_branch(&self, branch: &str, force: bool) -> Result<()> {
        let mut branch = self
            .repo
            .find_branch(branch, BranchType::Local)
            .with_context(|| format!("Branch '{branch}' not found"))?;

        if !force {
            // Check if branch is merged (simplified check)
            let branch_commit = branch.get().peel_to_commit()?;
            let head_commit = self.repo.head()?.peel_to_commit()?;

            // This is a simplified check - in practice you'd want to check if the branch
            // commit is reachable from HEAD
            if branch_commit.id() != head_commit.id() {
                return Err(anyhow::anyhow!("Branch is not merged and force=false"));
            }
        }

        let branch_name = branch
            .name()
            .map(|name| name.unwrap_or("unknown").to_string())
            .unwrap_or("unknown".to_string());
        branch
            .delete()
            .with_context(|| format!("Failed to delete branch '{branch_name}'"))?;

        Ok(())
    }

    fn get_github_repo_info(&self, remote_name: Option<&str>) -> Result<Option<GitHubRepoInfo>> {
        let remote_name = remote_name.unwrap_or("origin");

        let remote = match self.repo.find_remote(remote_name) {
            Ok(remote) => remote,
            Err(_) => return Ok(None), // Remote not found
        };

        let url = match remote.url() {
            Some(url) => url,
            None => return Ok(None), // No URL for remote
        };

        // Parse GitHub URL (both SSH and HTTPS formats)
        Self::parse_github_url(url)
    }
}

impl Git2Operations {
    /// Parse a GitHub URL and extract owner/repo information
    /// Handles both SSH (git@github.com:owner/repo.git) and HTTPS (https://github.com/owner/repo.git) formats
    pub(crate) fn parse_github_url(url: &str) -> Result<Option<GitHubRepoInfo>> {
        // Handle SSH format: git@github.com:owner/repo.git
        if url.starts_with("git@github.com:") {
            let path = url.strip_prefix("git@github.com:").unwrap();
            let path = path.strip_suffix(".git").unwrap_or(path);

            let parts: Vec<&str> = path.split('/').collect();
            if parts.len() == 2 {
                return Ok(Some(GitHubRepoInfo {
                    owner: parts[0].to_string(),
                    repo: parts[1].to_string(),
                }));
            }
        }

        // Handle HTTPS format: https://github.com/owner/repo.git
        if url.starts_with("https://github.com/") {
            let path = url.strip_prefix("https://github.com/").unwrap();
            let path = path.strip_suffix(".git").unwrap_or(path);

            let parts: Vec<&str> = path.split('/').collect();
            if parts.len() >= 2 {
                return Ok(Some(GitHubRepoInfo {
                    owner: parts[0].to_string(),
                    repo: parts[1].to_string(),
                }));
            }
        }

        // Not a recognized GitHub URL
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_repo() -> (TempDir, Git2Operations) {
        let temp_dir = TempDir::new().unwrap();
        let repo = Repository::init(temp_dir.path()).unwrap();

        // Create initial commit
        let signature = Signature::now("Test", "test@example.com").unwrap();
        let tree_id = repo.index().unwrap().write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();

        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            "Initial commit",
            &tree,
            &[],
        )
        .unwrap();

        let ops = Git2Operations::new(temp_dir.path()).unwrap();
        (temp_dir, ops)
    }

    #[test]
    fn test_branch_operations() {
        let (_temp_dir, ops) = create_test_repo();

        // Test creating a branch
        assert!(ops.create_branch("test-branch", "HEAD").is_ok());
        assert!(ops.branch_exists("test-branch").unwrap());

        // Test checking out the branch
        assert!(ops.checkout_branch("test-branch").is_ok());
    }

    #[test]
    fn test_status() {
        let (temp_dir, ops) = create_test_repo();

        // Create a file to show in status
        fs::write(temp_dir.path().join("test.txt"), "test content").unwrap();

        let status = ops.get_status().unwrap();
        assert!(!status.is_empty());
        assert!(status.iter().any(|s| s.contains("test.txt")));
    }

    #[test]
    fn test_parse_github_url_ssh_format() {
        // Test standard SSH format
        let result = Git2Operations::parse_github_url("git@github.com:owner/repo.git").unwrap();
        assert!(result.is_some());
        let repo_info = result.unwrap();
        assert_eq!(repo_info.owner, "owner");
        assert_eq!(repo_info.repo, "repo");

        // Test SSH format without .git suffix
        let result = Git2Operations::parse_github_url("git@github.com:owner/repo").unwrap();
        assert!(result.is_some());
        let repo_info = result.unwrap();
        assert_eq!(repo_info.owner, "owner");
        assert_eq!(repo_info.repo, "repo");
    }

    #[test]
    fn test_parse_github_url_https_format() {
        // Test standard HTTPS format
        let result = Git2Operations::parse_github_url("https://github.com/owner/repo.git").unwrap();
        assert!(result.is_some());
        let repo_info = result.unwrap();
        assert_eq!(repo_info.owner, "owner");
        assert_eq!(repo_info.repo, "repo");

        // Test HTTPS format without .git suffix
        let result = Git2Operations::parse_github_url("https://github.com/owner/repo").unwrap();
        assert!(result.is_some());
        let repo_info = result.unwrap();
        assert_eq!(repo_info.owner, "owner");
        assert_eq!(repo_info.repo, "repo");

        // Test HTTPS format with additional path components (should take first two)
        let result =
            Git2Operations::parse_github_url("https://github.com/owner/repo/extra/path").unwrap();
        assert!(result.is_some());
        let repo_info = result.unwrap();
        assert_eq!(repo_info.owner, "owner");
        assert_eq!(repo_info.repo, "repo");
    }

    #[test]
    fn test_parse_github_url_edge_cases() {
        // Test non-GitHub URLs
        let result = Git2Operations::parse_github_url("https://gitlab.com/owner/repo.git").unwrap();
        assert!(result.is_none());

        let result = Git2Operations::parse_github_url("git@gitlab.com:owner/repo.git").unwrap();
        assert!(result.is_none());

        // Test malformed URLs
        let result = Git2Operations::parse_github_url("https://github.com/owner").unwrap();
        assert!(result.is_none());

        let result = Git2Operations::parse_github_url("git@github.com:owner").unwrap();
        assert!(result.is_none());

        // Test empty and invalid URLs
        let result = Git2Operations::parse_github_url("").unwrap();
        assert!(result.is_none());

        let result = Git2Operations::parse_github_url("not-a-url").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_github_url_real_world_examples() {
        // Test with organization names that include dashes/underscores
        let result = Git2Operations::parse_github_url("git@github.com:my-org/my_repo.git").unwrap();
        assert!(result.is_some());
        let repo_info = result.unwrap();
        assert_eq!(repo_info.owner, "my-org");
        assert_eq!(repo_info.repo, "my_repo");

        // Test with numeric characters
        let result =
            Git2Operations::parse_github_url("https://github.com/user123/repo456").unwrap();
        assert!(result.is_some());
        let repo_info = result.unwrap();
        assert_eq!(repo_info.owner, "user123");
        assert_eq!(repo_info.repo, "repo456");

        // Test with dots in repo names
        let result =
            Git2Operations::parse_github_url("git@github.com:owner/my.repo.name.git").unwrap();
        assert!(result.is_some());
        let repo_info = result.unwrap();
        assert_eq!(repo_info.owner, "owner");
        assert_eq!(repo_info.repo, "my.repo.name");
    }
}
