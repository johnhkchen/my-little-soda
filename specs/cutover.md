# Cutover & Feature Flags Specification

> **Safe migration strategy with gradual rollout and instant rollback capability.**

## Overview

This specification defines the safe migration strategy for transitioning from the current Clambake system to the new architecture. The approach uses feature flags to enable gradual rollout, A/B testing, and instant rollback while maintaining zero-downtime operation.

## Migration Strategy

### Core Principles

1. **Zero Downtime**: System remains operational throughout entire migration
2. **Gradual Rollout**: Each component can be migrated independently
3. **Instant Rollback**: Any component can be reverted within 2 minutes
4. **Data Preservation**: No data loss during any phase of migration
5. **Backward Compatibility**: Database schema supports both old and new systems

## Feature Flags

### Environment Variables

All feature flags are controlled via environment variables for easy deployment control:

| Flag | Purpose | Default | Phases |
|------|---------|---------|--------|
| `CLAMBAKE_NEW_PLANNER` | Enable new planning system | `false` | Phase 1 |
| `CLAMBAKE_BUNDLER_V2` | Enable new bundling system | `false` | Phase 2 |
| `CLAMBAKE_NEW_EXECUTOR` | Enable new execution system | `false` | Phase 2 |
| `CLAMBAKE_READONLY` | Safe testing mode (no GitHub writes) | `false` | All phases |
| `CLAMBAKE_MIGRATION_PHASE` | Track current migration phase | `"legacy"` | All phases |

### Feature Flag Behavior

#### CLAMBAKE_NEW_PLANNER
- `true`: Routes planning through new WorkPlanner system
- `false`: Uses legacy planning algorithms
- **Rollback**: Set to `false` and restart service (< 30 seconds)

#### CLAMBAKE_BUNDLER_V2  
- `true`: Uses new bundle-aware conflict detection
- `false`: Uses legacy per-issue conflict detection
- **Rollback**: Set to `false` and restart service (< 30 seconds)

#### CLAMBAKE_NEW_EXECUTOR
- `true`: Uses new execution engine with enhanced observability
- `false`: Uses legacy execution paths
- **Rollback**: Set to `false` and restart service (< 30 seconds)

#### CLAMBAKE_READONLY
- `true`: All GitHub write operations are simulated (logs only)
- `false`: Normal GitHub API operations
- **Use Case**: Safe testing of new systems without side effects

#### CLAMBAKE_MIGRATION_PHASE
- `"legacy"`: All systems using old code paths
- `"phase1"`: New planner enabled, bundler/executor legacy
- `"phase2"`: New planner + bundler enabled, executor legacy  
- `"phase3"`: All new systems enabled
- `"cleanup"`: Legacy code removal in progress

## Migration Phases

### Phase 1: New Planner (Week 1)

**Goal**: Migrate work planning to new architecture

#### Preparation
1. Deploy code with `CLAMBAKE_NEW_PLANNER=false`
2. Verify all existing functionality works
3. Set up monitoring for planner performance metrics

#### Rollout Steps
1. **Day 1**: Enable read-only testing
   ```bash
   CLAMBAKE_READONLY=1 CLAMBAKE_NEW_PLANNER=true
   ```
   - Compare old vs new planner outputs
   - Verify no functional differences
   - Collect performance benchmarks

2. **Day 2**: Gradual traffic shift
   - 10% traffic: `CLAMBAKE_NEW_PLANNER=true` on 1 of 10 instances
   - Monitor error rates, latency, work assignment quality
   - Rollback threshold: >2% error rate increase

3. **Day 4**: Majority traffic
   - 75% traffic: `CLAMBAKE_NEW_PLANNER=true` on 7 of 10 instances
   - Monitor for edge cases, capacity issues
   - Validate work distribution fairness

4. **Day 7**: Full migration
   - 100% traffic: `CLAMBAKE_NEW_PLANNER=true` on all instances
   - Update default configuration
   - Schedule legacy code removal for Phase 3

#### Success Criteria
- [ ] Planning latency < 2 seconds (same as legacy)
- [ ] Zero work assignment conflicts
- [ ] Agent workload distribution within 10% of optimal
- [ ] No increase in failed work assignments

### Phase 2: Bundler v2 + New Executor (Week 2)

**Goal**: Migrate bundling and execution to new architecture

#### Preparation  
1. Ensure Phase 1 is stable for 48+ hours
2. Deploy code with new bundler/executor disabled
3. Set up bundle success rate monitoring

#### Rollout Steps
1. **Day 8**: Parallel bundling validation
   ```bash
   CLAMBAKE_BUNDLER_V2=true CLAMBAKE_READONLY=1
   ```
   - Run both bundlers in parallel
   - Compare bundle compositions and conflict detection
   - Verify bundle success rates match

2. **Day 10**: Staged bundler rollout
   - Staging: `CLAMBAKE_BUNDLER_V2=true`
   - Production: 25% traffic with new bundler
   - Monitor bundle conflicts, merge success rates

3. **Day 12**: Executor migration
   - Enable `CLAMBAKE_NEW_EXECUTOR=true` alongside bundler
   - Monitor execution latency and failure rates
   - Validate enhanced observability data

4. **Day 14**: Full Phase 2 migration
   - 100% traffic: Both `CLAMBAKE_BUNDLER_V2=true` and `CLAMBAKE_NEW_EXECUTOR=true`
   - Update default configuration
   - Prepare for legacy cleanup

#### Success Criteria
- [ ] Bundle success rate ≥ 95% (same as legacy)
- [ ] Execution latency < 5 minutes (same as legacy)
- [ ] Zero data corruption in work state
- [ ] Enhanced observability data available

### Phase 3: Legacy Cleanup (Week 3)

**Goal**: Remove old code paths and clean up technical debt

#### Cleanup Steps
1. **Day 15-16**: Code removal
   - Remove legacy planner code
   - Remove legacy bundler code  
   - Remove legacy executor code
   - Update configuration defaults

2. **Day 17-18**: Database cleanup
   - Archive old state tables
   - Remove deprecated columns
   - Validate data migration completeness

3. **Day 19-21**: Documentation and monitoring
   - Update operational runbooks
   - Remove legacy feature flags
   - Clean up monitoring dashboards
   - Update API documentation

#### Success Criteria
- [ ] All legacy code paths removed
- [ ] Database schema cleaned up
- [ ] Documentation updated
- [ ] Monitoring aligned with new architecture

## Validation Steps

### Pre-Migration Validation
1. **Load Testing**: New system handles peak load (8-12 concurrent agents)
2. **Data Consistency**: Dual-write validation ensures no data divergence
3. **Performance**: New system meets all performance targets
4. **Compatibility**: Database schema supports both systems

### Per-Phase Validation
1. **Functional Tests**: All user workflows continue working
2. **Performance Tests**: Latency and throughput within SLA
3. **Data Integrity**: Work state remains consistent
4. **Observability**: Metrics and logs available for troubleshooting

### Post-Migration Validation
1. **End-to-End Tests**: Complete workflows from issue to merge
2. **Load Tests**: System stable under production load
3. **Chaos Tests**: Graceful degradation under failure conditions
4. **Data Audit**: Historical data integrity maintained

## Rollback Strategy

### Instant Rollback (< 2 minutes)

#### Automatic Triggers
- Error rate increase > 5%
- Latency increase > 50%
- Data corruption detected
- Service availability < 99%

#### Manual Rollback Process
1. **Immediate**: Set problematic feature flag to `false`
2. **30 seconds**: Restart affected services
3. **2 minutes**: Verify system restored to previous state

#### Rollback Commands
```bash
# Phase 1 rollback
export CLAMBAKE_NEW_PLANNER=false
systemctl restart clambake

# Phase 2 rollback  
export CLAMBAKE_BUNDLER_V2=false
export CLAMBAKE_NEW_EXECUTOR=false
systemctl restart clambake

# Emergency full rollback
export CLAMBAKE_MIGRATION_PHASE=legacy
export CLAMBAKE_NEW_PLANNER=false
export CLAMBAKE_BUNDLER_V2=false  
export CLAMBAKE_NEW_EXECUTOR=false
systemctl restart clambake
```

### Graceful Degradation

#### Component Isolation
- Planner failure: Fall back to legacy planning
- Bundler failure: Disable bundling, use individual work items
- Executor failure: Queue work for manual processing

#### State Reconciliation
1. **Work Preservation**: In-progress work moved to safe queue
2. **Data Sync**: New system state synchronized with legacy
3. **Agent Reassignment**: Agents gracefully transitioned to legacy flow

### Emergency Procedures

#### Critical Failure Response
1. **0-30 seconds**: Set all feature flags to `false`
2. **30-60 seconds**: Restart all services
3. **1-2 minutes**: Verify legacy system operational
4. **2-5 minutes**: Assess data consistency and repair if needed

#### Communication Plan
1. **Incident Response**: Alert on-call engineer
2. **Stakeholder Notification**: Inform team within 5 minutes
3. **Status Updates**: Provide updates every 15 minutes during incident
4. **Post-Incident**: Complete retrospective within 24 hours

## Deployment Process

### Blue-Green Deployment

#### Environment Setup
- **Blue**: Current production system
- **Green**: New system with feature flags
- **Switch**: DNS/load balancer cutover

#### Deployment Steps
1. Deploy green environment with all flags `false`
2. Route 10% traffic to green for health validation
3. Enable feature flags progressively on green
4. Gradually shift traffic from blue to green
5. Decommission blue after green proven stable

### Canary Releases

#### Traffic Shifting Strategy
- **Phase 1**: 10% → 25% → 50% → 75% → 100%
- **Monitoring Window**: 2 hours at each percentage
- **Rollback Threshold**: Any SLA violation

#### Success Metrics for Progression
- Error rate increase < 2%
- P95 latency increase < 20%  
- Work success rate maintained ≥ 95%
- No data corruption incidents

## Monitoring and Alerting

### Key Metrics During Migration
- **Performance**: Latency, throughput, resource usage
- **Quality**: Error rates, work success rates, data consistency
- **Business**: Agent productivity, issue resolution time

### Alert Thresholds
- **Critical**: System unavailable, data corruption, security breach
- **Warning**: Performance degradation, increased error rates
- **Info**: Migration milestones, feature flag changes

### Health Checks
1. **Endpoint Health**: All APIs responding correctly
2. **Data Consistency**: Work state matches between systems
3. **Agent Coordination**: No duplicate assignments or race conditions
4. **GitHub Integration**: API rate limits and success rates within bounds

## Success Metrics

### Performance Targets
- **Zero Downtime**: 100% uptime during migration
- **Performance**: < 5% degradation during dual-system operation
- **Rollback Time**: < 2 minutes for any component
- **Data Consistency**: 100% between old and new systems

### Business Continuity
- All existing workflows continue functioning normally
- Agent productivity maintained within 10% of baseline
- Issue resolution time not increased by more than 15%
- No work lost or corrupted during migration

## Risk Mitigation

### Identified Risks
1. **Data Corruption**: Dual-write inconsistencies
2. **Performance Degradation**: New system slower than legacy
3. **Feature Incompatibility**: New system missing legacy features
4. **Rollback Failure**: Unable to revert to previous state

### Mitigation Strategies
1. **Comprehensive Testing**: Load tests, chaos engineering, data validation
2. **Gradual Rollout**: Phased approach with monitoring at each stage
3. **Automated Rollback**: Triggers and procedures for rapid reversion
4. **Data Backup**: Point-in-time snapshots before each phase

## Post-Migration Tasks

### Immediate (Week 4)
- [ ] Remove all feature flags from codebase
- [ ] Update configuration management systems  
- [ ] Archive legacy monitoring dashboards
- [ ] Conduct post-migration retrospective

### Short Term (Month 1)
- [ ] Performance optimization based on production data
- [ ] Documentation updates for new system
- [ ] Training updates for operations team
- [ ] Cost analysis and optimization

### Long Term (Quarter 1)
- [ ] Technical debt cleanup
- [ ] Architecture refinements based on lessons learned
- [ ] Capacity planning for new system
- [ ] Next architecture evolution planning

---

**This specification defines the law for safe migration. All migration activities must comply.**