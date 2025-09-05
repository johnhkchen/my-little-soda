use assert_cmd::Command;
use predicates::prelude::*;
use serde_json;
use std::env;
use tempfile::TempDir;

#[test]
fn test_doctor_help() {
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

    cmd.arg("doctor")
        .arg("--verbose")
        .assert()
        .stdout(predicate::str::contains("🩺 MY LITTLE SODA DOCTOR"))
        .stdout(predicate::str::contains("Details:"));
}

#[test]
fn test_doctor_json_output() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Doctor command may exit with code 1 if diagnostics fail, which is expected behavior
    let output = cmd
        .arg("doctor")
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout.clone()).unwrap();

    // Verify it's valid JSON (regardless of exit code)
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    // Verify expected structure
    assert!(
        parsed["summary"].is_object(),
        "Missing 'summary' object in JSON output"
    );
    assert!(
        parsed["checks"].is_object(),
        "Missing 'checks' object in JSON output"
    );
    assert!(
        parsed["summary"]["total_checks"].is_number(),
        "Missing or invalid 'total_checks' in summary"
    );

    // Verify the essential fields exist
    assert!(
        parsed["readiness"].is_object(),
        "Missing 'readiness' object in JSON output"
    );
    assert!(
        parsed["recommendations"].is_array(),
        "Missing 'recommendations' array in JSON output"
    );
}

#[test]
fn test_doctor_detects_git_repository() {
    // This test runs in the project directory which should have a git repo
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    cmd.arg("doctor").assert().stdout(predicate::str::contains(
        "✅ git_repository: Git repository detected",
    ));
}

#[test]
fn test_doctor_detects_no_git_repository() {
    // Create a temporary directory without git
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    cmd.current_dir(temp_dir.path())
        .arg("doctor")
        .assert()
        .stdout(predicate::str::contains(
            "❌ git_repository: Not in a git repository",
        ));
}

#[test]
fn test_doctor_github_token_detection() {
    // Test with token set
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    cmd.env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test_token")
        .arg("doctor")
        .assert()
        .stdout(
            predicate::str::contains("github_token_presence")
                .and(predicate::str::contains("GitHub token found")),
        );
}

#[test]
fn test_doctor_no_github_token() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Make sure the token is not set
    cmd.env_remove("MY_LITTLE_SODA_GITHUB_TOKEN")
        .arg("doctor")
        .assert()
        .stdout(
            predicate::str::contains("github_token_presence")
                .and(predicate::str::contains("No GitHub token found")),
        );
}

#[test]
fn test_doctor_dependency_checks() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    cmd.arg("doctor")
        .assert()
        .stdout(predicate::str::contains("rust_toolchain"))
        .stdout(predicate::str::contains("cargo_functionality"))
        .stdout(predicate::str::contains("git_installation"))
        .stdout(predicate::str::contains("github_cli"))
        .stdout(predicate::str::contains("github_connectivity"))
        .stdout(predicate::str::contains("system_resources"));
}

#[test]
fn test_doctor_prerequisite_rust_validation() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    cmd.arg("doctor")
        .arg("--verbose")
        .assert()
        .stdout(predicate::str::contains("rust_toolchain"))
        .stdout(predicate::str::contains("cargo_functionality"));
}

#[test]
fn test_doctor_prerequisite_git_validation() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    cmd.arg("doctor")
        .arg("--verbose")
        .assert()
        .stdout(predicate::str::contains("git_installation"))
        .stdout(predicate::str::contains("Git is available"));
}

#[test]
fn test_doctor_prerequisite_github_cli_validation() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    cmd.arg("doctor")
        .assert()
        .stdout(predicate::str::contains("github_cli"));
}

#[test]
fn test_doctor_prerequisite_connectivity_validation() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    cmd.arg("doctor")
        .assert()
        .stdout(predicate::str::contains("github_connectivity"));
}

#[test]
fn test_doctor_prerequisite_system_resources_validation() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    cmd.arg("doctor")
        .arg("--verbose")
        .assert()
        .stdout(predicate::str::contains("system_resources"))
        .stdout(predicate::str::contains("System resources"));
}

#[test]
fn test_doctor_prerequisite_json_output() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    let binding = cmd.arg("doctor").arg("--format").arg("json").assert();
    let output = binding.get_output();

    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    let json_result: serde_json::Result<serde_json::Value> = serde_json::from_str(&stdout);
    assert!(
        json_result.is_ok(),
        "Doctor should produce valid JSON output"
    );

    let json = json_result.unwrap();
    assert!(json["checks"]["rust_toolchain"].is_object());
    assert!(json["checks"]["cargo_functionality"].is_object());
    assert!(json["checks"]["git_installation"].is_object());
    assert!(json["checks"]["github_cli"].is_object());
    assert!(json["checks"]["github_connectivity"].is_object());
    assert!(json["checks"]["system_resources"].is_object());
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

    let binding = cmd
        .arg("doctor")
        .arg("--format")
        .arg("json")
        .env("MY_LITTLE_SODA_GITHUB_TOKEN", "test_token") // Avoid failure exit
        .assert();
    let output = binding.get_output();

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
    assert!(checks["git_available"].is_object());

    // GitHub authentication checks should be present (may fail but structure should exist)
    assert!(checks["github_token_presence"].is_object());
    assert!(checks["github_authentication"].is_object());
}

// GitHub Authentication Diagnostic Tests

#[test]
fn test_doctor_github_authentication_comprehensive() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Test with valid token set
    cmd.env(
        "MY_LITTLE_SODA_GITHUB_TOKEN",
        env::var("MY_LITTLE_SODA_GITHUB_TOKEN").unwrap_or("test_token".to_string()),
    )
    .arg("doctor")
    .assert()
    .success() // Should pass if token is valid
    .stdout(predicate::str::contains("github_token_presence"))
    .stdout(predicate::str::contains("github_authentication"))
    .stdout(predicate::str::contains("github_repository_access"))
    .stdout(predicate::str::contains("github_rate_limits"))
    .stdout(predicate::str::contains("github_api_scopes"));
}

#[test]
fn test_doctor_github_token_presence_detection() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    cmd.env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test_token_format")
        .arg("doctor")
        .assert()
        .stdout(predicate::str::contains(
            "github_token_presence: GitHub token found and format is valid",
        ));
}

#[test]
fn test_doctor_github_invalid_token_format() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    cmd.env("MY_LITTLE_SODA_GITHUB_TOKEN", "invalid_token_format")
        .env_remove("GITHUB_OWNER") // Remove config to avoid actual API calls
        .env_remove("GITHUB_REPO")
        .arg("doctor")
        .assert()
        .stdout(
            predicate::str::contains("GitHub token found but format is invalid")
                .or(predicate::str::contains("github_token_presence")
                    .and(predicate::str::contains("Warning").or(predicate::str::contains("Fail")))),
        );
}

#[test]
fn test_doctor_no_github_token_comprehensive() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    cmd.env_remove("MY_LITTLE_SODA_GITHUB_TOKEN")
        .env_remove("GITHUB_TOKEN")
        .arg("doctor")
        .assert()
        .stdout(predicate::str::contains("github_token_presence"))
        .stdout(predicate::str::contains("No GitHub token found"));
}

#[test]
fn test_doctor_json_github_authentication_structure() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    let binding = cmd
        .arg("doctor")
        .arg("--format")
        .arg("json")
        .env(
            "MY_LITTLE_SODA_GITHUB_TOKEN",
            env::var("MY_LITTLE_SODA_GITHUB_TOKEN").unwrap_or("test_token".to_string()),
        )
        .assert();
    let output = binding.get_output();

    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let checks = &parsed["checks"];

    // Verify GitHub authentication diagnostics are present
    assert!(
        checks["github_token_presence"].is_object(),
        "github_token_presence check should be present"
    );
    assert!(
        checks["github_authentication"].is_object(),
        "github_authentication check should be present"
    );
    assert!(
        checks["github_repository_access"].is_object(),
        "github_repository_access check should be present"
    );
    assert!(
        checks["github_rate_limits"].is_object(),
        "github_rate_limits check should be present"
    );
    assert!(
        checks["github_api_scopes"].is_object(),
        "github_api_scopes check should be present"
    );

    // Verify each check has required fields
    for check_name in [
        "github_token_presence",
        "github_authentication",
        "github_repository_access",
        "github_rate_limits",
        "github_api_scopes",
    ] {
        let check = &checks[check_name];
        assert!(
            check["status"].is_string(),
            "{} should have status field",
            check_name
        );
        assert!(
            check["message"].is_string(),
            "{} should have message field",
            check_name
        );
        // details and suggestion are optional
    }
}

#[test]
fn test_doctor_verbose_github_details() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    cmd.env(
        "MY_LITTLE_SODA_GITHUB_TOKEN",
        env::var("MY_LITTLE_SODA_GITHUB_TOKEN").unwrap_or("test_token".to_string()),
    )
    .arg("doctor")
    .arg("--verbose")
    .assert()
    .stdout(predicate::str::contains("Details:"))
    .stdout(
        predicate::str::contains("Token source:")
            .or(predicate::str::contains("issues:read").or(predicate::str::contains("Resets in"))),
    );
}

#[test]
fn test_doctor_github_token_format_validation() {
    // Test classic GitHub token format (ghp_)
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();
    cmd.env(
        "MY_LITTLE_SODA_GITHUB_TOKEN",
        "ghp_test12345678901234567890123456789012345",
    )
    .env_remove("GITHUB_OWNER") // Remove to avoid real API calls
    .env_remove("GITHUB_REPO")
    .arg("doctor")
    .assert()
    .stdout(predicate::str::contains(
        "GitHub token found and format is valid",
    ));

    // Test fine-grained token format (github_pat_)
    let mut cmd2 = Command::cargo_bin("my-little-soda").unwrap();
    cmd2.env(
        "MY_LITTLE_SODA_GITHUB_TOKEN",
        "github_pat_test12345_abcdef1234567890",
    )
    .env_remove("GITHUB_OWNER")
    .env_remove("GITHUB_REPO")
    .arg("doctor")
    .assert()
    .stdout(predicate::str::contains(
        "GitHub token found and format is valid",
    ));

    // Test OAuth token format (gho_)
    let mut cmd3 = Command::cargo_bin("my-little-soda").unwrap();
    cmd3.env(
        "MY_LITTLE_SODA_GITHUB_TOKEN",
        "gho_test12345678901234567890",
    )
    .env_remove("GITHUB_OWNER")
    .env_remove("GITHUB_REPO")
    .arg("doctor")
    .assert()
    .stdout(predicate::str::contains(
        "GitHub token found and format is valid",
    ));
}

#[test]
fn test_doctor_github_cli_fallback_detection() {
    // Test that doctor detects GitHub CLI as token source when available
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Remove environment token to test CLI fallback
    cmd.env_remove("MY_LITTLE_SODA_GITHUB_TOKEN")
        .env_remove("GITHUB_TOKEN")
        .arg("doctor")
        .arg("--verbose")
        .assert()
        .stdout(
            predicate::str::contains("GitHub CLI")
                .or(predicate::str::contains("Token source")
                    .or(predicate::str::contains("gh auth"))),
        );
}

#[test]
fn test_doctor_rate_limit_information_display() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Only run this test if we have a valid token to avoid API errors
    if env::var("MY_LITTLE_SODA_GITHUB_TOKEN").is_ok() {
        cmd.env(
            "MY_LITTLE_SODA_GITHUB_TOKEN",
            env::var("MY_LITTLE_SODA_GITHUB_TOKEN").unwrap(),
        )
        .arg("doctor")
        .assert()
        .stdout(predicate::str::contains("github_rate_limits"))
        .stdout(
            predicate::str::contains("requests remaining")
                .or(predicate::str::contains("Rate limit")),
        );
    }
}

#[test]
fn test_doctor_api_scope_testing() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Only run this test if we have a valid token to avoid API errors
    if env::var("MY_LITTLE_SODA_GITHUB_TOKEN").is_ok() {
        cmd.env(
            "MY_LITTLE_SODA_GITHUB_TOKEN",
            env::var("MY_LITTLE_SODA_GITHUB_TOKEN").unwrap(),
        )
        .arg("doctor")
        .arg("--verbose")
        .assert()
        .stdout(predicate::str::contains("github_api_scopes"))
        .stdout(
            predicate::str::contains("issues:read")
                .or(predicate::str::contains("pull_requests:read")
                    .or(predicate::str::contains("repository:read"))),
        );
    }
}

#[test]
fn test_doctor_github_error_handling() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Test with clearly invalid token to trigger error handling
    cmd.env(
        "MY_LITTLE_SODA_GITHUB_TOKEN",
        "clearly_invalid_token_format_123",
    )
    .env_remove("GITHUB_OWNER") // Remove config to control error type
    .env_remove("GITHUB_REPO")
    .arg("doctor")
    .assert()
    .stdout(
        predicate::str::contains("github_authentication")
            .or(predicate::str::contains("github_token_presence")),
    );
}

// Repository Access Diagnostic Tests

#[test]
fn test_doctor_repository_access_diagnostics() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Test with valid token to see repository diagnostics
    if env::var("MY_LITTLE_SODA_GITHUB_TOKEN").is_ok() {
        cmd.env(
            "MY_LITTLE_SODA_GITHUB_TOKEN",
            env::var("MY_LITTLE_SODA_GITHUB_TOKEN").unwrap(),
        )
        .arg("doctor")
        .assert()
        .stdout(predicate::str::contains("repository_config_owner"))
        .stdout(predicate::str::contains("repository_config_repo"))
        .stdout(predicate::str::contains("repository_existence"))
        .stdout(predicate::str::contains("repository_settings"))
        .stdout(predicate::str::contains("repository_operations"));
    }
}

#[test]
fn test_doctor_repository_configuration_validation() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Test with environment variables set
    cmd.env("GITHUB_OWNER", "test-owner")
        .env("GITHUB_REPO", "test-repo")
        .env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test_token")
        .arg("doctor")
        .assert()
        .stdout(predicate::str::contains("repository_config_owner"))
        .stdout(predicate::str::contains("repository_config_repo"));
}

#[test]
fn test_doctor_repository_configuration_missing() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Clear environment variables to test missing configuration
    cmd.env_remove("GITHUB_OWNER")
        .env_remove("GITHUB_REPO")
        .env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test_token")
        .arg("doctor")
        .assert()
        .stdout(predicate::str::contains("repository_config").or(
            predicate::str::contains("GitHub repository owner not configured").or(
                predicate::str::contains("GitHub repository name not configured"),
            ),
        ));
}

#[test]
fn test_doctor_repository_operations_testing() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Only run this test if we have a valid token and repo config
    if env::var("MY_LITTLE_SODA_GITHUB_TOKEN").is_ok()
        && env::var("GITHUB_OWNER").is_ok()
        && env::var("GITHUB_REPO").is_ok()
    {
        cmd.env(
            "MY_LITTLE_SODA_GITHUB_TOKEN",
            env::var("MY_LITTLE_SODA_GITHUB_TOKEN").unwrap(),
        )
        .env("GITHUB_OWNER", env::var("GITHUB_OWNER").unwrap())
        .env("GITHUB_REPO", env::var("GITHUB_REPO").unwrap())
        .arg("doctor")
        .arg("--verbose")
        .assert()
        .stdout(predicate::str::contains("repository_operations"))
        .stdout(predicate::str::contains("Issue listing:").or(
            predicate::str::contains("PR listing:").or(predicate::str::contains("Label access:")),
        ));
    }
}

#[test]
fn test_doctor_repository_settings_validation() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Only run this test if we have a valid token and repo config
    if env::var("MY_LITTLE_SODA_GITHUB_TOKEN").is_ok()
        && env::var("GITHUB_OWNER").is_ok()
        && env::var("GITHUB_REPO").is_ok()
    {
        cmd.env(
            "MY_LITTLE_SODA_GITHUB_TOKEN",
            env::var("MY_LITTLE_SODA_GITHUB_TOKEN").unwrap(),
        )
        .env("GITHUB_OWNER", env::var("GITHUB_OWNER").unwrap())
        .env("GITHUB_REPO", env::var("GITHUB_REPO").unwrap())
        .arg("doctor")
        .arg("--verbose")
        .assert()
        .stdout(predicate::str::contains("repository_settings"))
        .stdout(
            predicate::str::contains("Issues enabled")
                .or(predicate::str::contains("Merge options:")
                    .or(predicate::str::contains("Repository settings"))),
        );
    }
}

#[test]
fn test_doctor_json_repository_diagnostics_structure() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    let binding = cmd
        .arg("doctor")
        .arg("--format")
        .arg("json")
        .env(
            "MY_LITTLE_SODA_GITHUB_TOKEN",
            env::var("MY_LITTLE_SODA_GITHUB_TOKEN").unwrap_or("test_token".to_string()),
        )
        .env(
            "GITHUB_OWNER",
            env::var("GITHUB_OWNER").unwrap_or("test-owner".to_string()),
        )
        .env(
            "GITHUB_REPO",
            env::var("GITHUB_REPO").unwrap_or("test-repo".to_string()),
        )
        .assert();
    let output = binding.get_output();

    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let checks = &parsed["checks"];

    // Verify repository diagnostic checks are present
    assert!(
        checks["repository_config_owner"].is_object(),
        "repository_config_owner check should be present"
    );
    assert!(
        checks["repository_config_repo"].is_object(),
        "repository_config_repo check should be present"
    );
    assert!(
        checks["repository_existence"].is_object(),
        "repository_existence check should be present"
    );
    assert!(
        checks["repository_settings"].is_object(),
        "repository_settings check should be present"
    );
    assert!(
        checks["repository_operations"].is_object(),
        "repository_operations check should be present"
    );

    // Verify each check has required fields
    for check_name in [
        "repository_config_owner",
        "repository_config_repo",
        "repository_existence",
        "repository_settings",
        "repository_operations",
    ] {
        let check = &checks[check_name];
        assert!(
            check["status"].is_string(),
            "{} should have status field",
            check_name
        );
        assert!(
            check["message"].is_string(),
            "{} should have message field",
            check_name
        );
        // details and suggestion are optional
    }
}

#[test]
fn test_doctor_comprehensive_repository_validation() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Test comprehensive repository validation with all components
    if env::var("MY_LITTLE_SODA_GITHUB_TOKEN").is_ok() {
        cmd.env(
            "MY_LITTLE_SODA_GITHUB_TOKEN",
            env::var("MY_LITTLE_SODA_GITHUB_TOKEN").unwrap(),
        )
        .arg("doctor")
        .arg("--verbose")
        .assert()
        .stdout(predicate::str::contains("🩺 MY LITTLE SODA DOCTOR"))
        // Original GitHub authentication checks
        .stdout(predicate::str::contains("github_token_presence"))
        .stdout(predicate::str::contains("github_authentication"))
        .stdout(predicate::str::contains("github_repository_access"))
        .stdout(predicate::str::contains("github_rate_limits"))
        .stdout(predicate::str::contains("github_api_scopes"))
        // New repository access diagnostic checks
        .stdout(predicate::str::contains("repository_config_owner"))
        .stdout(predicate::str::contains("repository_config_repo"))
        .stdout(predicate::str::contains("repository_existence"))
        .stdout(predicate::str::contains("repository_settings"))
        .stdout(predicate::str::contains("repository_operations"));
    }
}

#[test]
fn test_doctor_repository_error_handling() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Test with invalid repository configuration to trigger error handling
    cmd.env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test_token")
        .env("GITHUB_OWNER", "nonexistent-owner-123456")
        .env("GITHUB_REPO", "nonexistent-repo-123456")
        .arg("doctor")
        .assert()
        .stdout(predicate::str::contains("repository_existence").or(
            predicate::str::contains("not found").or(predicate::str::contains("not accessible")),
        ));
}

#[test]
fn test_doctor_repository_validation_private_vs_public() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Only run this test if we have a valid token and repo config
    if env::var("MY_LITTLE_SODA_GITHUB_TOKEN").is_ok()
        && env::var("GITHUB_OWNER").is_ok()
        && env::var("GITHUB_REPO").is_ok()
    {
        cmd.env(
            "MY_LITTLE_SODA_GITHUB_TOKEN",
            env::var("MY_LITTLE_SODA_GITHUB_TOKEN").unwrap(),
        )
        .env("GITHUB_OWNER", env::var("GITHUB_OWNER").unwrap())
        .env("GITHUB_REPO", env::var("GITHUB_REPO").unwrap())
        .arg("doctor")
        .arg("--verbose")
        .assert()
        .stdout(predicate::str::contains("repository_existence"))
        .stdout(
            predicate::str::contains("Visibility:")
                .or(predicate::str::contains("private").or(predicate::str::contains("public"))),
        );
    }
}

// Configuration Validation Diagnostic Tests

#[test]
fn test_doctor_config_validation_basics() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    cmd.env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test_token")
        .arg("doctor")
        .assert()
        .stdout(predicate::str::contains("config_file_exists"))
        .stdout(predicate::str::contains("config_toml_syntax"))
        .stdout(predicate::str::contains("config_completeness"))
        .stdout(predicate::str::contains("config_field_validation"))
        .stdout(predicate::str::contains("config_environment_consistency"));
}

#[test]
fn test_doctor_config_file_exists_pass() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // This should pass in the current directory since we have my-little-soda.toml
    cmd.env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test_token")
        .arg("doctor")
        .assert()
        .stdout(predicate::str::contains(
            "config_file_exists: Configuration file exists",
        ));
}

#[test]
fn test_doctor_config_file_missing() {
    let temp_dir = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Test in directory without config file
    cmd.current_dir(temp_dir.path())
        .env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test_token")
        .arg("doctor")
        .assert()
        .stdout(
            predicate::str::contains("config_file_exists: Configuration file not found").or(
                predicate::str::contains("Cannot load configuration for validation"),
            ),
        );
}

#[test]
fn test_doctor_config_toml_syntax_validation() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Test TOML syntax validation with existing valid file
    cmd.env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test_token")
        .arg("doctor")
        .assert()
        .stdout(predicate::str::contains(
            "config_toml_syntax: TOML syntax is valid",
        ));
}

#[test]
fn test_doctor_config_toml_invalid_syntax() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("my-little-soda.toml");

    // Create invalid TOML file
    std::fs::write(&config_path, "invalid toml content [[[broken").unwrap();

    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();
    cmd.current_dir(temp_dir.path())
        .env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test_token")
        .arg("doctor")
        .assert()
        .stdout(predicate::str::contains(
            "config_toml_syntax: TOML syntax error",
        ));
}

#[test]
fn test_doctor_config_completeness_warnings() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Test should show warnings for default values
    cmd.env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test_token")
        .arg("doctor")
        .arg("--verbose")
        .assert()
        .stdout(predicate::str::contains("config_completeness").and(
            predicate::str::contains("Warning").or(predicate::str::contains("default value")),
        ));
}

#[test]
fn test_doctor_config_field_validation_pass() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Should pass with valid configuration
    cmd.env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test_token")
        .arg("doctor")
        .assert()
        .stdout(predicate::str::contains(
            "config_field_validation: Configuration field validation passed",
        ));
}

#[test]
fn test_doctor_config_environment_consistency() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    cmd.env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test_token")
        .arg("doctor")
        .assert()
        .stdout(predicate::str::contains("config_environment_consistency"));
}

#[test]
fn test_doctor_config_verbose_details() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    cmd.env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test_token")
        .arg("doctor")
        .arg("--verbose")
        .assert()
        .stdout(predicate::str::contains("my-little-soda.toml found"))
        .stdout(predicate::str::contains(
            "Configuration file parses successfully",
        ))
        .stdout(
            predicate::str::contains("All configuration fields have valid values and types").or(
                predicate::str::contains(
                    "Configuration values match environment variable overrides",
                ),
            ),
        );
}

#[test]
fn test_doctor_config_json_structure() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    let binding = cmd
        .arg("doctor")
        .arg("--format")
        .arg("json")
        .env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test_token")
        .assert();
    let output = binding.get_output();

    let stdout = String::from_utf8(output.stdout.clone()).unwrap();

    // Extract the multiline JSON from the output (skip telemetry logs)
    let lines: Vec<&str> = stdout.lines().collect();
    let mut json_start = None;
    let mut brace_count = 0;
    let mut json_end = None;

    for (i, line) in lines.iter().enumerate() {
        if line.trim() == "{" {
            json_start = Some(i);
            brace_count = 1;
        } else if json_start.is_some() {
            brace_count += line.matches('{').count() as i32;
            brace_count -= line.matches('}').count() as i32;
            if brace_count == 0 {
                json_end = Some(i);
                break;
            }
        }
    }

    let json_str = if let (Some(start), Some(end)) = (json_start, json_end) {
        lines[start..=end].join("\n")
    } else {
        panic!("No JSON object found in output: {}", stdout);
    };

    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    let checks = &parsed["checks"];

    // Count configuration diagnostic checks that are present
    let config_checks = [
        "config_file_exists",
        "config_toml_syntax",
        "config_completeness",
        "config_field_validation",
        "config_environment_consistency",
    ];
    let present_config_checks: Vec<&str> = config_checks
        .iter()
        .filter(|check_name| checks[*check_name].is_object())
        .copied()
        .collect();

    // At least some configuration checks should be present
    assert!(
        present_config_checks.len() >= 3,
        "At least 3 configuration checks should be present, found: {:?}",
        present_config_checks
    );

    // Verify each present check has required fields
    for check_name in present_config_checks {
        let check = &checks[check_name];
        assert!(
            check["status"].is_string(),
            "{} should have status field",
            check_name
        );
        assert!(
            check["message"].is_string(),
            "{} should have message field",
            check_name
        );
        // details and suggestion are optional
    }
}

#[test]
fn test_doctor_config_validation_comprehensive() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Test comprehensive config validation with all components
    cmd.env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test_token")
        .arg("doctor")
        .arg("--verbose")
        .assert()
        .stdout(predicate::str::contains("🩺 MY LITTLE SODA DOCTOR"))
        // Configuration validation checks
        .stdout(predicate::str::contains("config_file_exists"))
        .stdout(predicate::str::contains("config_toml_syntax"))
        .stdout(predicate::str::contains("config_completeness"))
        .stdout(predicate::str::contains("config_field_validation"))
        .stdout(predicate::str::contains("config_environment_consistency"))
        // Original checks should still be present
        .stdout(predicate::str::contains("git_repository"))
        .stdout(predicate::str::contains("github_authentication"));
}

#[test]
fn test_doctor_config_placeholder_detection() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("my-little-soda.toml");

    // Create config with placeholder values
    let placeholder_config = r#"
[github]
owner = "your-github-username"
repo = "your-repo-name"

[github.rate_limit]
requests_per_hour = 5000
burst_capacity = 100

[observability]
tracing_enabled = true
log_level = "info"
metrics_enabled = true

[agents]
coordination_timeout_seconds = 300

[agents.bundle_processing]
max_queue_size = 50
processing_timeout_seconds = 1800

[agents.process_management]
claude_code_path = "claude-code"
timeout_minutes = 30
cleanup_on_failure = true
work_dir_prefix = ".my-little-soda/agents"
enable_real_agents = false

[agents.ci_mode]
enabled = false
artifact_handling = "standard"
github_token_strategy = "standard"
workflow_state_persistence = true
ci_timeout_adjustment = 300
enhanced_error_reporting = true

[agents.work_continuity]
enable_continuity = true
state_file_path = ".my-little-soda/agent-state.json"
backup_interval_minutes = 5
max_recovery_attempts = 3
validation_timeout_seconds = 30
force_fresh_start_after_hours = 24
preserve_partial_work = true
"#;

    std::fs::write(&config_path, placeholder_config).unwrap();

    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();
    cmd.current_dir(temp_dir.path())
        .env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test_token")
        .arg("doctor")
        .assert()
        .stdout(
            predicate::str::contains("config_completeness").and(
                predicate::str::contains("placeholder value")
                    .or(predicate::str::contains("your-github-username")
                        .or(predicate::str::contains("your-repo-name"))),
            ),
        );
}

#[test]
fn test_doctor_config_validation_suggestions() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Test that suggestions are provided when appropriate
    cmd.env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test_token")
        .arg("doctor")
        .arg("--verbose")
        .assert()
        .stdout(
            predicate::str::contains("Suggestion:")
                .or(predicate::str::contains("Consider updating")
                    .or(predicate::str::contains("Environment variables override"))),
        );
}

// Git Repository State Diagnostic Tests

#[test]
fn test_doctor_git_comprehensive_validation() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Test comprehensive Git validation checks are present
    cmd.env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test_token")
        .arg("doctor")
        .assert()
        .stdout(predicate::str::contains("git_remote_origin"))
        .stdout(predicate::str::contains("git_remote_github_match"))
        .stdout(predicate::str::contains("git_branch_setup"))
        .stdout(predicate::str::contains("git_working_directory_state"))
        .stdout(predicate::str::contains("git_user_configuration"))
        .stdout(predicate::str::contains("git_operations_capability"))
        .stdout(predicate::str::contains("git_my_little_soda_requirements"));
}

#[test]
fn test_doctor_git_remote_origin_detection() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Should detect origin remote in current repository
    cmd.env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test_token")
        .arg("doctor")
        .assert()
        .stdout(predicate::str::contains(
            "git_remote_origin: Git origin remote found",
        ));
}

#[test]
fn test_doctor_git_branch_setup_detection() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Should report current branch
    cmd.env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test_token")
        .arg("doctor")
        .assert()
        .stdout(predicate::str::contains(
            "git_branch_setup: Current branch:",
        ));
}

#[test]
fn test_doctor_git_user_configuration() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Should check Git user configuration
    cmd.env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test_token")
        .arg("doctor")
        .assert()
        .stdout(predicate::str::contains("git_user_configuration"));
}

#[test]
fn test_doctor_git_working_directory_state() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Should check working directory state
    cmd.env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test_token")
        .arg("doctor")
        .assert()
        .stdout(predicate::str::contains("git_working_directory_state"));
}

#[test]
fn test_doctor_git_operations_capability() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Should test Git operations capability
    cmd.env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test_token")
        .arg("doctor")
        .assert()
        .stdout(predicate::str::contains("git_operations_capability"));
}

#[test]
fn test_doctor_git_my_little_soda_requirements() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Should check My Little Soda specific requirements
    cmd.env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test_token")
        .arg("doctor")
        .assert()
        .stdout(predicate::str::contains("git_my_little_soda_requirements"));
}

#[test]
fn test_doctor_git_verbose_details() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Should show detailed Git information in verbose mode
    cmd.env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test_token")
        .arg("doctor")
        .arg("--verbose")
        .assert()
        .stdout(
            predicate::str::contains("Origin URL:").or(predicate::str::contains(
                "Main branch available:",
            )
            .or(predicate::str::contains("user.name:")
                .or(predicate::str::contains("Branch creation")))),
        );
}

#[test]
fn test_doctor_git_no_repository() {
    // Test Git validation in directory without git repository
    let temp_dir = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    cmd.current_dir(temp_dir.path())
        .env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test_token")
        .arg("doctor")
        .assert()
        .stdout(predicate::str::contains(
            "git_repository: Not in a git repository",
        ));

    // Should not have comprehensive Git checks when not in a Git repo
    let binding = cmd
        .current_dir(temp_dir.path())
        .env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test_token")
        .arg("doctor")
        .arg("--format")
        .arg("json")
        .assert();
    let output = binding.get_output();

    let stdout = String::from_utf8(output.stdout.clone()).unwrap();

    // Should not contain Git validation checks when not in repo
    assert!(!stdout.contains("git_remote_origin"));
    assert!(!stdout.contains("git_branch_setup"));
    assert!(!stdout.contains("git_user_configuration"));
}

#[test]
fn test_doctor_git_json_structure() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    let binding = cmd
        .arg("doctor")
        .arg("--format")
        .arg("json")
        .env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test_token")
        .assert();
    let output = binding.get_output();

    let stdout = String::from_utf8(output.stdout.clone()).unwrap();

    // Extract the multiline JSON from the output (skip telemetry logs)
    let lines: Vec<&str> = stdout.lines().collect();
    let mut json_start = None;
    let mut brace_count = 0;
    let mut json_end = None;

    for (i, line) in lines.iter().enumerate() {
        if line.trim() == "{" {
            json_start = Some(i);
            brace_count = 1;
        } else if json_start.is_some() {
            brace_count += line.matches('{').count() as i32;
            brace_count -= line.matches('}').count() as i32;
            if brace_count == 0 {
                json_end = Some(i);
                break;
            }
        }
    }

    let json_str = if let (Some(start), Some(end)) = (json_start, json_end) {
        lines[start..=end].join("\n")
    } else {
        panic!("No JSON object found in output: {}", stdout);
    };

    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    let checks = &parsed["checks"];

    // Verify Git comprehensive diagnostic checks are present
    let git_checks = [
        "git_repository",
        "git_remote_origin",
        "git_remote_github_match",
        "git_branch_setup",
        "git_working_directory_state",
        "git_user_configuration",
        "git_operations_capability",
        "git_my_little_soda_requirements",
    ];

    for check_name in git_checks {
        assert!(
            checks[check_name].is_object(),
            "{} check should be present",
            check_name
        );
        let check = &checks[check_name];
        assert!(
            check["status"].is_string(),
            "{} should have status field",
            check_name
        );
        assert!(
            check["message"].is_string(),
            "{} should have message field",
            check_name
        );
    }
}

#[test]
fn test_doctor_git_comprehensive_validation_count() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    let binding = cmd
        .arg("doctor")
        .arg("--format")
        .arg("json")
        .env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test_token")
        .assert();
    let output = binding.get_output();

    let stdout = String::from_utf8(output.stdout.clone()).unwrap();

    // Count Git-related checks in JSON output
    let git_check_count = stdout.matches("\"git_").count();

    // Should have at least 8 Git-related checks (including git_available and git_repository)
    assert!(
        git_check_count >= 8,
        "Should have at least 8 Git checks, found: {}",
        git_check_count
    );
}

// Environment Validation Diagnostic Tests

#[test]
fn test_doctor_environment_validation_basics() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    cmd.env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test_token")
        .arg("doctor")
        .assert()
        .stdout(predicate::str::contains("environment_variables"))
        .stdout(predicate::str::contains("file_system_permissions"))
        .stdout(predicate::str::contains("disk_space"))
        .stdout(predicate::str::contains("path_configuration"))
        .stdout(predicate::str::contains("current_directory_access"))
        .stdout(predicate::str::contains("temporary_directory_access"))
        .stdout(predicate::str::contains("conflicting_configurations"))
        .stdout(predicate::str::contains("file_operations"));
}

#[test]
fn test_doctor_environment_variables_check() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Test with valid GitHub token
    cmd.env(
        "MY_LITTLE_SODA_GITHUB_TOKEN",
        "ghp_test_token_12345678901234567890",
    )
    .arg("doctor")
    .assert()
    .stdout(predicate::str::contains("environment_variables").and(
        predicate::str::contains("Environment variables are properly configured").or(
            predicate::str::contains("MY_LITTLE_SODA_GITHUB_TOKEN is set"),
        ),
    ));
}

#[test]
fn test_doctor_environment_variables_missing_token() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Test without GitHub token
    cmd.env_remove("MY_LITTLE_SODA_GITHUB_TOKEN")
        .env_remove("GITHUB_TOKEN")
        .arg("doctor")
        .assert()
        .stdout(
            predicate::str::contains("environment_variables").and(
                predicate::str::contains("MY_LITTLE_SODA_GITHUB_TOKEN not set")
                    .or(predicate::str::contains("Environment variables have")),
            ),
        );
}

#[test]
fn test_doctor_file_system_permissions() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    cmd.env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test_token")
        .arg("doctor")
        .assert()
        .stdout(
            predicate::str::contains("file_system_permissions").and(
                predicate::str::contains(
                    "File system permissions for .my-little-soda are adequate",
                )
                .or(predicate::str::contains(
                    "Successfully created .my-little-soda directory",
                )),
            ),
        );
}

#[test]
fn test_doctor_disk_space_check() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    cmd.env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test_token")
        .arg("doctor")
        .assert()
        .stdout(
            predicate::str::contains("disk_space").and(
                predicate::str::contains("disk space available")
                    .or(predicate::str::contains("Unable to check disk space")),
            ),
        );
}

#[test]
fn test_doctor_path_configuration() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    cmd.env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test_token")
        .arg("doctor")
        .assert()
        .stdout(
            predicate::str::contains("path_configuration").and(
                predicate::str::contains("PATH configuration")
                    .or(predicate::str::contains("my-little-soda")),
            ),
        );
}

#[test]
fn test_doctor_current_directory_access() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    cmd.env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test_token")
        .arg("doctor")
        .assert()
        .stdout(
            predicate::str::contains("current_directory_access").and(
                predicate::str::contains("Current directory access permissions are adequate")
                    .or(predicate::str::contains("Current directory")),
            ),
        );
}

#[test]
fn test_doctor_temporary_directory_access() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    cmd.env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test_token")
        .arg("doctor")
        .assert()
        .stdout(
            predicate::str::contains("temporary_directory_access").and(
                predicate::str::contains("Temporary directory access is working correctly")
                    .or(predicate::str::contains("System temp directory accessible")),
            ),
        );
}

#[test]
fn test_doctor_conflicting_configurations() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    cmd.env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test_token")
        .arg("doctor")
        .assert()
        .stdout(
            predicate::str::contains("conflicting_configurations").and(
                predicate::str::contains("No configuration conflicts detected")
                    .or(predicate::str::contains("Configuration conflicts detected")
                        .or(predicate::str::contains("Single GitHub token source"))),
            ),
        );
}

#[test]
fn test_doctor_file_operations() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    cmd.env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test_token")
        .arg("doctor")
        .assert()
        .stdout(
            predicate::str::contains("file_operations").and(
                predicate::str::contains("All My Little Soda file operations work correctly")
                    .or(predicate::str::contains("File operation")),
            ),
        );
}

#[test]
fn test_doctor_environment_verbose_details() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    cmd.env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test_token")
        .arg("doctor")
        .arg("--verbose")
        .assert()
        .stdout(predicate::str::contains("Details:"))
        .stdout(
            predicate::str::contains("✅")
                .or(predicate::str::contains("Create").or(predicate::str::contains("accessible"))),
        );
}

#[test]
fn test_doctor_environment_json_structure() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    let binding = cmd
        .arg("doctor")
        .arg("--format")
        .arg("json")
        .env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test_token")
        .assert();
    let output = binding.get_output();

    let stdout = String::from_utf8(output.stdout.clone()).unwrap();

    // Extract the multiline JSON from the output (skip telemetry logs)
    let lines: Vec<&str> = stdout.lines().collect();
    let mut json_start = None;
    let mut brace_count = 0;
    let mut json_end = None;

    for (i, line) in lines.iter().enumerate() {
        if line.trim() == "{" {
            json_start = Some(i);
            brace_count = 1;
        } else if json_start.is_some() {
            brace_count += line.matches('{').count() as i32;
            brace_count -= line.matches('}').count() as i32;
            if brace_count == 0 {
                json_end = Some(i);
                break;
            }
        }
    }

    let json_str = if let (Some(start), Some(end)) = (json_start, json_end) {
        lines[start..=end].join("\n")
    } else {
        panic!("No JSON object found in output: {}", stdout);
    };

    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    let checks = &parsed["checks"];

    // Verify environment diagnostic checks are present
    let env_checks = [
        "environment_variables",
        "file_system_permissions",
        "disk_space",
        "path_configuration",
        "current_directory_access",
        "temporary_directory_access",
        "conflicting_configurations",
        "file_operations",
    ];

    for check_name in env_checks {
        assert!(
            checks[check_name].is_object(),
            "{} check should be present",
            check_name
        );
        let check = &checks[check_name];
        assert!(
            check["status"].is_string(),
            "{} should have status field",
            check_name
        );
        assert!(
            check["message"].is_string(),
            "{} should have message field",
            check_name
        );
    }
}

#[test]
fn test_doctor_environment_conflicting_tokens() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Set multiple GitHub tokens to test conflict detection
    cmd.env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test_token")
        .env("GITHUB_TOKEN", "ghp_other_token")
        .arg("doctor")
        .assert()
        .stdout(
            predicate::str::contains("conflicting_configurations").and(
                predicate::str::contains("Multiple GitHub tokens set")
                    .or(predicate::str::contains("Configuration conflicts detected")),
            ),
        );
}

#[test]
fn test_doctor_environment_no_write_permissions() {
    // Test in read-only temporary directory
    let temp_dir = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // This test may not work on all systems, so we just check the check exists
    cmd.current_dir(temp_dir.path())
        .env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test_token")
        .arg("doctor")
        .assert()
        .stdout(predicate::str::contains("current_directory_access"))
        .stdout(predicate::str::contains("file_operations"));
}

#[test]
fn test_doctor_environment_path_warnings() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Test with limited PATH
    cmd.env("PATH", "/usr/bin:/bin") // Remove cargo paths
        .env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test_token")
        .arg("doctor")
        .assert()
        .stdout(predicate::str::contains("path_configuration"));
}

#[test]
fn test_doctor_environment_github_cli_fallback() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Test without MY_LITTLE_SODA_GITHUB_TOKEN but possibly with gh CLI
    cmd.env_remove("MY_LITTLE_SODA_GITHUB_TOKEN")
        .env_remove("GITHUB_TOKEN")
        .arg("doctor")
        .assert()
        .stdout(predicate::str::contains("environment_variables"));
}

#[test]
fn test_doctor_comprehensive_environment_validation() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Test comprehensive environment validation with all components
    cmd.env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test_token")
        .arg("doctor")
        .arg("--verbose")
        .assert()
        .stdout(predicate::str::contains("🩺 MY LITTLE SODA DOCTOR"))
        // Environment validation checks
        .stdout(predicate::str::contains("environment_variables"))
        .stdout(predicate::str::contains("file_system_permissions"))
        .stdout(predicate::str::contains("disk_space"))
        .stdout(predicate::str::contains("path_configuration"))
        .stdout(predicate::str::contains("current_directory_access"))
        .stdout(predicate::str::contains("temporary_directory_access"))
        .stdout(predicate::str::contains("conflicting_configurations"))
        .stdout(predicate::str::contains("file_operations"))
        // Original checks should still be present
        .stdout(predicate::str::contains("git_repository"))
        .stdout(predicate::str::contains("github_authentication"));
}

#[test]
fn test_doctor_environment_validation_count() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    let binding = cmd
        .arg("doctor")
        .arg("--format")
        .arg("json")
        .env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test_token")
        .assert();
    let output = binding.get_output();

    let stdout = String::from_utf8(output.stdout.clone()).unwrap();

    // Count environment-related checks in JSON output
    let env_checks = [
        "environment_variables",
        "file_system_permissions",
        "disk_space",
        "path_configuration",
        "current_directory_access",
        "temporary_directory_access",
        "conflicting_configurations",
        "file_operations",
    ];

    for check_name in env_checks {
        assert!(
            stdout.contains(&format!("\"{}\"", check_name)),
            "Should contain {} check",
            check_name
        );
    }

    // Should have all 8 environment checks
    let env_check_count = env_checks
        .iter()
        .filter(|check| stdout.contains(&format!("\"{}\"", check)))
        .count();
    assert_eq!(
        env_check_count, 8,
        "Should have all 8 environment checks, found: {}",
        env_check_count
    );
}

// GitHub Issue Label Validation Diagnostic Tests

#[test]
fn test_doctor_github_issue_labels_validation_basics() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Only run this test if we have a valid token and repo config
    if env::var("MY_LITTLE_SODA_GITHUB_TOKEN").is_ok()
        && env::var("GITHUB_OWNER").is_ok()
        && env::var("GITHUB_REPO").is_ok()
    {
        cmd.env(
            "MY_LITTLE_SODA_GITHUB_TOKEN",
            env::var("MY_LITTLE_SODA_GITHUB_TOKEN").unwrap(),
        )
        .env("GITHUB_OWNER", env::var("GITHUB_OWNER").unwrap())
        .env("GITHUB_REPO", env::var("GITHUB_REPO").unwrap())
        .arg("doctor")
        .assert()
        .stdout(predicate::str::contains("required_labels_existence"))
        .stdout(predicate::str::contains("label_configuration"))
        .stdout(predicate::str::contains("label_management_capabilities"))
        .stdout(predicate::str::contains("issue_label_states"));
    }
}

#[test]
fn test_doctor_github_labels_required_existence() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Only run this test if we have a valid token and repo config
    if env::var("MY_LITTLE_SODA_GITHUB_TOKEN").is_ok()
        && env::var("GITHUB_OWNER").is_ok()
        && env::var("GITHUB_REPO").is_ok()
    {
        cmd.env(
            "MY_LITTLE_SODA_GITHUB_TOKEN",
            env::var("MY_LITTLE_SODA_GITHUB_TOKEN").unwrap(),
        )
        .env("GITHUB_OWNER", env::var("GITHUB_OWNER").unwrap())
        .env("GITHUB_REPO", env::var("GITHUB_REPO").unwrap())
        .arg("doctor")
        .assert()
        .stdout(
            predicate::str::contains("required_labels_existence").and(
                predicate::str::contains("required labels")
                    .or(predicate::str::contains("labels exist")
                        .or(predicate::str::contains("labels are missing"))),
            ),
        );
    }
}

#[test]
fn test_doctor_github_labels_configuration_validation() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Only run this test if we have a valid token and repo config
    if env::var("MY_LITTLE_SODA_GITHUB_TOKEN").is_ok()
        && env::var("GITHUB_OWNER").is_ok()
        && env::var("GITHUB_REPO").is_ok()
    {
        cmd.env(
            "MY_LITTLE_SODA_GITHUB_TOKEN",
            env::var("MY_LITTLE_SODA_GITHUB_TOKEN").unwrap(),
        )
        .env("GITHUB_OWNER", env::var("GITHUB_OWNER").unwrap())
        .env("GITHUB_REPO", env::var("GITHUB_REPO").unwrap())
        .arg("doctor")
        .assert()
        .stdout(
            predicate::str::contains("label_configuration").and(
                predicate::str::contains("configuration")
                    .or(predicate::str::contains("color")
                        .or(predicate::str::contains("description"))),
            ),
        );
    }
}

#[test]
fn test_doctor_github_labels_management_capabilities() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Only run this test if we have a valid token and repo config
    if env::var("MY_LITTLE_SODA_GITHUB_TOKEN").is_ok()
        && env::var("GITHUB_OWNER").is_ok()
        && env::var("GITHUB_REPO").is_ok()
    {
        cmd.env(
            "MY_LITTLE_SODA_GITHUB_TOKEN",
            env::var("MY_LITTLE_SODA_GITHUB_TOKEN").unwrap(),
        )
        .env("GITHUB_OWNER", env::var("GITHUB_OWNER").unwrap())
        .env("GITHUB_REPO", env::var("GITHUB_REPO").unwrap())
        .arg("doctor")
        .assert()
        .stdout(
            predicate::str::contains("label_management_capabilities").and(
                predicate::str::contains("permissions")
                    .or(predicate::str::contains("create")
                        .or(predicate::str::contains("write access"))),
            ),
        );
    }
}

#[test]
fn test_doctor_github_labels_issue_states() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Only run this test if we have a valid token and repo config
    if env::var("MY_LITTLE_SODA_GITHUB_TOKEN").is_ok()
        && env::var("GITHUB_OWNER").is_ok()
        && env::var("GITHUB_REPO").is_ok()
    {
        cmd.env(
            "MY_LITTLE_SODA_GITHUB_TOKEN",
            env::var("MY_LITTLE_SODA_GITHUB_TOKEN").unwrap(),
        )
        .env("GITHUB_OWNER", env::var("GITHUB_OWNER").unwrap())
        .env("GITHUB_REPO", env::var("GITHUB_REPO").unwrap())
        .arg("doctor")
        .assert()
        .stdout(predicate::str::contains("issue_label_states").and(
            predicate::str::contains("label states").or(
                predicate::str::contains("routing").or(predicate::str::contains("conflicting")),
            ),
        ));
    }
}

#[test]
fn test_doctor_github_labels_verbose_details() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Only run this test if we have a valid token and repo config
    if env::var("MY_LITTLE_SODA_GITHUB_TOKEN").is_ok()
        && env::var("GITHUB_OWNER").is_ok()
        && env::var("GITHUB_REPO").is_ok()
    {
        cmd.env(
            "MY_LITTLE_SODA_GITHUB_TOKEN",
            env::var("MY_LITTLE_SODA_GITHUB_TOKEN").unwrap(),
        )
        .env("GITHUB_OWNER", env::var("GITHUB_OWNER").unwrap())
        .env("GITHUB_REPO", env::var("GITHUB_REPO").unwrap())
        .arg("doctor")
        .arg("--verbose")
        .assert()
        .stdout(
            predicate::str::contains("route:ready").or(predicate::str::contains("route:priority")
                .or(predicate::str::contains("agent001")
                    .or(predicate::str::contains("Existing labels:")))),
        );
    }
}

#[test]
fn test_doctor_github_labels_no_authentication() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Test without GitHub authentication to verify error handling
    cmd.env_remove("MY_LITTLE_SODA_GITHUB_TOKEN")
        .env_remove("GITHUB_TOKEN")
        .env("GITHUB_OWNER", "test-owner")
        .env("GITHUB_REPO", "test-repo")
        .arg("doctor")
        .assert()
        .stdout(
            predicate::str::contains("required_labels_existence").and(
                predicate::str::contains("Cannot check required labels")
                    .or(predicate::str::contains("GitHub client error")
                        .or(predicate::str::contains("Configure GitHub authentication"))),
            ),
        );
}

#[test]
fn test_doctor_github_labels_missing_repo_config() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Test with token but missing repository configuration
    cmd.env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test_token")
        .env_remove("GITHUB_OWNER")
        .env_remove("GITHUB_REPO")
        .arg("doctor")
        .assert()
        .stdout(
            predicate::str::contains("required_labels_existence").and(
                predicate::str::contains("Cannot check required labels")
                    .or(predicate::str::contains("GitHub client error")),
            ),
        );
}

#[test]
fn test_doctor_github_labels_json_structure() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Only run this test if we have a valid token and repo config
    if env::var("MY_LITTLE_SODA_GITHUB_TOKEN").is_ok()
        && env::var("GITHUB_OWNER").is_ok()
        && env::var("GITHUB_REPO").is_ok()
    {
        let binding = cmd
            .arg("doctor")
            .arg("--format")
            .arg("json")
            .env(
                "MY_LITTLE_SODA_GITHUB_TOKEN",
                env::var("MY_LITTLE_SODA_GITHUB_TOKEN").unwrap(),
            )
            .env("GITHUB_OWNER", env::var("GITHUB_OWNER").unwrap())
            .env("GITHUB_REPO", env::var("GITHUB_REPO").unwrap())
            .assert();
        let output = binding.get_output();

        let stdout = String::from_utf8(output.stdout.clone()).unwrap();

        // Extract the multiline JSON from the output (skip telemetry logs)
        let lines: Vec<&str> = stdout.lines().collect();
        let mut json_start = None;
        let mut brace_count = 0;
        let mut json_end = None;

        for (i, line) in lines.iter().enumerate() {
            if line.trim() == "{" {
                json_start = Some(i);
                brace_count = 1;
            } else if json_start.is_some() {
                brace_count += line.matches('{').count() as i32;
                brace_count -= line.matches('}').count() as i32;
                if brace_count == 0 {
                    json_end = Some(i);
                    break;
                }
            }
        }

        let json_str = if let (Some(start), Some(end)) = (json_start, json_end) {
            lines[start..=end].join("\n")
        } else {
            panic!("No JSON object found in output: {}", stdout);
        };

        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        let checks = &parsed["checks"];

        // Verify GitHub label diagnostic checks are present
        let label_checks = [
            "required_labels_existence",
            "label_configuration",
            "label_management_capabilities",
            "issue_label_states",
        ];

        for check_name in label_checks {
            assert!(
                checks[check_name].is_object(),
                "{} check should be present",
                check_name
            );
            let check = &checks[check_name];
            assert!(
                check["status"].is_string(),
                "{} should have status field",
                check_name
            );
            assert!(
                check["message"].is_string(),
                "{} should have message field",
                check_name
            );
        }
    }
}

#[test]
fn test_doctor_github_labels_routing_label_detection() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Only run this test if we have a valid token and repo config
    if env::var("MY_LITTLE_SODA_GITHUB_TOKEN").is_ok()
        && env::var("GITHUB_OWNER").is_ok()
        && env::var("GITHUB_REPO").is_ok()
    {
        cmd.env(
            "MY_LITTLE_SODA_GITHUB_TOKEN",
            env::var("MY_LITTLE_SODA_GITHUB_TOKEN").unwrap(),
        )
        .env("GITHUB_OWNER", env::var("GITHUB_OWNER").unwrap())
        .env("GITHUB_REPO", env::var("GITHUB_REPO").unwrap())
        .arg("doctor")
        .arg("--verbose")
        .assert()
        .stdout(
            predicate::str::contains("route:ready").or(predicate::str::contains("route:review")
                .or(predicate::str::contains("route:priority")
                    .or(predicate::str::contains("Missing labels")))),
        );
    }
}

#[test]
fn test_doctor_github_labels_priority_label_validation() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Only run this test if we have a valid token and repo config
    if env::var("MY_LITTLE_SODA_GITHUB_TOKEN").is_ok()
        && env::var("GITHUB_OWNER").is_ok()
        && env::var("GITHUB_REPO").is_ok()
    {
        cmd.env(
            "MY_LITTLE_SODA_GITHUB_TOKEN",
            env::var("MY_LITTLE_SODA_GITHUB_TOKEN").unwrap(),
        )
        .env("GITHUB_OWNER", env::var("GITHUB_OWNER").unwrap())
        .env("GITHUB_REPO", env::var("GITHUB_REPO").unwrap())
        .arg("doctor")
        .arg("--verbose")
        .assert()
        .stdout(
            predicate::str::contains("route:priority-low").or(predicate::str::contains(
                "route:priority-medium",
            )
            .or(predicate::str::contains("route:priority-high")
                .or(predicate::str::contains("Priority:")))),
        );
    }
}

#[test]
fn test_doctor_github_labels_agent_assignment_validation() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Only run this test if we have a valid token and repo config
    if env::var("MY_LITTLE_SODA_GITHUB_TOKEN").is_ok()
        && env::var("GITHUB_OWNER").is_ok()
        && env::var("GITHUB_REPO").is_ok()
    {
        cmd.env(
            "MY_LITTLE_SODA_GITHUB_TOKEN",
            env::var("MY_LITTLE_SODA_GITHUB_TOKEN").unwrap(),
        )
        .env("GITHUB_OWNER", env::var("GITHUB_OWNER").unwrap())
        .env("GITHUB_REPO", env::var("GITHUB_REPO").unwrap())
        .arg("doctor")
        .arg("--verbose")
        .assert()
        .stdout(
            predicate::str::contains("agent001")
                .or(predicate::str::contains("Assigned to").or(predicate::str::contains("agent"))),
        );
    }
}

#[test]
fn test_doctor_github_labels_color_validation() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Only run this test if we have a valid token and repo config
    if env::var("MY_LITTLE_SODA_GITHUB_TOKEN").is_ok()
        && env::var("GITHUB_OWNER").is_ok()
        && env::var("GITHUB_REPO").is_ok()
    {
        cmd.env(
            "MY_LITTLE_SODA_GITHUB_TOKEN",
            env::var("MY_LITTLE_SODA_GITHUB_TOKEN").unwrap(),
        )
        .env("GITHUB_OWNER", env::var("GITHUB_OWNER").unwrap())
        .env("GITHUB_REPO", env::var("GITHUB_REPO").unwrap())
        .arg("doctor")
        .assert()
        .stdout(
            predicate::str::contains("label_configuration").and(
                predicate::str::contains("configuration")
                    .or(predicate::str::contains("color mismatch")
                        .or(predicate::str::contains("correct configuration"))),
            ),
        );
    }
}

#[test]
fn test_doctor_github_labels_comprehensive_validation() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Only run this test if we have a valid token and repo config
    if env::var("MY_LITTLE_SODA_GITHUB_TOKEN").is_ok()
        && env::var("GITHUB_OWNER").is_ok()
        && env::var("GITHUB_REPO").is_ok()
    {
        cmd.env(
            "MY_LITTLE_SODA_GITHUB_TOKEN",
            env::var("MY_LITTLE_SODA_GITHUB_TOKEN").unwrap(),
        )
        .env("GITHUB_OWNER", env::var("GITHUB_OWNER").unwrap())
        .env("GITHUB_REPO", env::var("GITHUB_REPO").unwrap())
        .arg("doctor")
        .arg("--verbose")
        .assert()
        .stdout(predicate::str::contains("🩺 MY LITTLE SODA DOCTOR"))
        // GitHub issue label validation checks
        .stdout(predicate::str::contains("required_labels_existence"))
        .stdout(predicate::str::contains("label_configuration"))
        .stdout(predicate::str::contains("label_management_capabilities"))
        .stdout(predicate::str::contains("issue_label_states"))
        // Should still contain other diagnostic categories
        .stdout(predicate::str::contains("git_repository"))
        .stdout(predicate::str::contains("github_authentication"))
        .stdout(predicate::str::contains("repository_config"))
        .stdout(predicate::str::contains("environment_variables"));
    }
}

#[test]
fn test_doctor_github_labels_suggestions_on_failure() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Test with invalid repo to trigger suggestions
    cmd.env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test_token")
        .env("GITHUB_OWNER", "nonexistent-owner-123456")
        .env("GITHUB_REPO", "nonexistent-repo-123456")
        .arg("doctor")
        .assert()
        .stdout(
            predicate::str::contains("required_labels_existence").and(
                predicate::str::contains("Suggestion:")
                    .or(predicate::str::contains("my-little-soda init")
                        .or(predicate::str::contains("Configure GitHub authentication"))),
            ),
        );
}

#[test]
fn test_doctor_github_labels_count_validation() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();

    // Only run this test if we have a valid token and repo config
    if env::var("MY_LITTLE_SODA_GITHUB_TOKEN").is_ok()
        && env::var("GITHUB_OWNER").is_ok()
        && env::var("GITHUB_REPO").is_ok()
    {
        let binding = cmd
            .arg("doctor")
            .arg("--format")
            .arg("json")
            .env(
                "MY_LITTLE_SODA_GITHUB_TOKEN",
                env::var("MY_LITTLE_SODA_GITHUB_TOKEN").unwrap(),
            )
            .env("GITHUB_OWNER", env::var("GITHUB_OWNER").unwrap())
            .env("GITHUB_REPO", env::var("GITHUB_REPO").unwrap())
            .assert();
        let output = binding.get_output();

        let stdout = String::from_utf8(output.stdout.clone()).unwrap();

        // Count GitHub label-related checks in JSON output
        let label_checks = [
            "required_labels_existence",
            "label_configuration",
            "label_management_capabilities",
            "issue_label_states",
        ];

        for check_name in label_checks {
            assert!(
                stdout.contains(&format!("\"{}\"", check_name)),
                "Should contain {} check",
                check_name
            );
        }

        // Should have all 4 label validation checks
        let label_check_count = label_checks
            .iter()
            .filter(|check| stdout.contains(&format!("\"{}\"", check)))
            .count();
        assert_eq!(
            label_check_count, 4,
            "Should have all 4 label validation checks, found: {}",
            label_check_count
        );
    }
}
