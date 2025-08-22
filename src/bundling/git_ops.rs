use anyhow::{Result, anyhow};
use git2::{Repository, BranchType, Oid};

/// Strategy for handling merge conflicts during bundling
#[derive(Debug, Clone)]
pub enum ConflictStrategy {
    /// Abort bundle and create individual PRs
    IndividualFallback,
    /// Skip conflicting commits
    SkipConflicts,
    /// Manual resolution required
    ManualResolve,
}

/// Git operations for bundling using git2
pub struct GitOperations {
    repo: Repository,
}

impl GitOperations {
    /// Initialize Git operations for the current repository
    pub fn new() -> Result<Self> {
        let repo = Repository::open_from_env()?;
        Ok(Self { repo })
    }
    
    /// Create a new bundle branch from the base branch
    pub fn create_bundle_branch(&self, branch_name: &str, base_branch: &str) -> Result<()> {
        // Find the base branch reference
        let base_ref = self.repo.find_branch(base_branch, BranchType::Local)
            .or_else(|_| self.repo.find_branch(&format!("origin/{}", base_branch), BranchType::Remote))?;
        
        let base_commit = base_ref.get().peel_to_commit()?;
        
        // Create new branch
        let _bundle_branch = self.repo.branch(branch_name, &base_commit, false)?;
        
        println!("✅ Created bundle branch: {}", branch_name);
        Ok(())
    }
    
    /// Checkout the specified branch
    pub fn checkout_branch(&self, branch_name: &str) -> Result<()> {
        let branch_ref = format!("refs/heads/{}", branch_name);
        let obj = self.repo.revparse_single(&branch_ref)?;
        
        self.repo.checkout_tree(&obj, None)?;
        self.repo.set_head(&branch_ref)?;
        
        Ok(())
    }
    
    /// Cherry-pick commits from source branch onto current branch
    pub fn cherry_pick_branch(&self, source_branch: &str, strategy: ConflictStrategy) -> Result<Vec<Oid>> {
        let source_ref = self.repo.find_branch(source_branch, BranchType::Local)
            .or_else(|_| self.repo.find_branch(&format!("origin/{}", source_branch), BranchType::Remote))?;
        
        let source_commit = source_ref.get().peel_to_commit()?;
        
        // Get the current HEAD to find commits to cherry-pick
        let head = self.repo.head()?.peel_to_commit()?;
        
        // Find commits on source branch that aren't on current branch
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push(source_commit.id())?;
        revwalk.hide(head.id())?;
        
        let commits_to_pick: Vec<Oid> = revwalk.collect::<Result<Vec<_>, _>>()?;
        let mut picked_commits = Vec::new();
        
        for commit_oid in commits_to_pick.iter().rev() { // Cherry-pick in chronological order
            let commit = self.repo.find_commit(*commit_oid)?;
            
            match self.cherry_pick_commit(&commit) {
                Ok(new_oid) => {
                    picked_commits.push(new_oid);
                    println!("✅ Cherry-picked: {} ({})", 
                        &commit.id().to_string()[..8], 
                        commit.summary().unwrap_or("No message"));
                }
                Err(e) => {
                    match strategy {
                        ConflictStrategy::IndividualFallback => {
                            return Err(anyhow!("Conflict cherry-picking {}: {}", commit.id(), e));
                        }
                        ConflictStrategy::SkipConflicts => {
                            println!("⚠️  Skipping conflicted commit: {} ({})", 
                                &commit.id().to_string()[..8], 
                                commit.summary().unwrap_or("No message"));
                            continue;
                        }
                        ConflictStrategy::ManualResolve => {
                            return Err(anyhow!("Manual resolution required for commit {}: {}", commit.id(), e));
                        }
                    }
                }
            }
        }
        
        Ok(picked_commits)
    }
    
    /// Cherry-pick a single commit using git2's built-in cherry-pick
    fn cherry_pick_commit(&self, commit: &git2::Commit) -> Result<Oid> {
        // Use git2's built-in cherry-pick functionality
        let mut cherrypick_options = git2::CherrypickOptions::new();
        self.repo.cherrypick(commit, Some(&mut cherrypick_options))?;
        
        // Check if there are conflicts
        let mut index = self.repo.index()?;
        if index.has_conflicts() {
            return Err(anyhow!("Cherry-pick resulted in conflicts"));
        }
        
        // Commit the cherry-pick
        let signature = commit.author();
        let tree_oid = index.write_tree()?;
        let tree = self.repo.find_tree(tree_oid)?;
        let head = self.repo.head()?.peel_to_commit()?;
        
        let new_commit_oid = self.repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            &commit.message().unwrap_or("Cherry-picked commit"),
            &tree,
            &[&head],
        )?;
        
        Ok(new_commit_oid)
    }
    
    /// Push branch to remote
    pub fn push_branch(&self, branch_name: &str, remote_name: &str) -> Result<()> {
        let mut remote = self.repo.find_remote(remote_name)?;
        let refspec = format!("refs/heads/{}:refs/heads/{}", branch_name, branch_name);
        
        remote.push(&[&refspec], None)?;
        
        println!("✅ Pushed branch {} to {}", branch_name, remote_name);
        Ok(())
    }
    
    /// Check if a branch exists
    pub fn branch_exists(&self, branch_name: &str) -> bool {
        self.repo.find_branch(branch_name, BranchType::Local).is_ok() ||
        self.repo.find_branch(&format!("origin/{}", branch_name), BranchType::Remote).is_ok()
    }
    
    /// Get commits ahead of base branch
    pub fn commits_ahead(&self, branch_name: &str, base_branch: &str) -> Result<usize> {
        let branch_ref = self.repo.find_branch(branch_name, BranchType::Local)
            .or_else(|_| self.repo.find_branch(&format!("origin/{}", branch_name), BranchType::Remote))?;
        let base_ref = self.repo.find_branch(base_branch, BranchType::Local)
            .or_else(|_| self.repo.find_branch(&format!("origin/{}", base_branch), BranchType::Remote))?;
        
        let branch_commit = branch_ref.get().peel_to_commit()?;
        let base_commit = base_ref.get().peel_to_commit()?;
        
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push(branch_commit.id())?;
        revwalk.hide(base_commit.id())?;
        
        Ok(revwalk.count())
    }
}