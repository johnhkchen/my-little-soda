/// Git operations validation tests for real file system integration
/// Tests advanced Git scenarios and validation in test environments
use std::fs;
use std::process::Command;

mod fixtures;
use fixtures::repository_states::*;
use fixtures::test_harness::helpers::*;
use fixtures::test_harness::*;

#[test]
fn test_git_initialization_validation() {
    let mut harness = TestHarness::new().unwrap();

    let git_dir = harness.path().join(".git");
    assert!(!git_dir.exists());

    harness.init_git_repository().unwrap();

    assert!(git_dir.exists());
    assert!(git_dir.is_dir());
    assert!(git_dir.join("config").exists());
    assert!(git_dir.join("HEAD").exists());

    let config_output = Command::new("git")
        .args(["config", "--get", "user.name"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    assert!(config_output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&config_output.stdout).trim(),
        "Test User"
    );

    let email_output = Command::new("git")
        .args(["config", "--get", "user.email"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    assert!(email_output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&email_output.stdout).trim(),
        "test@example.com"
    );
}

#[test]
fn test_git_add_operations_validation() {
    let mut harness = git_harness().unwrap();

    let files = vec![
        ("staged_file_1.txt", "Content for staged file 1"),
        ("staged_file_2.rs", "fn test() { println!(\"Rust file\"); }"),
        ("nested/staged_file_3.md", "# Nested Markdown File"),
    ];

    for (filename, content) in &files {
        harness.create_file(filename, content).unwrap();
    }

    let status_output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    let status_text = String::from_utf8_lossy(&status_output.stdout);

    for (filename, _) in &files {
        assert!(status_text.contains(&format!("?? {}", filename)));
    }

    let add_output = Command::new("git")
        .args(["add", "."])
        .current_dir(harness.path())
        .output()
        .unwrap();
    assert!(add_output.status.success());

    let staged_output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    let staged_text = String::from_utf8_lossy(&staged_output.stdout);

    for (filename, _) in &files {
        assert!(staged_text.contains(&format!("A  {}", filename)));
    }
}

#[test]
fn test_git_commit_with_validation() {
    let mut harness = git_harness().unwrap();

    harness
        .create_file("commit_test.txt", "File for commit validation")
        .unwrap();

    let result = harness.commit_all("Test commit for validation");
    assert!(result.is_ok());

    let log_output = Command::new("git")
        .args(["log", "--format=%s", "-1"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    assert!(log_output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&log_output.stdout).trim(),
        "Test commit for validation"
    );

    let show_output = Command::new("git")
        .args(["show", "--name-only", "HEAD"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    let show_text = String::from_utf8_lossy(&show_output.stdout);
    assert!(show_text.contains("commit_test.txt"));

    let status_output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    let status_text = String::from_utf8_lossy(&status_output.stdout);
    assert!(status_text.is_empty());
}

#[test]
fn test_git_remote_validation() {
    let mut harness = git_harness().unwrap();

    harness
        .add_git_remote("origin", "https://github.com/test-owner/test-repo.git")
        .unwrap();
    harness
        .add_git_remote(
            "upstream",
            "https://github.com/upstream-owner/upstream-repo.git",
        )
        .unwrap();

    let remotes_output = Command::new("git")
        .args(["remote", "-v"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    assert!(remotes_output.status.success());

    let remotes_text = String::from_utf8_lossy(&remotes_output.stdout);
    assert!(remotes_text.contains("origin\thttps://github.com/test-owner/test-repo.git (fetch)"));
    assert!(remotes_text.contains("origin\thttps://github.com/test-owner/test-repo.git (push)"));
    assert!(remotes_text
        .contains("upstream\thttps://github.com/upstream-owner/upstream-repo.git (fetch)"));
    assert!(remotes_text
        .contains("upstream\thttps://github.com/upstream-owner/upstream-repo.git (push)"));

    let remote_count = Command::new("git")
        .args(["remote"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    let count_text = String::from_utf8_lossy(&remote_count.stdout);
    assert_eq!(count_text.lines().count(), 2);
}

#[test]
fn test_git_branch_operations() {
    let mut harness = git_harness().unwrap();

    harness.create_file("initial.txt", "Initial file").unwrap();
    harness.commit_all("Initial commit").unwrap();

    let create_branch = Command::new("git")
        .args(["branch", "feature-branch"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    assert!(create_branch.status.success());

    let branch_output = Command::new("git")
        .args(["branch"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    let branch_text = String::from_utf8_lossy(&branch_output.stdout);
    assert!(branch_text.contains("feature-branch"));
    assert!(branch_text.contains("* main") || branch_text.contains("* master"));

    let checkout_output = Command::new("git")
        .args(["checkout", "feature-branch"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    assert!(checkout_output.status.success());

    harness.create_file("feature.txt", "Feature file").unwrap();
    harness.commit_all("Add feature").unwrap();

    let current_branch = Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&current_branch.stdout).trim(),
        "feature-branch"
    );
}

#[test]
fn test_git_diff_operations() {
    let mut harness = git_harness().unwrap();

    harness
        .create_file("diff_test.txt", "Original content")
        .unwrap();
    harness.commit_all("Initial commit").unwrap();

    fs::write(harness.path().join("diff_test.txt"), "Modified content").unwrap();

    let diff_output = Command::new("git")
        .args(["diff", "diff_test.txt"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    assert!(diff_output.status.success());

    let diff_text = String::from_utf8_lossy(&diff_output.stdout);
    assert!(diff_text.contains("-Original content"));
    assert!(diff_text.contains("+Modified content"));

    let diff_stat = Command::new("git")
        .args(["diff", "--stat"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    let stat_text = String::from_utf8_lossy(&diff_stat.stdout);
    assert!(stat_text.contains("diff_test.txt"));
    assert!(stat_text.contains("1 file changed"));
}

#[test]
fn test_git_log_and_history_validation() {
    let mut harness = git_harness().unwrap();

    let commits = vec![
        ("file1.txt", "Content 1", "First commit"),
        ("file2.txt", "Content 2", "Second commit"),
        ("file3.txt", "Content 3", "Third commit"),
    ];

    for (filename, content, message) in &commits {
        harness.create_file(filename, content).unwrap();
        harness.commit_all(message).unwrap();
    }

    let log_output = Command::new("git")
        .args(["log", "--oneline"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    let log_text = String::from_utf8_lossy(&log_output.stdout);

    assert_eq!(log_text.lines().count(), 3);
    for (_, _, message) in &commits {
        assert!(log_text.contains(message));
    }

    let log_format = Command::new("git")
        .args(["log", "--format=%H %s", "-3"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    let format_text = String::from_utf8_lossy(&log_format.stdout);

    for line in format_text.lines() {
        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        assert_eq!(parts[0].len(), 40); // SHA-1 hash length
        assert!(commits.iter().any(|(_, _, msg)| parts[1] == *msg));
    }
}

#[test]
fn test_git_status_comprehensive_validation() {
    let mut harness = git_harness().unwrap();

    harness.create_file("tracked.txt", "Tracked file").unwrap();
    harness.commit_all("Add tracked file").unwrap();

    harness
        .create_file("untracked.txt", "Untracked file")
        .unwrap();
    fs::write(harness.path().join("tracked.txt"), "Modified tracked file").unwrap();

    let status_output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    let status_text = String::from_utf8_lossy(&status_output.stdout);

    assert!(status_text.contains(" M tracked.txt"));
    assert!(status_text.contains("?? untracked.txt"));

    let add_output = Command::new("git")
        .args(["add", "tracked.txt"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    assert!(add_output.status.success());

    let staged_status = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    let staged_text = String::from_utf8_lossy(&staged_status.stdout);

    assert!(staged_text.contains("M  tracked.txt"));
    assert!(staged_text.contains("?? untracked.txt"));
}

#[test]
fn test_git_reset_operations() {
    let mut harness = git_harness().unwrap();

    harness.create_file("reset_test.txt", "Original").unwrap();
    harness.commit_all("Initial commit").unwrap();

    fs::write(harness.path().join("reset_test.txt"), "Modified").unwrap();

    let add_output = Command::new("git")
        .args(["add", "reset_test.txt"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    assert!(add_output.status.success());

    let staged_status = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    let staged_text = String::from_utf8_lossy(&staged_status.stdout);
    assert!(staged_text.contains("M  reset_test.txt"));

    let reset_output = Command::new("git")
        .args(["reset", "HEAD", "reset_test.txt"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    assert!(reset_output.status.success());

    let unstaged_status = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    let unstaged_text = String::from_utf8_lossy(&unstaged_status.stdout);
    assert!(unstaged_text.contains(" M reset_test.txt"));
}

#[test]
fn test_git_clean_operations() {
    let mut harness = git_harness().unwrap();

    harness.create_file("tracked.txt", "Tracked").unwrap();
    harness.commit_all("Add tracked file").unwrap();

    harness
        .create_file("untracked1.txt", "Untracked 1")
        .unwrap();
    harness
        .create_file("untracked2.tmp", "Untracked 2")
        .unwrap();

    let status_before = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    let before_text = String::from_utf8_lossy(&status_before.stdout);
    assert!(before_text.contains("?? untracked1.txt"));
    assert!(before_text.contains("?? untracked2.tmp"));

    let clean_check = Command::new("git")
        .args(["clean", "-n"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    let check_text = String::from_utf8_lossy(&clean_check.stdout);
    assert!(check_text.contains("untracked1.txt"));
    assert!(check_text.contains("untracked2.tmp"));

    let clean_output = Command::new("git")
        .args(["clean", "-f"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    assert!(clean_output.status.success());

    let status_after = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    let after_text = String::from_utf8_lossy(&status_after.stdout);
    assert!(after_text.is_empty());

    assert!(!harness.path().join("untracked1.txt").exists());
    assert!(!harness.path().join("untracked2.tmp").exists());
    assert!(harness.path().join("tracked.txt").exists());
}

#[test]
fn test_git_repository_validation_with_fixtures() {
    let fixture = RepositoryStateFixture::empty_repository();
    let temp_repo = fixture.create_temp_repository().unwrap();

    let git_check = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .current_dir(temp_repo.path())
        .output()
        .unwrap();
    assert!(git_check.status.success());
    assert_eq!(String::from_utf8_lossy(&git_check.stdout).trim(), ".git");

    let repo_root = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(temp_repo.path())
        .output()
        .unwrap();
    assert!(repo_root.status.success());

    let config_output = Command::new("git")
        .args(["config", "--list", "--local"])
        .current_dir(temp_repo.path())
        .output()
        .unwrap();
    let config_text = String::from_utf8_lossy(&config_output.stdout);
    assert!(config_text.contains("user.name=Test User"));
    assert!(config_text.contains("user.email=test@example.com"));
}

#[test]
fn test_git_operations_error_handling() {
    let mut harness = TestHarness::new().unwrap();

    let invalid_git_command = Command::new("git")
        .args(["status"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    assert!(!invalid_git_command.status.success());

    harness.init_git_repository().unwrap();

    let empty_commit = Command::new("git")
        .args(["commit", "-m", "Empty commit"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    assert!(!empty_commit.status.success());

    harness.create_file("test.txt", "content").unwrap();
    harness.commit_all("Valid commit").unwrap();

    let valid_status = Command::new("git")
        .args(["status"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    assert!(valid_status.status.success());
}

#[test]
fn test_git_tag_operations() {
    let mut harness = git_harness().unwrap();

    harness
        .create_file("tagged.txt", "File for tagging")
        .unwrap();
    harness.commit_all("Commit for tagging").unwrap();

    let tag_output = Command::new("git")
        .args(["tag", "v1.0.0"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    assert!(tag_output.status.success());

    let tags_list = Command::new("git")
        .args(["tag", "-l"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    let tags_text = String::from_utf8_lossy(&tags_list.stdout);
    assert!(tags_text.contains("v1.0.0"));

    let annotated_tag = Command::new("git")
        .args(["tag", "-a", "v2.0.0", "-m", "Version 2.0.0"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    assert!(annotated_tag.status.success());

    let show_tag = Command::new("git")
        .args(["show", "v2.0.0"])
        .current_dir(harness.path())
        .output()
        .unwrap();
    let show_text = String::from_utf8_lossy(&show_tag.stdout);
    assert!(show_text.contains("Version 2.0.0"));
}
