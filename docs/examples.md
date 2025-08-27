# Init Command Examples

This guide provides practical examples of using My Little Soda's init command across different repository scenarios. All examples are based on real-world testing across 240+ test cases.

## Basic Usage Examples

### Fresh Project Initialization

**Scenario:** Starting a new project from scratch

```bash
# Create new directory
mkdir my-new-project
cd my-new-project

# Initialize git repository
git init
git remote add origin https://github.com/username/my-new-project.git

# Initialize My Little Soda
my-little-soda init
```

**Result:** Creates configuration files and directory structure for autonomous agent operation.

### Existing Project Integration

**Scenario:** Adding My Little Soda to an existing project

```bash
# Navigate to existing project
cd existing-project

# Verify git remote exists
git remote -v

# Initialize My Little Soda
my-little-soda init
```

**Result:** Non-destructive integration preserving all existing files and configurations.

## Repository Type Examples

### Empty Repository (C1a)

**Scenario:** Clean repository with no existing files

```bash
# Clone empty repository
git clone https://github.com/username/empty-repo.git
cd empty-repo

# Initialize My Little Soda
my-little-soda init

# Verify installation
ls -la
# Output: .git/ .my-little-soda/ my-little-soda.toml
```

### Repository with README (C1b)

**Scenario:** Repository with existing documentation

```bash
# Repository already contains README.md
cd project-with-readme

# Initialize My Little Soda (preserves README.md)
my-little-soda init

# Verify README preserved
cat README.md  # Original content intact
cat my-little-soda.toml  # New configuration file
```

### Repository with CI/CD (C1c)

**Scenario:** Project with existing GitHub Actions workflows

```bash
# Repository structure:
# .github/workflows/ci.yml
# .github/workflows/release.yml

cd project-with-cicd

# Initialize My Little Soda (preserves all CI/CD)
my-little-soda init

# Verify workflows preserved
ls .github/workflows/
# Output: ci.yml release.yml (unchanged)
```

### Complex Directory Structure (C1d)

**Scenario:** Multi-module project with complex layout

```bash
# Repository structure:
# src/main/
# src/test/
# docs/api/
# config/production/

cd complex-project

# Initialize My Little Soda (respects structure)
my-little-soda init

# Verify structure preserved
tree .
# All existing directories and files remain unchanged
# New: .my-little-soda/ and my-little-soda.toml added
```

### Repository with Issue Templates (C1e)

**Scenario:** Project with GitHub issue templates

```bash
# Repository structure:
# .github/ISSUE_TEMPLATE/bug_report.md
# .github/ISSUE_TEMPLATE/feature_request.md

cd project-with-templates

# Initialize My Little Soda (preserves templates)
my-little-soda init

# Verify templates preserved
ls .github/ISSUE_TEMPLATE/
# Output: bug_report.md feature_request.md (unchanged)
```

## Command Line Options

### Dry Run Mode

**Use Case:** Test initialization without making changes

```bash
# Preview what init would do
my-little-soda init --dry-run

# Safe to run multiple times
my-little-soda init --dry-run
my-little-soda init --dry-run
```

**Output:** Shows planned actions without file system changes.

### Force Mode

**Use Case:** Override existing configuration

```bash
# First initialization
my-little-soda init

# Later update with force (overwrites config)
my-little-soda init --force
```

**Safety:** Preserves all existing files, only updates My Little Soda configuration.

### Verbose Mode

**Use Case:** Detailed diagnostic information

```bash
# Maximum information for troubleshooting
my-little-soda init --verbose
```

**Output:** Detailed steps, authentication status, and validation results.

### Combined Options

**Use Case:** Maximum safety and information

```bash
# Test with full diagnostics (no changes made)
my-little-soda init --dry-run --verbose --force
```

## Authentication Examples

### GitHub HTTPS Authentication

**Scenario:** Using GitHub CLI with HTTPS

```bash
# Ensure GitHub CLI authentication
gh auth login
# Select: GitHub.com > HTTPS > Y > Login with web browser

# Verify authentication
gh auth status
# Output: Logged in to github.com as username

# Initialize with authenticated CLI
my-little-soda init
```

### GitHub SSH Authentication  

**Scenario:** Using SSH keys with GitHub

```bash
# Test SSH connection
ssh -T git@github.com
# Output: Hi username! You've successfully authenticated...

# Repository with SSH remote
git remote -v
# Output: origin git@github.com:username/repo.git

# Initialize with SSH remote
my-little-soda init
```

### Corporate Network Setup

**Scenario:** Behind corporate firewall with proxy

```bash
# Configure git for corporate proxy
git config --global http.proxy http://proxy.company.com:8080
git config --global https.proxy https://proxy.company.com:8080

# Ensure GitHub CLI works through proxy
gh auth status

# Initialize normally
my-little-soda init
```

## Platform-Specific Examples

### GitHub (Supported)

**HTTPS Remote:**
```bash
git remote add origin https://github.com/username/repository.git
my-little-soda init  # ✅ Fully supported
```

**SSH Remote:**
```bash
git remote add origin git@github.com:username/repository.git
my-little-soda init  # ✅ Fully supported
```

### GitLab (Graceful Failure)

**HTTPS Remote:**
```bash
git remote add origin https://gitlab.com/username/repository.git
my-little-soda init  # ❌ Fails with informative error
```

**SSH Remote:**
```bash
git remote add origin git@gitlab.com:username/repository.git  
my-little-soda init  # ❌ Fails with informative error
```

### Bitbucket (Graceful Failure)

**HTTPS Remote:**
```bash
git remote add origin https://bitbucket.org/username/repository.git
my-little-soda init  # ❌ Fails with informative error
```

### Self-hosted Git (Graceful Failure)

**Corporate Git Server:**
```bash
git remote add origin https://git.company.com/team/project.git
my-little-soda init  # ❌ Fails with informative error
```

## Multiple Remote Examples

### Standard Development Setup

**Scenario:** Fork-based development workflow

```bash
# Setup multiple remotes
git remote add origin https://github.com/username/my-fork.git
git remote add upstream https://github.com/original/repository.git
git remote add deploy git@heroku.com:my-app.git

# Initialize My Little Soda (uses 'origin' by convention)
my-little-soda init
```

**Result:** Uses 'origin' remote, other remotes preserved.

### Team Collaboration Setup

**Scenario:** Shared repository with multiple contributors

```bash
# Main repository
git remote add origin https://github.com/team/main-repo.git
# Team member forks
git remote add alice https://github.com/alice/main-repo.git
git remote add bob https://github.com/bob/main-repo.git

# Initialize My Little Soda
my-little-soda init
```

**Result:** Works with 'origin' as primary remote.

## Idempotency Examples

### Safe Multiple Executions

**Scenario:** Running init multiple times safely

```bash
# Initial setup
my-little-soda init

# Later executions (safe)
my-little-soda init  # No changes, confirms setup
my-little-soda init  # Still safe
my-little-soda init  # Always safe
```

**Result:** Subsequent runs confirm configuration without changes.

### Dry Run Idempotency

**Scenario:** Testing dry run consistency

```bash
# Run dry run multiple times
for i in {1..10}; do
    echo "Dry run $i:"
    my-little-soda init --dry-run
done
```

**Result:** All executions produce identical output, no state changes.

### Force Mode Updates

**Scenario:** Updating configuration with force

```bash
# Initial setup
my-little-soda init

# Configuration update
my-little-soda init --force
my-little-soda init --force  # Idempotent updates
```

**Result:** Configuration consistently updated, files preserved.

## Error Recovery Examples

### Authentication Recovery

**Problem:** GitHub authentication expired

```bash
# Attempt init (fails)
my-little-soda init
# Error: GitHub CLI not authenticated

# Fix authentication
gh auth login

# Retry init (succeeds)
my-little-soda init
```

### Remote URL Fix

**Problem:** Malformed remote URL

```bash
# Broken remote
git remote add origin not-a-url
my-little-soda init
# Error: Invalid remote URL format

# Fix remote
git remote set-url origin https://github.com/username/repo.git
my-little-soda init  # Now succeeds
```

### Complete Reset

**Problem:** Corrupted My Little Soda state

```bash
# Remove My Little Soda files
rm -f my-little-soda.toml
rm -rf .my-little-soda/

# Start fresh
my-little-soda init
```

## Integration Workflow Examples

### New Project Workflow

```bash
# 1. Create project
mkdir awesome-project
cd awesome-project

# 2. Initialize git
git init
echo "# Awesome Project" > README.md
git add README.md
git commit -m "Initial commit"

# 3. Add GitHub remote
git remote add origin https://github.com/username/awesome-project.git
git push -u origin main

# 4. Initialize My Little Soda
my-little-soda init

# 5. Verify setup
my-little-soda status
```

### Existing Project Integration

```bash
# 1. Navigate to project
cd existing-awesome-project

# 2. Verify git setup
git remote -v
git status

# 3. Ensure clean state
git add .
git commit -m "Save work before My Little Soda init"

# 4. Initialize My Little Soda
my-little-soda init

# 5. Verify integration
ls -la  # Check for new files
cat my-little-soda.toml  # Review configuration
my-little-soda status  # Confirm operation
```

## Best Practices

### Pre-Init Checklist

```bash
# 1. Verify git repository
git status

# 2. Check remote configuration  
git remote -v

# 3. Ensure GitHub CLI authentication
gh auth status

# 4. Test with dry run first
my-little-soda init --dry-run

# 5. Execute actual initialization
my-little-soda init
```

### Post-Init Verification

```bash
# 1. Check configuration
cat my-little-soda.toml

# 2. Verify directory structure
ls -la .my-little-soda/

# 3. Test agent status
my-little-soda status

# 4. Commit new files
git add my-little-soda.toml .my-little-soda/
git commit -m "Add My Little Soda configuration"
git push origin main
```

### Troubleshooting Workflow

```bash
# 1. Enable verbose mode
my-little-soda init --verbose --dry-run

# 2. Check individual components
gh auth status
git remote -v
git status

# 3. Fix identified issues
# (authentication, remotes, etc.)

# 4. Retry with verbose output
my-little-soda init --verbose

# 5. Verify success
my-little-soda status
```

## Advanced Examples

### Custom SSH Port

**Scenario:** Git server on non-standard SSH port

```bash
# SSH with custom port
git remote add origin ssh://git@gitlab.com:2222/username/repository.git

# Initialize (may fail gracefully for non-GitHub)
my-little-soda init
```

### Enterprise GitHub

**Scenario:** GitHub Enterprise Server

```bash
# Enterprise GitHub remote
git remote add origin https://github.company.com/team/project.git

# Configure GitHub CLI for enterprise
gh auth login --hostname github.company.com

# Initialize My Little Soda
my-little-soda init
```

### Monorepo Integration

**Scenario:** Large monorepo with multiple projects

```bash
# Navigate to specific project within monorepo
cd monorepo/project-a/

# Initialize git submodule or subtree if needed
# (depends on monorepo strategy)

# Initialize My Little Soda for this project
my-little-soda init

# Verify isolation from other projects
ls ../project-b/  # Should not contain My Little Soda files
```

These examples demonstrate My Little Soda's flexibility and safety across diverse repository configurations and development workflows.