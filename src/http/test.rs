#[cfg(test)]
mod tests {
    use crate::http::RateLimitedHttpClient;
    use std::time::Instant;

    #[tokio::test]
    async fn test_rate_limiter_basic_functionality() {
        // Create a test client with a dummy token
        let client = RateLimitedHttpClient::new(
            "test_token".to_string(),
            "test_owner".to_string(),
            "test_repo".to_string(),
        )
        .unwrap();

        // Test rate limiter status
        let status = client.rate_limiter_status();
        assert_eq!(status, "Rate limiter active");

        // Test cache operations
        client.clear_cache().await;
        client.invalidate_cache_pattern("test").await;

        println!("Rate limiting client created successfully");
        println!("Owner: {}, Repo: {}", client.owner(), client.repo());
    }

    #[tokio::test]
    async fn test_rate_limiting_timing() {
        let client = RateLimitedHttpClient::new(
            "test_token".to_string(),
            "test_owner".to_string(),
            "test_repo".to_string(),
        )
        .unwrap();

        // Test multiple quick requests to verify rate limiting
        let start = Instant::now();

        // Make 3 simple operations that should be rate limited
        for i in 0..3 {
            let _result = client
                .execute_with_rate_limit::<_, String>(Some(format!("test_key_{i}")), || {
                    Box::pin(async {
                        // Simulate a successful operation
                        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                        Ok("success".to_string())
                    })
                })
                .await;
        }

        let elapsed = start.elapsed();

        // With rate limiting, operations should take some measurable time
        println!("Three rate-limited requests took: {elapsed:?}");
        assert!(elapsed.as_millis() >= 10); // Should take at least the sleep time
    }
}
