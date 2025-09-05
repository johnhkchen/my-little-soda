/// Comprehensive integration tests for the enhanced doctor command
///
/// This test file covers all the new functionality added to the doctor command:
/// - Advanced label management capabilities testing
/// - Issue label state validation diagnostics  
/// - Comprehensive system health checks with recommendations
/// - End-to-end workflow validation diagnostics
use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;

/// Test that the doctor command includes all new diagnostic categories
#[test]
fn test_doctor_comprehensive_diagnostics() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    let assert_result = cmd.arg("doctor").arg("--format").arg("json").assert();
    let output = assert_result.get_output();

    let stdout = String::from_utf8(output.stdout.clone()).unwrap();

    // Parse JSON output to verify all diagnostic categories are present
    if let Ok(json_value) = serde_json::from_str::<Value>(&stdout) {
        let checks = json_value["checks"].as_object().unwrap();

        // Verify advanced label management diagnostics are present
        assert!(
            checks.contains_key("repository_write_permissions"),
            "repository_write_permissions diagnostic missing"
        );
        assert!(
            checks.contains_key("label_management_capabilities"),
            "label_management_capabilities diagnostic missing"
        );

        // Verify issue label state validation diagnostics are present
        assert!(
            checks.contains_key("issue_label_states"),
            "issue_label_states diagnostic missing"
        );
        assert!(
            checks.contains_key("workflow_label_compliance"),
            "workflow_label_compliance diagnostic missing"
        );

        // Verify end-to-end workflow validation diagnostics are present
        assert!(
            checks.contains_key("agent_lifecycle_readiness"),
            "agent_lifecycle_readiness diagnostic missing"
        );
        assert!(
            checks.contains_key("issue_workflow_integration"),
            "issue_workflow_integration diagnostic missing"
        );
        assert!(
            checks.contains_key("branch_pr_workflow"),
            "branch_pr_workflow diagnostic missing"
        );
        assert!(
            checks.contains_key("agent_coordination_readiness"),
            "agent_coordination_readiness diagnostic missing"
        );
        assert!(
            checks.contains_key("workflow_simulation"),
            "workflow_simulation diagnostic missing"
        );
    } else {
        panic!("Failed to parse doctor JSON output");
    }
}

/// Test that the doctor command with verbose flag shows detailed diagnostic information
#[test]
fn test_doctor_verbose_comprehensive_output() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    cmd.arg("doctor")
        .arg("--verbose")
        .assert()
        .stdout(predicate::str::contains("🩺 MY LITTLE SODA DOCTOR"))
        .stdout(predicate::str::contains("📊 DIAGNOSTIC SUMMARY"))
        .stdout(predicate::str::contains("🔍 DETAILED RESULTS"))
        .stdout(predicate::str::contains(
            "🏥 COMPREHENSIVE HEALTH RECOMMENDATIONS",
        ))
        .stdout(predicate::str::contains("Details:"));
}

/// Test comprehensive health recommendations section
#[test]
fn test_doctor_health_recommendations() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    cmd.arg("doctor")
        .assert()
        .stdout(predicate::str::contains(
            "🏥 COMPREHENSIVE HEALTH RECOMMENDATIONS",
        ))
        .stdout(predicate::str::contains("SYSTEM STATUS:"));
}

/// Test that the doctor command handles various diagnostic statuses correctly
#[test]
fn test_doctor_diagnostic_status_handling() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    let assert_result = cmd.arg("doctor").arg("--format").arg("json").assert();
    let output = assert_result.get_output();

    let stdout = String::from_utf8(output.stdout.clone()).unwrap();

    if let Ok(json_value) = serde_json::from_str::<Value>(&stdout) {
        let summary = &json_value["summary"];

        // Verify that diagnostic summary includes all status counts
        assert!(
            summary.get("total_checks").is_some(),
            "total_checks missing from summary"
        );
        assert!(
            summary.get("passed").is_some(),
            "passed count missing from summary"
        );
        assert!(
            summary.get("failed").is_some(),
            "failed count missing from summary"
        );
        assert!(
            summary.get("warnings").is_some(),
            "warnings count missing from summary"
        );

        // Verify that individual checks have proper status and message structure
        let checks = json_value["checks"].as_object().unwrap();
        for (_check_name, check_data) in checks {
            assert!(
                check_data.get("status").is_some(),
                "Check missing status field"
            );
            assert!(
                check_data.get("message").is_some(),
                "Check missing message field"
            );

            // Status should be one of: Pass, Fail, Warning, Info
            let status = check_data["status"].as_str().unwrap();
            assert!(
                ["Pass", "Fail", "Warning", "Info"].contains(&status),
                "Invalid status: {}",
                status
            );
        }
    }
}

/// Test label management capabilities diagnostic in isolation
#[test]
fn test_label_management_diagnostics() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    let assert_result = cmd.arg("doctor").arg("--format").arg("json").assert();
    let output = assert_result.get_output();

    let stdout = String::from_utf8(output.stdout.clone()).unwrap();

    if let Ok(json_value) = serde_json::from_str::<Value>(&stdout) {
        let checks = json_value["checks"].as_object().unwrap();

        // Test repository write permissions diagnostic
        if let Some(write_perms) = checks.get("repository_write_permissions") {
            assert!(write_perms.get("status").is_some());
            assert!(write_perms.get("message").is_some());
        }

        // Test label management capabilities diagnostic
        if let Some(label_mgmt) = checks.get("label_management_capabilities") {
            assert!(label_mgmt.get("status").is_some());
            assert!(label_mgmt.get("message").is_some());
        }
    }
}

/// Test issue label state validation diagnostic
#[test]
fn test_issue_label_state_validation() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    let assert_result = cmd.arg("doctor").arg("--format").arg("json").assert();
    let output = assert_result.get_output();

    let stdout = String::from_utf8(output.stdout.clone()).unwrap();

    if let Ok(json_value) = serde_json::from_str::<Value>(&stdout) {
        let checks = json_value["checks"].as_object().unwrap();

        // Test issue label states diagnostic
        if let Some(label_states) = checks.get("issue_label_states") {
            assert!(label_states.get("status").is_some());
            assert!(label_states.get("message").is_some());
        }

        // Test workflow label compliance diagnostic
        if let Some(workflow_compliance) = checks.get("workflow_label_compliance") {
            assert!(workflow_compliance.get("status").is_some());
            assert!(workflow_compliance.get("message").is_some());
        }
    }
}

/// Test end-to-end workflow validation diagnostics
#[test]
fn test_end_to_end_workflow_validation() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    let assert_result = cmd.arg("doctor").arg("--format").arg("json").assert();
    let output = assert_result.get_output();

    let stdout = String::from_utf8(output.stdout.clone()).unwrap();

    if let Ok(json_value) = serde_json::from_str::<Value>(&stdout) {
        let checks = json_value["checks"].as_object().unwrap();

        // Test all end-to-end workflow validation diagnostics
        let workflow_checks = [
            "agent_lifecycle_readiness",
            "issue_workflow_integration",
            "branch_pr_workflow",
            "agent_coordination_readiness",
            "workflow_simulation",
        ];

        for check_name in &workflow_checks {
            if let Some(check_data) = checks.get(*check_name) {
                assert!(
                    check_data.get("status").is_some(),
                    "{} missing status",
                    check_name
                );
                assert!(
                    check_data.get("message").is_some(),
                    "{} missing message",
                    check_name
                );
            }
        }
    }
}

/// Test that comprehensive health recommendations provide actionable guidance
#[test]
fn test_comprehensive_health_recommendations_content() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    cmd.arg("doctor")
        .assert()
        .stdout(predicate::str::contains(
            "🏥 COMPREHENSIVE HEALTH RECOMMENDATIONS",
        ))
        .stdout(predicate::str::contains("SYSTEM STATUS:"))
        .stdout(predicate::str::contains("Next steps:"));
}

/// Test that the doctor command integrates properly with the help system
#[test]
fn test_doctor_help_integration() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    cmd.arg("doctor")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Run system diagnostics and health checks",
        ))
        .stdout(predicate::str::contains("--format"))
        .stdout(predicate::str::contains("--verbose"));
}

/// Test that the doctor command handles edge cases gracefully
#[test]
fn test_doctor_error_handling() {
    // Test with invalid format option
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    cmd.arg("doctor")
        .arg("--format")
        .arg("invalid")
        .assert()
        .failure();
}

/// Integration test for the complete doctor command workflow
#[test]
fn test_doctor_complete_workflow() {
    // Test JSON format
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();
    let assert_result = cmd.arg("doctor").arg("--format").arg("json").assert();
    let json_output = assert_result.get_output();

    let json_stdout = String::from_utf8(json_output.stdout.clone()).unwrap();
    assert!(
        serde_json::from_str::<Value>(&json_stdout).is_ok(),
        "JSON output is not valid JSON"
    );

    // Test text format
    let mut cmd2 = Command::cargo_bin("my-little-soda").unwrap();
    cmd2.arg("doctor")
        .assert()
        .stdout(predicate::str::contains("🩺 MY LITTLE SODA DOCTOR"));

    // Test verbose text format
    let mut cmd3 = Command::cargo_bin("my-little-soda").unwrap();
    cmd3.arg("doctor")
        .arg("--verbose")
        .assert()
        .stdout(predicate::str::contains("Details:"));
}

/// Performance test to ensure doctor command completes in reasonable time
#[test]
fn test_doctor_performance() {
    use std::time::Instant;

    let start = Instant::now();

    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();
    cmd.arg("doctor").assert();

    let duration = start.elapsed();

    // Doctor command should complete within 30 seconds
    assert!(
        duration.as_secs() < 30,
        "Doctor command took too long: {:?}",
        duration
    );
}
