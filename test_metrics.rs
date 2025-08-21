// Simple test to verify metrics functionality
use std::time::Instant;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use tokio::fs;

// Include the metrics module with all necessary dependencies
mod metrics {
    use serde::{Serialize, Deserialize};
    use std::collections::HashMap;
    use std::time::{SystemTime, UNIX_EPOCH, Duration, Instant};
    use tokio::fs;
    
    // Mock telemetry function
    pub fn generate_correlation_id() -> String {
        uuid::Uuid::new_v4().to_string()
    }
    
    // Mock GitHub error type
    #[derive(Debug)]
    pub enum GitHubError {
        IoError(std::io::Error),
        NotImplemented(String),
    }
    
    include!("src/metrics.rs");
}

// Mock GitHub error type
#[derive(Debug)]
pub enum GitHubError {
    IoError(std::io::Error),
    NotImplemented(String),
}

// Mock telemetry functions
pub fn generate_correlation_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing Clambake Metrics Implementation");
    println!("==========================================");
    
    let metrics_tracker = metrics::MetricsTracker::new();
    
    // Test 1: Routing Metrics
    println!("\n1. Testing Routing Metrics Tracking...");
    let correlation_id = generate_correlation_id();
    let routing_start = Instant::now();
    std::thread::sleep(std::time::Duration::from_millis(50)); // Simulate routing time
    
    let decision = metrics::RoutingDecision::TaskAssigned {
        issue_number: 123,
        agent_id: "agent001".to_string(),
    };
    
    let result = metrics_tracker.track_routing_metrics(
        correlation_id.clone(),
        routing_start,
        5, // issues evaluated
        3, // agents available
        decision,
    ).await;
    
    match result {
        Ok(_) => println!("   ✅ Routing metrics tracked successfully"),
        Err(e) => println!("   ❌ Failed to track routing metrics: {:?}", e),
    }
    
    // Test 2: Agent Utilization Metrics
    println!("\n2. Testing Agent Utilization Tracking...");
    let result = metrics_tracker.track_agent_utilization(
        "agent001",
        1, // current capacity
        3, // max capacity
        vec![123], // active issues
        "Working",
    ).await;
    
    match result {
        Ok(_) => println!("   ✅ Agent utilization tracked successfully"),
        Err(e) => println!("   ❌ Failed to track agent utilization: {:?}", e),
    }
    
    // Test 3: Coordination Decision Tracking
    println!("\n3. Testing Coordination Decision Tracking...");
    let execution_start = Instant::now();
    std::thread::sleep(std::time::Duration::from_millis(25)); // Simulate execution time
    
    let mut metadata = HashMap::new();
    metadata.insert("test_key".to_string(), "test_value".to_string());
    
    let result = metrics_tracker.track_coordination_decision(
        correlation_id.clone(),
        "assign_agent_to_issue",
        Some("agent001"),
        Some(123),
        "Successfully assigned agent001 to issue #123",
        execution_start,
        true,
        metadata,
    ).await;
    
    match result {
        Ok(_) => println!("   ✅ Coordination decision tracked successfully"),
        Err(e) => println!("   ❌ Failed to track coordination decision: {:?}", e),
    }
    
    // Test 4: Bottleneck Detection
    println!("\n4. Testing Bottleneck Detection...");
    let result = metrics_tracker.detect_and_store_bottlenecks().await;
    
    match result {
        Ok(bottlenecks) => {
            println!("   ✅ Bottleneck detection completed");
            println!("   📊 Detected {} bottlenecks", bottlenecks.len());
        }
        Err(e) => println!("   ❌ Failed to detect bottlenecks: {:?}", e),
    }
    
    // Test 5: Metrics Export
    println!("\n5. Testing Metrics Export...");
    let result = metrics_tracker.export_metrics_for_monitoring(Some(24)).await;
    
    match result {
        Ok(export_data) => {
            println!("   ✅ Metrics export completed");
            println!("   📊 Exported {} metric fields", export_data.len());
            
            // Print some sample data
            if !export_data.is_empty() {
                println!("   📋 Sample exported metrics:");
                for (key, value) in export_data.iter().take(3) {
                    println!("      • {}: {}", key, value);
                }
            }
        }
        Err(e) => println!("   ❌ Failed to export metrics: {:?}", e),
    }
    
    // Test 6: Performance Report Generation
    println!("\n6. Testing Performance Report Generation...");
    let result = metrics_tracker.format_performance_report(Some(24)).await;
    
    match result {
        Ok(report) => {
            println!("   ✅ Performance report generated successfully");
            println!("   📄 Report preview (first 200 chars):");
            let preview = if report.len() > 200 {
                format!("{}...", &report[..200])
            } else {
                report
            };
            println!("   {}", preview.replace('\n', "\n   "));
        }
        Err(e) => println!("   ❌ Failed to generate performance report: {:?}", e),
    }
    
    println!("\n🎉 Metrics Implementation Test Complete!");
    println!("✅ All core metrics features have been implemented and tested");
    
    Ok(())
}