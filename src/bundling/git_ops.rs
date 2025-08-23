use anyhow::{Result, anyhow};
use git2::{Repository, BranchType, Oid, DiffOptions, ErrorCode};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};
use chrono::{DateTime, Utc};
use super::types::{BundleErrorType, BundleAuditEntry, BundleOperationStatus, RecoveryData};
use uuid::Uuid;

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

/// Report on bundle compatibility and potential conflicts
#[derive(Debug, Clone)]
pub struct ConflictCompatibilityReport {
    pub is_bundle_safe: bool,
    pub compatibility_score: f64, // 0-100, higher = better
    pub potential_conflicts: HashMap<String, Vec<String>>, // file -> conflicting branches
    pub safe_files: Vec<String>,
    pub analyzed_branches: Vec<String>,
    pub analysis_errors: Vec<String>,
    pub analysis_timestamp: DateTime<Utc>,
}

impl ConflictCompatibilityReport {
    pub fn new() -> Self {
        Self {
            is_bundle_safe: true,
            compatibility_score: 100.0,
            potential_conflicts: HashMap::new(),
            safe_files: Vec::new(),
            analyzed_branches: Vec::new(),
            analysis_errors: Vec::new(),
            analysis_timestamp: Utc::now(),
        }
    }
}

/// Prediction for cherry-pick conflicts
#[derive(Debug, Clone)]
pub struct ConflictPrediction {
    pub source_branch: String,
    pub target_branch: String,
    pub commits_analyzed: usize,
    pub conflict_likelihood: f64, // 0-100, higher = more likely to conflict
    pub problematic_files: Vec<String>,
    pub estimated_conflicts: usize,
    pub analysis_timestamp: DateTime<Utc>,
}

/// Git operations for bundling using git2
pub struct GitOperations {
    pub repo: Repository,
    audit_trail: Vec<BundleAuditEntry>,
    correlation_id: String,
}

impl GitOperations {
    /// Initialize Git operations for the current repository
    pub fn new() -> Result<Self> {
        let repo = Repository::open_from_env()?;
        Ok(Self { 
            repo,
            audit_trail: Vec::new(),
            correlation_id: Uuid::new_v4().to_string(),
        })
    }

    /// Log operation to audit trail
    fn log_operation(&mut self, operation: &str, branch_name: Option<String>, affected_issues: Vec<u64>, status: BundleOperationStatus, error: Option<BundleErrorType>, execution_time_ms: u64) {
        let entry = BundleAuditEntry {
            timestamp: Utc::now(),
            operation: operation.to_string(),
            branch_name,
            affected_issues,
            status,
            error,
            recovery_action: None,
            execution_time_ms,
            correlation_id: self.correlation_id.clone(),
        };
        self.audit_trail.push(entry);
    }

    /// Get audit trail
    pub fn get_audit_trail(&self) -> &[BundleAuditEntry] {
        &self.audit_trail
    }

    /// Create recovery data for current state
    fn create_recovery_data(&self, last_commit: Option<String>) -> RecoveryData {
        RecoveryData {
            last_successful_commit: last_commit,
            cleanup_commands: vec![
                "git reset --hard HEAD".to_string(),
                "git clean -fd".to_string(),
            ],
            rollback_branch: Some("main".to_string()),
            temp_files: vec![".git/CHERRY_PICK_HEAD".to_string()],
        }
    }

    /// Handle git2 errors with enhanced context
    fn handle_git_error(&self, operation: &str, error: git2::Error) -> BundleErrorType {
        match error.code() {
            ErrorCode::NotFound => BundleErrorType::GitOperation {
                operation: operation.to_string(),
                details: format!("Git object not found: {}", error.message()),
            },
            ErrorCode::Exists => BundleErrorType::GitOperation {
                operation: operation.to_string(),
                details: format!("Git object already exists: {}", error.message()),
            },
            ErrorCode::Conflict => BundleErrorType::ConflictResolution {
                conflicted_files: vec!["Unknown".to_string()],
                branches: vec!["Unknown".to_string()],
            },
            ErrorCode::Locked => BundleErrorType::PermissionDenied {
                resource: "Git repository".to_string(),
                required_permission: "Write access".to_string(),
            },
            _ => BundleErrorType::GitOperation {
                operation: operation.to_string(),
                details: format!("Git error: {}", error.message()),
            },
        }
    }

    /// Execute git operation with retry logic
    async fn execute_with_retry<T, F>(&self, operation: &str, max_retries: u32, mut operation_fn: F) -> Result<(T, Vec<BundleAuditEntry>)>
    where
        F: FnMut() -> Result<T, git2::Error>,
    {
        let start_time = Instant::now();
        let mut last_error = None;
        let mut audit_entries = Vec::new();

        for attempt in 0..=max_retries {
            match operation_fn() {
                Ok(result) => {
                    let execution_time = start_time.elapsed().as_millis() as u64;
                    let entry = BundleAuditEntry {
                        timestamp: chrono::Utc::now(),
                        operation: operation.to_string(),
                        branch_name: None,
                        affected_issues: vec![],
                        status: BundleOperationStatus::Completed,
                        error: None,
                        recovery_action: None,
                        execution_time_ms: execution_time,
                        correlation_id: self.correlation_id.clone(),
                    };
                    audit_entries.push(entry);
                    return Ok((result, audit_entries));
                }
                Err(error) => {
                    last_error = Some(error);
                    
                    if attempt < max_retries {
                        let backoff_ms = (1 << attempt) * 100; // Exponential backoff
                        println!("⚠️  Git operation '{}' failed (attempt {}/{}), retrying in {}ms: {}", 
                            operation, attempt + 1, max_retries + 1, backoff_ms, last_error.as_ref().unwrap().message());
                        tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                    }
                }
            }
        }

        let execution_time = start_time.elapsed().as_millis() as u64;
        let bundle_error = self.handle_git_error(operation, last_error.unwrap());
        let entry = BundleAuditEntry {
            timestamp: chrono::Utc::now(),
            operation: operation.to_string(),
            branch_name: None,
            affected_issues: vec![],
            status: BundleOperationStatus::Failed,
            error: Some(bundle_error.clone()),
            recovery_action: None,
            execution_time_ms: execution_time,
            correlation_id: self.correlation_id.clone(),
        };
        audit_entries.push(entry);
        
        Err(anyhow!("Git operation '{}' failed after {} attempts", operation, max_retries + 1))
    }
    
    /// Create a new bundle branch from the base branch with error handling
    pub fn create_bundle_branch(&mut self, branch_name: &str, base_branch: &str) -> Result<()> {
        let operation = "create_bundle_branch";
        let start_time = Instant::now();
        
        self.log_operation(operation, Some(branch_name.to_string()), vec![], BundleOperationStatus::Started, None, 0);
        
        let result: Result<(), BundleErrorType> = (|| {
            // Find the base branch reference
            let base_ref = self.repo.find_branch(base_branch, BranchType::Local)
                .or_else(|_| self.repo.find_branch(&format!("origin/{}", base_branch), BranchType::Remote))
                .map_err(|e| self.handle_git_error(operation, e))?;
            
            let base_commit = base_ref.get().peel_to_commit()
                .map_err(|e| self.handle_git_error(operation, e))?;
            
            // Create new branch
            let _bundle_branch = self.repo.branch(branch_name, &base_commit, false)
                .map_err(|e| self.handle_git_error(operation, e))?;
            
            Ok(())
        })();
        
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        match &result {
            Ok(_) => {
                println!("✅ Created bundle branch: {}", branch_name);
                self.log_operation(operation, Some(branch_name.to_string()), vec![], BundleOperationStatus::Completed, None, execution_time);
            }
            Err(error) => {
                println!("❌ Failed to create bundle branch: {}", branch_name);
                self.log_operation(operation, Some(branch_name.to_string()), vec![], BundleOperationStatus::Failed, Some(error.clone()), execution_time);
            }
        }
        
        result.map_err(|_| anyhow!("Failed to create bundle branch"))
    }
    
    /// Checkout the specified branch with error handling
    pub fn checkout_branch(&mut self, branch_name: &str) -> Result<()> {
        let operation = "checkout_branch";
        let start_time = Instant::now();
        
        let result: Result<(), BundleErrorType> = (|| {
            let branch_ref = format!("refs/heads/{}", branch_name);
            let obj = self.repo.revparse_single(&branch_ref)
                .map_err(|e| self.handle_git_error(operation, e))?;
            
            self.repo.checkout_tree(&obj, None)
                .map_err(|e| self.handle_git_error(operation, e))?;
            self.repo.set_head(&branch_ref)
                .map_err(|e| self.handle_git_error(operation, e))?;
            
            Ok(())
        })();
        
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        match &result {
            Ok(_) => {
                self.log_operation(operation, Some(branch_name.to_string()), vec![], BundleOperationStatus::Completed, None, execution_time);
            }
            Err(error) => {
                self.log_operation(operation, Some(branch_name.to_string()), vec![], BundleOperationStatus::Failed, Some(error.clone()), execution_time);
            }
        }
        
        result.map_err(|_| anyhow!("Failed to checkout branch"))
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
    
    /// Push branch to remote with error handling
    pub fn push_branch(&mut self, branch_name: &str, remote_name: &str) -> Result<()> {
        let operation = "push_branch";
        let start_time = Instant::now();
        
        let result: Result<(), BundleErrorType> = (|| {
            let mut remote = self.repo.find_remote(remote_name)
                .map_err(|e| self.handle_git_error(operation, e))?;
            let refspec = format!("refs/heads/{}:refs/heads/{}", branch_name, branch_name);
            
            remote.push(&[&refspec], None)
                .map_err(|e| self.handle_git_error(operation, e))?;
            
            Ok(())
        })();
        
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        match &result {
            Ok(_) => {
                println!("✅ Pushed branch {} to {}", branch_name, remote_name);
                self.log_operation(operation, Some(branch_name.to_string()), vec![], BundleOperationStatus::Completed, None, execution_time);
            }
            Err(error) => {
                self.log_operation(operation, Some(branch_name.to_string()), vec![], BundleOperationStatus::Failed, Some(error.clone()), execution_time);
            }
        }
        
        result.map_err(|_| anyhow!("Failed to push branch"))
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
    
    /// Pre-flight conflict analysis for multiple branches
    pub fn analyze_bundle_conflicts(&self, branches: &[String], base_branch: &str) -> Result<ConflictCompatibilityReport> {
        let mut report = ConflictCompatibilityReport::new();
        let mut file_changes: HashMap<String, Vec<String>> = HashMap::new();
        
        // Analyze file changes for each branch
        for branch_name in branches {
            match self.get_changed_files(branch_name, base_branch) {
                Ok(changed_files) => {
                    for file in changed_files {
                        file_changes.entry(file.clone()).or_default().push(branch_name.clone());
                    }
                    report.analyzed_branches.push(branch_name.clone());
                }
                Err(e) => {
                    report.analysis_errors.push(format!("Failed to analyze {}: {}", branch_name, e));
                }
            }
        }
        
        // Identify potential conflicts (same file modified by multiple branches)
        for (file, modifying_branches) in file_changes {
            if modifying_branches.len() > 1 {
                report.potential_conflicts.insert(file, modifying_branches);
            } else {
                report.safe_files.extend(modifying_branches);
            }
        }
        
        // Calculate compatibility score
        let total_branches = branches.len();
        let conflicting_branches: HashSet<_> = report.potential_conflicts.values()
            .flat_map(|branches| branches.iter())
            .collect();
        
        if total_branches > 0 {
            report.compatibility_score = ((total_branches - conflicting_branches.len()) as f64 / total_branches as f64) * 100.0;
        }
        
        report.is_bundle_safe = report.potential_conflicts.is_empty();
        report.analysis_timestamp = Utc::now();
        
        Ok(report)
    }
    
    /// Get list of files changed in a branch compared to base
    pub fn get_changed_files(&self, branch_name: &str, base_branch: &str) -> Result<Vec<String>> {
        let branch_ref = self.repo.find_branch(branch_name, BranchType::Local)
            .or_else(|_| self.repo.find_branch(&format!("origin/{}", branch_name), BranchType::Remote))?;
        let base_ref = self.repo.find_branch(base_branch, BranchType::Local)
            .or_else(|_| self.repo.find_branch(&format!("origin/{}", base_branch), BranchType::Remote))?;
        
        let branch_tree = branch_ref.get().peel_to_tree()?;
        let base_tree = base_ref.get().peel_to_tree()?;
        
        let mut diff_opts = DiffOptions::new();
        let diff = self.repo.diff_tree_to_tree(Some(&base_tree), Some(&branch_tree), Some(&mut diff_opts))?;
        
        let mut changed_files = Vec::new();
        diff.foreach(
            &mut |delta: git2::DiffDelta, _progress: f32| -> bool {
                if let Some(path) = delta.new_file().path() {
                    if let Some(path_str) = path.to_str() {
                        changed_files.push(path_str.to_string());
                    }
                }
                true
            },
            None,
            None,
            None,
        )?;
        
        Ok(changed_files)
    }
    
    /// Simulate cherry-pick to detect conflicts without making changes
    pub fn simulate_cherry_pick(&self, source_branch: &str, target_branch: &str) -> Result<ConflictPrediction> {
        let source_ref = self.repo.find_branch(source_branch, BranchType::Local)
            .or_else(|_| self.repo.find_branch(&format!("origin/{}", source_branch), BranchType::Remote))?;
        let target_ref = self.repo.find_branch(target_branch, BranchType::Local)
            .or_else(|_| self.repo.find_branch(&format!("origin/{}", target_branch), BranchType::Remote))?;
        
        let source_commit = source_ref.get().peel_to_commit()?;
        let target_commit = target_ref.get().peel_to_commit()?;
        
        // Find commits to cherry-pick
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push(source_commit.id())?;
        revwalk.hide(target_commit.id())?;
        
        let commits_to_pick: Vec<Oid> = revwalk.collect::<Result<Vec<_>, _>>()?;
        
        let mut prediction = ConflictPrediction {
            source_branch: source_branch.to_string(),
            target_branch: target_branch.to_string(),
            commits_analyzed: commits_to_pick.len(),
            conflict_likelihood: 0.0,
            problematic_files: Vec::new(),
            estimated_conflicts: 0,
            analysis_timestamp: Utc::now(),
        };
        
        // For each commit, check for potential conflicts by comparing file changes
        let mut file_conflict_risk: HashMap<String, u32> = HashMap::new();
        
        for commit_oid in commits_to_pick {
            let commit = self.repo.find_commit(commit_oid)?;
            
            if let Some(parent) = commit.parent(0).ok() {
                let commit_tree = commit.tree()?;
                let parent_tree = parent.tree()?;
                
                let mut diff_opts = DiffOptions::new();
                let diff = self.repo.diff_tree_to_tree(Some(&parent_tree), Some(&commit_tree), Some(&mut diff_opts))?;
                
                diff.foreach(
                    &mut |delta: git2::DiffDelta, _progress: f32| -> bool {
                        if let Some(path) = delta.new_file().path() {
                            if let Some(path_str) = path.to_str() {
                                *file_conflict_risk.entry(path_str.to_string()).or_insert(0) += 1;
                            }
                        }
                        true
                    },
                    None,
                    None,
                    None,
                ).ok();
            }
        }
        
        // Calculate conflict likelihood based on file modification frequency
        if !file_conflict_risk.is_empty() {
            let high_risk_files: Vec<_> = file_conflict_risk.iter()
                .filter(|(_, &count)| count > 1)
                .map(|(file, count)| format!("{} ({} modifications)", file, count))
                .collect();
            
            prediction.problematic_files = high_risk_files;
            prediction.estimated_conflicts = file_conflict_risk.values().filter(|&&count| count > 1).count();
            prediction.conflict_likelihood = (prediction.estimated_conflicts as f64 / file_conflict_risk.len().max(1) as f64) * 100.0;
        }
        
        Ok(prediction)
    }
}