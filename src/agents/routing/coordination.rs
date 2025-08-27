use crate::agents::routing::{AssignmentOperations, IssueFilter, RoutingDecisions};
use crate::agents::{Agent, AgentCoordinator};
use crate::github::{GitHubClient, GitHubError};
#[cfg(feature = "metrics")]
use crate::metrics::{MetricsTracker, RoutingDecision};
use crate::telemetry::{create_coordination_span, generate_correlation_id};
use octocrab::models::issues::Issue;
use std::time::Instant;
use tracing::Instrument;

#[derive(Debug)]
pub struct RoutingAssignment {
    pub issue: Issue,
    pub assigned_agent: Agent,
    pub branch_name: String,
}

#[derive(Debug)]
pub struct RoutingCoordinator {
    pub assignment_ops: AssignmentOperations,
    pub issue_filter: IssueFilter,
    pub decisions: RoutingDecisions,
    #[cfg(feature = "metrics")]
    pub metrics_tracker: MetricsTracker,
}

impl RoutingCoordinator {
    #[cfg(feature = "metrics")]
    pub fn new(
        assignment_ops: AssignmentOperations,
        issue_filter: IssueFilter,
        decisions: RoutingDecisions,
        metrics_tracker: MetricsTracker,
    ) -> Self {
        Self {
            assignment_ops,
            issue_filter,
            decisions,
            metrics_tracker,
        }
    }

    #[cfg(not(feature = "metrics"))]
    pub fn new(
        assignment_ops: AssignmentOperations,
        issue_filter: IssueFilter,
        decisions: RoutingDecisions,
        _metrics_tracker: (),
    ) -> Self {
        Self {
            assignment_ops,
            issue_filter,
            decisions,
        }
    }

    pub async fn route_issues_to_agents(
        &self,
        coordinator: &AgentCoordinator,
        github_client: &GitHubClient,
    ) -> Result<Vec<RoutingAssignment>, GitHubError> {
        let correlation_id = generate_correlation_id();
        let span =
            create_coordination_span("route_issues_to_agents", None, None, Some(&correlation_id));

        async move {
            tracing::info!(correlation_id = %correlation_id, "Starting issue routing");

            let mut issues = self
                .issue_filter
                .fetch_routable_issues(github_client)
                .await?;
            let available_agents = coordinator.get_available_agents().await?;

            tracing::info!(
                issue_count = issues.len(),
                available_agent_count = available_agents.len(),
                "Fetched issues and agents for routing"
            );

            self.decisions.sort_issues_by_priority(&mut issues);

            let mut assignments = Vec::new();

            if let Some(agent) = available_agents.first() {
                if let Some(issue) = issues.first() {
                    let branch_name = self.assignment_ops.generate_branch_name(
                        &agent.id,
                        issue.number,
                        &issue.title,
                    );
                    let assignment = RoutingAssignment {
                        issue: issue.clone(),
                        assigned_agent: agent.clone(),
                        branch_name,
                    };

                    if !self.decisions.should_skip_assignment(issue) {
                        self.assignment_ops
                            .assign_agent_to_issue(coordinator, &agent.id, issue.number)
                            .await?;
                        tracing::info!(
                            agent_id = %agent.id,
                            issue_number = issue.number,
                            issue_title = %issue.title,
                            "Assigned agent to issue"
                        );
                    } else {
                        tracing::info!(
                            issue_number = issue.number,
                            issue_title = %issue.title,
                            "Skipped assignment for route:ready_to_merge task"
                        );
                    }

                    assignments.push(assignment);
                }
            }

            tracing::info!(
                assignment_count = assignments.len(),
                "Completed issue routing"
            );
            Ok(assignments)
        }
        .instrument(span)
        .await
    }

    pub async fn pop_any_available_task(
        &self,
        coordinator: &AgentCoordinator,
        github_client: &GitHubClient,
        current_user: &str,
    ) -> Result<Option<RoutingAssignment>, GitHubError> {
        let routing_start = Instant::now();
        let correlation_id = generate_correlation_id();

        let all_issues = github_client.fetch_issues().await?;
        let available_issues = self
            .issue_filter
            .filter_available_issues(&all_issues, current_user);

        if available_issues.is_empty() {
            return Ok(None);
        }

        let mut sorted_issues = available_issues;
        self.decisions.sort_issues_by_priority(&mut sorted_issues);

        let available_agents = coordinator.get_available_agents().await?;

        let decision_outcome =
            if let (Some(issue), Some(agent)) = (sorted_issues.first(), available_agents.first()) {
                let branch_name = if self.decisions.is_route_ready_to_merge_task(issue) {
                    self.assignment_ops
                        .create_agent_branch(github_client, &agent.id, issue.number, &issue.title)
                        .await
                        .unwrap_or_else(|_| {
                            self.assignment_ops.generate_branch_name(
                                &agent.id,
                                issue.number,
                                &issue.title,
                            )
                        })
                } else if self.decisions.is_unassigned(issue) {
                    self.assignment_ops
                        .assign_agent_to_issue(coordinator, &agent.id, issue.number)
                        .await?;
                    self.assignment_ops
                        .generate_branch_name(&agent.id, issue.number, &issue.title)
                } else {
                    self.assignment_ops
                        .create_agent_branch(github_client, &agent.id, issue.number, &issue.title)
                        .await
                        .unwrap_or_else(|_| {
                            self.assignment_ops.generate_branch_name(
                                &agent.id,
                                issue.number,
                                &issue.title,
                            )
                        })
                };

                let active_issues = vec![issue.number];
                #[cfg(feature = "metrics")]
                let _ = self
                    .metrics_tracker
                    .track_agent_utilization(
                        &agent.id,
                        1,
                        1, // Default capacity for single agent
                        active_issues,
                        "Working", // Single agent state
                    )
                    .await;

                #[cfg(feature = "metrics")]
                {
                    let decision = RoutingDecision::TaskAssigned {
                        issue_number: issue.number,
                        agent_id: agent.id.clone(),
                    };

                    let _ = self
                        .metrics_tracker
                        .track_routing_metrics(
                            correlation_id.clone(),
                            routing_start,
                            all_issues.len() as u64,
                            available_agents.len() as u64,
                            decision.clone(),
                        )
                        .await;
                }

                Ok(Some(RoutingAssignment {
                    issue: issue.clone(),
                    assigned_agent: agent.clone(),
                    branch_name,
                }))
            } else if available_agents.is_empty() {
                #[cfg(feature = "metrics")]
                {
                    let decision = RoutingDecision::NoAgentsAvailable;
                    let _ = self
                        .metrics_tracker
                        .track_routing_metrics(
                            correlation_id.clone(),
                            routing_start,
                            all_issues.len() as u64,
                            0,
                            decision,
                        )
                        .await;
                }
                Ok(None)
            } else {
                #[cfg(feature = "metrics")]
                {
                    let decision = RoutingDecision::NoTasksAvailable;
                    let _ = self
                        .metrics_tracker
                        .track_routing_metrics(
                            correlation_id.clone(),
                            routing_start,
                            all_issues.len() as u64,
                            available_agents.len() as u64,
                            decision,
                        )
                        .await;
                }
                Ok(None)
            };

        decision_outcome
    }

    pub async fn pop_task_assigned_to_me(
        &self,
        coordinator: &AgentCoordinator,
        github_client: &GitHubClient,
        current_user: &str,
    ) -> Result<Option<RoutingAssignment>, GitHubError> {
        let correlation_id = generate_correlation_id();
        let span =
            create_coordination_span("pop_task_assigned_to_me", None, None, Some(&correlation_id));

        async move {
            tracing::info!(correlation_id = %correlation_id, "Starting task pop operation");

            let all_issues = github_client.fetch_issues().await?;

            tracing::debug!(
                current_user = %current_user,
                total_issues = all_issues.len(),
                "Fetched issues for task filtering"
            );

            let mut my_issues = self
                .issue_filter
                .filter_assigned_issues(&all_issues, current_user);

            if my_issues.is_empty() {
                return Ok(None);
            }

            self.decisions.sort_issues_by_priority(&mut my_issues);

            let available_agents = coordinator.get_available_agents().await?;
            if let (Some(issue), Some(agent)) = (my_issues.first(), available_agents.first()) {
                let branch_name = self
                    .assignment_ops
                    .create_agent_branch(github_client, &agent.id, issue.number, &issue.title)
                    .await?;

                Ok(Some(RoutingAssignment {
                    issue: issue.clone(),
                    assigned_agent: agent.clone(),
                    branch_name,
                }))
            } else {
                tracing::info!("No assigned tasks available for current user");
                Ok(None)
            }
        }
        .instrument(span)
        .await
    }

    #[allow(dead_code)] // Used by router for specific issue routing
    pub async fn route_specific_issue(
        &self,
        coordinator: &AgentCoordinator,
        github_client: &GitHubClient,
        issue_number: u64,
    ) -> Result<Option<RoutingAssignment>, GitHubError> {
        let issue = github_client.fetch_issue(issue_number).await?;
        let available_agents = coordinator.get_available_agents().await?;

        if let Some(agent) = available_agents.first() {
            let branch_name = if !self.decisions.should_skip_assignment(&issue) {
                self.assignment_ops
                    .assign_agent_to_issue(coordinator, &agent.id, issue.number)
                    .await?;
                self.assignment_ops
                    .generate_branch_name(&agent.id, issue.number, &issue.title)
            } else {
                self.assignment_ops
                    .create_agent_branch(github_client, &agent.id, issue.number, &issue.title)
                    .await
                    .unwrap_or_else(|_| {
                        self.assignment_ops.generate_branch_name(
                            &agent.id,
                            issue.number,
                            &issue.title,
                        )
                    })
            };

            Ok(Some(RoutingAssignment {
                issue,
                assigned_agent: agent.clone(),
                branch_name,
            }))
        } else {
            Ok(None)
        }
    }
}
