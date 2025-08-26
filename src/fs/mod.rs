/// File system operations abstraction for testing
use anyhow::Result;
use std::path::Path;

#[cfg(test)]
use mockall::{automock, predicate::*};

/// Trait for file system operations that can be mocked in tests
#[cfg_attr(test, automock)]
#[async_trait::async_trait]
pub trait FileSystemOperations: Send + Sync {
    /// Create a directory and all its parent directories
    async fn create_dir_all(&self, path: &str) -> Result<()>;
    
    /// Write data to a file, creating the file if it doesn't exist
    async fn write(&self, path: &str, contents: &[u8]) -> Result<()>;
    
    /// Check if a path exists
    fn exists(&self, path: &str) -> bool;
    
    /// Execute a command and return its output
    async fn execute_command(&self, program: &str, args: &[String]) -> Result<std::process::Output>;
}

/// Standard implementation that uses actual file system operations
pub struct StandardFileSystem;

#[async_trait::async_trait]
impl FileSystemOperations for StandardFileSystem {
    async fn create_dir_all(&self, path: &str) -> Result<()> {
        tokio::fs::create_dir_all(path).await.map_err(Into::into)
    }
    
    async fn write(&self, path: &str, contents: &[u8]) -> Result<()> {
        tokio::fs::write(path, contents).await.map_err(Into::into)
    }
    
    fn exists(&self, path: &str) -> bool {
        Path::new(path).exists()
    }
    
    async fn execute_command(&self, program: &str, args: &[String]) -> Result<std::process::Output> {
        let output = tokio::process::Command::new(program)
            .args(args)
            .output()
            .await?;
        Ok(output)
    }
}