//! GitHub API failure handling mock tests
//!
//! These tests verify proper error handling and recovery when GitHub API calls fail,
//! preventing silent failures that could cause state management issues.

use clambake::github::GitHubError;
use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::time::{Duration, Instant};

mod fixtures;

// Mock failure scenarios for GitHub API testing
#[derive(Debug, Clone)]
pub enum ApiFailureType {
    NetworkTimeout,
    RateLimitExceeded,
    AuthenticationFailure,
    ResourceNotFound,
    InternalServerError,
    InvalidResponse,
}

#[derive(Debug, Clone)]
pub struct ApiFailureConfig {
    pub failure_type: ApiFailureType,
    pub failure_rate: f64, // 0.0 to 1.0
    pub retry_count: usize,
    pub delay_ms: u64,
}

// Mock GitHub client with configurable failure scenarios
#[derive(Debug, Clone)]
pub struct MockFailureGitHubClient {
    pub owner: String,
    pub repo: String,
    pub failure_configs: Arc<Mutex<HashMap<String, ApiFailureConfig>>>,
    pub call_counts: Arc<Mutex<HashMap<String, usize>>>,
    pub failure_history: Arc<Mutex<Vec<ApiFailureEvent>>>,
}

#[derive(Debug, Clone)]
pub struct ApiFailureEvent {
    pub operation: String,
    pub failure_type: ApiFailureType,
    pub retry_attempt: usize,
    pub timestamp: Instant,
    pub recovered: bool,
}

impl MockFailureGitHubClient {
    pub fn new(owner: &str, repo: &str) -> Self {
        Self {
            owner: owner.to_string(),
            repo: repo.to_string(),
            failure_configs: Arc::new(Mutex::new(HashMap::new())),
            call_counts: Arc::new(Mutex::new(HashMap::new())),
            failure_history: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    pub fn configure_failure(&self, operation: &str, config: ApiFailureConfig) {
        self.failure_configs.lock().unwrap().insert(operation.to_string(), config);
    }
    
    pub fn get_call_count(&self, operation: &str) -> usize {
        self.call_counts.lock().unwrap().get(operation).copied().unwrap_or(0)
    }
    
    pub fn get_failure_history(&self) -> Vec<ApiFailureEvent> {
        self.failure_history.lock().unwrap().clone()
    }
    
    pub fn clear_failure_history(&self) {
        self.failure_history.lock().unwrap().clear();
        self.call_counts.lock().unwrap().clear();
    }
    
    // Simulate API call with potential failures and retries
    async fn simulate_api_call(&self, operation: &str) -> Result<(), GitHubError> {
        // Increment call count
        {
            let mut counts = self.call_counts.lock().unwrap();
            *counts.entry(operation.to_string()).or_insert(0) += 1;
        }
        
        // Check if failure is configured for this operation
        let failure_config = {
            let configs = self.failure_configs.lock().unwrap();
            configs.get(operation).cloned()
        };
        
        if let Some(config) = failure_config {
            // Determine if this call should fail based on failure rate
            let should_fail = rand::random::<f64>() < config.failure_rate;
            
            if should_fail {
                // Record failure event
                let failure_event = ApiFailureEvent {
                    operation: operation.to_string(),
                    failure_type: config.failure_type.clone(),
                    retry_attempt: 0,
                    timestamp: Instant::now(),
                    recovered: false,
                };
                self.failure_history.lock().unwrap().push(failure_event);
                
                // Return appropriate error based on failure type
                return Err(self.create_error_for_failure_type(&config.failure_type));
            }
        }
        
        // Simulate API delay
        tokio::time::sleep(Duration::from_millis(10)).await;
        Ok(())
    }
    
    // Simulate API call with retry logic
    async fn simulate_api_call_with_retry(&self, operation: &str) -> Result<(), GitHubError> {
        let failure_config = {
            let configs = self.failure_configs.lock().unwrap();
            configs.get(operation).cloned()
        };
        
        let max_retries = failure_config.as_ref().map(|c| c.retry_count).unwrap_or(0);
        
        for attempt in 0..=max_retries {
            match self.simulate_api_call(operation).await {
                Ok(()) => {
                    // If we had previous failures for this operation, mark as recovered
                    if attempt > 0 {
                        let mut history = self.failure_history.lock().unwrap();
                        if let Some(last_failure) = history.iter_mut()
                            .filter(|e| e.operation == operation)
                            .last() {
                            last_failure.recovered = true;
                        }
                    }
                    return Ok(());
                }
                Err(e) => {
                    if attempt < max_retries {
                        // Record retry attempt
                        let retry_event = ApiFailureEvent {
                            operation: operation.to_string(),
                            failure_type: failure_config.as_ref().unwrap().failure_type.clone(),
                            retry_attempt: attempt + 1,
                            timestamp: Instant::now(),
                            recovered: false,
                        };
                        self.failure_history.lock().unwrap().push(retry_event);
                        
                        // Wait before retry
                        let delay = failure_config.as_ref().unwrap().delay_ms;
                        tokio::time::sleep(Duration::from_millis(delay)).await;
                    } else {
                        return Err(e);
                    }
                }
            }
        }
        
        unreachable!()
    }
    
    fn create_error_for_failure_type(&self, failure_type: &ApiFailureType) -> GitHubError {
        match failure_type {
            ApiFailureType::NetworkTimeout => {
                GitHubError::IoError(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "Network timeout"
                ))
            }
            ApiFailureType::RateLimitExceeded => {
                GitHubError::IoError(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Rate limit exceeded"
                ))
            }
            ApiFailureType::AuthenticationFailure => {
                GitHubError::IoError(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "Authentication failed"
                ))
            }
            ApiFailureType::ResourceNotFound => {
                GitHubError::IoError(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Resource not found"
                ))
            }
            ApiFailureType::InternalServerError => {
                GitHubError::IoError(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Internal server error"
                ))
            }
            ApiFailureType::InvalidResponse => {
                GitHubError::IoError(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Invalid response format"
                ))
            }
        }
    }
}

#[async_trait]
impl super::state_management_regression_tests::GitHubOps for MockFailureGitHubClient {
    async fn remove_label_from_issue(&self, issue_number: u64, label_name: &str) -> Result<(), GitHubError> {
        self.simulate_api_call_with_retry("remove_label").await
    }
    
    async fn add_label_to_issue(&self, issue_number: u64, label_name: &str) -> Result<(), GitHubError> {
        self.simulate_api_call_with_retry("add_label").await
    }
    
    async fn fetch_issues(&self) -> Result<Vec<octocrab::models::issues::Issue>, GitHubError> {
        self.simulate_api_call_with_retry("fetch_issues").await?;
        Ok(fixtures::load_test_issues())
    }
    
    fn owner(&self) -> &str {
        &self.owner
    }
    
    fn repo(&self) -> &str {
        &self.repo
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_network_timeout_handling() {
        // Given: A client configured to simulate network timeouts
        let client = MockFailureGitHubClient::new("johnhkchen", "clambake");
        client.configure_failure("remove_label", ApiFailureConfig {
            failure_type: ApiFailureType::NetworkTimeout,
            failure_rate: 1.0, // Always fail
            retry_count: 0,     // No retries
            delay_ms: 100,
        });
        
        // When: We attempt to remove a label
        let result = client.remove_label_from_issue(95, "agent001").await;
        
        // Then: The operation should fail with timeout error
        assert!(result.is_err(), "Operation should fail due to timeout");
        
        let error = result.unwrap_err();
        let error_msg = format!("{:?}", error);
        assert!(error_msg.contains("TimedOut") || error_msg.contains("timeout"), 
                "Error should indicate timeout: {}", error_msg);
        
        // And: Failure should be recorded
        let history = client.get_failure_history();
        assert_eq!(history.len(), 1);
        assert!(matches!(history[0].failure_type, ApiFailureType::NetworkTimeout));
        assert!(!history[0].recovered);
    }
    
    #[tokio::test]
    async fn test_rate_limit_handling_with_retry() {
        // Given: A client configured to simulate rate limiting with retry
        let client = MockFailureGitHubClient::new("johnhkchen", "clambake");
        client.configure_failure("add_label", ApiFailureConfig {
            failure_type: ApiFailureType::RateLimitExceeded,
            failure_rate: 0.8, // 80% failure rate
            retry_count: 3,     // Retry up to 3 times
            delay_ms: 50,       // 50ms delay between retries
        });
        
        // When: We attempt to add a label multiple times
        let mut successful_calls = 0;
        let mut failed_calls = 0;
        
        for _ in 0..10 {
            match client.add_label_to_issue(95, "route:land").await {
                Ok(()) => successful_calls += 1,
                Err(_) => failed_calls += 1,
            }
            client.clear_failure_history(); // Clear for next attempt
        }
        
        // Then: Some calls should succeed due to retry logic
        assert!(successful_calls > 0, "Some calls should succeed with retry logic");
        
        // Note: Due to randomness, we can't guarantee exact numbers, but we verify retry behavior
    }
    
    #[tokio::test]
    async fn test_authentication_failure_no_retry() {
        // Given: A client configured to simulate authentication failures
        let client = MockFailureGitHubClient::new("johnhkchen", "clambake");
        client.configure_failure("fetch_issues", ApiFailureConfig {
            failure_type: ApiFailureType::AuthenticationFailure,
            failure_rate: 1.0, // Always fail
            retry_count: 2,     // Retries won't help with auth failures
            delay_ms: 100,
        });
        
        // When: We attempt to fetch issues
        let result = client.fetch_issues().await;
        
        // Then: The operation should fail with authentication error
        assert!(result.is_err(), "Operation should fail due to authentication");
        
        let error = result.unwrap_err();
        let error_msg = format!("{:?}", error);
        assert!(error_msg.contains("PermissionDenied") || error_msg.contains("Authentication"), 
                "Error should indicate authentication failure: {}", error_msg);
        
        // And: Multiple failure attempts should be recorded (initial + retries)
        let history = client.get_failure_history();
        assert!(history.len() >= 3, "Should have multiple failure attempts due to retries");
        
        // And: None should be marked as recovered
        assert!(history.iter().all(|e| !e.recovered), "Auth failures should not recover");
    }
    
    #[tokio::test]
    async fn test_intermittent_failure_recovery() {
        // Given: A client with intermittent failures (50% failure rate)
        let client = MockFailureGitHubClient::new("johnhkchen", "clambake");
        client.configure_failure("remove_label", ApiFailureConfig {
            failure_type: ApiFailureType::InternalServerError,
            failure_rate: 0.5, // 50% failure rate
            retry_count: 5,     // More retries for recovery testing
            delay_ms: 10,       // Fast retries for testing
        });
        
        // When: We perform multiple operations
        let mut total_attempts = 0;
        let mut successful_operations = 0;
        
        for _ in 0..20 {
            total_attempts += 1;
            if client.remove_label_from_issue(95, "agent001").await.is_ok() {
                successful_operations += 1;
            }
        }
        
        // Then: Some operations should succeed due to intermittent nature and retries
        assert!(successful_operations > 0, "Some operations should succeed with retries");
        assert!(successful_operations < total_attempts, "Not all should succeed due to failures");
        
        // And: Some failures should show recovery
        let history = client.get_failure_history();
        let recovered_failures = history.iter().filter(|e| e.recovered).count();
        assert!(recovered_failures > 0, "Some failures should show recovery");
    }
    
    #[tokio::test]
    async fn test_resource_not_found_immediate_failure() {
        // Given: A client configured to simulate resource not found
        let client = MockFailureGitHubClient::new("johnhkchen", "clambake");
        client.configure_failure("add_label", ApiFailureConfig {
            failure_type: ApiFailureType::ResourceNotFound,
            failure_rate: 1.0, // Always fail
            retry_count: 0,     // No retries for 404s
            delay_ms: 0,
        });
        
        // When: We attempt to add a label to a non-existent issue
        let start_time = Instant::now();
        let result = client.add_label_to_issue(999999, "nonexistent").await;
        let elapsed = start_time.elapsed();
        
        // Then: The operation should fail quickly without retries
        assert!(result.is_err(), "Operation should fail for non-existent resource");
        assert!(elapsed < Duration::from_millis(100), "Should fail quickly without retries");
        
        let error = result.unwrap_err();
        let error_msg = format!("{:?}", error);
        assert!(error_msg.contains("NotFound") || error_msg.contains("not found"), 
                "Error should indicate resource not found: {}", error_msg);
    }
    
    #[tokio::test]
    async fn test_invalid_response_handling() {
        // Given: A client configured to simulate invalid responses
        let client = MockFailureGitHubClient::new("johnhkchen", "clambake");
        client.configure_failure("fetch_issues", ApiFailureConfig {
            failure_type: ApiFailureType::InvalidResponse,
            failure_rate: 1.0, // Always fail
            retry_count: 2,     // Limited retries for parsing errors
            delay_ms: 50,
        });
        
        // When: We attempt to fetch issues
        let result = client.fetch_issues().await;
        
        // Then: The operation should fail with invalid data error
        assert!(result.is_err(), "Operation should fail due to invalid response");
        
        let error = result.unwrap_err();
        let error_msg = format!("{:?}", error);
        assert!(error_msg.contains("InvalidData") || error_msg.contains("Invalid"), 
                "Error should indicate invalid response: {}", error_msg);
        
        // And: Retries should be attempted
        let history = client.get_failure_history();
        assert!(history.len() >= 2, "Should attempt retries for invalid response");
    }
    
    #[tokio::test]
    async fn test_failure_pattern_analysis() {
        // Given: A client with multiple configured failure types
        let client = MockFailureGitHubClient::new("johnhkchen", "clambake");
        
        // Configure different failures for different operations
        client.configure_failure("add_label", ApiFailureConfig {
            failure_type: ApiFailureType::RateLimitExceeded,
            failure_rate: 0.6,
            retry_count: 2,
            delay_ms: 25,
        });
        
        client.configure_failure("remove_label", ApiFailureConfig {
            failure_type: ApiFailureType::NetworkTimeout,
            failure_rate: 0.4,
            retry_count: 1,
            delay_ms: 50,
        });
        
        client.configure_failure("fetch_issues", ApiFailureConfig {
            failure_type: ApiFailureType::InternalServerError,
            failure_rate: 0.3,
            retry_count: 3,
            delay_ms: 30,
        });
        
        // When: We perform various operations
        for _ in 0..10 {
            let _ = client.add_label_to_issue(95, "test").await;
            let _ = client.remove_label_from_issue(95, "test").await;
            let _ = client.fetch_issues().await;
        }
        
        // Then: We should have a variety of failure types recorded
        let history = client.get_failure_history();
        
        let rate_limit_failures = history.iter()
            .filter(|e| matches!(e.failure_type, ApiFailureType::RateLimitExceeded))
            .count();
        let timeout_failures = history.iter()
            .filter(|e| matches!(e.failure_type, ApiFailureType::NetworkTimeout))
            .count();
        let server_error_failures = history.iter()
            .filter(|e| matches!(e.failure_type, ApiFailureType::InternalServerError))
            .count();
        
        // Verify we have diverse failure types (exact numbers depend on randomness)
        assert!(rate_limit_failures > 0 || timeout_failures > 0 || server_error_failures > 0,
                "Should have recorded various failure types");
        
        // And: Some operations should have retry attempts
        let retry_attempts = history.iter().filter(|e| e.retry_attempt > 0).count();
        assert!(retry_attempts > 0, "Should have retry attempts recorded");
    }
}