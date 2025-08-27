use octocrab::Error as OctocrabError;

#[derive(Debug)]
#[allow(dead_code)] // Some variants are architectural - for future error handling
pub enum GitHubError {
    TokenNotFound(String),
    ConfigNotFound(String),
    ApiError(OctocrabError),
    IoError(std::io::Error),
    NotImplemented(String),
    RateLimit {
        reset_time: chrono::DateTime<chrono::Utc>,
        remaining: u32,
    },
    Timeout {
        operation: String,
        duration_ms: u64,
    },
    NetworkError(String),
    TokenScopeInsufficient {
        required_scopes: Vec<String>,
        current_error: String,
        token_url: String,
    },
}

impl From<OctocrabError> for GitHubError {
    fn from(err: OctocrabError) -> Self {
        GitHubError::ApiError(err)
    }
}

impl From<std::io::Error> for GitHubError {
    fn from(err: std::io::Error) -> Self {
        GitHubError::IoError(err)
    }
}

impl std::fmt::Display for GitHubError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GitHubError::TokenNotFound(msg) => {
                writeln!(f, "GitHub Authentication Error")?;
                writeln!(f, "──────────────────────────")?;
                write!(f, "🔑 {msg}\n\n")?;
                writeln!(f, "🔧 QUICK FIXES:")?;
                writeln!(f, "   → Use GitHub CLI: gh auth login")?;
                writeln!(
                    f,
                    "   → Set token directly: export MY_LITTLE_SODA_GITHUB_TOKEN=your_token"
                )?;
                writeln!(
                    f,
                    "   → Create token at: https://github.com/settings/tokens"
                )?;
                write!(
                    f,
                    "     (needs 'repo' scope for private repos, 'public_repo' for public)"
                )
            }
            GitHubError::ConfigNotFound(msg) => {
                writeln!(f, "GitHub Configuration Error")?;
                writeln!(f, "─────────────────────────")?;
                write!(f, "📂 {msg}\n\n")?;
                writeln!(f, "🔧 QUICK FIXES:")?;
                writeln!(f, "   → Set environment variables: export GITHUB_OWNER=username GITHUB_REPO=reponame")?;
                writeln!(f, "   → Use GitHub CLI in repo: gh repo view")?;
                write!(f, "   → Run setup: my-little-soda init")
            }
            GitHubError::ApiError(octocrab_err) => {
                writeln!(f, "GitHub API Error")?;
                writeln!(f, "────────────────")?;
                
                // Provide specific error details based on error type
                match octocrab_err {
                    octocrab::Error::GitHub { source, .. } => {
                        writeln!(f, "🌐 HTTP {}: {}", source.status_code, source.message)?;
                        writeln!(f)?;
                        
                        match source.status_code.as_u16() {
                            401 => {
                                writeln!(f, "🔧 AUTHENTICATION FAILED:")?;
                                writeln!(f, "   → Token is invalid or expired")?;
                                writeln!(f, "   → Run: gh auth login")?;
                                write!(f, "   → Or export MY_LITTLE_SODA_GITHUB_TOKEN=\"$(gh auth token)\"")
                            },
                            403 => {
                                writeln!(f, "🔧 PERMISSION DENIED:")?;
                                writeln!(f, "   → Token lacks required permissions")?;
                                writeln!(f, "   → Check repository access: gh repo view")?;
                                write!(f, "   → May need 'repo' scope: https://github.com/settings/tokens")
                            },
                            404 => {
                                writeln!(f, "🔧 RESOURCE NOT FOUND:")?;
                                writeln!(f, "   → Repository may not exist or be private")?;
                                writeln!(f, "   → Check GITHUB_OWNER and GITHUB_REPO settings")?;
                                write!(f, "   → Verify access: gh repo view")
                            },
                            422 => {
                                writeln!(f, "🔧 VALIDATION ERROR:")?;
                                writeln!(f, "   → Request data is invalid")?;
                                write!(f, "   → Check issue labels and repository configuration")
                            },
                            _ => {
                                writeln!(f, "🔧 TROUBLESHOOTING:")?;
                                writeln!(f, "   → Check authentication: gh auth status")?;
                                writeln!(f, "   → Test connection: curl -I https://api.github.com")?;
                                writeln!(f, "   → Verify repository access: gh repo view")?;
                                write!(f, "   → Check rate limits: gh api rate_limit")
                            }
                        }
                    },
                    octocrab::Error::Http { .. } => {
                        writeln!(f, "🌐 Network connection failed to GitHub API")?;
                        writeln!(f)?;
                        
                        // Detect common network environments and provide specific guidance
                        let is_ci = std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok();
                        let is_codespace = std::env::var("CODESPACES").is_ok();
                        let is_dev_container = std::env::var("REMOTE_CONTAINERS").is_ok();
                        
                        if is_ci {
                            writeln!(f, "🏗️  CI/CD ENVIRONMENT TROUBLESHOOTING:")?;
                            writeln!(f, "   → GitHub Actions should have internet access by default")?;
                            writeln!(f, "   → Check for custom network configurations or runners")?;
                            writeln!(f, "   → Verify GitHub API status: https://status.github.com")?;
                            writeln!(f, "   → Test: curl -v https://api.github.com")?;
                        } else if is_codespace || is_dev_container {
                            writeln!(f, "📦 CONTAINER ENVIRONMENT TROUBLESHOOTING:")?;
                            writeln!(f, "   → Codespaces/Dev Containers should have GitHub access by default")?;
                            writeln!(f, "   → Check container network settings")?;
                            writeln!(f, "   → Verify port forwarding configuration")?;
                            writeln!(f, "   → Test basic connectivity: curl -I https://api.github.com")?;
                        } else {
                            writeln!(f, "🔧 LOCAL NETWORK TROUBLESHOOTING:")?;
                            writeln!(f, "   → Step 1: Test basic internet: ping 8.8.8.8")?;
                            writeln!(f, "   → Step 2: Test DNS resolution: nslookup api.github.com")?;
                            writeln!(f, "   → Step 3: Test HTTPS: curl -I https://api.github.com")?;
                            writeln!(f, "   → Step 4: Check corporate firewall/proxy settings")?;
                            writeln!(f)?;
                            writeln!(f, "🏢 CORPORATE NETWORK SOLUTIONS:")?;
                            writeln!(f, "   → Configure HTTP proxy: export https_proxy=proxy.company.com:8080")?;
                            writeln!(f, "   → Add GitHub to allowlist: api.github.com, github.com")?;
                            writeln!(f, "   → Contact IT about GitHub API access")?;
                        }
                        writeln!(f)?;
                        write!(f, "📊 GitHub status: https://status.github.com")
                    },
                    _ => {
                        write!(f, "🌐 {octocrab_err}\n\n")?;
                        writeln!(f, "🔧 TROUBLESHOOTING:")?;
                        writeln!(f, "   → Check authentication: gh auth status")?;
                        writeln!(f, "   → Test connection: curl -I https://api.github.com")?;
                        writeln!(f, "   → Verify repository access: gh repo view")?;
                        write!(f, "   → Check rate limits: gh api rate_limit")
                    }
                }
            }
            GitHubError::IoError(io_err) => {
                writeln!(f, "File System Error")?;
                writeln!(f, "─────────────────")?;
                write!(f, "📁 {io_err}\n\n")?;
                writeln!(f, "🔧 POSSIBLE CAUSES:")?;
                writeln!(f, "   → File permissions issue")?;
                writeln!(f, "   → Directory doesn't exist")?;
                write!(f, "   → Disk space or I/O error")
            }
            GitHubError::NotImplemented(msg) => {
                writeln!(f, "Feature Not Yet Implemented")?;
                writeln!(f, "──────────────────────────")?;
                write!(f, "🚧 {msg}\n\n")?;
                writeln!(f, "🔧 ALTERNATIVES:")?;
                writeln!(f, "   → Manual workaround may be available")?;
                write!(f, "   → Feature coming in future release")
            }
            GitHubError::RateLimit {
                reset_time,
                remaining,
            } => {
                writeln!(f, "GitHub Rate Limit Exceeded")?;
                writeln!(f, "──────────────────────────")?;
                writeln!(f, "⏱️  Rate limit exceeded. {} requests remaining", remaining)?;
                
                let now = chrono::Utc::now();
                let wait_duration = reset_time.signed_duration_since(now);
                let wait_minutes = wait_duration.num_minutes().max(0);
                
                write!(
                    f,
                    "⏳ Rate limit resets at: {} (in ~{} minutes)\n\n",
                    reset_time.format("%Y-%m-%d %H:%M:%S UTC"),
                    wait_minutes
                )?;
                writeln!(f, "🔧 IMMEDIATE SOLUTIONS:")?;
                if wait_minutes <= 60 {
                    writeln!(f, "   → Wait {} minutes for automatic reset", wait_minutes)?;
                } else {
                    writeln!(f, "   → Wait ~{} hours for automatic reset", (wait_minutes + 30) / 60)?;
                }
                writeln!(f, "   → Use authenticated requests (higher limits)")?;
                writeln!(f, "   → Check current status: gh api rate_limit")?;
                writeln!(f)?;
                writeln!(f, "📊 RATE LIMIT INFO:")?;
                writeln!(f, "   → Authenticated: 5,000 requests/hour")?;
                writeln!(f, "   → Unauthenticated: 60 requests/hour")?;
                write!(f, "   → Enterprise: Up to 15,000 requests/hour")
            }
            GitHubError::Timeout {
                operation,
                duration_ms,
            } => {
                writeln!(f, "GitHub Operation Timeout")?;
                writeln!(f, "─────────────────────────")?;
                write!(
                    f,
                    "⏰ Operation '{operation}' timed out after {duration_ms}ms\n\n"
                )?;
                writeln!(f, "🔧 RECOMMENDED ACTIONS:")?;
                writeln!(f, "   → Check network connectivity")?;
                writeln!(f, "   → Retry the operation")?;
                write!(f, "   → Check GitHub status: https://status.github.com")
            }
            GitHubError::NetworkError(msg) => {
                writeln!(f, "GitHub Network Error")?;
                writeln!(f, "───────────────────")?;
                write!(f, "🌐 {msg}\n\n")?;
                
                // Environment-specific troubleshooting
                let is_ci = std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok();
                let has_proxy = std::env::var("HTTP_PROXY").is_ok() || std::env::var("HTTPS_PROXY").is_ok();
                
                if is_ci {
                    writeln!(f, "🔧 CI/CD TROUBLESHOOTING:")?;
                    writeln!(f, "   → Check runner network configuration")?;
                    writeln!(f, "   → Verify internet access is enabled")?;
                    writeln!(f, "   → Test GitHub connectivity in workflow")?;
                } else if has_proxy {
                    writeln!(f, "🔧 PROXY ENVIRONMENT DETECTED:")?;
                    writeln!(f, "   → Verify proxy settings are correct")?;
                    writeln!(f, "   → Check proxy authentication")?;
                    writeln!(f, "   → Test: curl -v --proxy $HTTP_PROXY https://api.github.com")?;
                } else {
                    writeln!(f, "🔧 LOCAL TROUBLESHOOTING:")?;
                    writeln!(f, "   → Check internet connectivity: ping 8.8.8.8")?;
                    writeln!(f, "   → Verify DNS resolution: nslookup api.github.com")?;
                    writeln!(f, "   → Test HTTPS access: curl -I https://api.github.com")?;
                    writeln!(f, "   → Check firewall rules for outbound HTTPS (port 443)")?;
                }
                writeln!(f)?;
                writeln!(f, "💡 NETWORK DEBUGGING TIPS:")?;
                writeln!(f, "   → Try from different network (mobile hotspot)")?;
                writeln!(f, "   → Check corporate firewall/VPN settings")?;
                writeln!(f, "   → Verify system time is correct (affects TLS)")?;
                write!(f, "   → GitHub status page: https://status.github.com")
            }
            GitHubError::TokenScopeInsufficient { required_scopes, current_error, token_url } => {
                writeln!(f, "GitHub Token Scope Insufficient")?;
                writeln!(f, "──────────────────────────────")?;
                write!(f, "🔐 {current_error}\n\n")?;
                writeln!(f, "📋 REQUIRED TOKEN SCOPES:")?;
                for scope in required_scopes {
                    writeln!(f, "   ✓ {}", scope)?;
                }
                writeln!(f)?;
                writeln!(f, "🔧 HOW TO FIX:")?;
                writeln!(f, "   1. Visit: {}", token_url)?;
                writeln!(f, "   2. Edit your existing token or create a new one")?;
                writeln!(f, "   3. Enable the required scopes listed above")?;
                writeln!(f, "   4. Copy the token and run one of:")?;
                writeln!(f, "      → gh auth login (recommended)")?;
                writeln!(f, "      → export MY_LITTLE_SODA_GITHUB_TOKEN=your_new_token")?;
                writeln!(f)?;
                writeln!(f, "💡 SCOPE GUIDE:")?;
                writeln!(f, "   → 'repo' = Full access to private repositories")?;
                writeln!(f, "   → 'public_repo' = Access to public repositories only")?;
                writeln!(f, "   → 'issues:write' = Create and modify issues")?;
                write!(f, "   → 'pull_requests:write' = Create and modify pull requests")
            }
        }
    }
}

impl std::error::Error for GitHubError {}
