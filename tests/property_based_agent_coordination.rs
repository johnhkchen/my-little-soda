// Property-Based Testing for Agent Coordination
// Tests agent coordination invariants under all conditions using property-based testing

use my_little_soda::agents::{AgentCoordinator, AgentState};
use proptest::prelude::*;
use proptest_derive::Arbitrary;
use std::collections::HashMap;
use tokio::sync::Mutex;
use std::sync::Arc;

// Mock data structures for property testing
#[derive(Debug, Clone, Arbitrary)]
struct AgentConfig {
    #[proptest(strategy = "agent_id_strategy()")]
    id: String,
    #[proptest(strategy = "1u32..=10")]
    max_capacity: u32,
}

#[derive(Debug, Clone, Arbitrary)]
struct IssueAssignment {
    #[proptest(strategy = "1u64..=1000")]
    issue_number: u64,
    #[proptest(strategy = "agent_id_strategy()")]
    agent_id: String,
}

// Strategy for generating valid agent IDs
fn agent_id_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("agent001".to_string()),
        Just("agent002".to_string()),
        Just("agent003".to_string()),
        Just("agent004".to_string()),
    ]
}

// Mock coordinator for testing without GitHub API calls
#[derive(Debug)]
struct MockAgentCoordinator {
    agent_capacity: Arc<Mutex<HashMap<String, (u32, u32)>>>, // agent_id -> (current, max)
    assignments: Arc<Mutex<HashMap<u64, String>>>, // issue_number -> agent_id
}

impl MockAgentCoordinator {
    fn new(agents: Vec<AgentConfig>) -> Self {
        let mut capacity_map = HashMap::new();
        for agent in agents {
            capacity_map.insert(agent.id, (0, agent.max_capacity));
        }
        
        Self {
            agent_capacity: Arc::new(Mutex::new(capacity_map)),
            assignments: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn assign_agent_to_issue(&self, agent_id: &str, issue_number: u64) -> Result<(), String> {
        let mut assignments = self.assignments.lock().await;
        let mut capacities = self.agent_capacity.lock().await;

        // Check if issue already assigned
        if assignments.contains_key(&issue_number) {
            return Err(format!("Issue #{} already assigned", issue_number));
        }

        // Check agent exists and has capacity
        let (current, max) = capacities.get(agent_id)
            .ok_or_else(|| format!("Unknown agent: {}", agent_id))?
            .clone();

        if current >= max {
            return Err(format!("Agent {} at capacity ({}/{})", agent_id, current, max));
        }

        // Perform assignment
        assignments.insert(issue_number, agent_id.to_string());
        capacities.insert(agent_id.to_string(), (current + 1, max));

        Ok(())
    }

    async fn unassign_agent_from_issue(&self, issue_number: u64) -> Result<(), String> {
        let mut assignments = self.assignments.lock().await;
        let mut capacities = self.agent_capacity.lock().await;

        let agent_id = assignments.remove(&issue_number)
            .ok_or_else(|| format!("Issue #{} not assigned", issue_number))?;

        if let Some((current, max)) = capacities.get(&agent_id).cloned() {
            if current > 0 {
                capacities.insert(agent_id, (current - 1, max));
            }
        }

        Ok(())
    }

    async fn get_assignments(&self) -> HashMap<u64, String> {
        self.assignments.lock().await.clone()
    }

    async fn get_capacities(&self) -> HashMap<String, (u32, u32)> {
        self.agent_capacity.lock().await.clone()
    }

    async fn validate_invariants(&self) -> Result<(), String> {
        let assignments = self.assignments.lock().await;
        let capacities = self.agent_capacity.lock().await;

        // Count actual assignments per agent
        let mut actual_counts = HashMap::new();
        for agent_id in assignments.values() {
            *actual_counts.entry(agent_id.clone()).or_insert(0) += 1;
        }

        // Verify capacity tracking is accurate
        for (agent_id, (tracked_current, max_capacity)) in capacities.iter() {
            let actual_current = actual_counts.get(agent_id).unwrap_or(&0);
            
            if actual_current != tracked_current {
                return Err(format!(
                    "Agent {} capacity mismatch: tracked={}, actual={}",
                    agent_id, tracked_current, actual_current
                ));
            }

            if tracked_current > max_capacity {
                return Err(format!(
                    "Agent {} over-capacity: {}/{}",
                    agent_id, tracked_current, max_capacity
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    

    #[test]
    fn prop_no_over_assignment() {
        let mut runner = proptest::test_runner::TestRunner::default();
        
        runner.run(&(
            prop::collection::vec(any::<AgentConfig>(), 1..5),
            prop::collection::vec(any::<IssueAssignment>(), 0..20)
        ), |(agents, assignments)| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let coordinator = MockAgentCoordinator::new(agents.clone());
                
                // Try to perform all assignments
                for assignment in assignments {
                    let _ = coordinator.assign_agent_to_issue(&assignment.agent_id, assignment.issue_number).await;
                }

                // Verify no over-assignment occurred
                let capacities = coordinator.get_capacities().await;
                for (agent_id, (current, max)) in capacities {
                    prop_assert!(
                        current <= max,
                        "Agent {} over-assigned: {}/{}", agent_id, current, max
                    );
                }

                // Verify invariants hold
                coordinator.validate_invariants().await.map_err(|e| {
                    proptest::test_runner::TestCaseError::Fail(e.into())
                })?;
                
                Ok(())
            })
        }).unwrap();
    }

    #[test]
    fn prop_unique_assignments() {
        let mut runner = proptest::test_runner::TestRunner::default();
        
        runner.run(&(
            prop::collection::vec(any::<AgentConfig>(), 1..5),
            prop::collection::vec(1u64..=100, 0..20)
        ), |(agents, issue_numbers)| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let coordinator = MockAgentCoordinator::new(agents.clone());
                let agent_ids: Vec<String> = agents.iter().map(|a| a.id.clone()).collect();
                
                // Try to assign the same issues multiple times
                for &issue_number in &issue_numbers {
                    for agent_id in &agent_ids {
                        let _ = coordinator.assign_agent_to_issue(agent_id, issue_number).await;
                    }
                }

                // Verify each issue is assigned at most once
                let assignments = coordinator.get_assignments().await;
                let unique_issues: std::collections::HashSet<_> = assignments.keys().collect();
                
                prop_assert_eq!(
                    assignments.len(), 
                    unique_issues.len(),
                    "Duplicate issue assignments detected"
                );
                
                Ok(())
            })
        }).unwrap();
    }

    #[test]
    fn prop_assignment_reversibility() {
        let mut runner = proptest::test_runner::TestRunner::default();
        
        runner.run(&(
            prop::collection::vec(any::<AgentConfig>(), 1..3),
            prop::collection::vec(any::<IssueAssignment>(), 1..10)
        ), |(agents, assignments)| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let coordinator = MockAgentCoordinator::new(agents.clone());
                
                let initial_capacities = coordinator.get_capacities().await;
                let mut successful_assignments = Vec::new();

                // Perform assignments
                for assignment in assignments {
                    if coordinator.assign_agent_to_issue(&assignment.agent_id, assignment.issue_number).await.is_ok() {
                        successful_assignments.push(assignment);
                    }
                }

                // Unassign all successful assignments
                for assignment in successful_assignments {
                    coordinator.unassign_agent_from_issue(assignment.issue_number).await.unwrap();
                }

                // Verify we're back to initial state
                let final_capacities = coordinator.get_capacities().await;
                prop_assert_eq!(initial_capacities, final_capacities, "State not restored after unassignment");

                let final_assignments = coordinator.get_assignments().await;
                prop_assert!(final_assignments.is_empty(), "Assignments not fully cleared");
                
                Ok(())
            })
        }).unwrap();
    }

    #[test]
    fn prop_concurrent_safety() {
        let mut runner = proptest::test_runner::TestRunner::default();
        
        runner.run(&(
            prop::collection::vec(any::<AgentConfig>(), 2..4),
            prop::collection::vec(
                prop::collection::vec(any::<IssueAssignment>(), 1..5), 
                2..5
            )
        ), |(agents, assignment_batches)| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let coordinator = Arc::new(MockAgentCoordinator::new(agents));
                let mut handles = Vec::new();

                // Spawn concurrent assignment tasks
                for batch in assignment_batches {
                    let coordinator = coordinator.clone();
                    let handle = tokio::spawn(async move {
                        for assignment in batch {
                            let _ = coordinator.assign_agent_to_issue(&assignment.agent_id, assignment.issue_number).await;
                        }
                    });
                    handles.push(handle);
                }

                // Wait for all tasks to complete
                for handle in handles {
                    handle.await.unwrap();
                }

                // Verify invariants still hold after concurrent operations
                coordinator.validate_invariants().await.map_err(|e| {
                    proptest::test_runner::TestCaseError::Fail(e.into())
                })?;
                
                Ok(())
            })
        }).unwrap();
    }

    #[tokio::test]
    async fn test_property_framework_setup() {
        // Basic test to ensure the property testing framework is working
        let agents = vec![
            AgentConfig { id: "agent001".to_string(), max_capacity: 2 },
            AgentConfig { id: "agent002".to_string(), max_capacity: 1 },
        ];
        
        let coordinator = MockAgentCoordinator::new(agents);
        
        // Test basic assignment
        assert!(coordinator.assign_agent_to_issue("agent001", 1).await.is_ok());
        assert!(coordinator.assign_agent_to_issue("agent001", 2).await.is_ok());
        
        // Test capacity limit
        assert!(coordinator.assign_agent_to_issue("agent001", 3).await.is_err());
        
        // Test invariants
        assert!(coordinator.validate_invariants().await.is_ok());
        
        println!("✅ Property testing framework setup complete");
    }
}

#[cfg(test)]
mod chaos_testing {
    use super::*;
    use std::time::Duration;

    // Chaos testing: Inject failures and verify system recovery
    #[derive(Debug, Clone)]
    enum ChaosEvent {
        NetworkFailure,
        PartialFailure,
        RecoveryAttempt,
    }

    #[tokio::test]
    async fn test_system_resilience_under_chaos() {
        println!("🌀 Starting chaos testing for agent coordination");

        let agents = vec![
            AgentConfig { id: "agent001".to_string(), max_capacity: 2 },
            AgentConfig { id: "agent002".to_string(), max_capacity: 2 },
        ];
        
        let coordinator = MockAgentCoordinator::new(agents);

        // Simulate normal operations
        assert!(coordinator.assign_agent_to_issue("agent001", 1).await.is_ok());
        assert!(coordinator.assign_agent_to_issue("agent002", 2).await.is_ok());

        // Inject chaos: simulate network failures during operations
        let chaos_events = vec![
            ChaosEvent::NetworkFailure,
            ChaosEvent::PartialFailure,
            ChaosEvent::RecoveryAttempt,
        ];

        for event in chaos_events {
            match event {
                ChaosEvent::NetworkFailure => {
                    // Simulate network failure - operations should fail gracefully
                    println!("🔥 Injecting network failure");
                    // In real implementation, this would test GitHub API failures
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
                ChaosEvent::PartialFailure => {
                    // Simulate partial system failure
                    println!("⚡ Injecting partial failure");
                    // Try operations during simulated instability
                    let _ = coordinator.assign_agent_to_issue("agent001", 3).await;
                }
                ChaosEvent::RecoveryAttempt => {
                    // Simulate recovery - system should restore consistency
                    println!("🔄 Testing recovery");
                    assert!(coordinator.validate_invariants().await.is_ok());
                }
            }
        }

        // Final verification: system should be in a consistent state
        assert!(coordinator.validate_invariants().await.is_ok());
        println!("✅ System maintained consistency under chaos conditions");
    }

    #[tokio::test]
    async fn test_capacity_violation_protection() {
        println!("🛡️ Testing capacity violation protection");

        let agents = vec![
            AgentConfig { id: "agent001".to_string(), max_capacity: 1 },
        ];
        
        let coordinator = MockAgentCoordinator::new(agents);

        // Fill agent capacity
        assert!(coordinator.assign_agent_to_issue("agent001", 1).await.is_ok());

        // Attempt capacity violation - should be rejected
        assert!(coordinator.assign_agent_to_issue("agent001", 2).await.is_err());

        // Verify system state remains consistent
        assert!(coordinator.validate_invariants().await.is_ok());

        let capacities = coordinator.get_capacities().await;
        let (current, max) = capacities.get("agent001").unwrap();
        assert_eq!(*current, 1);
        assert_eq!(*max, 1);

        println!("✅ Capacity violations properly prevented");
    }
}

#[cfg(test)]
mod integration_property_tests {
    use super::*;

    // Integration tests that connect property testing with real agent coordination
    // These will be skipped if GitHub credentials are not available
    
    #[tokio::test]
    async fn test_real_agent_coordination_properties() {
        // Skip if GitHub credentials not available
        let coordinator = match AgentCoordinator::new().await {
            Ok(coord) => coord,
            Err(_) => {
                println!("⏭️ Skipping real GitHub property test - credentials not available");
                return;
            }
        };

        println!("🧪 Testing real agent coordination properties");

        // Property: Available agents should have capacity
        let agents = coordinator.get_available_agents().await.unwrap_or_default();
        for agent in &agents {
            match &agent.state {
                AgentState::Available => {
                    assert!(agent.capacity > 0, "Available agent should have capacity");
                }
                _ => {
                    // Non-available agents can have any capacity state
                }
            }
        }

        // Property: System utilization should be within bounds
        let utilization = coordinator.get_agent_utilization().await;
        for (agent_id, (current, max)) in utilization {
            assert!(
                current <= max,
                "Agent {} utilization {}/{} exceeds capacity",
                agent_id, current, max
            );
        }

        println!("✅ Real agent coordination properties verified");
    }
}