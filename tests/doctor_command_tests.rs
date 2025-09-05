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
    
    let binding = cmd.arg("doctor").arg("--format").arg("json")
        .assert()
        .success();
    let output = binding.get_output();
    
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
    
    cmd.env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test_token")
        .arg("doctor")
        .assert()
        .stdout(predicate::str::contains("github_token_presence").and(
            predicate::str::contains("GitHub token found")
        ));
}

#[test]
fn test_doctor_no_github_token() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();
    
    // Make sure the token is not set
    cmd.env_remove("MY_LITTLE_SODA_GITHUB_TOKEN")
        .arg("doctor")
        .assert()
        .stdout(predicate::str::contains("github_token_presence").and(
            predicate::str::contains("No GitHub token found")
        ));
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
    
    let binding = cmd.arg("doctor").arg("--format").arg("json")
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
    cmd.env("MY_LITTLE_SODA_GITHUB_TOKEN", env::var("MY_LITTLE_SODA_GITHUB_TOKEN").unwrap_or("test_token".to_string()))
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
        .stdout(predicate::str::contains("github_token_presence: GitHub token found and format is valid"));
}

#[test]
fn test_doctor_github_invalid_token_format() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();
    
    cmd.env("MY_LITTLE_SODA_GITHUB_TOKEN", "invalid_token_format")
        .env_remove("GITHUB_OWNER") // Remove config to avoid actual API calls
        .env_remove("GITHUB_REPO")
        .arg("doctor")
        .assert()
        .stdout(predicate::str::contains("GitHub token found but format is invalid").or(
            predicate::str::contains("github_token_presence").and(
                predicate::str::contains("Warning").or(predicate::str::contains("Fail"))
            )
        ));
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
    
    let binding = cmd.arg("doctor").arg("--format").arg("json")
        .env("MY_LITTLE_SODA_GITHUB_TOKEN", env::var("MY_LITTLE_SODA_GITHUB_TOKEN").unwrap_or("test_token".to_string()))
        .assert();
    let output = binding.get_output();
    
    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    
    let checks = &parsed["checks"];
    
    // Verify GitHub authentication diagnostics are present
    assert!(checks["github_token_presence"].is_object(), "github_token_presence check should be present");
    assert!(checks["github_authentication"].is_object(), "github_authentication check should be present");
    assert!(checks["github_repository_access"].is_object(), "github_repository_access check should be present");  
    assert!(checks["github_rate_limits"].is_object(), "github_rate_limits check should be present");
    assert!(checks["github_api_scopes"].is_object(), "github_api_scopes check should be present");
    
    // Verify each check has required fields
    for check_name in ["github_token_presence", "github_authentication", "github_repository_access", "github_rate_limits", "github_api_scopes"] {
        let check = &checks[check_name];
        assert!(check["status"].is_string(), "{} should have status field", check_name);
        assert!(check["message"].is_string(), "{} should have message field", check_name);
        // details and suggestion are optional
    }
}

#[test]
fn test_doctor_verbose_github_details() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();
    
    cmd.env("MY_LITTLE_SODA_GITHUB_TOKEN", env::var("MY_LITTLE_SODA_GITHUB_TOKEN").unwrap_or("test_token".to_string()))
        .arg("doctor").arg("--verbose")
        .assert()
        .stdout(predicate::str::contains("Details:"))
        .stdout(predicate::str::contains("Token source:").or(
            predicate::str::contains("issues:read").or(
                predicate::str::contains("Resets in")
            )
        ));
}

#[test]
fn test_doctor_github_token_format_validation() {
    // Test classic GitHub token format (ghp_)
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();
    cmd.env("MY_LITTLE_SODA_GITHUB_TOKEN", "ghp_test12345678901234567890123456789012345")
        .env_remove("GITHUB_OWNER") // Remove to avoid real API calls  
        .env_remove("GITHUB_REPO")
        .arg("doctor")
        .assert()
        .stdout(predicate::str::contains("GitHub token found and format is valid"));
        
    // Test fine-grained token format (github_pat_)
    let mut cmd2 = Command::cargo_bin("my-little-soda").unwrap();
    cmd2.env("MY_LITTLE_SODA_GITHUB_TOKEN", "github_pat_test12345_abcdef1234567890")
        .env_remove("GITHUB_OWNER")
        .env_remove("GITHUB_REPO")
        .arg("doctor")
        .assert()
        .stdout(predicate::str::contains("GitHub token found and format is valid"));
        
    // Test OAuth token format (gho_)  
    let mut cmd3 = Command::cargo_bin("my-little-soda").unwrap();
    cmd3.env("MY_LITTLE_SODA_GITHUB_TOKEN", "gho_test12345678901234567890")
        .env_remove("GITHUB_OWNER")
        .env_remove("GITHUB_REPO")
        .arg("doctor")
        .assert()
        .stdout(predicate::str::contains("GitHub token found and format is valid"));
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
        .stdout(predicate::str::contains("GitHub CLI").or(
            predicate::str::contains("Token source").or(
                predicate::str::contains("gh auth")
            )
        ));
}

#[test]
fn test_doctor_rate_limit_information_display() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();
    
    // Only run this test if we have a valid token to avoid API errors
    if env::var("MY_LITTLE_SODA_GITHUB_TOKEN").is_ok() {
        cmd.env("MY_LITTLE_SODA_GITHUB_TOKEN", env::var("MY_LITTLE_SODA_GITHUB_TOKEN").unwrap())
            .arg("doctor")
            .assert()
            .stdout(predicate::str::contains("github_rate_limits"))
            .stdout(predicate::str::contains("requests remaining").or(
                predicate::str::contains("Rate limit")
            ));
    }
}

#[test]
fn test_doctor_api_scope_testing() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();
    
    // Only run this test if we have a valid token to avoid API errors
    if env::var("MY_LITTLE_SODA_GITHUB_TOKEN").is_ok() {
        cmd.env("MY_LITTLE_SODA_GITHUB_TOKEN", env::var("MY_LITTLE_SODA_GITHUB_TOKEN").unwrap())
            .arg("doctor")
            .arg("--verbose")
            .assert()
            .stdout(predicate::str::contains("github_api_scopes"))
            .stdout(predicate::str::contains("issues:read").or(
                predicate::str::contains("pull_requests:read").or(
                    predicate::str::contains("repository:read")
                )
            ));
    }
}

#[test]
fn test_doctor_github_error_handling() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();
    
    // Test with clearly invalid token to trigger error handling
    cmd.env("MY_LITTLE_SODA_GITHUB_TOKEN", "clearly_invalid_token_format_123")
        .env_remove("GITHUB_OWNER") // Remove config to control error type
        .env_remove("GITHUB_REPO")
        .arg("doctor")
        .assert()
        .stdout(predicate::str::contains("github_authentication").or(
            predicate::str::contains("github_token_presence")
        ));
}

// Repository Access Diagnostic Tests

#[test]
fn test_doctor_repository_access_diagnostics() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();
    
    // Test with valid token to see repository diagnostics
    if env::var("MY_LITTLE_SODA_GITHUB_TOKEN").is_ok() {
        cmd.env("MY_LITTLE_SODA_GITHUB_TOKEN", env::var("MY_LITTLE_SODA_GITHUB_TOKEN").unwrap())
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
                predicate::str::contains("GitHub repository name not configured")
            )
        ));
}

#[test]
fn test_doctor_repository_operations_testing() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();
    
    // Only run this test if we have a valid token and repo config
    if env::var("MY_LITTLE_SODA_GITHUB_TOKEN").is_ok() &&
       env::var("GITHUB_OWNER").is_ok() &&
       env::var("GITHUB_REPO").is_ok() {
        cmd.env("MY_LITTLE_SODA_GITHUB_TOKEN", env::var("MY_LITTLE_SODA_GITHUB_TOKEN").unwrap())
            .env("GITHUB_OWNER", env::var("GITHUB_OWNER").unwrap())
            .env("GITHUB_REPO", env::var("GITHUB_REPO").unwrap())
            .arg("doctor")
            .arg("--verbose")
            .assert()
            .stdout(predicate::str::contains("repository_operations"))
            .stdout(predicate::str::contains("Issue listing:").or(
                predicate::str::contains("PR listing:").or(
                    predicate::str::contains("Label access:")
                )
            ));
    }
}

#[test]
fn test_doctor_repository_settings_validation() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();
    
    // Only run this test if we have a valid token and repo config
    if env::var("MY_LITTLE_SODA_GITHUB_TOKEN").is_ok() &&
       env::var("GITHUB_OWNER").is_ok() &&
       env::var("GITHUB_REPO").is_ok() {
        cmd.env("MY_LITTLE_SODA_GITHUB_TOKEN", env::var("MY_LITTLE_SODA_GITHUB_TOKEN").unwrap())
            .env("GITHUB_OWNER", env::var("GITHUB_OWNER").unwrap())
            .env("GITHUB_REPO", env::var("GITHUB_REPO").unwrap())
            .arg("doctor")
            .arg("--verbose")
            .assert()
            .stdout(predicate::str::contains("repository_settings"))
            .stdout(predicate::str::contains("Issues enabled").or(
                predicate::str::contains("Merge options:").or(
                    predicate::str::contains("Repository settings")
                )
            ));
    }
}

#[test]
fn test_doctor_json_repository_diagnostics_structure() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();
    
    let binding = cmd.arg("doctor").arg("--format").arg("json")
        .env("MY_LITTLE_SODA_GITHUB_TOKEN", env::var("MY_LITTLE_SODA_GITHUB_TOKEN").unwrap_or("test_token".to_string()))
        .env("GITHUB_OWNER", env::var("GITHUB_OWNER").unwrap_or("test-owner".to_string()))
        .env("GITHUB_REPO", env::var("GITHUB_REPO").unwrap_or("test-repo".to_string()))
        .assert();
    let output = binding.get_output();
    
    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    
    let checks = &parsed["checks"];
    
    // Verify repository diagnostic checks are present
    assert!(checks["repository_config_owner"].is_object(), "repository_config_owner check should be present");
    assert!(checks["repository_config_repo"].is_object(), "repository_config_repo check should be present");
    assert!(checks["repository_existence"].is_object(), "repository_existence check should be present");
    assert!(checks["repository_settings"].is_object(), "repository_settings check should be present");
    assert!(checks["repository_operations"].is_object(), "repository_operations check should be present");
    
    // Verify each check has required fields
    for check_name in ["repository_config_owner", "repository_config_repo", "repository_existence", "repository_settings", "repository_operations"] {
        let check = &checks[check_name];
        assert!(check["status"].is_string(), "{} should have status field", check_name);
        assert!(check["message"].is_string(), "{} should have message field", check_name);
        // details and suggestion are optional
    }
}

#[test]
fn test_doctor_comprehensive_repository_validation() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();
    
    // Test comprehensive repository validation with all components
    if env::var("MY_LITTLE_SODA_GITHUB_TOKEN").is_ok() {
        cmd.env("MY_LITTLE_SODA_GITHUB_TOKEN", env::var("MY_LITTLE_SODA_GITHUB_TOKEN").unwrap())
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
            predicate::str::contains("not found").or(
                predicate::str::contains("not accessible")
            )
        ));
}

#[test]
fn test_doctor_repository_validation_private_vs_public() {
    let mut cmd = Command::cargo_bin("my-little-soda").unwrap();
    
    // Only run this test if we have a valid token and repo config
    if env::var("MY_LITTLE_SODA_GITHUB_TOKEN").is_ok() &&
       env::var("GITHUB_OWNER").is_ok() &&
       env::var("GITHUB_REPO").is_ok() {
        cmd.env("MY_LITTLE_SODA_GITHUB_TOKEN", env::var("MY_LITTLE_SODA_GITHUB_TOKEN").unwrap())
            .env("GITHUB_OWNER", env::var("GITHUB_OWNER").unwrap())
            .env("GITHUB_REPO", env::var("GITHUB_REPO").unwrap())
            .arg("doctor")
            .arg("--verbose")
            .assert()
            .stdout(predicate::str::contains("repository_existence"))
            .stdout(predicate::str::contains("Visibility:").or(
                predicate::str::contains("private").or(
                    predicate::str::contains("public")
                )
            ));
    }
}