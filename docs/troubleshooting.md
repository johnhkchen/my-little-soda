# Troubleshooting Guide

This guide provides solutions to common issues encountered when using My Little Soda's init command. All scenarios are based on extensive testing across 240+ test cases.

## Authentication Issues

### GitHub CLI Not Authenticated

**Symptoms:**
- Error: "GitHub CLI not authenticated"
- Command: `my-little-soda init` fails with authentication error

**Solution:**
```bash
# Login to GitHub CLI
gh auth login

# Verify authentication status
gh auth status

# Re-run init command
my-little-soda init
```

**Prevention:**
Always verify GitHub CLI authentication before running init:
```bash
gh auth status
```

### GitHub CLI Not Installed

**Symptoms:**
- Error: "Failed to run 'gh auth status'"
- Message mentions GitHub CLI installation

**Solution:**
Install GitHub CLI for your platform:

**Linux:**
```bash
# Ubuntu/Debian
sudo apt update
sudo apt install gh

# CentOS/RHEL/Fedora
sudo dnf install gh
```

**macOS:**
```bash
# Using Homebrew
brew install gh

# Using MacPorts
sudo port install gh
```

**Windows:**
```powershell
# Using Chocolatey
choco install gh

# Using Winget
winget install GitHub.cli
```

### Corrupted Authentication Token

**Symptoms:**
- Authentication commands succeed but API calls fail
- Empty or invalid token responses

**Solution:**
```bash
# Logout and re-authenticate
gh auth logout
gh auth login

# Verify new authentication
gh auth status
```

### Network Connectivity Issues

**Symptoms:**
- Local authentication passes but repository validation fails
- Network timeout errors

**Solution:**
1. Check internet connectivity:
```bash
ping github.com
```

2. Verify firewall/proxy settings allow GitHub access
3. For corporate networks, ensure GitHub domains are whitelisted

## Repository Configuration Issues

### No Git Remote Configured

**Symptoms:**
- Error about missing remote origin
- Local repository without upstream connection

**Solution:**
```bash
# Add GitHub remote
git remote add origin https://github.com/username/repository.git

# Verify remote configuration
git remote -v

# Re-run init
my-little-soda init
```

### Multiple Remotes with Conflicts

**Symptoms:**
- Ambiguous remote selection
- Init command uncertain which remote to use

**Solution:**
My Little Soda uses 'origin' by convention. Ensure origin points to your primary repository:
```bash
# Check current remotes
git remote -v

# Set origin to primary repository
git remote set-url origin https://github.com/username/primary-repo.git

# Other remotes can remain as upstream, fork, etc.
```

### Malformed Remote URLs

**Symptoms:**
- Parsing errors for remote URLs
- Invalid URL format messages

**Common Malformed URLs:**
- `not-a-url`
- `http://`
- `git@`
- `https://github.com/owner` (missing repo name)

**Solution:**
Fix remote URL format:
```bash
# Remove malformed remote
git remote remove origin

# Add properly formatted remote
git remote add origin https://github.com/username/repository.git
```

**Valid URL Formats:**
- HTTPS: `https://github.com/username/repository.git`
- SSH: `git@github.com:username/repository.git`
- Custom SSH port: `ssh://git@github.com:2222/username/repository.git`

## Platform-Specific Issues

### Non-GitHub Platforms

**Symptoms:**
- GitLab/Bitbucket remote detected
- Platform not supported messages

**Current Behavior:**
My Little Soda currently focuses on GitHub integration. Non-GitHub platforms will produce informative errors:

**GitLab:**
```bash
# These will fail gracefully with guidance
git remote add origin https://gitlab.com/username/repository.git
git remote add origin git@gitlab.com:username/repository.git
```

**Bitbucket:**
```bash
# These will fail gracefully with guidance  
git remote add origin https://bitbucket.org/username/repository.git
git remote add origin git@bitbucket.org:username/repository.git
```

**Self-hosted Git:**
```bash
# These will fail gracefully with guidance
git remote add origin https://git.company.com/team/project.git
git remote add origin git@git.company.com:team/project.git
```

### SSH Configuration Issues

**Symptoms:**
- SSH authentication failures
- Key permission errors

**Solution:**
1. Verify SSH key setup:
```bash
# Test SSH connection
ssh -T git@github.com

# Check SSH key permissions
chmod 600 ~/.ssh/id_rsa
chmod 644 ~/.ssh/id_rsa.pub
```

2. Add SSH key to ssh-agent:
```bash
# Start ssh-agent
eval "$(ssh-agent -s)"

# Add key to agent
ssh-add ~/.ssh/id_rsa
```

## Repository State Issues

### Existing Configuration Files

**Symptoms:**
- Conflicts with existing `my-little-soda.toml`
- Directory `.my-little-soda` already exists

**Solution:**
Use force flag to override existing configuration:
```bash
my-little-soda init --force
```

**Safety Note:** This preserves existing files while updating configuration.

### Dirty Working Directory

**Symptoms:**
- Uncommitted changes detected
- Working directory not clean

**Solution:**
```bash
# Check current status
git status

# Option 1: Commit changes
git add .
git commit -m "Save work before init"

# Option 2: Stash changes
git stash push -m "Temporary stash for init"

# Run init
my-little-soda init

# Restore stashed changes if needed
git stash pop
```

## Idempotency and Safety

### Multiple Init Executions

**Expected Behavior:**
- Safe to run multiple times
- Dry run mode never modifies files
- Force mode consistently overwrites configuration

**Verification:**
```bash
# Test dry run safety (run multiple times)
my-little-soda init --dry-run
my-little-soda init --dry-run
my-little-soda init --dry-run

# All should succeed identically
```

### State Preservation

**Guarantee:**
My Little Soda preserves all existing files and directories:
- Custom files remain unchanged
- Existing project structure maintained
- Only adds `.my-little-soda/` directory and `my-little-soda.toml`

**Verification:**
```bash
# Create custom files
echo "important data" > custom_file.txt
mkdir custom_dir
echo "nested data" > custom_dir/nested.txt

# Run init
my-little-soda init

# Verify preservation
cat custom_file.txt  # Should show "important data"
cat custom_dir/nested.txt  # Should show "nested data"
```

## Concurrent Execution

### Race Conditions

**Protection:**
My Little Soda includes race condition protection for concurrent executions.

**Testing:**
```bash
# Safe to run multiple instances simultaneously
my-little-soda init --dry-run &
my-little-soda init --dry-run &
my-little-soda init --dry-run &
wait

# All should complete successfully
```

## Diagnostic Commands

### Verbose Mode

Enable verbose output for detailed diagnostics:
```bash
my-little-soda init --verbose
```

### Dry Run Mode

Test init without making changes:
```bash
my-little-soda init --dry-run
```

### Combined Diagnostic Mode

Maximum information for troubleshooting:
```bash
my-little-soda init --dry-run --verbose --force
```

## Getting Help

### Command Help

```bash
# General help
my-little-soda --help

# Init command specific help
my-little-soda init --help
```

### Status Information

```bash
# Check repository and agent status
my-little-soda status
```

### Configuration Validation

```bash
# Verify configuration after init
cat my-little-soda.toml

# Check directory structure
ls -la .my-little-soda/
```

## Common Error Patterns

### Pattern 1: Authentication Chain
1. GitHub CLI not installed → Install GitHub CLI
2. GitHub CLI not authenticated → Run `gh auth login`
3. Corrupted credentials → Logout and re-authenticate
4. Network issues → Check connectivity and firewall

### Pattern 2: Repository Setup Chain  
1. No git repository → Run `git init`
2. No remote configured → Add remote with `git remote add origin`
3. Malformed remote URL → Fix URL format
4. Non-GitHub remote → Switch to GitHub or expect graceful failure

### Pattern 3: File System Chain
1. Permission issues → Check file/directory permissions
2. Existing configuration → Use `--force` flag if intentional
3. Dirty working directory → Commit or stash changes
4. Custom files → Verify preservation after init

## Recovery Procedures

### Complete Reset

If init command state becomes corrupted:
```bash
# Remove My Little Soda files
rm -f my-little-soda.toml
rm -rf .my-little-soda/

# Clean git state if needed
git status
git clean -fd  # Only if safe to remove untracked files

# Start fresh
my-little-soda init
```

### Backup Strategy

Before major changes:
```bash
# Backup current state
tar -czf backup-$(date +%Y%m%d-%H%M%S).tar.gz .git my-little-soda.toml .my-little-soda/ 2>/dev/null

# Run init with confidence
my-little-soda init --force
```

## Support

For issues not covered in this guide:
1. Check the [GitHub repository](https://github.com/johnhkchen/my-little-soda/issues) for existing issues
2. Create a new issue with:
   - Complete error messages
   - Output from `my-little-soda init --verbose --dry-run`
   - Platform and version information
   - Repository configuration details