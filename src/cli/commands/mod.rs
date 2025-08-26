use crate::agents::AgentRouter;
use anyhow::Result;

pub mod actions;
pub mod agent;
pub mod bundle;
pub mod init;
pub mod land;
#[cfg(feature = "metrics")]
pub mod metrics;
pub mod peek;
pub mod pop;
pub mod reset;
pub mod route;
pub mod status;

#[allow(async_fn_in_trait)]
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
            println!("❌ Failed to initialize AgentRouter: {e:?}");
            Err(e.into())
        }
    }
}

pub async fn show_how_to_get_work() -> Result<()> {
    println!("🎯 My Little Soda - Multi-Agent Development Orchestration");
    println!();
    println!("To get started:");
    println!("  🚀 my-little-soda pop      # Claim your next task");
    println!("  📊 my-little-soda status   # See system overview");
    println!("  👁️  my-little-soda peek     # Preview available work");
    println!("  🍼 my-little-soda bottle   # Complete work and bundle");
    println!();
    println!("Admin commands:");
    println!("  🔀 my-little-soda route    # Route tasks to agents");
    println!("  ⚙️  my-little-soda init     # Setup development environment");
    println!();
    println!("💡 Start with 'my-little-soda pop' to claim your first task!");
    Ok(())
}
