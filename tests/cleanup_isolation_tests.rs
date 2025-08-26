/// Integration tests for enhanced cleanup and isolation mechanisms
use std::sync::{Arc, Mutex};

mod fixtures;
use fixtures::test_harness::*;

#[test]
fn test_enhanced_isolation_verification() {
    let mut harness = TestHarness::new().unwrap();
    harness.verify_isolation().unwrap();
    assert!(harness.is_isolation_verified());
}

#[test]
fn test_cross_test_isolation() {
    let mut harness1 = TestHarness::new().unwrap();
    let mut harness2 = TestHarness::new().unwrap();
    let mut harness3 = TestHarness::new().unwrap();
    
    harness1.verify_isolation().unwrap();
    harness2.verify_isolation().unwrap();
    harness3.verify_isolation().unwrap();
    
    let others = vec![&harness2, &harness3];
    harness1.verify_cross_test_isolation(&others).unwrap();
}

#[test]
fn test_cleanup_strategies() {
    let mut harness = TestHarness::with_cleanup_strategy(CleanupStrategy::Immediate).unwrap();
    harness.create_file("test.txt", "content").unwrap();
    
    let cleanup_errors = harness.cleanup().unwrap();
    assert!(cleanup_errors.is_empty());
}

#[test]
fn test_graceful_cleanup_with_retry() {
    let mut harness = TestHarness::with_cleanup_strategy(
        CleanupStrategy::GracefulWithRetry { max_attempts: 3, delay_ms: 50 }
    ).unwrap();
    
    harness.create_file("test.txt", "content").unwrap();
    let cleanup_errors = harness.cleanup().unwrap();
    assert!(cleanup_errors.is_empty());
}

#[test]
fn test_cleanup_hooks() {
    let mut harness = TestHarness::new().unwrap();
    
    let hook_flag = Arc::new(Mutex::new(false));
    let hook_flag_clone = hook_flag.clone();
    
    harness.add_cleanup_hook(move || {
        *hook_flag_clone.lock().unwrap() = true;
        Ok(())
    });
    
    harness.cleanup().unwrap();
    assert!(*hook_flag.lock().unwrap());
}

#[test]
fn test_resource_leak_detection() {
    let mut harness = TestHarness::new().unwrap();
    harness.create_file("test.txt", "content").unwrap();
    
    let leaks = harness.detect_resource_leaks();
    assert!(leaks.is_empty(), "No leaks should be detected for temp directory files");
}

#[test]
fn test_cleanup_error_recovery() {
    let mut harness = TestHarness::new().unwrap();
    
    // Add a hook that will fail
    harness.add_cleanup_hook(|| {
        anyhow::bail!("Intentional cleanup failure for testing")
    });
    
    let cleanup_errors = harness.cleanup().unwrap();
    assert!(!cleanup_errors.is_empty());
    assert!(cleanup_errors[0].contains("Intentional cleanup failure"));
}

#[test]
fn test_isolation_under_error_conditions() {
    let mut harness = TestHarness::new().unwrap();
    
    // Create file and simulate partial failure
    harness.create_file("test.txt", "content").unwrap();
    
    // Should still maintain isolation
    harness.verify_isolation().unwrap();
    
    // Cleanup should work even after errors
    let cleanup_errors = harness.cleanup().unwrap();
    assert!(cleanup_errors.is_empty());
}

#[test] 
fn test_isolation_verification_prevents_file_interference() {
    let mut harness1 = TestHarness::new().unwrap();
    let mut harness2 = TestHarness::new().unwrap();
    
    // Verify basic isolation
    harness1.verify_isolation().unwrap();
    harness2.verify_isolation().unwrap();
    
    // Create files in both harnesses
    harness1.create_file("shared_name.txt", "content from harness 1").unwrap();
    harness2.create_file("shared_name.txt", "content from harness 2").unwrap();
    
    // Verify files exist and have correct content
    let content1 = std::fs::read_to_string(harness1.path().join("shared_name.txt")).unwrap();
    let content2 = std::fs::read_to_string(harness2.path().join("shared_name.txt")).unwrap();
    
    assert_eq!(content1, "content from harness 1");
    assert_eq!(content2, "content from harness 2");
    
    // Verify cross-test isolation
    let others = vec![&harness2];
    harness1.verify_cross_test_isolation(&others).unwrap();
}

#[test]
fn test_cleanup_strategy_configuration() {
    let mut harness = TestHarness::new().unwrap();
    
    // Default should be graceful with retry
    match harness.cleanup_strategy() {
        CleanupStrategy::GracefulWithRetry { max_attempts: 3, delay_ms: 100 } => {},
        _ => panic!("Default cleanup strategy should be GracefulWithRetry"),
    }
    
    // Should be able to change strategy
    harness.set_cleanup_strategy(CleanupStrategy::Immediate);
    match harness.cleanup_strategy() {
        CleanupStrategy::Immediate => {},
        _ => panic!("Cleanup strategy should have changed to Immediate"),
    }
}