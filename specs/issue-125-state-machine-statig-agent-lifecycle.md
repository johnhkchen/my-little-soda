# Issue #125: Implement State Machine with statig & Fix Agent Lifecycle

> **Replace custom state management with production-grade state machine library and eliminate agent stuck states**

## Problem Statement

### Current State
- **Incomplete State Machine**: `/src/agent_lifecycle/state_machine.rs` contains only placeholder comment "TODO: Implement AgentStateMachine"
- **Ad-hoc State Management**: Agent states managed through manual GitHub label manipulation without formal transitions
- **Agent Stuck States**: No systematic recovery from inconsistent states (labeled but no branch, branch but no label, etc.)
- **Over-engineered Foundation**: Complex agent lifecycle framework exists but lacks the core state machine logic to make it functional

### Context
- Agent lifecycle documentation (`docs/agent_lifecycle.md`) defines comprehensive state model but lacks implementation
- Current system has 27+ compiler warnings indicating incomplete features and unused components  
- Manual state management creates race conditions and inconsistent agent states
- Existing coordination logic is sound but needs reliable state machine foundation

### Strategic Value
A robust state machine implementation provides:
- **Eliminates Stuck Agents**: Systematic detection and recovery from inconsistent states
- **Enables Autonomous Operation**: Reliable state management for continuous single-agent operation
- **Reduces Operational Overhead**: Automatic state validation and recovery vs manual intervention
- **Foundation for Autonomy**: Prerequisite for unattended autonomous agent operation

## Target State

### Vision
A production-ready agent lifecycle system powered by `statig` state machine that:
1. **Enforces Valid Transitions**: Only allows legitimate state changes with proper validation
2. **Detects Inconsistencies**: Automatically identifies and reports state synchronization issues
3. **Provides Recovery Actions**: Clear remediation paths for all identified inconsistent states
4. **Maintains GitHub Sync**: Bidirectional synchronization between internal state and GitHub labels/branches

### Success Metrics
- **Zero Agent Downtime**: Agent never remains in inconsistent states for >5 minutes
- **State Consistency**: 100% accuracy between internal state and GitHub/Git reality
- **Automatic Recovery**: >90% of detected inconsistencies resolved without manual intervention
- **Transition Validation**: All state changes go through validated state machine transitions
- **Diagnostic Clarity**: Clear reporting on why agent is in specific state

### Non-Goals
- Advanced workflow orchestration beyond basic agent lifecycle
- Cross-repository coordination (use separate My Little Soda instances per repo)
- Integration with external state storage systems (Redis, databases)
- Complex agent assignment algorithms (keep existing routing logic)

## Interfaces & Contracts

### Core State Machine Interface
```rust
use statig::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub enum AgentState {
    Idle,
    Assigned { agent_id: String, issue: u64, branch: String },
    Working { agent_id: String, issue: u64, branch: String, commits_ahead: u32 },
    Landed { issue: u64 }, // Agent freed immediately
    Bundled { issues: Vec<u64>, bundle_pr: u64 },
    Merged { issues: Vec<u64> },
}

#[derive(Debug, Clone)]
pub enum AgentEvent {
    Assign { agent_id: String, issue: u64 },
    StartWork { commits_ahead: u32 },
    CompleteWork,
    Bundle { bundle_pr: u64 },
    Merge,
    Abandon,
    ForceReset,
}

pub trait AgentStateMachine {
    /// Process state transition with validation
    async fn transition(&mut self, event: AgentEvent) -> Result<AgentState, TransitionError>;
    
    /// Detect inconsistencies between internal state and external reality
    async fn detect_inconsistencies(&self) -> Result<Vec<Inconsistency>, StateError>;
    
    /// Attempt automatic recovery for detected inconsistencies  
    async fn recover_inconsistencies(&mut self) -> Result<RecoveryReport, StateError>;
    
    /// Get current state with validation against external systems
    async fn get_validated_state(&self) -> Result<AgentState, StateError>;
}
```

### State Validation Interface
```rust
pub struct StateValidator {
    github_client: GitHubClient,
    git_ops: GitOperations,
}

impl StateValidator {
    /// Validate agent state against GitHub labels and Git branches
    async fn validate_state(&self, agent_id: &str, claimed_state: &AgentState) -> Result<ValidationReport>;
    
    /// Detect specific inconsistency patterns
    async fn detect_stuck_patterns(&self) -> Result<Vec<StuckAgentPattern>>;
}

pub enum StuckAgentPattern {
    LabeledButNoBranch { agent_id: String, issue: u64 },
    BranchButNoLabel { agent_id: String, branch: String },
    WorkingButNoCommits { agent_id: String, issue: u64 },
    LandedButNotFreed { agent_id: String, issue: u64 },
}
```

### Command-Line Interface
```bash
# State machine diagnostics
clambake agent status --agent agent001           # Show current state with validation
clambake agent diagnose --all                    # Detect all agent inconsistencies
clambake agent recover --agent agent001          # Attempt automatic recovery
clambake agent force-reset --agent agent001      # Force agent to idle state

# State machine reporting
clambake agent status --show-state               # Show current agent state
clambake agent history --agent agent001          # Show state transition history
clambake agent validate --all                    # Validate all agent states
```

### Configuration Integration
```toml
# clambake.toml additions
[state_machine]
enabled = true
inconsistency_check_interval_minutes = 5
auto_recovery_enabled = true
transition_logging = "detailed"  # or "minimal"

[recovery]
max_stuck_time_minutes = 15
force_reset_after_minutes = 60
cleanup_abandoned_branches = true
```

## Technical Implementation Strategy

### Phase 1: statig Integration (2 hours)
**Implement formal state machine using statig library**

1. **State Machine Definition** (60 minutes)
   - Add `statig` dependency to Cargo.toml
   - Define agent states, events, and transitions using statig macros
   - Implement basic state machine with transition validation
   - Add state persistence using GitHub as source of truth

2. **Transition Logic Implementation** (60 minutes)
   - Map existing agent operations to state machine events
   - Add transition guards for validation (branch exists, issue assignable, etc.)
   - Implement state entry/exit actions (create branches, update labels, etc.)
   - Add comprehensive error handling for transition failures

### Phase 2: State Validation System (2 hours)
**Build comprehensive state consistency detection and validation**

1. **Inconsistency Detection** (60 minutes)
   - Implement state validator that checks GitHub labels vs Git branches
   - Add detection for common stuck agent patterns
   - Create diagnostic reporting for identified inconsistencies
   - Add automated scanning for system-wide state validation

2. **Recovery Operations** (60 minutes)
   - Implement automatic recovery actions for each inconsistency pattern
   - Add manual recovery commands for complex scenarios
   - Create safe force-reset operations that preserve work
   - Add rollback capabilities for failed recovery attempts

### Phase 3: Integration & Testing (2 hours)
**Connect state machine to existing agent lifecycle and validate robustness**

1. **Agent Lifecycle Integration** (60 minutes)
   - Replace manual state tracking with statig state machine
   - Update agent coordination to use state machine transitions
   - Integrate state validation into existing workflows
   - Add state machine status reporting to CLI commands

2. **Comprehensive Testing** (60 minutes)
   - Add property tests for state machine invariants
   - Test all identified stuck agent scenarios
   - Validate recovery operations don't lose work
   - Add integration tests with real GitHub API interactions

## Implementation Tasks (1-Hour Chunks)

### Task 1: Add statig Dependency and Basic State Machine (1 hour)
- **Objective**: Replace placeholder with working statig state machine
- **Acceptance**: Agent states defined with valid transitions using statig
- **Files**: `Cargo.toml`, `src/agent_lifecycle/state_machine.rs`
- **Test**: State transitions work correctly with validation

### Task 2: Implement State Machine Transitions (1 hour)
- **Objective**: Map agent operations to formal state machine events
- **Acceptance**: All agent lifecycle operations use state machine transitions
- **Files**: State machine implementation, transition logic
- **Test**: Agent assignment, work, and landing trigger correct transitions

### Task 3: Build State Validation System (1 hour)
- **Objective**: Detect inconsistencies between states and external reality
- **Acceptance**: System identifies all defined stuck agent patterns
- **Files**: State validator, inconsistency detection
- **Test**: Detect artificially created inconsistent states

### Task 4: Implement Automatic Recovery (1 hour)
- **Objective**: Resolve common inconsistencies without manual intervention
- **Acceptance**: >90% of detected inconsistencies automatically resolved
- **Files**: Recovery operations, automatic remediation
- **Test**: Create stuck states, verify automatic recovery

### Task 5: Integrate with Agent Coordination (1 hour)
- **Objective**: Connect state machine to existing agent lifecycle workflows
- **Acceptance**: Existing workflows use state machine for state management
- **Files**: Agent coordinator, command handlers, workflow integration
- **Test**: End-to-end workflows maintain consistent state

### Task 6: Add Diagnostic and Manual Recovery Commands (1 hour)
- **Objective**: Provide operational tools for state management
- **Acceptance**: Operators can diagnose and resolve state issues via CLI
- **Files**: CLI commands, diagnostic reporting, manual recovery
- **Test**: Manual state validation and recovery work correctly

## Data Model & Storage

### State Persistence Strategy
The state machine uses **GitHub as the authoritative state store**:
- **Agent Assignment**: `agent001` labels on issues
- **Work Status**: Issue labels (`route:ready`, `route:review`)
- **Branch State**: Git branch existence and commit status
- **Recovery Metadata**: PR descriptions and issue comments for traceability

### In-Memory State Management
```rust
pub struct AgentStateMachine {
    agent_id: String,
    current_state: AgentState,
    state_machine: StateMachine<AgentState, AgentEvent>,
    last_validated: DateTime<Utc>,
    pending_transitions: VecDeque<AgentEvent>,
}
```

### No Additional Storage Required
- Leverages existing GitHub and Git storage patterns
- State machine operates statelessly with validation on demand
- Recovery operations leave audit trails in GitHub comments

## Observability & Monitoring

### State Machine Metrics
```rust
// Key metrics for state machine health
struct StateMachineMetrics {
    pub transitions_per_minute: Counter,
    pub transition_failures: Counter,
    pub inconsistencies_detected: Counter,
    pub automatic_recoveries: Counter,
    pub stuck_agent_duration: Histogram,
}
```

### Structured Logging
```rust
// State transition logging
info!(
    agent_id = %agent_id,
    from_state = ?old_state,
    to_state = ?new_state,
    event = ?event,
    "Agent state transition completed"
);

// Inconsistency detection
warn!(
    agent_id = %agent_id,
    inconsistency_type = %inconsistency.pattern,
    detected_at = %Utc::now(),
    "Agent state inconsistency detected"
);
```

### Health Checks
- State machine transition success rate
- Inconsistency detection frequency
- Average time to recover from stuck states
- State validation accuracy

## Rollback Strategy

### Immediate Rollback (< 2 minutes)
```bash
# Disable state machine validation
clambake config set state_machine.enabled false

# Force agent to idle state
clambake agent force-reset --all
```

### Graceful Degradation
- State machine failures revert to manual state management
- Agent coordination continues without formal state validation
- Recovery operations can be disabled independently
- No data loss during rollback (GitHub state preserved)

### Recovery Path
- State machine is additive to existing workflows
- Manual state management serves as backup system
- Inconsistency detection can be re-enabled incrementally
- Recovery operations tested independently before full re-deployment

## Definition of Done

### Functional Requirements
- [ ] statig state machine handles all agent lifecycle transitions
- [ ] State validation detects all defined inconsistency patterns
- [ ] Automatic recovery resolves >90% of detected inconsistencies
- [ ] CLI commands provide clear diagnostic and recovery capabilities
- [ ] Integration with existing workflows maintains backward compatibility

### Quality Requirements
- [ ] Property tests validate state machine invariants hold
- [ ] Integration tests verify state consistency with real GitHub API
- [ ] Chaos tests demonstrate recovery from artificially induced stuck states
- [ ] Performance tests show <100ms latency for state transitions
- [ ] Manual testing confirms agent operates continuously over 24-hour period

### Documentation Requirements
- [ ] State machine architecture documentation updated
- [ ] Troubleshooting guide for stuck agent scenarios
- [ ] Recovery runbook for operational teams
- [ ] Configuration guide for state machine tuning

## Risks & Mitigation

### Technical Risks
| Risk | Impact | Probability | Mitigation |
|------|---------|-------------|------------|
| statig library limitations | Medium | Low | Evaluate library capabilities early, maintain fallback |
| State synchronization race conditions | High | Medium | Comprehensive locking strategy, transaction-like operations |
| Recovery operations corrupt agent work | High | Low | Safe recovery with work preservation, comprehensive testing |

### Operational Risks
| Risk | Impact | Probability | Mitigation |
|------|---------|-------------|------------|
| Automatic recovery creates more problems | Medium | Low | Conservative recovery actions, manual override capability |
| State machine complexity overwhelms debugging | Medium | Medium | Clear diagnostic output, detailed logging, manual overrides |

## Success Measurement

### Pre-Implementation Baseline
- Current stuck agent incidents: ~2-3 per week requiring manual intervention
- Average resolution time: 15-30 minutes manual diagnosis and fix
- State consistency issues: ~20% of agent operations have temporary inconsistencies

### Post-Implementation Targets
- Stuck agent incidents: <1 per month with automatic resolution
- Average resolution time: <2 minutes automatic recovery
- State consistency: >99% accuracy between internal state and external systems
- Operational efficiency: Zero manual intervention required for standard stuck states

This specification transforms the agent lifecycle from an ad-hoc manual system into a reliable, self-healing state machine that eliminates operational overhead while enabling autonomous single-agent operation.