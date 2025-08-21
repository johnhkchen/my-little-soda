# Agent Lifecycle Documentation

## Overview

The Clambake multi-agent orchestration system manages agents through a carefully orchestrated lifecycle designed to maximize throughput while maintaining code quality. The key insight is that **agents are freed immediately after landing their work**, not after merge completion, enabling a 10-minute release cadence.

## Core Principles

1. **Immediate Agent Freedom**: Agents are freed as soon as they land their work, not when it merges
2. **10-minute Release Cadence**: Bundling happens every 10 minutes (unless unblockers override)
3. **State-driven Workflow**: Clear states with explicit transitions prevent agents getting "stuck"
4. **Failure Recovery**: System detects and recovers from common failure modes
5. **Dependency Injection**: All operations are testable and can emit commands vs execute them

## Agent States

### 1. IDLE
- **Definition**: Agent has no active assignment
- **GitHub State**: No agent label on any issue
- **Git State**: Any branch (typically main)
- **Capacity**: Available for new work

### 2. ASSIGNED
- **Definition**: Agent has been assigned to an issue but hasn't committed work
- **GitHub State**: `agent001` label on issue, issue has `route:ready` label
- **Git State**: On `agent001/issue_number` branch
- **Capacity**: Busy (1/1)

### 3. WORKING
- **Definition**: Agent is actively working (has made commits)
- **GitHub State**: Same as ASSIGNED
- **Git State**: On agent branch with unpushed commits
- **Capacity**: Busy (1/1)

### 4. LANDED
- **Definition**: Agent has completed work and landed it (freed immediately)
- **GitHub State**: `route:review` label added, `agent001` label removed, `route:ready` removed
- **Git State**: Agent branch pushed to origin, agent back on main
- **Capacity**: Available (0/1) - **Agent is freed at this point**

### 5. BUNDLED
- **Definition**: Work has been included in a bundle PR
- **GitHub State**: Part of bundle PR, original issue references bundle
- **Git State**: Changes cherry-picked into bundle branch
- **Capacity**: N/A (agent already freed)

### 6. MERGED
- **Definition**: Work has been integrated into main branch
- **GitHub State**: Issue closed, PR merged
- **Git State**: Changes in main branch
- **Capacity**: N/A (agent already freed)

## State Transitions

### 1. Assignment: IDLE → ASSIGNED
- **Trigger**: `clambake pop`
- **Preconditions**: Agent capacity available, work available
- **Operations**:
  - Add `agent001` label to issue
  - Create/checkout `agent001/issue_number` branch
  - Update agent capacity to 1/1
- **Failure Modes**: 
  - Issue already assigned → Skip and find next
  - Branch creation fails → Report error, stay IDLE
  - GitHub API failure → Retry with backoff

### 2. Work Completion: ASSIGNED/WORKING → LANDED
- **Trigger**: `clambake land`
- **Preconditions**: Agent on correct branch, work exists
- **Operations**:
  - **Pre-flight Checklist** (detect and fix issues):
    - Check if commits exist → Warn if none
    - Check if commits pushed → Auto-push if needed
    - Check for merge conflicts → Report and guide resolution
    - Validate branch is ahead of main → Warn if behind
  - Add `route:review` label to issue
  - Remove `agent001` label from issue (**FREES AGENT**)
  - Remove `route:ready` label from issue  
  - Switch agent back to main branch
  - Update agent capacity to 0/1
- **Failure Modes**:
  - No commits → Guide agent to commit or abandon
  - Push fails → Auto-retry, then guide manual resolution
  - Label operations fail → Log warning, continue (agent still functionally freed)
  - Branch switch fails → Warn but don't block (agent still freed)

### 3. Bundle Creation: LANDED → BUNDLED  
- **Trigger**: 10-minute schedule OR unblocker priority
- **Preconditions**: Issues with `route:review` label exist
- **Operations**:
  - Collect all `route:review` branches
  - Create bundle branch: `bundle/issue1-issue2-issue3`
  - Cherry-pick commits from each branch
  - Create bundle PR with aggregated changes
  - Update original issues to reference bundle PR
  - Remove `route:review` labels
- **Failure Modes**:
  - Cherry-pick conflicts → Create individual PRs instead
  - Bundle branch exists → Increment counter and retry
  - GitHub API limits → Wait and retry with exponential backoff

### 4. Integration: BUNDLED → MERGED
- **Trigger**: Bundle PR approval + CI success
- **Preconditions**: All checks pass, PR approved
- **Operations**:
  - Merge bundle PR to main
  - Close referenced issues
  - Delete agent branches
  - Delete bundle branch
- **Failure Modes**:
  - Merge conflicts → Manual intervention required
  - CI failures → Block merge until fixed
  - Permission issues → Escalate to admin

## Special Cases and Recovery

### Unblocker Priority
- **Override**: Bypasses 10-minute bundling schedule
- **Processing**: Creates immediate individual PR instead of waiting for bundle
- **Agent Impact**: No change - agent was already freed at LANDED state

### Abandoned Work Recovery
- **Detection**: Agent branch exists but no commits for >24 hours
- **Action**: Remove `agent001` label, delete branch, free agent
- **Notification**: Create issue for manual review of abandonment

### Stuck Agent Recovery  
- **Detection**: Agent labeled but no corresponding branch, or vice versa
- **Action**: Sync GitHub labels with Git branches, prioritize freeing agent
- **Manual Override**: `clambake land --force-free agent001`

### Pre-flight Checklist Details

The `clambake land` command acts as an inflight checklist assistant:

1. **Commit Check**: 
   ```bash
   git log main..HEAD --oneline
   # If empty: "No commits detected. Did you forget to commit your work?"
   ```

2. **Push Check**:
   ```bash
   git rev-list HEAD...origin/$(git branch --show-current) --count
   # If >0: "Unpushed commits detected. Auto-pushing..."
   ```

3. **Sync Check**:
   ```bash
   git rev-list HEAD..main --count  
   # If >0: "Branch is behind main. Consider rebasing."
   ```

4. **Conflict Check**:
   ```bash
   git merge-tree $(git merge-base HEAD main) HEAD main
   # If conflicts: "Merge conflicts detected. Please resolve before landing."
   ```

## Testing Requirements

### Unit Tests Needed
- State transition validation for all paths
- Failure mode handling for each operation  
- Pre-flight checklist logic
- Label synchronization logic
- Bundle creation algorithms

### Integration Tests Needed
- End-to-end agent lifecycle workflows
- Multi-agent coordination scenarios
- GitHub API failure recovery
- Git operation failure recovery
- Schedule-based bundling triggers

### Property Tests Needed
- Agent capacity never exceeds limits
- No agent gets permanently stuck
- All work eventually reaches MERGED state
- System maintains consistency under failures

## Implementation Strategy

### Phase 1: Dependency Injection
- Extract all GitHub API calls into testable service
- Extract all Git operations into testable service  
- Create command emitter vs executor modes
- Build comprehensive test harness

### Phase 2: State Machine
- Implement explicit state tracking
- Add state transition validation
- Create recovery operations for each failure mode
- Add comprehensive logging for state changes

### Phase 3: Pre-flight Checklist
- Implement all detection logic
- Add auto-fix capabilities where safe
- Create clear guidance for manual resolution
- Test all failure scenarios

### Phase 4: Integration
- Connect new state machine to existing commands
- Migrate existing workflows
- Add monitoring and alerting
- Performance optimization

This lifecycle design ensures agents never get permanently stuck while maintaining rapid release cadence and code quality.