use octocrab::{Octocrab, Error as OctocrabError};
use governor::{
    RateLimiter, 
    Quota, 
    DefaultDirectRateLimiter,
    Jitter
};
// Retry functionality will be integrated in future versions
// use reqwest_middleware::ClientBuilder;
// use reqwest_retry::{RetryTransientMiddleware, policies::ExponentialBackoff};
use moka::future::Cache;
use std::sync::Arc;
use std::time::Duration;
use std::num::NonZeroU32;
use tracing::{debug, info};
use serde::{Serialize, Deserialize};

/// Rate-limited HTTP client that wraps Octocrab with proper GitHub API rate limiting
#[derive(Debug)]
pub struct RateLimitedHttpClient {
    octocrab: Octocrab,
    rate_limiter: Arc<DefaultDirectRateLimiter>,
    cache: Cache<String, CacheEntry>,
    owner: String,
    repo: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    data: serde_json::Value,
    timestamp: u64,
}

impl RateLimitedHttpClient {
    /// Create a new rate-limited HTTP client
    pub fn new(token: String, owner: String, repo: String) -> Result<Self, OctocrabError> {
        // GitHub API allows 5000 requests per hour for authenticated users
        // That's approximately 83 requests per minute, or ~1.4 requests per second
        // We'll be conservative and limit to 1 request per second with bursts up to 10
        let quota = Quota::per_second(NonZeroU32::new(1).unwrap())
            .allow_burst(NonZeroU32::new(10).unwrap());
        let rate_limiter = Arc::new(RateLimiter::direct(quota));

        // Build octocrab with personal token
        // Note: reqwest-middleware/retry will be added in future octocrab integration
        let octocrab = Octocrab::builder()
            .personal_token(token)
            .build()?;

        // Create cache for responses (5 minute TTL, 1000 entry capacity)
        let cache = Cache::builder()
            .max_capacity(1000)
            .time_to_live(Duration::from_secs(300))
            .build();

        Ok(Self {
            octocrab,
            rate_limiter,
            cache,
            owner,
            repo,
        })
    }

    /// Execute a request with rate limiting and caching
    pub async fn execute_with_rate_limit<F, T>(&self, cache_key: Option<String>, request: F) -> Result<T, OctocrabError>
    where
        F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, OctocrabError>> + Send>>,
        T: Clone + Serialize + for<'de> Deserialize<'de>,
    {
        // Check cache first if cache_key is provided
        if let Some(ref key) = cache_key {
            if let Some(cached) = self.cache.get(key).await {
                debug!("Cache hit for key: {}", key);
                if let Ok(value) = serde_json::from_value(cached.data) {
                    return Ok(value);
                }
            }
        }

        // Wait for rate limit permission
        self.rate_limiter.until_ready_with_jitter(Jitter::up_to(Duration::from_millis(100))).await;

        debug!("Executing GitHub API request with rate limiting");

        // Execute the request
        let result = request().await?;

        // Cache the result if cache_key is provided
        if let Some(key) = cache_key {
            if let Ok(serialized) = serde_json::to_value(&result) {
                let entry = CacheEntry {
                    data: serialized,
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                };
                self.cache.insert(key, entry).await;
                debug!("Cached response for future requests");
            }
        }

        Ok(result)
    }

    /// Get the underlying octocrab instance for direct API calls
    pub fn octocrab(&self) -> &Octocrab {
        &self.octocrab
    }

    /// Get owner
    pub fn owner(&self) -> &str {
        &self.owner
    }

    /// Get repository name
    pub fn repo(&self) -> &str {
        &self.repo
    }

    /// Clear cache (useful for testing or after write operations)
    pub async fn clear_cache(&self) {
        self.cache.invalidate_all();
        info!("HTTP client cache cleared");
    }

    /// Invalidate specific cache entries (useful after write operations)  
    pub async fn invalidate_cache_pattern(&self, pattern: &str) {
        // Simple pattern matching - invalidate all keys containing the pattern
        let keys_to_remove: Vec<String> = self.cache.iter()
            .filter(|(key, _)| key.contains(pattern))
            .map(|(key, _)| key.as_ref().clone())
            .collect();

        for key in keys_to_remove {
            self.cache.invalidate(&key).await;
        }

        debug!("Invalidated cache entries matching pattern: {}", pattern);
    }

    /// Get current rate limiter status for monitoring
    pub fn rate_limiter_status(&self) -> String {
        // Return a simple status message since governor 0.6.3 doesn't expose detailed metrics
        "Rate limiter active".to_string()
    }
}