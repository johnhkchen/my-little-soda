// Enhanced error handling and retry logic for GitHub API operations
// Following research recommendations for rate limiting and resilience

use std::time::Duration;
use tokio_retry::strategy::{ExponentialBackoff, jitter};
use tokio_retry::Retry;
use crate::github::GitHubError;
use tracing::{warn, debug, error};

#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(30),
            jitter: true,
        }
    }
}

#[derive(Debug)]
pub struct GitHubRetryHandler {
    config: RetryConfig,
}

impl GitHubRetryHandler {
    pub fn new(config: RetryConfig) -> Self {
        Self { config }
    }

    pub fn default() -> Self {
        Self::new(RetryConfig::default())
    }

    /// Execute a GitHub API operation with exponential backoff retry
    /// Note: Simplified version - full retry implementation would require more complex closure handling
    pub async fn execute_with_retry<F, R, E>(&self, operation: F) -> Result<R, GitHubError>
    where
        F: Fn() -> Result<R, E>,
        E: Into<GitHubError> + std::fmt::Debug,
    {
        let mut last_error = None;
        
        for attempt in 0..self.config.max_attempts {
            match operation() {
                Ok(result) => {
                    debug!("Operation succeeded on attempt {}", attempt + 1);
                    return Ok(result);
                }
                Err(error) => {
                    let github_error: GitHubError = error.into();
                    
                    if !self.should_retry(&github_error) {
                        error!("Operation failed (non-retryable): {:?}", github_error);
                        return Err(github_error);
                    }
                    
                    warn!("Operation failed (attempt {}/{}): {:?}", attempt + 1, self.config.max_attempts, github_error);
                    last_error = Some(github_error);
                    
                    if attempt < self.config.max_attempts - 1 {
                        let delay = std::cmp::min(
                            self.config.base_delay * (2_u32.pow(attempt)),
                            self.config.max_delay
                        );
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }
        
        Err(last_error.unwrap_or_else(|| GitHubError::NetworkError("Retry exhausted".to_string())))
    }

    /// Determine if an error is retryable based on GitHub API patterns
    fn should_retry(&self, error: &GitHubError) -> bool {
        match error {
            GitHubError::ApiError(octocrab_error) => {
                // Check for rate limiting, temporary failures, etc.
                // Note: This is simplified - in production we'd parse the specific error codes
                let error_string = format!("{:?}", octocrab_error);
                
                // Rate limiting (403 with rate limit headers)
                if error_string.contains("rate") || error_string.contains("limit") {
                    return true;
                }
                
                // Server errors (5xx)
                if error_string.contains("500") || error_string.contains("502") || 
                   error_string.contains("503") || error_string.contains("504") {
                    return true;
                }
                
                // Temporary network issues
                if error_string.contains("timeout") || error_string.contains("connection") {
                    return true;
                }
                
                false
            }
            GitHubError::IoError(_) => true, // Network issues are retryable
            GitHubError::TokenNotFound(_) => false, // Auth issues are not retryable
            GitHubError::ConfigNotFound(_) => false, // Config issues are not retryable
            GitHubError::NotImplemented(_) => false, // Feature missing, not retryable
            GitHubError::RateLimit { .. } => true, // Rate limits are definitely retryable
            GitHubError::Timeout { .. } => true, // Timeouts are retryable
            GitHubError::NetworkError(_) => true, // Network errors are retryable
        }
    }
}

/// Convenience macro for wrapping operations with retry logic
#[macro_export]
macro_rules! with_retry {
    ($retry_handler:expr, $operation:expr) => {
        $retry_handler.execute_with_retry(|| $operation).await
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_retry_success_after_failure() {
        let retry_handler = GitHubRetryHandler::default();
        let attempt_count = Arc::new(AtomicU32::new(0));
        let attempt_count_clone = attempt_count.clone();

        let result = retry_handler.execute_with_retry(move || {
            let count = attempt_count_clone.fetch_add(1, Ordering::SeqCst);
            if count < 2 {
                Err(GitHubError::IoError(std::io::Error::new(
                    std::io::ErrorKind::ConnectionRefused,
                    "test error"
                )))
            } else {
                Ok("success")
            }
        }).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
        assert_eq!(attempt_count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_non_retryable_error() {
        let retry_handler = GitHubRetryHandler::default();

        let result: Result<(), _> = retry_handler.execute_with_retry(|| {
            Err(GitHubError::TokenNotFound("test".to_string()))
        }).await;

        assert!(result.is_err());
        // Should fail immediately without retries
    }
}