use octocrab::Error as OctocrabError;

#[derive(Debug)]
pub enum GitHubError {
    TokenNotFound(String),
    ConfigNotFound(String),
    ApiError(OctocrabError),
    IoError(std::io::Error),
    NotImplemented(String),
    RateLimit { reset_time: chrono::DateTime<chrono::Utc>, remaining: u32 },
    Timeout { operation: String, duration_ms: u64 },
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
                write!(f, "GitHub Authentication Error\n")?;
                write!(f, "──────────────────────────\n")?;
                write!(f, "🔑 {}\n\n", msg)?;
                write!(f, "🔧 QUICK FIXES:\n")?;
                write!(f, "   → Use GitHub CLI: gh auth login\n")?;
                write!(f, "   → Set token directly: export MY_LITTLE_SODA_GITHUB_TOKEN=your_token\n")?;
                write!(f, "   → Create token at: https://github.com/settings/tokens\n")?;
                write!(f, "     (needs 'repo' scope for private repos, 'public_repo' for public)")
            },
            GitHubError::ConfigNotFound(msg) => {
                write!(f, "GitHub Configuration Error\n")?;
                write!(f, "─────────────────────────\n")?;
                write!(f, "📂 {}\n\n", msg)?;
                write!(f, "🔧 QUICK FIXES:\n")?;
                write!(f, "   → Set environment variables: export GITHUB_OWNER=username GITHUB_REPO=reponame\n")?;
                write!(f, "   → Use GitHub CLI in repo: gh repo view\n")?;
                write!(f, "   → Run setup: clambake init")
            },
            GitHubError::ApiError(octocrab_err) => {
                write!(f, "GitHub API Error\n")?;
                write!(f, "────────────────\n")?;
                write!(f, "🌐 {}\n\n", octocrab_err)?;
                write!(f, "🔧 TROUBLESHOOTING:\n")?;
                write!(f, "   → Check authentication: gh auth status\n")?;
                write!(f, "   → Test connection: curl -I https://api.github.com\n")?;
                write!(f, "   → Verify repository access: gh repo view\n")?;
                write!(f, "   → Check rate limits: gh api rate_limit")
            },
            GitHubError::IoError(io_err) => {
                write!(f, "File System Error\n")?;
                write!(f, "─────────────────\n")?;
                write!(f, "📁 {}\n\n", io_err)?;
                write!(f, "🔧 POSSIBLE CAUSES:\n")?;
                write!(f, "   → File permissions issue\n")?;
                write!(f, "   → Directory doesn't exist\n")?;
                write!(f, "   → Disk space or I/O error")
            },
            GitHubError::NotImplemented(msg) => {
                write!(f, "Feature Not Yet Implemented\n")?;
                write!(f, "──────────────────────────\n")?;
                write!(f, "🚧 {}\n\n", msg)?;
                write!(f, "🔧 ALTERNATIVES:\n")?;
                write!(f, "   → Manual workaround may be available\n")?;
                write!(f, "   → Feature coming in future release")
            },
            GitHubError::RateLimit { reset_time, remaining } => {
                write!(f, "GitHub Rate Limit Exceeded\n")?;
                write!(f, "──────────────────────────\n")?;
                write!(f, "⏱️  Rate limit exceeded. {} requests remaining\n", remaining)?;
                write!(f, "⏳ Rate limit resets at: {}\n\n", reset_time.format("%Y-%m-%d %H:%M:%S UTC"))?;
                write!(f, "🔧 RECOMMENDED ACTIONS:\n")?;
                write!(f, "   → Wait for rate limit reset\n")?;
                write!(f, "   → Use authentication to increase limits\n")?;
                write!(f, "   → Check rate limit status: gh api rate_limit")
            },
            GitHubError::Timeout { operation, duration_ms } => {
                write!(f, "GitHub Operation Timeout\n")?;
                write!(f, "─────────────────────────\n")?;
                write!(f, "⏰ Operation '{}' timed out after {}ms\n\n", operation, duration_ms)?;
                write!(f, "🔧 RECOMMENDED ACTIONS:\n")?;
                write!(f, "   → Check network connectivity\n")?;
                write!(f, "   → Retry the operation\n")?;
                write!(f, "   → Check GitHub status: https://status.github.com")
            },
            GitHubError::NetworkError(msg) => {
                write!(f, "GitHub Network Error\n")?;
                write!(f, "───────────────────\n")?;
                write!(f, "🌐 {}\n\n", msg)?;
                write!(f, "🔧 RECOMMENDED ACTIONS:\n")?;
                write!(f, "   → Check internet connectivity\n")?;
                write!(f, "   → Verify DNS resolution\n")?;
                write!(f, "   → Check firewall/proxy settings")
            }
        }
    }
}

impl std::error::Error for GitHubError {}