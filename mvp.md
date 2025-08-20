# Clambake MVP: Test-Driven Development Approach

## Overview

This MVP focuses on the core functionality needed to prove that Clambake can orchestrate multi-agent development workflows successfully. We use our expressive testing framework to drive development, ensuring each feature works correctly before moving to the next.

## MVP Scope: Core Agent Orchestration

### What We're Building
1. **Basic GitHub Integration**: Issue querying, project board management, branch operations
2. **Agent Lifecycle Management**: Route tickets, track status, coordinate cleanup
3. **Work Integration Pipeline**: Create PRs, merge to main, preserve work
4. **Observability Foundation**: Basic Phoenix tracing and metrics
5. **CLI Interface**: Essential commands for multi-agent coordination

### What We're NOT Building (Yet)
- Claude Code sub-agent spawning (mocked for now)
- Advanced dependency resolution
- Complex conflict resolution strategies
- Performance optimization
- Enterprise-scale features

## Test-Driven MVP Development Strategy

### Phase 1: GitHub Integration Foundation

**Target**: Prove we can reliably interact with GitHub APIs for basic orchestration

```rust
// Test that drives GitHub integration development
scenario! {
    name: "Basic GitHub issue routing",
    given: {
        github_repo: test_repo!(
            issues: [
                issue!(id: 101, labels: ["route:ready"], status: "open"),
                issue!(id: 102, labels: ["route:ready", "priority:high"], status: "open"),
                issue!(id: 103, labels: ["bug"], status: "open"), // Should not be routed
            ]
        ),
        clambake_config: config!(max_agents: 2),
    },
    when: {
        action: clambake_route!(args: ["--dry-run"]),
    },
    then: {
        should_identify_routable_issues!(count: 2),
        should_respect_routing_labels!(),
        should_output_routing_plan!(),
        should_not_modify_github_state!(), // dry-run mode
    },
}

// Test that drives actual GitHub state changes
scenario! {
    name: "GitHub issue assignment and branch creation",
    given: {
        github_repo: test_repo!(issues: [
            issue!(id: 201, labels: ["route:ready"], assignee: None)
        ]),
        mock_agents: [
            mock_agent!(id: "agent-001", status: Available)
        ],
    },
    when: {
        action: clambake_route!(args: ["--agents", "1"]),
    },
    then: {
        github_state: {
            issue!(201).should_be_assigned_to("agent-001"),
            branch!("agent-001/201-*").should_exist!(),
            project_board!().should_show_agent_in_progress!("agent-001"),
        },
        local_state: {
            agent!("agent-001").should_have_assignment!(issue: 201),
        },
    },
}
```

**MVP Deliverable 1: GitHub Integration Module**
- `src/github/client.rs` - Octocrab wrapper with error handling
- `src/github/issues.rs` - Issue querying and assignment logic  
- `src/github/projects.rs` - Project board state management
- `src/github/branches.rs` - Branch creation and cleanup

### Phase 2: Agent Lifecycle Coordination

**Target**: Prove we can track agent state and coordinate work assignments

```rust
// Test that drives agent state management
scenario! {
    name: "Agent state transitions",
    given: {
        agents: [
            agent!(id: "agent-001", status: Available, capacity: 2),
            agent!(id: "agent-002", status: Available, capacity: 1),
        ],
        pending_tickets: [
            ticket!(id: 301, complexity: Low),
            ticket!(id: 302, complexity: High),
            ticket!(id: 303, complexity: Medium),
        ],
    },
    when: {
        actions: [
            assign_ticket!(301, to: "agent-001"),
            assign_ticket!(302, to: "agent-002"),
            assign_ticket!(303, to: "agent-001"), // Should succeed (under capacity)
        ],
    },
    then: {
        agent_state: {
            agent!("agent-001").should_have_assignments!(count: 2),
            agent!("agent-002").should_have_assignments!(count: 1),
            agent!("agent-001").should_be_at_capacity!(),
            agent!("agent-002").should_be_at_capacity!(),
        },
        coordination_checks: {
            no_over_assignment!(),
            capacity_limits_respected!(),
            state_consistency!(),
        },
    },
}

// Test that drives work completion and cleanup
scenario! {
    name: "Agent work completion cycle",
    given: {
        agent: agent!(id: "agent-001", 
                     assigned_issue: 401,
                     work_status: Completed,
                     branch: "agent-001/401-feature"),
        github_repo: test_repo!(),
    },
    when: {
        action: clambake_land!(agent: "agent-001"),
    },
    then: {
        github_state: {
            pr!("agent-001/401-feature" -> "main").should_be_created!(),
            issue!(401).should_be_linked_to_pr!(),
        },
        agent_state: {
            agent!("agent-001").should_transition_to!(Available),
            agent!("agent-001").assignments.should_be_empty!(),
        },
        work_preservation: {
            branch!("agent-001/401-feature").should_exist_until_merged!(),
            commits_should_be_preserved!(),
        },
    },
}
```

**MVP Deliverable 2: Agent Coordination Module**
- `src/agents/coordinator.rs` - Agent state management
- `src/agents/router.rs` - Ticket assignment logic
- `src/workflows/state_machine.rs` - State transition management
- `src/agents/integrator.rs` - Work completion handling

### Phase 3: Work Integration Pipeline

**Target**: Prove completed work safely lands on main branch

```rust
// Test that drives PR creation and merge workflow
integration_flow! {
    name: "Complete work integration pipeline",
    participants: {
        github: mock_github_api!(),
        local_git: temp_git_repo!(),
        ci_system: mock_github_actions!(),
    },
    timeline: {
        t0: {
            setup: {
                github.create_pr!(
                    from: "agent-001/501-auth",
                    to: "main",
                    title: "Implement user authentication",
                    body: "Completed by agent-001"
                ),
                ci_system.set_status!("pending"),
            },
        },
        t1: {
            simulate: ci_system.complete_successfully!(),
            expect: pr_ready_for_merge!(),
        },
        t2: {
            action: clambake_land!(auto_merge: true),
            expect: {
                pr_merged_to_main!(),
                branch_cleaned_up!(),
                issue_closed!(),
                agent_freed!(),
            },
        },
        t3: {
            validate: {
                main_branch_contains_work!(),
                no_orphaned_branches!(),
                github_state_consistent!(),
            },
        },
    },
}

// Test that drives conflict resolution
scenario! {
    name: "Merge conflict handling",
    given: {
        main_branch: with_conflicting_changes!(),
        agent_branch: "agent-002/502-feature",
        work_completed: true,
    },
    when: {
        action: clambake_land!(agent: "agent-002"),
    },
    then: {
        should_detect_merge_conflict!(),
        should_create_recovery_pr!(),
        should_preserve_agent_work!(),
        should_request_human_review!(),
        should_not_block_other_agents!(),
    },
}
```

**MVP Deliverable 3: Integration Pipeline Module**
- `src/integration/pr_manager.rs` - PR creation and merge logic
- `src/integration/conflict_resolver.rs` - Conflict detection and recovery
- `src/workflows/atomic_operations.rs` - Transaction-safe operations
- `src/error/recovery.rs` - Error handling and work preservation

### Phase 4: Basic Observability

**Target**: Prove we can trace and monitor agent coordination

```rust
// Test that drives Phoenix integration
scenario! {
    name: "Coordination tracing and metrics",
    given: {
        phoenix_server: embedded_phoenix!(),
        tracing_enabled: true,
    },
    when: {
        actions: [
            clambake_route!(agents: 2),
            simulate_work_completion!(),
            clambake_land!(),
        ],
    },
    then: {
        phoenix_traces: {
            should_contain_spans!([
                "ticket-routing",
                "agent-assignment", 
                "work-integration",
                "cleanup-operations"
            ]),
            should_record_metrics!([
                "routing.latency_ms",
                "integration.success_rate",
                "agent.utilization"
            ]),
            should_correlate_operations!(),
        },
        debugging_capability: {
            can_trace_agent_workflow!(agent: "agent-001"),
            can_identify_bottlenecks!(),
            can_analyze_coordination_patterns!(),
        },
    },
}
```

**MVP Deliverable 4: Observability Module**
- `src/phoenix/tracer.rs` - OpenTelemetry integration
- `src/phoenix/metrics.rs` - Coordination metrics collection
- `src/phoenix/dashboard.rs` - Basic dashboard setup

### Phase 5: CLI Interface

**Target**: Prove the CLI provides a good developer experience

```rust
// Test that drives CLI command implementation
scenario! {
    name: "Complete CLI workflow",
    given: {
        fresh_repository: git_repo!(),
        github_configured: true,
    },
    when: {
        cli_commands: [
            "clambake init --project-type webapp --agents 3",
            "clambake route --priority high --max-agents 2", 
            "clambake status",
            "clambake land --auto-merge",
            "clambake cleanup",
        ],
    },
    then: {
        init_command: {
            should_create_clambake_config!(),
            should_setup_github_integration!(),
            should_initialize_observability!(),
        },
        route_command: {
            should_assign_tickets_to_agents!(),
            should_provide_clear_output!(),
            should_respect_constraints!(),
        },
        status_command: {
            should_show_agent_states!(),
            should_show_pending_work!(),
            should_show_system_health!(),
        },
        land_command: {
            should_integrate_completed_work!(),
            should_handle_merge_conflicts!(),
        },
        cleanup_command: {
            should_remove_merged_branches!(),
            should_free_idle_agents!(),
        },
    },
}
```

**MVP Deliverable 5: CLI Module**
- `src/commands/init.rs` - Project initialization
- `src/commands/route.rs` - Ticket routing command
- `src/commands/status.rs` - System status display
- `src/commands/land.rs` - Work integration command
- `src/commands/cleanup.rs` - Resource cleanup

## MVP Success Criteria

### Functional Requirements Met
```rust
property_test! {
    name: "MVP coordination guarantees",
    invariants: {
        work_preservation: "No completed work is ever lost",
        state_consistency: "GitHub and local state always match",
        agent_coordination: "No conflicting assignments or race conditions",
        integration_reliability: "Work successfully lands on main branch",
        observability: "All coordination decisions are traceable",
    },
}
```

### Performance Targets Met
```rust
performance_benchmark! {
    name: "MVP performance requirements",
    targets: {
        routing_latency: less_than!(5.seconds),    // Generous for MVP
        integration_time: less_than!(2.minutes),
        concurrent_agents: supports!(3),           // Modest for MVP
        memory_usage: less_than!(100.mb),
    },
}
```

### Developer Experience Validated
```rust
scenario! {
    name: "New developer onboarding",
    given: {
        developer: new_to_clambake!(),
        repository: fresh_webapp_project!(),
    },
    when: {
        developer.runs!("clambake init"),
        developer.creates_issues!(count: 5),
        developer.runs!("clambake route"),
        developer.waits_for_agents!(),
        developer.runs!("clambake land"),
    },
    then: {
        should_complete_without_errors!(),
        should_produce_working_code!(),
        should_provide_clear_feedback!(),
        should_build_developer_confidence!(),
    },
}
```

## Implementation Strategy

### Test-First Development Process

1. **Write the Test Spell**: Define the behavior we want using our expressive testing macros
2. **Run the Test**: Watch it fail (red)
3. **Implement Minimum Code**: Make the test pass (green)
4. **Refactor**: Clean up implementation while keeping tests green
5. **Add Edge Cases**: Expand test coverage with property-based testing

### Mock Strategy for MVP

Since we're building the coordination layer first:

- **Mock Claude Code Integration**: Simulate agent behaviors without actual Claude Code
- **Real GitHub API**: Use GitHub's API in sandbox mode for realistic testing
- **Embedded Phoenix**: Run real Phoenix for observability (lightweight)
- **Simulated Work**: Generate realistic agent work patterns for testing

### Quality Gates

Every MVP deliverable must pass:
- ✅ All scenario tests pass
- ✅ Property-based tests find no invariant violations  
- ✅ Performance benchmarks meet targets
- ✅ Integration tests with real GitHub API succeed
- ✅ CLI commands provide good developer experience

## MVP Timeline & Milestones

### Week 1: Foundation
- GitHub integration tests passing
- Basic agent state management
- CLI framework established

### Week 2: Coordination  
- Agent routing and assignment working
- State consistency guaranteed
- Basic error handling implemented

### Week 3: Integration
- PR creation and merge pipeline
- Conflict detection and recovery
- Work preservation guaranteed

### Week 4: Polish
- Phoenix observability integrated
- CLI experience refined
- Documentation and examples

## Post-MVP: What Comes Next

Once MVP proves the core concept:
1. **Real Claude Code Integration**: Replace mocks with actual sub-agent spawning
2. **Advanced Conflict Resolution**: Implement sophisticated merge strategies
3. **Performance Optimization**: Scale to 8-12 concurrent agents
4. **Enterprise Features**: Advanced dependency tracking, custom workflows
5. **Ecosystem Integration**: VS Code extensions, Slack notifications, etc.

The MVP establishes the foundation that proves multi-agent coordination is not only possible but reliable, setting the stage for the full vision of industrial-scale agent development.