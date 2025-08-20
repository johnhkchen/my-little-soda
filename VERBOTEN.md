# VERBOTEN: Things We Must Never Allow in Clambake

## Overview

Based on lessons learned from the event-api disaster (455 backup files, 90+ orphaned branches, catastrophic state corruption), this document establishes hard rules that prevent Clambake from becoming another "free for all" unstructured mess.

**Philosophy**: "You can't put dirty shoes on the couch" - some behaviors are simply not allowed, regardless of convenience or developer preference.

## 🚫 ABSOLUTE PROHIBITIONS

### 1. Dual State Stores - THE ORIGINAL SIN

**VERBOTEN**: Any system that maintains state in multiple places without atomic synchronization.

❌ **Never Allow**:
```rust
// This pattern killed event-api
struct BadCoordination {
    kanban_state: KanbanFile,     // State in YAML file
    git_state: GitWorktrees,      // State in git worktrees  
    agent_state: LocalConfig,     // State in local config
    github_state: GitHubAPI,      // State in GitHub
}

// The inevitable corruption
fn update_state() {
    kanban_state.update()?;       // Succeeds
    git_state.update()?;          // Fails - now out of sync
    // System is now corrupted
}
```

✅ **Must Use**:
```rust
// Single source of truth with atomic operations
struct ClambakeCoordination {
    github: GitHubClient,  // GitHub is the ONLY authoritative state
    local_cache: ReadOnlyCache,  // Read-only cache only
}

// All operations are atomic GitHub transactions
fn update_state() -> Result<()> {
    github.atomic_transaction(|tx| {
        tx.update_issue_assignment()?;
        tx.update_project_board()?;
        tx.create_branch()?;
        // Either all succeed or all fail
    })
}
```

### 2. Manual State Synchronization

**VERBOTEN**: Any code that manually tries to "sync" states between systems.

❌ **Never Allow**:
```rust
// This is the path to 455 backup files
fn sync_states() -> Result<()> {
    let github_state = github.get_state()?;
    let local_state = local.get_state()?;
    
    if github_state != local_state {
        // Manual reconciliation - DISASTER WAITING TO HAPPEN
        local.update_to_match(github_state)?;
    }
}
```

✅ **Must Use**:
```rust
// GitHub is authoritative, local is derived
fn get_current_state() -> Result<SystemState> {
    github.get_authoritative_state()  // Always query GitHub
}
```

### 3. Environment Variable Overrides for Critical Operations

**VERBOTEN**: Environment variables that can bypass safety mechanisms.

❌ **Never Allow**:
```bash
# This pattern caused assignment drift in event-api
WORKSPACE_DIR="$PWD" clambake cleanup  # Bypasses workspace detection
FORCE_OVERRIDE=true clambake land      # Bypasses safety checks
DEBUG_MODE=true clambake route         # Changes production behavior
```

✅ **Must Use**:
```rust
// Safety mechanisms cannot be bypassed
struct SafeOperations {
    workspace_validator: WorkspaceValidator,  // Always validates
    safety_checker: SafetyChecker,           // Always checks
}

impl SafeOperations {
    fn cleanup(&self) -> Result<()> {
        self.workspace_validator.ensure_safe_context()?;  // Cannot bypass
        self.safety_checker.verify_no_active_work()?;     // Cannot bypass
        // Only then proceed
    }
}
```

### 4. Destructive Operations Without Confirmation

**VERBOTEN**: Any operation that can lose work without explicit confirmation.

❌ **Never Allow**:
```rust
// Silent destruction - caused massive work loss in event-api
fn cleanup_agent_workspace(agent_id: &str) {
    fs::remove_dir_all(&workspace_path)?;  // WORK LOST FOREVER
    git::delete_branch(&branch_name)?;     // COMMITS LOST FOREVER
}
```

✅ **Must Use**:
```rust
// Work preservation is mandatory
fn cleanup_agent_workspace(agent_id: &str) -> Result<()> {
    let workspace = Workspace::load(agent_id)?;
    
    // MUST preserve work before any destructive operations
    let preservation_ref = workspace.create_preservation_backup()?;
    let pr_created = workspace.create_recovery_pr_if_needed()?;
    
    // Only destroy after work is safely preserved
    workspace.safe_cleanup_with_preservation(preservation_ref)
}
```

### 5. Mutable Global State

**VERBOTEN**: Global mutable state that can be modified from multiple places.

❌ **Never Allow**:
```rust
// Global state - coordination nightmare
static mut AGENT_ASSIGNMENTS: HashMap<AgentId, TicketId> = HashMap::new();
static mut SYSTEM_STATE: SystemState = SystemState::new();

// Multiple places modifying global state
fn route_ticket() {
    unsafe { AGENT_ASSIGNMENTS.insert(agent, ticket); }  // RACE CONDITIONS
}
```

✅ **Must Use**:
```rust
// Immutable state with controlled mutation
#[derive(Clone)]
struct ImmutableState {
    assignments: Arc<HashMap<AgentId, TicketId>>,
}

// State changes go through controlled channels
struct StateManager {
    state: RwLock<ImmutableState>,
    github_client: GitHubClient,
}

impl StateManager {
    fn update_assignment(&self, agent: AgentId, ticket: TicketId) -> Result<()> {
        // Atomic update through GitHub
        self.github_client.assign_issue(ticket, agent)?;
        
        // Local state is derived from GitHub
        let new_state = self.derive_from_github()?;
        *self.state.write()? = new_state;
        
        Ok(())
    }
}
```

### 6. Custom Configuration Files

**VERBOTEN**: Any custom configuration format that duplicates GitHub's native capabilities.

❌ **Never Allow**:
```yaml
# Custom kanban.yaml - THE DEVIL ITSELF
agents:
  agent-001:
    status: "working"
    assigned_ticket: 123
    branch: "feature/auth"
  agent-002:
    status: "available"

tickets:
  - id: 123
    status: "in_progress"
    assigned_to: "agent-001"
```

✅ **Must Use**:
```rust
// GitHub Projects V2 is the only configuration
struct GitHubNativeConfig {
    project_id: u64,
    issue_labels: Vec<String>,
    agent_users: Vec<String>,
}

// Configuration comes from GitHub setup, not files
fn get_configuration() -> Result<Configuration> {
    let project = github.get_project_v2(project_id)?;
    let labels = github.get_repository_labels()?;
    Configuration::from_github_native(project, labels)
}
```

### 7. Silent Failures

**VERBOTEN**: Operations that fail silently and continue as if they succeeded.

❌ **Never Allow**:
```rust
// Silent failure - creates false success scenarios
fn assign_ticket(ticket_id: u64, agent_id: &str) -> bool {
    match github.assign_issue(ticket_id, agent_id) {
        Ok(_) => true,
        Err(_) => {
            // SILENT FAILURE - looks like success but isn't
            eprintln!("Assignment failed, continuing anyway...");
            true  // LIE - this will cause coordination drift
        }
    }
}
```

✅ **Must Use**:
```rust
// All failures are explicit and propagated
fn assign_ticket(ticket_id: u64, agent_id: &str) -> Result<Assignment> {
    let assignment = github.assign_issue(ticket_id, agent_id)
        .context("Failed to assign ticket to agent")?;
    
    // Return successful assignment or error - no middle ground
    Ok(assignment)
}
```

### 8. Ad-Hoc Error Recovery

**VERBOTEN**: Custom error recovery logic that differs per operation.

❌ **Never Allow**:
```rust
// Different recovery strategies everywhere - chaos
fn route_tickets() {
    match assign_ticket(123, "agent-001") {
        Err(_) => {
            // Custom recovery logic #1
            retry_with_different_agent()?;
        }
    }
}

fn land_work() {
    match create_pr("agent-001") {
        Err(_) => {
            // Different custom recovery logic #2  
            create_manual_merge_request()?;
        }
    }
}
```

✅ **Must Use**:
```rust
// Consistent recovery patterns through type system
trait RecoverableOperation {
    type Output;
    type Error;
    
    fn execute(&self) -> Result<Self::Output, Self::Error>;
    fn recover(&self, error: Self::Error) -> RecoveryAction;
}

// Same recovery logic for all operations
fn execute_with_recovery<T: RecoverableOperation>(op: T) -> Result<T::Output> {
    match op.execute() {
        Ok(result) => Ok(result),
        Err(error) => {
            let recovery = op.recover(error);
            recovery.execute() // Consistent recovery patterns
        }
    }
}
```

## 🚨 COORDINATION ANTI-PATTERNS

### 9. Race Condition Patterns

**VERBOTEN**: Code patterns that allow race conditions in multi-agent scenarios.

❌ **Never Allow**:
```rust
// Check-then-act pattern - RACE CONDITION GUARANTEED
fn assign_if_available(ticket_id: u64, agent_id: &str) -> Result<()> {
    if is_agent_available(agent_id)? {  // Check
        // Another thread can assign work here!
        assign_ticket(ticket_id, agent_id)?;  // Act - TOO LATE
    }
}
```

✅ **Must Use**:
```rust
// Atomic compare-and-swap operations
fn assign_if_available(ticket_id: u64, agent_id: &str) -> Result<Assignment> {
    github.atomic_assignment(ticket_id, agent_id, |current_state| {
        if current_state.agent_available(agent_id) {
            Some(Assignment::new(ticket_id, agent_id))  // Atomic
        } else {
            None  // Agent not available
        }
    })
}
```

### 10. Cross-Agent File Conflicts

**VERBOTEN**: Allowing multiple agents to modify the same files simultaneously.

❌ **Never Allow**:
```rust
// Multiple agents editing same file - MERGE HELL
fn assign_tickets() {
    assign_ticket(101, "agent-001");  // Both agents might edit
    assign_ticket(102, "agent-002");  // the same source files
}
```

✅ **Must Use**:
```rust
// File-level conflict detection before assignment
fn assign_with_conflict_detection(ticket_id: u64, agent_id: &str) -> Result<()> {
    let ticket = github.get_issue(ticket_id)?;
    let affected_files = analyze_likely_file_changes(&ticket)?;
    
    // Check for conflicts with existing assignments
    let conflicts = detect_file_conflicts(&affected_files)?;
    if !conflicts.is_empty() {
        return Err(CoordinationError::FileConflicts(conflicts));
    }
    
    assign_ticket(ticket_id, agent_id)
}
```

## 📁 FILE SYSTEM ANTI-PATTERNS

### 11. Shared Workspace Directories

**VERBOTEN**: Agents sharing workspace directories or modifying shared files.

❌ **Never Allow**:
```bash
# Shared workspace - FILE CORRUPTION GUARANTEED
workspace/
├── shared/          # Multiple agents modify this - BAD
├── agent-001/       # Agent 1 might write here
└── agent-002/       # Agent 2 might write here too
```

✅ **Must Use**:
```bash
# Isolated git worktrees - NO SHARED FILE ACCESS
worktrees/
├── agent-001/       # Completely isolated filesystem
│   └── .git -> main-repo/.git/worktrees/agent-001
├── agent-002/       # Completely isolated filesystem  
│   └── .git -> main-repo/.git/worktrees/agent-002
└── main/            # Main worktree - read-only for agents
```

### 12. Manual Git Operations

**VERBOTEN**: Manual git commands that bypass Clambake's coordination.

❌ **Never Allow**:
```bash
# Manual git operations - BYPASSES COORDINATION
cd worktrees/agent-001
git checkout main           # Might conflict with assignments
git merge agent-002/feature  # Bypasses conflict detection
git push origin main        # Bypasses integration pipeline
```

✅ **Must Use**:
```rust
// All git operations go through Clambake coordination
fn merge_agent_work(agent_id: &str) -> Result<()> {
    let coordination_lock = acquire_coordination_lock()?;
    let integration_plan = create_integration_plan(agent_id)?;
    
    github.create_pr_with_coordination_metadata(integration_plan)?;
    // Manual git operations not allowed
}
```

## 🔧 DEVELOPMENT PROCESS PROHIBITIONS

### 13. Skipping Tests for "Quick Fixes"

**VERBOTEN**: Any code that doesn't have corresponding test coverage.

❌ **Never Allow**:
```rust
// "Quick fix" without tests - TECHNICAL DEBT ACCUMULATION
fn urgent_hotfix() {
    // TODO: Add tests later (never happens)
    // Direct implementation without test coverage
}
```

✅ **Must Use**:
```rust
// Test-first, always
#[cfg(test)]
mod tests {
    scenario! {
        name: "Hotfix behavior",
        given: { urgent_situation: true },
        when: { apply_hotfix() },
        then: { problem_solved!(), no_side_effects!() },
    }
}

fn urgent_hotfix() -> Result<()> {
    // Implementation driven by failing test
    // No exceptions, even for "urgent" fixes
}
```

### 14. Direct Database/API Mutations

**VERBOTEN**: Bypassing the coordination layer to directly modify GitHub state.

❌ **Never Allow**:
```rust
// Direct API calls bypass coordination
fn emergency_fix() {
    // BYPASS ALL SAFETY - COORDINATION CORRUPTION
    octocrab_client.issues().update(123, UpdateIssue {
        assignee: Some("agent-001".to_string()),
        ..Default::default()
    }).await?;
}
```

✅ **Must Use**:
```rust
// All GitHub changes go through coordination layer
fn emergency_fix() -> Result<()> {
    coordination_manager.emergency_assign(
        ticket_id: 123,
        agent_id: "agent-001",
        reason: EmergencyReason::CriticalBug,
        override_safety: false,  // Even emergencies follow safety rules
    )
}
```

### 15. Configuration Through Environment Variables

**VERBOTEN**: Runtime behavior changes through environment variables.

❌ **Never Allow**:
```rust
// Environment-dependent behavior - TESTING NIGHTMARE
fn get_coordination_strategy() -> Strategy {
    match env::var("COORDINATION_MODE") {
        Ok("aggressive") => Strategy::Aggressive,  // Different behavior per env
        Ok("conservative") => Strategy::Conservative,
        _ => Strategy::Default,
    }
}
```

✅ **Must Use**:
```rust
// Explicit configuration only
#[derive(Debug, Clone)]
struct ClambakeConfig {
    coordination_strategy: CoordinationStrategy,
    max_concurrent_agents: u32,
    github_config: GitHubConfig,
}

impl ClambakeConfig {
    fn from_file(path: &Path) -> Result<Self> {
        // Explicit configuration file, not environment variables
    }
}
```

## 🏭 PRODUCTION SAFETY RULES

### 16. Debug Code in Production

**VERBOTEN**: Debug prints, panics, or experimental code in production builds.

❌ **Never Allow**:
```rust
// Debug code that can leak to production
fn assign_ticket() -> Result<()> {
    println!("DEBUG: Assigning ticket..."); // INFORMATION LEAK
    
    if cfg!(debug_assertions) {
        panic!("This shouldn't happen!");  // PRODUCTION CRASH
    }
    
    // Experimental code
    #[cfg(feature = "experimental")]
    try_new_algorithm()?;  // UNSTABLE IN PRODUCTION
}
```

✅ **Must Use**:
```rust
// Production-safe logging and error handling
fn assign_ticket() -> Result<()> {
    tracing::info!("Starting ticket assignment");  // Structured logging
    
    assignment_operation()
        .context("Failed to assign ticket")  // Proper error context
        .map_err(|e| {
            tracing::error!("Assignment failed: {:?}", e);
            e
        })
}
```

### 17. Hardcoded Values

**VERBOTEN**: Magic numbers or hardcoded configurations that can't be tested.

❌ **Never Allow**:
```rust
// Magic numbers - IMPOSSIBLE TO TEST VARIATIONS
fn is_agent_overloaded(agent: &Agent) -> bool {
    agent.current_assignments.len() > 5  // Why 5? What if we need 3 or 8?
}

fn retry_operation() {
    thread::sleep(Duration::from_secs(30));  // Hardcoded retry timing
}
```

✅ **Must Use**:
```rust
// Configurable values that can be tested
#[derive(Debug, Clone)]
struct AgentLimits {
    max_concurrent_assignments: usize,
    retry_delay: Duration,
    max_retries: u32,
}

fn is_agent_overloaded(agent: &Agent, limits: &AgentLimits) -> bool {
    agent.current_assignments.len() > limits.max_concurrent_assignments
}
```

## 💀 THE ULTIMATE VERBOTEN: Event-API Patterns

### 18. Anything That Resembles kanban.yaml

**VERBOTEN**: Any attempt to recreate the kanban.yaml disaster.

❌ **NEVER, EVER, UNDER ANY CIRCUMSTANCES**:
- Custom YAML/JSON/TOML files for state management
- Manual state synchronization between files and APIs
- Backup file generation (if you need backups, your state management is broken)
- Environment variable overrides for safety mechanisms
- Manual conflict resolution that developers have to remember

✅ **THE CLAMBAKE WAY**:
- GitHub is the single source of truth
- All operations are atomic and transactional
- State corruption is impossible by design
- Recovery is automatic and tested
- Developer experience is safe by default

## 🎯 ENFORCEMENT MECHANISMS

### Code Review Checklist

Every PR must answer "NO" to these questions:
- [ ] Does this create dual state stores?
- [ ] Does this allow manual state synchronization?
- [ ] Does this create race conditions?
- [ ] Does this allow silent failures?
- [ ] Does this bypass safety mechanisms?
- [ ] Does this use environment variables for behavior changes?
- [ ] Does this lack test coverage?
- [ ] Does this create shared mutable state?

### Automated Enforcement

```rust
// Compile-time enforcement where possible
#[deny(unsafe_code)]           // No unsafe code
#[deny(dead_code)]            // No unused code
#[deny(missing_docs)]         // All public APIs documented

// Custom lints for Clambake-specific anti-patterns
#[forbid(dual_state_stores)]   // Custom lint: no dual state
#[forbid(manual_sync)]         // Custom lint: no manual sync
#[forbid(environment_overrides)] // Custom lint: no env overrides
```

### Runtime Enforcement

```rust
// Runtime assertions that cannot be disabled
fn critical_operation() -> Result<()> {
    debug_assert!(invariant_holds(), "Invariant violated");
    
    // Even in release builds, certain checks are mandatory
    if !safety_check_passes() {
        return Err(SafetyViolation::CriticalInvariantViolated);
    }
}
```

## 🎪 THE LESSON OF EVENT-API

The event-api project showed us what happens when "anything goes":
- **455 backup files** from constant state corruption
- **90+ orphaned branches** from broken integration
- **Coordination failures** from dual state management
- **Work loss** from unsafe cleanup operations
- **Developer frustration** from unreliable tooling

**Clambake exists to prove that multi-agent coordination can be reliable, predictable, and safe.** These rules ensure we never repeat those mistakes.

**Remember**: Every "verboten" rule exists because someone got burned by ignoring it. Following these rules isn't about being restrictive - it's about building a system that developers can trust with their most important work.