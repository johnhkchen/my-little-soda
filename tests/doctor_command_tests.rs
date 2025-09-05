use assert_cmd::Command;
use predicates::prelude::*;
use serde_json;
use tempfile::TempDir;
use std::env;

#[test]
fn test_doctor_help() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();
    
    cmd.arg("doctor").arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Run system diagnostics and health checks"))
        .stdout(predicate::str::contains("--format"))
        .stdout(predicate::str::contains("--verbose"));
}

#[test]
fn test_doctor_default_text_output() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();
    
    cmd.arg("doctor")
        .assert()
        .stdout(predicate::str::contains("🩺 MY LITTLE SODA DOCTOR"))
        .stdout(predicate::str::contains("📊 DIAGNOSTIC SUMMARY"))
        .stdout(predicate::str::contains("🔍 DETAILED RESULTS"));
}

#[test]
fn test_doctor_verbose_flag() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();
    
    cmd.arg("doctor").arg("--verbose")
        .assert()
        .stdout(predicate::str::contains("🩺 MY LITTLE SODA DOCTOR"))
        .stdout(predicate::str::contains("Details:"));
}

#[test]
fn test_doctor_json_output() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();
    
    let output = cmd.arg("doctor").arg("--format").arg("json")
        .assert()
        .success()
        .get_output();
    
    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    
    // Verify it's valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    
    // Verify expected structure
    assert!(parsed["summary"].is_object());
    assert!(parsed["checks"].is_object());
    assert!(parsed["summary"]["total_checks"].is_number());
}

#[test]
fn test_doctor_detects_git_repository() {
    // This test runs in the project directory which should have a git repo
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();
    
    cmd.arg("doctor")
        .assert()
        .stdout(predicate::str::contains("✅ git_repository: Git repository detected"));
}

#[test]
fn test_doctor_detects_no_git_repository() {
    // Create a temporary directory without git
    let temp_dir = TempDir::new().unwrap();
    
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();
    
    cmd.current_dir(temp_dir.path())
        .arg("doctor")
        .assert()
        .stdout(predicate::str::contains("❌ git_repository: Not in a git repository"));
}

#[test]
fn test_doctor_github_token_detection() {
    // Test with token set
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();
    
    cmd.env("MY_LITTLE_SODA_GITHUB_TOKEN", "test_token")
        .arg("doctor")
        .assert()
        .stdout(predicate::str::contains("✅ github_token: GitHub token configured"));
}

#[test]
fn test_doctor_no_github_token() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();
    
    // Make sure the token is not set
    cmd.env_remove("MY_LITTLE_SODA_GITHUB_TOKEN")
        .arg("doctor")
        .assert()
        .stdout(predicate::str::contains("❌ github_token: GitHub token not configured"));
}

#[test]
fn test_doctor_dependency_checks() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();
    
    cmd.arg("doctor")
        .assert()
        .stdout(predicate::str::contains("git_available"))
        .stdout(predicate::str::contains("gh_available"));
}

#[test]
fn test_doctor_exit_code_on_failure() {
    // Run in temp dir without git to force failures
    let temp_dir = TempDir::new().unwrap();
    
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();
    
    cmd.current_dir(temp_dir.path())
        .env_remove("MY_LITTLE_SODA_GITHUB_TOKEN")
        .arg("doctor")
        .assert()
        .failure(); // Should exit with non-zero code
}

#[test]
fn test_doctor_json_format_structure() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();
    
    let output = cmd.arg("doctor").arg("--format").arg("json")
        .env("MY_LITTLE_SODA_GITHUB_TOKEN", "test_token") // Avoid failure exit
        .assert()
        .get_output();
    
    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    
    // Verify summary structure
    let summary = &parsed["summary"];
    assert!(summary["total_checks"].is_number());
    assert!(summary["passed"].is_number());
    assert!(summary["failed"].is_number());
    assert!(summary["warnings"].is_number());
    assert!(summary["info"].is_number());
    
    // Verify checks structure
    let checks = &parsed["checks"];
    assert!(checks.is_object());
    
    // Check that basic diagnostic checks are present
    assert!(checks["git_repository"].is_object());
    assert!(checks["github_token"].is_object());
    assert!(checks["git_available"].is_object());
}