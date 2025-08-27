# Test Fixtures Documentation

This directory contains comprehensive test fixtures for My Little Soda testing, with a focus on repository state fixtures for init command testing.

## Overview

The fixture system provides:
- **Repository State Fixtures**: Different repository states (empty, with files, partial setup, conflicts)
- **Test Harness**: Utilities for managing temporary directories and git repositories  
- **Integration Helpers**: Bridge between fixtures and init command testing
- **Assertion Helpers**: Common test patterns and validations

## Quick Start

```rust
use crate::tests::fixtures::init_integration::{
    InitCommandTestEnvironment, TestScenario, assertions
};

#[tokio::test]
async fn test_init_on_empty_repository() {
    // Create test environment from fixture
    let env = InitCommandTestEnvironment::from_fixture_name("empty_repository").unwrap();
    
    // Run init command
    let result = env.run_and_validate_init(1, false, false).await.unwrap();
    
    // Validate result matches expectations
    assertions::assert_result_matches_expectation(&result, "empty_repository");
    
    // Validate post-init state
    let validation = env.validate_post_init_state(false).unwrap();
    assertions::assert_post_init_validation_passes(&validation, "empty_repository");
}
```

## Fixture Types

### Repository State Fixtures

Located in `repository_states.rs`, these fixtures simulate different repository conditions:

#### 1. Empty Repository (`empty_repository`)
- **Use Case**: Testing init on a fresh, minimal repository
- **Characteristics**: Only basic files (README, .gitignore), clean git state
- **Expected Behavior**: Should succeed without force, create config and directories
- **Example**:
  ```rust
  let fixture = RepositoryStateFixture::empty_repository();
  assert!(fixture.expected_init_behavior().should_succeed_without_force);
  ```

#### 2. Repository with Existing Files (`repository_with_existing_files`) 
- **Use Case**: Testing init on established projects with substantial codebases
- **Characteristics**: Complete Rust project structure (Cargo.toml, src/, tests)
- **Expected Behavior**: Should succeed without force, preserve existing files
- **Example**:
  ```rust
  let env = InitCommandTestEnvironment::from_fixture_name("repository_with_existing_files").unwrap();
  assert!(env.has_file("Cargo.toml"));
  assert!(env.has_file("src/main.rs"));
  ```

#### 3. Partial Initialization (`repository_with_partial_initialization`)
- **Use Case**: Testing init behavior when clambake config already exists
- **Characteristics**: Existing partial my-little-soda.toml, incomplete setup
- **Expected Behavior**: Should fail without force, require --force to proceed
- **Example**:
  ```rust
  let fixture = RepositoryStateFixture::repository_with_partial_initialization();
  assert!(!fixture.expected_init_behavior().should_succeed_without_force);
  assert!(fixture.existing_my_little_soda_config.is_some());
  ```

#### 4. Repository with Conflicts (`repository_with_conflicts`)
- **Use Case**: Testing init behavior with uncommitted changes and merge conflicts
- **Characteristics**: Merge conflict markers in files, uncommitted changes
- **Expected Behavior**: Should fail without force due to uncommitted state
- **Example**:
  ```rust
  let env = InitCommandTestEnvironment::from_fixture_name("repository_with_conflicts").unwrap();
  let content = env.read_file("src/main.rs").unwrap();
  assert!(content.contains("<<<<<<< HEAD"));
  ```

## Usage Patterns

### Pattern 1: Single Fixture Testing

Test init command against one specific repository state:

```rust
#[tokio::test]
async fn test_init_specific_scenario() {
    let env = InitCommandTestEnvironment::from_fixture_name("empty_repository").unwrap();
    let result = env.run_and_validate_init(1, false, false).await.unwrap();
    
    // Custom validation logic
    if env.expected_behavior.should_succeed_without_force {
        assert!(result.success, "Clean repository should allow init without force");
    }
}
```

### Pattern 2: Batch Testing Across All Fixtures

Test the same scenario across all available fixtures:

```rust
#[tokio::test]
async fn test_dry_run_across_all_fixtures() {
    let scenario = TestScenario::dry_run();
    let results = InitCommandBatchTester::test_scenario_across_fixtures(scenario).await.unwrap();
    
    // All fixtures should handle dry run without issues
    assertions::assert_all_fixtures_pass(&results, "dry run");
}
```

### Pattern 3: Force Flag Testing

Test how force flag affects different repository states:

```rust
#[tokio::test]
async fn test_force_flag_behavior() {
    let fixtures = ["empty_repository", "repository_with_partial_initialization"];
    
    for fixture_name in &fixtures {
        let env = InitCommandTestEnvironment::from_fixture_name(fixture_name).unwrap();
        
        // Test without force
        let result_no_force = env.run_and_validate_init(1, false, true).await.unwrap();
        
        // Test with force
        let result_with_force = env.run_and_validate_init(1, true, true).await.unwrap();
        
        // Force should always succeed
        assertions::assert_init_succeeds_with_force(&result_with_force, fixture_name);
    }
}
```

### Pattern 4: Post-Init State Validation

Validate repository state after init command execution:

```rust
#[tokio::test]
async fn test_post_init_state_changes() {
    let env = InitCommandTestEnvironment::from_fixture_name("empty_repository").unwrap();
    
    // Verify initial state
    assert!(!env.has_file("my-little-soda.toml"));
    assert!(!env.has_file(".my-little-soda"));
    
    // Run init (not dry run)
    let _result = env.run_and_validate_init(1, false, false).await.unwrap();
    
    // Verify post-init state
    let validation = env.validate_post_init_state(false).unwrap();
    assert!(validation.config_created);
    assert!(validation.directories_created);
    assertions::assert_post_init_validation_passes(&validation, "empty_repository");
}
```

### Pattern 5: Multi-Agent Configuration Testing

Test init command with different agent counts:

```rust
#[tokio::test]
async fn test_multi_agent_setup() {
    let env = InitCommandTestEnvironment::from_fixture_name("empty_repository").unwrap();
    
    for agent_count in [1, 4, 8, 12] {
        let result = env.run_and_validate_init(agent_count, false, true).await.unwrap();
        assert!(result.success, "Init should succeed with {} agents", agent_count);
    }
}
```

## Integration with Existing Tests

### Migrating Existing Init Tests

To migrate existing init command tests to use fixtures:

1. **Replace manual setup** with fixture-based environments:
   ```rust
   // Old way
   let temp_dir = tempfile::tempdir().unwrap();
   std::env::set_current_dir(temp_dir.path()).unwrap();
   
   // New way
   let env = InitCommandTestEnvironment::from_fixture_name("empty_repository").unwrap();
   ```

2. **Use fixture expectations** instead of hardcoded assumptions:
   ```rust
   // Old way
   assert!(result.is_ok()); // Always expect success
   
   // New way
   assertions::assert_result_matches_expectation(&result, fixture_name);
   ```

3. **Leverage batch testing** for comprehensive coverage:
   ```rust
   // Old way - test one scenario
   let result = init_command.execute().await;
   
   // New way - test across all relevant fixtures
   let results = InitCommandBatchTester::test_all_fixtures(1, false, false).await.unwrap();
   ```

## Advanced Usage

### Custom Fixtures

Create custom fixtures for specific test scenarios:

```rust
impl RepositoryStateFixture {
    pub fn custom_scenario() -> Self {
        Self {
            name: "custom_scenario".to_string(),
            description: "Custom test scenario".to_string(),
            files: HashMap::from([
                ("special_file.txt".to_string(), "special content".to_string()),
            ]),
            git_config: GitConfig::default(),
            existing_my_little_soda_config: None,
        }
    }
}
```

### Custom Validation

Extend post-init validation for specific needs:

```rust
impl InitCommandTestEnvironment {
    pub fn validate_custom_setup(&self) -> Result<bool> {
        // Custom validation logic
        let has_special_config = self.has_file("special_config.toml");
        let config_content = if has_special_config {
            self.read_file("special_config.toml")?
        } else {
            String::new()
        };
        
        Ok(config_content.contains("expected_setting"))
    }
}
```

### Fixture Composition

Combine multiple fixtures or extend existing ones:

```rust
fn create_composite_fixture() -> RepositoryStateFixture {
    let mut base = RepositoryStateFixture::repository_with_existing_files();
    
    // Add additional files
    base.files.insert("custom.toml".to_string(), "custom = true".to_string());
    
    // Modify git config
    base.git_config.current_branch = "feature".to_string();
    
    base
}
```

## Best Practices

### 1. Use Appropriate Fixtures
- **Empty repository** for basic functionality tests
- **Existing files** for integration with established projects  
- **Partial initialization** for conflict resolution tests
- **Conflicts** for error handling and recovery tests

### 2. Test Both Success and Failure Paths
```rust
// Test expected successes
assertions::assert_init_succeeds_for_clean_repo(&result, fixture_name);

// Test expected failures  
assertions::assert_init_fails_without_force(&result, fixture_name);

// Test force flag behavior
assertions::assert_init_succeeds_with_force(&force_result, fixture_name);
```

### 3. Validate Comprehensive State
```rust
// Not just command success, but complete state validation
let command_result = env.run_and_validate_init(1, false, false).await.unwrap();
let post_init_state = env.validate_post_init_state(false).unwrap();

assertions::assert_fixture_test_passes(&FixtureTestResult {
    fixture_name: env.fixture.name.clone(),
    fixture_description: env.fixture.description.clone(),  
    command_result,
    post_init_validation: post_init_state,
});
```

### 4. Use Batch Testing for Comprehensive Coverage
```rust
// Test critical scenarios across all fixtures
let scenarios = [
    TestScenario::normal_init(),
    TestScenario::force_init(),
    TestScenario::dry_run(),
];

for scenario in scenarios {
    let results = InitCommandBatchTester::test_scenario_across_fixtures(scenario).await.unwrap();
    assertions::assert_all_fixtures_pass(&results, &scenario.description);
}
```

### 5. Descriptive Test Names and Documentation
```rust
#[tokio::test]
async fn test_init_preserves_existing_rust_project_structure() {
    // Clear description of what's being tested
    let env = InitCommandTestEnvironment::from_fixture_name("repository_with_existing_files").unwrap();
    
    // Verify existing structure is preserved
    assert!(env.has_file("Cargo.toml"));
    assert!(env.has_file("src/main.rs"));
    
    let result = env.run_and_validate_init(1, false, false).await.unwrap();
    assertions::assert_result_matches_expectation(&result, "repository_with_existing_files");
    
    // Files should still exist after init
    assert!(env.has_file("Cargo.toml"));
    assert!(env.has_file("src/main.rs"));
}
```

## Troubleshooting

### Common Issues

1. **Test fails with "Fixture not found"**
   - Verify fixture name spelling in `from_fixture_name()` calls
   - Ensure fixture is registered in `all_fixtures()` method

2. **Directory permission errors**  
   - Tests may need write permissions for temporary directory creation
   - Check that temp directories are properly cleaned up

3. **Git command failures**
   - Ensure git is available in test environment
   - Some fixtures require git user configuration

4. **Expectation mismatches**
   - Review fixture's `expected_init_behavior()` settings
   - Verify test scenario matches fixture characteristics

### Debugging Tips

```rust
#[tokio::test]  
async fn debug_fixture_behavior() {
    let env = InitCommandTestEnvironment::from_fixture_name("debug_fixture").unwrap();
    
    // Inspect fixture properties
    println!("Fixture: {}", env.fixture.name);
    println!("Description: {}", env.fixture.description);
    println!("Expected to succeed without force: {}", 
             env.expected_behavior.should_succeed_without_force);
    
    // Inspect repository state
    println!("Repository path: {:?}", env.path());
    println!("Has my-little-soda.toml: {}", env.has_file("my-little-soda.toml"));
    
    // Run and inspect results
    let result = env.run_and_validate_init(1, false, true).await.unwrap();
    println!("Command succeeded: {}", result.success);
    if let Some(error) = &result.error_message {
        println!("Error: {}", error);
    }
}
```

## Contributing

When adding new fixtures or integration helpers:

1. **Add comprehensive tests** for new functionality
2. **Update this documentation** with new patterns and examples
3. **Follow naming conventions** (snake_case for fixture names)
4. **Provide clear descriptions** for fixture purposes and expected behaviors
5. **Test across different scenarios** (force, dry-run, multi-agent, etc.)

The fixture system is designed to make init command testing reliable, comprehensive, and maintainable. Use these patterns to ensure your tests are robust and cover all relevant scenarios.