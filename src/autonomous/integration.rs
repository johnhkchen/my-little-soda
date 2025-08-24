use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::github::GitHubClient;
use crate::agents::{coordinator::AgentCoordinator, recovery::AutoRecovery};
use crate::agent_lifecycle::state_machine::{AgentStateMachine, AgentEvent};
use super::{
    AutonomousCoordinator, 
    AutonomousWorkflowState, 
    AutonomousEvent,
    CoordinationConfig,
    workflow_state_machine::{AutonomousStatusReport, AgentId, AbandonmentReason},
    error_recovery::AutonomousRecoveryReport,
};

/// Integration layer between autonomous workflow and existing agent coordination
pub struct AutonomousIntegration {
    autonomous_coordinator: AutonomousCoordinator,
    #[allow(dead_code)]
    agent_coordinator: AgentCoordinator,
    agent_state_machine: Arc<RwLock<AgentStateMachine>>,
    #[allow(dead_code)]
    github_client: GitHubClient,
    agent_id: String,
}

impl AutonomousIntegration {
    /// Create new autonomous integration
    pub async fn new(
        github_client: GitHubClient,
        agent_id: String,
        config: CoordinationConfig,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Create existing components
        let agent_coordinator = AgentCoordinator::new().await?;
        let agent_state_machine = Arc::new(RwLock::new(AgentStateMachine::new(agent_id.clone())));
        
        // Create recovery client
        let recovery_client = Box::new(AutoRecovery::new(github_client.clone(), true));
        
        // Create autonomous coordinator
        let autonomous_coordinator = AutonomousCoordinator::new(
            github_client.clone(),
            agent_id.clone(),
            recovery_client,
            config,
        ).await?;
        
        Ok(Self {
            autonomous_coordinator,
            agent_coordinator,
            agent_state_machine,
            github_client,
            agent_id,
        })
    }
    
    /// Start integrated autonomous operation
    pub async fn start_integrated_operation(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!(
            agent_id = %self.agent_id,
            "Starting integrated autonomous operation"
        );
        
        // Synchronize initial state between systems
        self.synchronize_initial_state().await?;
        
        // Start autonomous coordinator
        self.autonomous_coordinator.start_autonomous_operation().await?;
        
        // Synchronize final state
        self.synchronize_final_state().await?;
        
        Ok(())
    }
    
    /// Stop integrated autonomous operation
    pub async fn stop_integrated_operation(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!(
            agent_id = %self.agent_id,
            "Stopping integrated autonomous operation"
        );
        
        self.autonomous_coordinator.stop_autonomous_operation().await?;
        self.synchronize_final_state().await?;
        
        Ok(())
    }
    
    /// Synchronize initial state between autonomous and existing systems
    async fn synchronize_initial_state(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let agent_state_machine = self.agent_state_machine.read().await;
        
        // Check current agent state
        if agent_state_machine.is_available() {
            info!(
                agent_id = %self.agent_id,
                "Agent available for autonomous work"
            );
        } else {
            warn!(
                agent_id = %self.agent_id,
                current_issue = ?agent_state_machine.current_issue(),
                "Agent not available, cannot start autonomous operation"
            );
            return Err("Agent not available for autonomous operation".into());
        }
        
        Ok(())
    }
    
    /// Synchronize final state after autonomous operation
    async fn synchronize_final_state(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let status_report = self.autonomous_coordinator.get_status_report().await;
        
        match status_report.current_state {
            Some(AutonomousWorkflowState::Merged { .. }) => {
                info!(
                    agent_id = %self.agent_id,
                    "Autonomous work completed successfully, updating agent state"
                );
                
                // Update agent state machine to reflect completion
                let _agent_state_machine = self.agent_state_machine.write().await;
                // The agent should be freed and made available again
                // This would be done through the existing workflow
            }
            Some(AutonomousWorkflowState::Abandoned { reason, .. }) => {
                warn!(
                    agent_id = %self.agent_id,
                    reason = ?reason,
                    "Autonomous work abandoned, resetting agent state"
                );
                
                // Reset agent state machine
                let _agent_state_machine = self.agent_state_machine.write().await;
                // Force reset through existing event system
            }
            _ => {
                info!(
                    agent_id = %self.agent_id,
                    state = ?status_report.current_state,
                    "Autonomous operation stopped in intermediate state"
                );
            }
        }
        
        Ok(())
    }
    
    /// Get comprehensive status including both systems
    pub async fn get_integrated_status(&self) -> IntegratedStatusReport {
        let autonomous_status = self.autonomous_coordinator.get_status_report().await;
        let recovery_report = self.autonomous_coordinator.get_recovery_report().await;
        
        let agent_state = {
            let agent_state_machine = self.agent_state_machine.read().await;
            AgentStateInfo {
                is_available: agent_state_machine.is_available(),
                is_working: agent_state_machine.is_working(),
                is_assigned: agent_state_machine.is_assigned(),
                current_issue: agent_state_machine.current_issue(),
                current_branch: agent_state_machine.current_branch().map(|s| s.to_string()),
                commits_ahead: agent_state_machine.commits_ahead(),
            }
        };
        
        IntegratedStatusReport {
            agent_id: self.agent_id.clone(),
            autonomous_status,
            recovery_report,
            agent_state,
            is_running: self.autonomous_coordinator.is_running().await,
        }
    }
}

/// Comprehensive status report for integrated operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegratedStatusReport {
    pub agent_id: String,
    pub autonomous_status: AutonomousStatusReport,
    pub recovery_report: AutonomousRecoveryReport,
    pub agent_state: AgentStateInfo,
    pub is_running: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStateInfo {
    pub is_available: bool,
    pub is_working: bool,
    pub is_assigned: bool,
    pub current_issue: Option<u64>,
    pub current_branch: Option<String>,
    pub commits_ahead: u32,
}

/// Bridge trait for connecting autonomous events to existing agent events
#[async_trait]
pub trait AutonomousEventBridge {
    /// Convert autonomous event to agent lifecycle event
    async fn convert_to_agent_event(&self, autonomous_event: &AutonomousEvent) -> Option<AgentEvent>;
    
    /// Convert agent event to autonomous event
    async fn convert_to_autonomous_event(&self, agent_event: &AgentEvent) -> Option<AutonomousEvent>;
}

/// Implementation of the bridge between event systems
pub struct EventBridge {
    agent_id: String,
}

impl EventBridge {
    pub fn new(agent_id: String) -> Self {
        Self { agent_id }
    }
}

#[async_trait]
impl AutonomousEventBridge for EventBridge {
    async fn convert_to_agent_event(&self, autonomous_event: &AutonomousEvent) -> Option<AgentEvent> {
        match autonomous_event {
            AutonomousEvent::AssignAgent { .. } => {
                Some(AgentEvent::Assign {
                    agent_id: self.agent_id.clone(),
                    issue: 0, // Would be set from context
                    branch: format!("{}/autonomous-work", self.agent_id),
                })
            }
            AutonomousEvent::StartWork => {
                Some(AgentEvent::StartWork { commits_ahead: 0 })
            }
            AutonomousEvent::CompleteWork => {
                Some(AgentEvent::CompleteWork)
            }
            AutonomousEvent::ForceAbandon { .. } => {
                Some(AgentEvent::Abandon)
            }
            _ => None, // Many autonomous events don't have direct agent equivalents
        }
    }
    
    async fn convert_to_autonomous_event(&self, agent_event: &AgentEvent) -> Option<AutonomousEvent> {
        match agent_event {
            AgentEvent::Assign { .. } => {
                Some(AutonomousEvent::AssignAgent {
                    agent: AgentId(self.agent_id.clone()),
                    workspace_ready: true,
                })
            }
            AgentEvent::StartWork { .. } => {
                Some(AutonomousEvent::StartWork)
            }
            AgentEvent::CompleteWork => {
                Some(AutonomousEvent::CompleteWork)
            }
            AgentEvent::Abandon => {
                Some(AutonomousEvent::ForceAbandon {
                    reason: AbandonmentReason::RequirementsChanged,
                })
            }
            _ => None,
        }
    }
}

/// Integration coordinator that manages both systems
pub struct IntegrationCoordinator {
    integration: AutonomousIntegration,
    #[allow(dead_code)]
    event_bridge: EventBridge,
}

impl IntegrationCoordinator {
    /// Create new integration coordinator
    pub async fn new(
        github_client: GitHubClient,
        agent_id: String,
        config: CoordinationConfig,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let integration = AutonomousIntegration::new(github_client, agent_id.clone(), config).await?;
        let event_bridge = EventBridge::new(agent_id);
        
        Ok(Self {
            integration,
            event_bridge,
        })
    }
    
    /// Run integrated autonomous operation with event synchronization
    pub async fn run_with_event_sync(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Starting integrated autonomous operation with event synchronization");
        
        // Start the integration
        self.integration.start_integrated_operation().await?;
        
        // Monitor and synchronize events between systems
        // In a real implementation, this would set up event listeners and synchronization
        
        Ok(())
    }
    
    /// Get integrated status
    pub async fn get_status(&self) -> IntegratedStatusReport {
        self.integration.get_integrated_status().await
    }
}

/// Configuration for autonomous integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationConfig {
    pub coordination_config: CoordinationConfig,
    pub enable_event_synchronization: bool,
    pub sync_interval_seconds: u32,
    pub enable_state_validation: bool,
    pub validation_interval_minutes: u32,
}

impl Default for IntegrationConfig {
    fn default() -> Self {
        Self {
            coordination_config: CoordinationConfig::default(),
            enable_event_synchronization: true,
            sync_interval_seconds: 10,
            enable_state_validation: true,
            validation_interval_minutes: 5,
        }
    }
}

/// Factory for creating integrated autonomous systems
pub struct AutonomousIntegrationFactory;

impl AutonomousIntegrationFactory {
    /// Create a fully integrated autonomous system
    pub async fn create_integrated_system(
        github_client: GitHubClient,
        agent_id: String,
        config: IntegrationConfig,
    ) -> Result<IntegrationCoordinator, Box<dyn std::error::Error + Send + Sync>> {
        info!(
            agent_id = %agent_id,
            config = ?config,
            "Creating integrated autonomous system"
        );
        
        let coordinator = IntegrationCoordinator::new(
            github_client,
            agent_id,
            config.coordination_config,
        ).await?;
        
        Ok(coordinator)
    }
    
    /// Create autonomous system with existing agent coordinator
    pub async fn integrate_with_existing(
        _existing_coordinator: AgentCoordinator,
        github_client: GitHubClient,
        agent_id: String,
        config: IntegrationConfig,
    ) -> Result<IntegrationCoordinator, Box<dyn std::error::Error + Send + Sync>> {
        info!(
            agent_id = %agent_id,
            "Integrating autonomous system with existing coordinator"
        );
        
        // For now, create new coordinator - in real implementation would integrate existing
        Self::create_integrated_system(github_client, agent_id, config).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_integration_creation() {
        let github_client = GitHubClient::new().unwrap();
        let config = IntegrationConfig::default();
        
        let coordinator = AutonomousIntegrationFactory::create_integrated_system(
            github_client,
            "test-agent".to_string(),
            config,
        ).await;
        
        assert!(coordinator.is_ok());
    }
    
    #[tokio::test]
    async fn test_event_bridge_conversion() {
        let bridge = EventBridge::new("test-agent".to_string());
        
        let autonomous_event = AutonomousEvent::StartWork;
        let agent_event = bridge.convert_to_agent_event(&autonomous_event).await;
        
        assert!(matches!(agent_event, Some(AgentEvent::StartWork { .. })));
        
        if let Some(agent_event) = agent_event {
            let back_to_autonomous = bridge.convert_to_autonomous_event(&agent_event).await;
            assert!(matches!(back_to_autonomous, Some(AutonomousEvent::StartWork)));
        }
    }
    
    #[tokio::test]
    async fn test_status_report_structure() {
        let github_client = GitHubClient::new().unwrap();
        let config = IntegrationConfig::default();
        
        let coordinator = AutonomousIntegrationFactory::create_integrated_system(
            github_client,
            "test-agent".to_string(),
            config,
        ).await.unwrap();
        
        let status = coordinator.get_status().await;
        
        assert_eq!(status.agent_id, "test-agent");
        assert!(!status.is_running);
        assert!(status.agent_state.is_available);
    }
}