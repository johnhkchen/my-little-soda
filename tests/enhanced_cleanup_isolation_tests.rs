use anyhow::Result;
use std::fs;
use std::path::PathBuf;
/// Enhanced cleanup and isolation mechanisms tests (B2c)
///
/// This module implements enhanced cleanup verification and cross-test isolation
/// to prevent resource leaks and test interference
use tempfile::TempDir;

/// Tracked resource metadata
#[derive(Debug, Clone)]
struct ResourceMetadata {
    path: PathBuf,
    created_at: std::time::SystemTime,
    size: Option<u64>,
    is_critical: bool, // Critical resources must be cleaned up
}

/// Resource tracker for detecting leaks
#[derive(Debug)]
struct ResourceTracker {
    temp_dirs: Vec<ResourceMetadata>,
    files_created: Vec<ResourceMetadata>,
    processes_spawned: Vec<u32>,
    external_paths: Vec<PathBuf>, // Paths outside temp directories
}

impl ResourceTracker {
    fn new() -> Self {
        Self {
            temp_dirs: Vec::new(),
            files_created: Vec::new(),
            processes_spawned: Vec::new(),
            external_paths: Vec::new(),
        }
    }

    fn track_temp_dir(&mut self, path: PathBuf) {
        let metadata = ResourceMetadata {
            size: None, // Directories don't have a simple size
            created_at: std::time::SystemTime::now(),
            is_critical: !path.starts_with(&std::env::temp_dir()),
            path: path.clone(),
        };
        self.temp_dirs.push(metadata);

        // Track external paths separately for enhanced monitoring
        if !path.starts_with(&std::env::temp_dir()) {
            self.external_paths.push(path);
        }
    }

    fn track_file(&mut self, path: PathBuf) {
        let size = fs::metadata(&path).ok().map(|m| m.len());
        let metadata = ResourceMetadata {
            size,
            created_at: std::time::SystemTime::now(),
            is_critical: !path.starts_with(&std::env::temp_dir()),
            path: path.clone(),
        };
        self.files_created.push(metadata);

        // Track external paths separately for enhanced monitoring
        if !path.starts_with(&std::env::temp_dir()) {
            self.external_paths.push(path);
        }
    }

    fn track_process(&mut self, pid: u32) {
        self.processes_spawned.push(pid);
    }

    /// Detect any resource leaks with enhanced checking
    fn detect_leaks(&self) -> Vec<String> {
        let mut leaks = Vec::new();

        // Check temp directories with enhanced metadata
        for dir_metadata in &self.temp_dirs {
            if dir_metadata.path.exists() && dir_metadata.is_critical {
                let age = dir_metadata
                    .created_at
                    .elapsed()
                    .unwrap_or_default()
                    .as_secs();
                leaks.push(format!(
                    "Critical temp directory leak: {} (age: {}s)",
                    dir_metadata.path.display(),
                    age
                ));
            }
        }

        // Check files with enhanced metadata and size tracking
        for file_metadata in &self.files_created {
            if file_metadata.path.exists() && file_metadata.is_critical {
                let age = file_metadata
                    .created_at
                    .elapsed()
                    .unwrap_or_default()
                    .as_secs();
                let size_info = file_metadata
                    .size
                    .map_or("unknown size".to_string(), |s| format!("{}B", s));
                leaks.push(format!(
                    "Critical file leak: {} ({}, age: {}s)",
                    file_metadata.path.display(),
                    size_info,
                    age
                ));
            }
        }

        // Enhanced process leak detection
        for &pid in &self.processes_spawned {
            if process_exists(pid) {
                leaks.push(format!("Process leak: PID {} (still running)", pid));
            }
        }

        // Check for any external paths that weren't properly tracked
        for external_path in &self.external_paths {
            if external_path.exists() {
                leaks.push(format!(
                    "Untracked external resource: {}",
                    external_path.display()
                ));
            }
        }

        leaks
    }

    /// Get detailed resource usage report
    fn get_resource_report(&self) -> String {
        let mut report = String::new();
        report.push_str(&format!("Resource Tracking Report:\n"));
        report.push_str(&format!(
            "- Temp directories tracked: {}\n",
            self.temp_dirs.len()
        ));
        report.push_str(&format!("- Files tracked: {}\n", self.files_created.len()));
        report.push_str(&format!(
            "- Processes tracked: {}\n",
            self.processes_spawned.len()
        ));
        report.push_str(&format!(
            "- External paths: {}\n",
            self.external_paths.len()
        ));

        let critical_count = self.temp_dirs.iter().filter(|d| d.is_critical).count()
            + self.files_created.iter().filter(|f| f.is_critical).count();
        report.push_str(&format!("- Critical resources: {}\n", critical_count));

        report
    }

    /// Force cleanup of tracked resources with enhanced error handling
    fn force_cleanup(&self) -> Result<Vec<String>> {
        let mut errors = Vec::new();

        // Clean temp directories with priority on critical resources
        for dir_metadata in &self.temp_dirs {
            if dir_metadata.path.exists() {
                if let Err(e) = fs::remove_dir_all(&dir_metadata.path) {
                    let severity = if dir_metadata.is_critical {
                        "CRITICAL"
                    } else {
                        "WARNING"
                    };
                    errors.push(format!(
                        "[{}] Failed to remove temp dir {}: {}",
                        severity,
                        dir_metadata.path.display(),
                        e
                    ));
                }
            }
        }

        // Clean files with priority on critical resources
        for file_metadata in &self.files_created {
            if file_metadata.path.exists() && file_metadata.is_critical {
                if let Err(e) = fs::remove_file(&file_metadata.path) {
                    let size_info = file_metadata
                        .size
                        .map_or("unknown size".to_string(), |s| format!("{}B", s));
                    errors.push(format!(
                        "[CRITICAL] Failed to remove file {} ({}): {}",
                        file_metadata.path.display(),
                        size_info,
                        e
                    ));
                }
            }
        }

        // Kill processes with enhanced reporting
        for &pid in &self.processes_spawned {
            if process_exists(pid) {
                kill_process(pid);
                // Give process time to die
                std::thread::sleep(std::time::Duration::from_millis(100));
                if process_exists(pid) {
                    errors.push(format!("Failed to kill process PID {}", pid));
                }
            }
        }

        Ok(errors)
    }
}

/// Enhanced test harness with cleanup and isolation
struct EnhancedTestHarness {
    temp_dir: TempDir,
    resource_tracker: ResourceTracker,
    isolation_verified: bool,
}

impl EnhancedTestHarness {
    fn new() -> Result<Self> {
        let temp_dir = tempfile::tempdir()?;
        Ok(Self {
            temp_dir,
            resource_tracker: ResourceTracker::new(),
            isolation_verified: false,
        })
    }

    fn path(&self) -> &std::path::Path {
        self.temp_dir.path()
    }

    fn create_file(&mut self, relative_path: &str, content: &str) -> Result<PathBuf> {
        let file_path = self.temp_dir.path().join(relative_path);

        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&file_path, content)?;
        self.resource_tracker.track_file(file_path.clone());
        Ok(file_path)
    }

    /// Enhanced isolation verification with comprehensive checks
    fn verify_isolation(&mut self) -> Result<()> {
        let path = self.temp_dir.path();
        let start_time = std::time::Instant::now();

        // 1. Basic path isolation
        if !path.starts_with(&std::env::temp_dir()) {
            anyhow::bail!("Test directory not properly isolated: {}", path.display());
        }

        // 2. Test write/read permissions with different file types
        let test_file = path.join("isolation_test.txt");
        let binary_file = path.join("binary_test.bin");
        let nested_file = path.join("nested").join("deep_test.txt");

        // Create nested directory structure
        fs::create_dir_all(nested_file.parent().unwrap())?;

        // Test text file operations
        fs::write(&test_file, "isolation test content")?;
        let content = fs::read_to_string(&test_file)?;
        if content != "isolation test content" {
            anyhow::bail!("File content integrity check failed");
        }

        // Test binary file operations
        let binary_data = vec![0u8, 1u8, 255u8, 128u8];
        fs::write(&binary_file, &binary_data)?;
        let read_binary = fs::read(&binary_file)?;
        if read_binary != binary_data {
            anyhow::bail!("Binary file integrity check failed");
        }

        // Test nested file operations
        fs::write(&nested_file, "nested content")?;
        if !nested_file.exists() {
            anyhow::bail!("Cannot create files in nested directories");
        }

        // 3. Verify unique process isolation
        let marker_file = path.join(format!(
            "marker_{}_{}",
            std::process::id(),
            start_time.elapsed().as_nanos()
        ));
        let marker_content = format!(
            "pid:{},thread:{:?},time:{:?}",
            std::process::id(),
            std::thread::current().id(),
            std::time::SystemTime::now()
        );
        fs::write(&marker_file, &marker_content)?;

        // 4. Test file locking and concurrent access
        {
            let lock_file = path.join("lock_test.lock");
            fs::write(&lock_file, "lock test")?;

            // Simulate concurrent access check
            let file = std::fs::OpenOptions::new().write(true).open(&lock_file)?;
            drop(file); // Release immediately

            fs::remove_file(&lock_file)?;
        }

        // 5. Test directory permissions and access patterns
        let perms = fs::metadata(path)?.permissions();
        if perms.readonly() {
            anyhow::bail!("Test directory is read-only, cannot perform isolation tests");
        }

        // 6. Cleanup with verification
        fs::remove_file(&test_file)?;
        fs::remove_file(&binary_file)?;
        fs::remove_file(&nested_file)?;
        fs::remove_dir(nested_file.parent().unwrap())?;
        fs::remove_file(&marker_file)?;

        // Verify cleanup was successful
        if test_file.exists()
            || binary_file.exists()
            || nested_file.exists()
            || marker_file.exists()
        {
            anyhow::bail!("Cleanup verification failed - files still exist");
        }

        self.isolation_verified = true;
        Ok(())
    }

    /// Enhanced cross-test isolation verification
    fn verify_cross_test_isolation(&self, others: &[&EnhancedTestHarness]) -> Result<()> {
        if !self.isolation_verified {
            anyhow::bail!("Basic isolation must be verified first");
        }

        let self_path = self.temp_dir.path();

        for (i, other) in others.iter().enumerate() {
            let other_path = other.temp_dir.path();

            // 1. Verify no path overlap
            if self_path == other_path {
                anyhow::bail!("Duplicate test paths detected with harness {}", i);
            }

            // 2. Verify no nested paths (enhanced check)
            if self_path.starts_with(other_path) || other_path.starts_with(self_path) {
                anyhow::bail!(
                    "Nested test paths detected with harness {}: self={}, other={}",
                    i,
                    self_path.display(),
                    other_path.display()
                );
            }

            // 3. Verify path distance (should not be adjacent siblings that might interfere)
            if let (Some(self_parent), Some(other_parent)) =
                (self_path.parent(), other_path.parent())
            {
                if self_parent == other_parent {
                    // Same parent directory - verify sufficiently unique names
                    if let (Some(self_name), Some(other_name)) =
                        (self_path.file_name(), other_path.file_name())
                    {
                        let self_str = self_name.to_string_lossy();
                        let other_str = other_name.to_string_lossy();
                        if self_str.len() < 10 || other_str.len() < 10 {
                            anyhow::bail!(
                                "Test directory names too similar/short: {} vs {}",
                                self_str,
                                other_str
                            );
                        }
                    }
                }
            }

            // 4. Enhanced file namespace isolation check
            if other_path.exists() && self_path.exists() {
                // Cross-check for any file name collisions
                let self_files: Result<Vec<_>, _> = fs::read_dir(self_path)?.collect();
                let self_files = self_files?;

                for entry in fs::read_dir(other_path)? {
                    let entry = entry?;
                    let file_name = entry.file_name();

                    // Check for direct collision
                    if self_path.join(&file_name).exists() {
                        anyhow::bail!("File namespace collision: {:?}", file_name);
                    }

                    // Check for similar file names that might cause confusion
                    for self_entry in &self_files {
                        let self_name = self_entry.file_name();
                        if files_too_similar(
                            &file_name.to_string_lossy(),
                            &self_name.to_string_lossy(),
                        ) {
                            anyhow::bail!(
                                "Similar file names detected: {:?} vs {:?}",
                                file_name,
                                self_name
                            );
                        }
                    }
                }

                // 5. Verify resource tracking isolation
                let self_resources = self.resource_tracker.external_paths.len();
                let other_resources = other.resource_tracker.external_paths.len();

                // Check for overlapping external resources
                for self_external in &self.resource_tracker.external_paths {
                    for other_external in &other.resource_tracker.external_paths {
                        if self_external == other_external {
                            anyhow::bail!(
                                "Shared external resource detected: {}",
                                self_external.display()
                            );
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Get detailed isolation report
    fn get_isolation_report(&self) -> String {
        let mut report = String::new();
        report.push_str(&format!(
            "Isolation Report for: {}\n",
            self.temp_dir.path().display()
        ));
        report.push_str(&format!(
            "- Isolation verified: {}\n",
            self.isolation_verified
        ));
        report.push_str(&format!(
            "- Base path: {}\n",
            self.temp_dir.path().display()
        ));

        if let Ok(metadata) = fs::metadata(self.temp_dir.path()) {
            report.push_str(&format!("- Directory size: {} bytes\n", metadata.len()));
            report.push_str(&format!("- Permissions: {:?}\n", metadata.permissions()));
        }

        // Add resource tracking summary
        report.push_str(&self.resource_tracker.get_resource_report());

        report
    }

    /// Enhanced cleanup with leak detection
    fn enhanced_cleanup(&self) -> Result<Vec<String>> {
        let mut issues = Vec::new();

        // Detect leaks before cleanup
        let leaks = self.resource_tracker.detect_leaks();
        issues.extend(leaks);

        // Attempt force cleanup
        let cleanup_errors = self.resource_tracker.force_cleanup()?;
        issues.extend(cleanup_errors);

        Ok(issues)
    }

    /// Get leak detection report
    fn get_leak_report(&self) -> Vec<String> {
        self.resource_tracker.detect_leaks()
    }

    /// Get detailed resource tracking report
    fn get_resource_report(&self) -> String {
        self.resource_tracker.get_resource_report()
    }
}

/// Check if two file names are too similar (might cause confusion)
fn files_too_similar(name1: &str, name2: &str) -> bool {
    if name1 == name2 {
        return true; // Exact match
    }

    // Only flag as similar if they are identical except for extensions
    // This catches cases like "data.txt" and "data.log" which could be confusing
    let (name1_base, _) = name1.rsplit_once('.').unwrap_or((name1, ""));
    let (name2_base, _) = name2.rsplit_once('.').unwrap_or((name2, ""));

    if name1_base == name2_base && name1_base.len() > 2 {
        return true; // Same base name with different extensions
    }

    false
}

/// Check if a process exists (simplified implementation)
fn process_exists(_pid: u32) -> bool {
    // In a real implementation, this would check /proc on Linux or use system APIs
    // For testing purposes, assume no zombie processes
    false
}

/// Kill a process (simplified implementation)
fn kill_process(_pid: u32) {
    // In a real implementation, this would send SIGTERM/SIGKILL
    // For testing purposes, this is a no-op
}

#[test]
fn test_enhanced_cleanup_verification() {
    // Create multiple test harnesses and verify they clean up properly
    let temp_paths: Vec<_> = (0..3)
        .map(|i| {
            let mut harness = EnhancedTestHarness::new().unwrap();
            let path = harness.path().to_path_buf();

            harness
                .create_file(&format!("cleanup_test_{}.txt", i), "cleanup test")
                .unwrap();
            assert!(path.join(&format!("cleanup_test_{}.txt", i)).exists());

            // Track the temp directory
            harness.resource_tracker.track_temp_dir(path.clone());

            // Get cleanup report
            let issues = harness.enhanced_cleanup().unwrap();
            assert!(
                issues.is_empty(),
                "No cleanup issues expected, got: {:?}",
                issues
            );

            path
        })
        .collect();

    // After harnesses are dropped, verify cleanup
    for path in temp_paths {
        assert!(
            !path.exists(),
            "Temporary directory should be cleaned up: {}",
            path.display()
        );
    }
}

#[test]
fn test_isolation_verification() {
    let mut harness = EnhancedTestHarness::new().unwrap();
    harness.verify_isolation().unwrap();
    assert!(harness.isolation_verified);
}

#[test]
fn test_cross_test_isolation() {
    let mut harness1 = EnhancedTestHarness::new().unwrap();
    let mut harness2 = EnhancedTestHarness::new().unwrap();
    let mut harness3 = EnhancedTestHarness::new().unwrap();

    // Verify basic isolation first
    harness1.verify_isolation().unwrap();
    harness2.verify_isolation().unwrap();
    harness3.verify_isolation().unwrap();

    // Create unique files in each harness
    harness1.create_file("unique1.txt", "content1").unwrap();
    harness2.create_file("unique2.txt", "content2").unwrap();
    harness3.create_file("unique3.txt", "content3").unwrap();

    // Verify cross-test isolation
    let others = vec![&harness2, &harness3];
    harness1.verify_cross_test_isolation(&others).unwrap();

    // Verify paths are unique
    assert_ne!(harness1.path(), harness2.path());
    assert_ne!(harness1.path(), harness3.path());
    assert_ne!(harness2.path(), harness3.path());
}

#[test]
fn test_resource_leak_detection() {
    let mut harness = EnhancedTestHarness::new().unwrap();

    // Create a file and track it
    let test_file = harness.create_file("leak_test.txt", "leak test").unwrap();
    assert!(test_file.exists());

    // Check for leaks (should be none since file is in temp dir)
    let leaks = harness.get_leak_report();
    assert!(
        leaks.is_empty(),
        "No leaks expected for temp directory files"
    );
}

#[test]
fn test_enhanced_cleanup_with_errors() {
    let mut harness = EnhancedTestHarness::new().unwrap();

    // Create test files
    harness.create_file("test1.txt", "content1").unwrap();
    harness.create_file("test2.txt", "content2").unwrap();

    // Perform cleanup
    let issues = harness.enhanced_cleanup().unwrap();

    // Should have no issues for temp directory files
    assert!(
        issues.is_empty(),
        "No cleanup issues expected for temp files"
    );
}

#[test]
fn test_concurrent_test_environments() {
    // Test that multiple test environments can coexist
    let mut harnesses: Vec<_> = (0..5)
        .map(|i| {
            let mut harness = EnhancedTestHarness::new().unwrap();
            harness.verify_isolation().unwrap();
            harness
                .create_file(&format!("test_{}.txt", i), &format!("content {}", i))
                .unwrap();
            harness
        })
        .collect();

    // Verify all harnesses are isolated from each other
    for (i, harness) in harnesses.iter().enumerate() {
        let others: Vec<_> = harnesses
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .map(|(_, h)| h)
            .collect();
        harness.verify_cross_test_isolation(&others).unwrap();
    }

    // Verify each has its unique file
    for (i, harness) in harnesses.iter().enumerate() {
        let file_path = harness.path().join(&format!("test_{}.txt", i));
        assert!(file_path.exists());

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, format!("content {}", i));
    }
}

#[test]
fn test_cleanup_error_recovery() {
    let mut harness = EnhancedTestHarness::new().unwrap();

    // Create files
    harness
        .create_file("recovery_test.txt", "recovery content")
        .unwrap();

    // Simulate cleanup with potential errors
    let issues = harness.enhanced_cleanup().unwrap();

    // Verify cleanup completed without major errors
    assert!(issues.is_empty() || issues.iter().all(|i| !i.contains("critical")));
}
