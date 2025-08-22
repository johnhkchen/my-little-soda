# Issue #131: Implement Functional Branch Bundling System

> **Transform existing bundling prototype into a production-ready system that achieves 10-minute release cadence**

## Problem Statement

### Current State
- **Incomplete Implementation**: Bundling logic exists in `/src/bundling/bundler.rs` but lacks integration with the main workflow
- **Critical Bug**: Train schedule detection works (`TrainSchedule::is_departure_time()`) but never triggers bundling action
- **Manual Intervention Required**: Agents get "stuck" at capacity when work is ready to bundle but system doesn't act
- **Prototype-Level Robustness**: Cherry-picking logic exists but conflict resolution falls back to individual PRs without optimization

### Context  
- System currently processes individual PRs which creates GitHub API bottlenecks and review fatigue
- 10-minute release cadence is a core architectural requirement for agent throughput
- Agent capacity management depends on reliable bundling to free agents after landing work
- This blocks Architecture v2 progress as bundling is foundation for multi-agent coordination

### Strategic Value
A functional bundling system directly enables:
- **5x throughput increase**: From individual PRs to bundled releases every 10 minutes
- **Agent efficiency**: Prevents agents from getting stuck waiting for manual PR creation
- **Review efficiency**: Consolidates related changes into coherent review units
- **API rate limit management**: Reduces GitHub API calls by 80% through bundling

## Target State

### Vision
A fully automated bundling system that:
1. **Monitors continuously** for work ready to bundle (`route:review` labeled issues)
2. **Executes automatically** every 10 minutes during departure windows
3. **Handles conflicts intelligently** with fallback strategies that optimize reviewer time
4. **Integrates seamlessly** with existing agent lifecycle without breaking current workflows

### Success Metrics
- **Bundling Success Rate**: >90% of eligible work successfully bundled (not falling back to individual PRs)
- **Throughput Increase**: 5x increase in issues processed per hour
- **Agent Utilization**: <5% of time agents spend waiting for bundle creation
- **Review Efficiency**: Average PR size increases from 1 issue to 3-5 issues
- **System Reliability**: Zero instances of agents getting permanently stuck due to bundling failures

### Non-Goals
- Advanced merge conflict resolution (AI-powered conflict resolution, complex rebasing strategies)
- Custom bundling schedules per repository or team
- Bundling across multiple repositories
- Integration with external CI systems beyond GitHub Actions

## Interfaces & Contracts

### Core Bundling Interface
```rust
pub trait BundlingService {
    /// Check if bundling should occur based on schedule and available work
    async fn should_bundle(&self) -> Result<bool>;
    
    /// Execute bundling for all eligible work
    async fn execute_bundling(&self) -> Result<BundlingResult>;
    
    /// Get current bundling status and metrics
    fn get_status(&self) -> BundlingStatus;
}

pub enum BundlingResult {
    Success { bundle_pr: u64, issues_count: usize },
    PartialSuccess { bundle_pr: u64, bundled: usize, individual_prs: HashMap<String, u64> },
    AllIndividual { individual_prs: HashMap<String, u64> },
    NoWorkAvailable,
    Failed { error: String }
}
```

### Command-Line Interface
```bash
# Existing command behavior (no changes)
clambake land  # Still lands work and adds route:review labels

# Enhanced bundling command  
clambake bundle --dry-run     # Show what would be bundled
clambake bundle --force       # Force bundling outside schedule
clambake bundle --status      # Show bundling system status

# New diagnostic command
clambake bundle --diagnose    # Show why bundling isn't working
```

### Configuration Integration
```toml
# clambake.toml additions
[bundling]
enabled = true
schedule_minutes = 10
max_bundle_size = 8
conflict_strategy = "individual_fallback"  # or "skip_conflicts"
```

## Data Model & Storage

### State Management
The bundling system operates statelessly using GitHub as the source of truth:
- **Input State**: Issues with `route:review` labels
- **Process State**: Temporary bundle branches during creation
- **Output State**: Bundle PRs created, original issues updated with bundle references
- **Failure State**: Individual PRs created with clear attribution to bundling system

### No New Storage Required
Leverages existing GitHub state management patterns:
- Uses GitHub labels for work state tracking
- Uses Git branches for atomic bundling operations
- Uses PR descriptions for bundling metadata and traceability

## Technical Implementation Strategy

### Phase 1: Integration Repair (2 hours)
**Fix the critical bug preventing bundling execution**

1. **Fix Missing Bundling Call** (30 minutes)
   - Locate `TrainSchedule::is_departure_time()` check in main workflow
   - Add missing `bundle_all_branches().await` call
   - Add proper error handling for bundling failures

2. **Add Bundling to Main Workflow** (90 minutes)  
   - Integrate bundling check into `clambake land` command flow
   - Add bundling trigger to scheduled operations
   - Ensure bundling doesn't break existing workflows

### Phase 2: Production Hardening (3 hours)
**Make bundling robust and reliable for production use**

1. **Conflict Resolution Enhancement** (90 minutes)
   - Improve conflict detection before cherry-picking attempts
   - Add pre-flight checks for merge compatibility
   - Optimize individual PR fallback with better PR descriptions

2. **Error Recovery and Diagnostics** (90 minutes)
   - Add comprehensive error handling for Git operations
   - Implement bundling status reporting and diagnostics
   - Add automatic recovery from partially-failed bundling operations

### Phase 3: User Experience Polish (1 hour)
**Ensure clear feedback and operational visibility**

1. **Enhanced CLI Feedback** (30 minutes)
   - Improve bundling progress reporting
   - Add clear success/failure messaging
   - Show bundling impact on agent capacity

2. **Diagnostic Tooling** (30 minutes)
   - Add `clambake bundle --diagnose` command
   - Show why bundling isn't working when expected
   - Display bundling schedule and next departure time

## Implementation Tasks (1-Hour Chunks)

### Task 1: Fix Core Bundling Bug (1 hour)
- **Objective**: Repair the missing bundling execution call
- **Acceptance**: Train schedule detection triggers actual bundling
- **Files**: Main workflow, train schedule integration
- **Test**: Manual verification that bundling executes on schedule

### Task 2: Integrate Bundling into Agent Workflow (1 hour)  
- **Objective**: Make bundling part of the standard agent lifecycle
- **Acceptance**: `clambake land` triggers bundling checks appropriately
- **Files**: Agent lifecycle, command handlers
- **Test**: End-to-end agent workflow includes bundling

### Task 3: Enhance Conflict Resolution (1 hour)
- **Objective**: Improve bundling success rate through better conflict handling
- **Acceptance**: >90% bundling success rate in test scenarios
- **Files**: Git operations, cherry-picking logic
- **Test**: Property tests for various conflict scenarios

### Task 4: Add Production Error Handling (1 hour)
- **Objective**: Make bundling robust against Git and GitHub failures
- **Acceptance**: System recovers gracefully from all identified failure modes
- **Files**: Error handling, recovery logic
- **Test**: Chaos testing for failure injection

### Task 5: Implement Diagnostic Tooling (1 hour)
- **Objective**: Add observability for bundling system operation
- **Acceptance**: Clear feedback on bundling status and issues
- **Files**: CLI commands, status reporting
- **Test**: Manual verification of diagnostic output

### Task 6: End-to-End Integration Testing (1 hour)
- **Objective**: Validate complete bundling workflow works reliably
- **Acceptance**: Multi-agent scenario successfully bundles work
- **Files**: Integration tests, test scenarios
- **Test**: Full workflow validation with multiple agents

## Rollback Strategy

### Immediate Rollback (< 5 minutes)
```bash
# Disable bundling via configuration
sed -i 's/bundling.enabled = true/bundling.enabled = false/' clambake.toml
```

### Full Rollback (< 30 minutes)  
- Revert to individual PR creation for all `route:review` work
- Maintain agent lifecycle integrity (agents still freed after landing)
- Manual cleanup of any partial bundle branches

### Recovery Strategy
- Bundling failures never break agent lifecycle
- Individual PR fallback ensures work continues flowing
- System designed to be "bundling optional" for reliability

## Definition of Done

### Functional Requirements
- [ ] Train schedule correctly triggers bundling execution
- [ ] Bundling succeeds for >90% of eligible work
- [ ] Conflict fallback creates individual PRs with clear attribution
- [ ] Agent capacity management works correctly with bundling
- [ ] All existing agent workflows remain unchanged

### Quality Requirements  
- [ ] Property tests validate bundling invariants
- [ ] Integration tests cover multi-agent bundling scenarios
- [ ] Chaos tests demonstrate failure recovery
- [ ] Performance tests show API rate limit improvements
- [ ] Manual testing confirms 10-minute release cadence

### Documentation Requirements
- [ ] Update agent lifecycle documentation with bundling flow
- [ ] Add bundling troubleshooting guide
- [ ] Document configuration options and tuning guidance
- [ ] Update deployment runbooks with bundling considerations

## Risks & Mitigation

### Technical Risks
| Risk | Impact | Probability | Mitigation |
|------|---------|-------------|------------|
| Git conflicts break bundling | Medium | High | Robust individual PR fallback strategy |
| GitHub API rate limits during bundling | High | Medium | Exponential backoff and bundling batching |
| Bundling creates review bottlenecks | Medium | Low | Bundle size limits and conflict avoidance |

### Business Risks  
| Risk | Impact | Probability | Mitigation |
|------|---------|-------------|------------|
| Reduced code review quality | High | Low | Maintain individual issue traceability in bundles |
| Agent workflow disruption | High | Low | Comprehensive testing and gradual rollout |

## Success Measurement

### Pre-Implementation Baseline
- Current throughput: ~12 issues/hour (individual PRs)  
- Current API usage: ~50 calls per agent workflow
- Current agent stuck time: ~15% due to manual bundling

### Post-Implementation Targets
- Target throughput: >60 issues/hour (5x improvement)
- Target API usage: <15 calls per bundled workflow (70% reduction)
- Target agent stuck time: <2% (automated bundling)

This specification transforms the existing bundling prototype into a production-ready system that achieves the 10-minute release cadence essential for Clambake's multi-agent orchestration value proposition.