# Platform Support Guide

This guide details My Little Soda's support for different Git platforms and authentication methods. Platform support is based on comprehensive testing across multiple scenarios.

## Supported Platforms

### GitHub (Full Support)

**Status:** ✅ Fully Supported  
**Authentication:** GitHub CLI required  
**Features:** Complete integration with all My Little Soda features

#### HTTPS Remotes
```bash
# Supported formats
https://github.com/username/repository.git
https://github.com/organization/repository.git
```

#### SSH Remotes  
```bash
# Supported formats
git@github.com:username/repository.git
git@github.com:organization/repository.git
```

#### GitHub Enterprise
```bash
# Enterprise GitHub servers
https://github.company.com/team/project.git
git@github.company.com:team/project.git
```

**Setup Requirements:**
1. GitHub CLI installed and authenticated
2. Valid repository remote pointing to GitHub
3. Repository access permissions

## Unsupported Platforms (Graceful Failure)

### GitLab

**Status:** ❌ Not Supported (Graceful Failure)  
**Behavior:** Fails with informative error message  
**Future:** Potential support in future releases

#### HTTPS Remotes
```bash
# Detected but not supported
https://gitlab.com/username/repository.git
https://gitlab.company.com/team/project.git
```

#### SSH Remotes
```bash
# Detected but not supported  
git@gitlab.com:username/repository.git
git@gitlab.company.com:team/project.git
```

**Error Message:** Clear guidance about GitHub requirement

### Bitbucket

**Status:** ❌ Not Supported (Graceful Failure)  
**Behavior:** Fails with informative error message  
**Future:** Potential support in future releases

#### HTTPS Remotes
```bash
# Detected but not supported
https://bitbucket.org/username/repository.git
https://company.bitbucket.org/projects/PROJECT/repos/repository.git
```

#### SSH Remotes  
```bash
# Detected but not supported
git@bitbucket.org:username/repository.git
ssh://git@bitbucket.company.com:7999/project/repository.git
```

### Self-Hosted Git Servers

**Status:** ❌ Not Supported (Graceful Failure)  
**Behavior:** Fails with informative error message  
**Scope:** Any non-GitHub git server

#### Common Self-Hosted Platforms
- Gitea
- Forgejo  
- Gogs
- Custom Git servers
- Corporate Git instances

```bash
# Examples of unsupported self-hosted servers
https://git.company.com/team/project.git
git@git.internal:team/project.git
ssh://git@code.company.com:22/project/repository.git
```

## Authentication Methods

### GitHub CLI (Required for GitHub)

**Status:** ✅ Required  
**Installation:** Platform-specific GitHub CLI required  
**Configuration:** Must be authenticated before using My Little Soda

#### Authentication Setup
```bash
# Install GitHub CLI (platform-specific)
# See installation guide for your platform

# Authenticate with GitHub
gh auth login

# Verify authentication
gh auth status
# Expected: "Logged in to github.com as username"
```

#### Authentication Modes
GitHub CLI supports multiple authentication modes:

**Web Browser (Recommended):**
```bash
gh auth login
# Select: GitHub.com > HTTPS > Y > Login with web browser
```

**Personal Access Token:**  
```bash
gh auth login
# Select: GitHub.com > HTTPS > Y > Paste authentication token
```

**SSH Key:**
```bash  
gh auth login
# Select: GitHub.com > SSH > Y > Generate/Upload SSH key
```

#### Corporate Networks
```bash
# For corporate GitHub Enterprise
gh auth login --hostname github.company.com

# Verify enterprise authentication  
gh auth status --hostname github.company.com
```

### SSH Key Authentication

**Status:** ✅ Supported for GitHub  
**Requirement:** GitHub CLI still required for API access  
**Use Case:** Git operations via SSH, API via GitHub CLI

#### SSH Key Setup
```bash
# Generate SSH key (if needed)
ssh-keygen -t ed25519 -C "your.email@example.com"

# Add key to SSH agent
ssh-add ~/.ssh/id_ed25519  

# Test SSH connection to GitHub
ssh -T git@github.com
# Expected: "Hi username! You've successfully authenticated..."
```

#### Combined Authentication
For repositories with SSH remotes:
1. SSH keys handle Git operations
2. GitHub CLI handles API access
3. Both must be configured for full functionality

## Remote Configuration

### Single Remote (Standard)

**Pattern:** Most common configuration  
**Remote Name:** `origin` (used by convention)

```bash
# Standard single remote
git remote add origin https://github.com/username/repository.git

# Verify configuration
git remote -v
# origin https://github.com/username/repository.git (fetch)
# origin https://github.com/username/repository.git (push)
```

### Multiple Remotes (Supported)

**Pattern:** Fork-based or team development workflows  
**Priority:** Uses `origin` remote by convention  
**Behavior:** Other remotes preserved but not used

```bash
# Multiple remote setup
git remote add origin https://github.com/username/my-fork.git
git remote add upstream https://github.com/original/repository.git
git remote add deploy git@heroku.com:my-app.git

# My Little Soda uses 'origin'
# Other remotes remain untouched
```

### No Remote (Error)

**Status:** ❌ Not Supported  
**Behavior:** Clear error message about missing remote  
**Solution:** Add remote before initialization

```bash
# Repository without remote (fails)
git remote -v
# (no output)

my-little-soda init
# Error: No remote origin configured

# Solution: Add GitHub remote
git remote add origin https://github.com/username/repository.git
my-little-soda init  # Now succeeds
```

## URL Format Support

### Valid GitHub URL Formats

#### HTTPS Formats
```bash
# Standard GitHub.com
https://github.com/username/repository.git
https://github.com/organization/repository.git

# GitHub Enterprise
https://github.company.com/username/repository.git
https://github.company.com/organization/repository.git
```

#### SSH Formats
```bash
# Standard SSH format
git@github.com:username/repository.git
git@github.com:organization/repository.git

# SSH with custom port (non-standard but handled)
ssh://git@github.com:2222/username/repository.git

# Enterprise SSH
git@github.company.com:username/repository.git
```

### Invalid URL Formats (Error)

**Behavior:** Clear error messages for malformed URLs  
**Recovery:** Fix URL format and retry

```bash
# Common malformed URLs that fail gracefully
not-a-url
http://
git@
https://github.com/
https://github.com/username  # Missing repository name
ftp://github.com/username/repository.git  # Invalid protocol
```

## Platform Detection

### Automatic Detection

My Little Soda automatically detects Git platform based on remote URL:

#### Detection Logic
1. Parse remote URL from `git remote get-url origin`
2. Extract hostname and path components
3. Match against supported platforms
4. Provide appropriate error for unsupported platforms

#### Supported Hostname Patterns
```bash
# GitHub patterns (supported)
github.com
*.github.com  # Enterprise subdomains
github.*.com  # Enterprise domains
```

#### Unsupported Hostname Patterns
```bash
# GitLab patterns (graceful failure)
gitlab.com
*.gitlab.com
gitlab.*.com

# Bitbucket patterns (graceful failure)  
bitbucket.org
*.bitbucket.org
bitbucket.*.com

# Generic Git patterns (graceful failure)
git.company.com
code.company.com  
*.git.company.com
```

## Error Handling

### Informative Errors

My Little Soda provides clear, actionable error messages for unsupported platforms:

#### GitLab Example
```
Error: GitLab remote detected
Remote: https://gitlab.com/username/repository.git
My Little Soda currently supports GitHub repositories only.
Please use a GitHub repository or consider migrating your project.
```

#### Bitbucket Example  
```
Error: Bitbucket remote detected
Remote: https://bitbucket.org/username/repository.git
My Little Soda currently supports GitHub repositories only.
Please use a GitHub repository or consider migrating your project.
```

#### Self-Hosted Example
```
Error: Unsupported Git server detected  
Remote: https://git.company.com/team/project.git
My Little Soda currently supports GitHub repositories only.
Please use a GitHub repository for autonomous agent functionality.
```

### Recovery Guidance

Each error includes specific guidance:
1. Platform limitation explanation
2. Suggested alternatives
3. Migration considerations
4. GitHub setup instructions

## Migration Considerations

### From GitLab to GitHub

```bash
# 1. Create new GitHub repository
gh repo create username/repository --public

# 2. Update remote URL
git remote set-url origin https://github.com/username/repository.git

# 3. Push existing code
git push -u origin main

# 4. Initialize My Little Soda
my-little-soda init
```

### From Bitbucket to GitHub

```bash
# 1. Create new GitHub repository  
gh repo create username/repository --private

# 2. Update remote configuration
git remote remove origin
git remote add origin https://github.com/username/repository.git

# 3. Push all branches
git push -u origin --all
git push -u origin --tags

# 4. Initialize My Little Soda
my-little-soda init
```

### From Self-Hosted to GitHub

```bash
# 1. Export existing repository
git clone --bare https://git.company.com/team/project.git
cd project.git

# 2. Create GitHub repository
gh repo create team/project --private

# 3. Mirror to GitHub
git push --mirror https://github.com/team/project.git

# 4. Clone from GitHub and initialize
cd ..
git clone https://github.com/team/project.git
cd project
my-little-soda init
```

## Future Platform Support

### Planned Support

**GitLab Integration:**
- Status: Under consideration  
- Requirement: GitLab CLI equivalent
- Scope: GitLab.com and self-hosted GitLab

**Bitbucket Integration:**
- Status: Under consideration
- Requirement: Bitbucket API integration
- Scope: Bitbucket Cloud and Server

### Contributing Platform Support

**Architecture:** Plugin-based platform support system  
**Requirements:**
- Platform CLI tool or API SDK
- Issue/PR management capabilities  
- Authentication integration
- Testing infrastructure

**Development Process:**
1. Create platform-specific module
2. Implement authentication interface
3. Add comprehensive testing
4. Update documentation
5. Submit pull request

## Platform-Specific Best Practices

### GitHub Optimization

**Authentication:**
- Use GitHub CLI for API access
- Configure SSH keys for Git operations  
- Set up GPG signing for commits

**Repository Setup:**
- Enable branch protection rules
- Configure automated testing workflows
- Set up issue templates and labels

**Network Configuration:**
- Configure corporate proxies if needed
- Ensure GitHub domains are whitelisted
- Test connectivity with `gh api /user`

### Cross-Platform Considerations

**Repository Portability:**
- Use standard Git features
- Avoid platform-specific Git hooks
- Document platform dependencies

**Team Workflows:**
- Standardize on GitHub for My Little Soda projects
- Provide migration guides for existing projects
- Train team on GitHub CLI usage

## Testing Platform Support

### Validation Commands

```bash
# Test platform detection
my-little-soda init --dry-run --verbose

# Verify authentication
gh auth status

# Test API connectivity  
gh api /user

# Validate repository access
gh repo view username/repository
```

### Troubleshooting Tools

```bash
# Debug remote configuration
git remote -v
git remote get-url origin

# Test network connectivity
ping github.com
curl -I https://github.com

# Validate GitHub CLI
gh --version
gh auth status --hostname github.com
```

This platform support guide ensures users understand current limitations and have clear paths for successful My Little Soda integration.