// Test-driven development for cargo run default behavior
// Following TDD approach: Red -> Green -> Refactor

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_cargo_run_shows_task_instructions() {
    // Test that running `cargo run` without arguments shows helpful guidance
    let mut cmd = Command::cargo_bin("clambake").unwrap();
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("CLAMBAKE - Get a Task to Work On"))
        .stdout(predicate::str::contains("clambake pop"))
        .stdout(predicate::str::contains("route:ready"))
        .stdout(predicate::str::contains("gh issue create"));
}

#[test]
fn test_cargo_run_shows_status_information() {
    // Test that the default output includes system status
    let mut cmd = Command::cargo_bin("clambake").unwrap();
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("🤖 SYSTEM STATUS"))
        .stdout(predicate::str::contains("Available agents:"))
        .stdout(predicate::str::contains("Ready tasks:"));
}

#[test]
fn test_cargo_run_provides_actionable_guidance() {
    // Test that the output gives users clear next steps
    let mut cmd = Command::cargo_bin("clambake").unwrap();
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("clambake pop --mine"))
        .stdout(predicate::str::contains("git checkout"))
        .stdout(predicate::str::contains("EXAMPLE WORKFLOW:"))
        .stdout(predicate::str::contains("📊 Quick start:"));
}