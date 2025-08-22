use anyhow::Result;
use crate::agents::AgentRouter;

pub mod pop;
pub mod route;
pub mod land;
pub mod peek;
pub mod status;
pub mod init;
pub mod reset;
pub mod metrics;

pub trait Command {
    async fn execute(&self) -> Result<()>;
}

pub async fn with_agent_router<F, Fut, R>(f: F) -> Result<R>
where
    F: FnOnce(AgentRouter) -> Fut + Send,
    Fut: std::future::Future<Output = Result<R>> + Send,
    R: Send,
{
    print!("🔄 Connecting to GitHub... ");
    std::io::Write::flush(&mut std::io::stdout()).unwrap();
    
    match AgentRouter::new().await {
        Ok(router) => {
            println!("✅");
            f(router).await
        }
        Err(e) => {
            println!("❌ Failed to initialize AgentRouter: {:?}", e);
            Err(e.into())
        }
    }
}

pub async fn show_how_to_get_work() -> Result<()> {
    println!("🎯 Clambake - Multi-Agent Development Orchestration");
    println!();
    println!("To get started:");
    println!("  🚀 clambake pop      # Claim your next task");
    println!("  📊 clambake status   # See system overview");
    println!("  👁️  clambake peek     # Preview available work");
    println!();
    println!("Admin commands:");
    println!("  🔀 clambake route    # Route tasks to agents");
    println!("  ⚙️  clambake init     # Setup development environment");
    println!();
    println!("💡 Start with 'clambake pop' to claim your first task!");
    Ok(())
}