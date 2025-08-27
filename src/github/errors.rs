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
                        writeln!(f, "🌐 Network connection failed")?;
                        writeln!(f)?;
                        writeln!(f, "🔧 NETWORK TROUBLESHOOTING:")?;
                        writeln!(f, "   → Check internet connectivity")?;
                        writeln!(f, "   → Test GitHub: curl -I https://api.github.com")?;
                        writeln!(f, "   → Check firewall/proxy settings")?;
                        write!(f, "   → GitHub status: https://status.github.com")
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
                writeln!(f, "⏱️  Rate limit exceeded. {remaining} requests remaining")?;
                write!(
                    f,
                    "⏳ Rate limit resets at: {}\n\n",
                    reset_time.format("%Y-%m-%d %H:%M:%S UTC")
                )?;
                writeln!(f, "🔧 RECOMMENDED ACTIONS:")?;
                writeln!(f, "   → Wait for rate limit reset")?;
                writeln!(f, "   → Use authentication to increase limits")?;
                write!(f, "   → Check rate limit status: gh api rate_limit")
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
                writeln!(f, "🔧 RECOMMENDED ACTIONS:")?;
                writeln!(f, "   → Check internet connectivity")?;
                writeln!(f, "   → Verify DNS resolution")?;
                write!(f, "   → Check firewall/proxy settings")
            }
        }
    }
}

impl std::error::Error for GitHubError {}
