/// File system operations abstraction for testing
/// 
/// This module provides a trait-based abstraction over file system operations
/// that can be easily mocked in tests using the `mockall` crate.
/// 
/// # Examples
/// 
/// ```rust,no_run
/// use my_little_soda::fs::{FileSystemOperations, StandardFileSystem};
/// use std::sync::Arc;
/// 
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let fs_ops: Arc<dyn FileSystemOperations> = Arc::new(StandardFileSystem);
///     
///     // Create a directory
///     fs_ops.create_dir_all(".my-little-soda/test").await?;
///     
///     // Write a file
///     fs_ops.write("test.txt", b"Hello, world!").await?;
///     
///     // Check if file exists
///     if fs_ops.exists("test.txt") {
///         println!("File was created successfully");
///     }
///     
///     Ok(())
/// }
/// ```
/// 
/// # Testing with Mocks
/// 
/// ```rust
/// #[cfg(test)]
/// mod tests {
///     use super::*;
///     use crate::fs::MockFileSystemOperations;
///     use mockall::predicate::eq;
///     use std::sync::Arc;
/// 
///     #[tokio::test]
///     async fn test_with_mocked_filesystem() {
///         let mut mock_fs = MockFileSystemOperations::new();
///         
///         // Set up expectations
///         mock_fs
///             .expect_write()
///             .with(eq("config.toml"), eq(b"test = true"))
///             .times(1)
///             .returning(|_, _| Ok(()));
///         
///         mock_fs
///             .expect_exists()
///             .with(eq("config.toml"))
///             .return_const(true);
///         
///         let fs_ops = Arc::new(mock_fs);
///         
///         // Test your code
///         fs_ops.write("config.toml", b"test = true").await.unwrap();
///         assert!(fs_ops.exists("config.toml"));
///     }
/// }
/// ```
use anyhow::Result;
use std::path::Path;

#[cfg(test)]
use mockall::{automock, predicate::*};

/// Trait for file system operations that can be mocked in tests
/// 
/// This trait abstracts common file system operations to enable easy testing
/// through mocking. All methods are designed to be mockable using the `mockall` crate.
#[cfg_attr(test, automock)]
#[async_trait::async_trait]
pub trait FileSystemOperations: Send + Sync {
    /// Create a directory and all its parent directories
    /// 
    /// # Arguments
    /// * `path` - The directory path to create
    /// 
    /// # Examples
    /// ```rust,no_run
    /// # use my_little_soda::fs::{FileSystemOperations, StandardFileSystem};
    /// # #[tokio::main]
    /// # async fn main() -> anyhow::Result<()> {
    /// let fs_ops = StandardFileSystem;
    /// fs_ops.create_dir_all(".my-little-soda/data").await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn create_dir_all(&self, path: &str) -> Result<()>;
    
    /// Write data to a file, creating the file if it doesn't exist
    /// 
    /// # Arguments
    /// * `path` - The file path to write to
    /// * `contents` - The data to write to the file
    /// 
    /// # Examples
    /// ```rust,no_run
    /// # use my_little_soda::fs::{FileSystemOperations, StandardFileSystem};
    /// # #[tokio::main]
    /// # async fn main() -> anyhow::Result<()> {
    /// let fs_ops = StandardFileSystem;
    /// fs_ops.write("config.toml", b"[settings]\nvalue = true").await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn write(&self, path: &str, contents: &[u8]) -> Result<()>;
    
    /// Check if a path exists
    /// 
    /// # Arguments
    /// * `path` - The path to check for existence
    /// 
    /// # Examples
    /// ```rust,no_run
    /// # use my_little_soda::fs::{FileSystemOperations, StandardFileSystem};
    /// let fs_ops = StandardFileSystem;
    /// if fs_ops.exists("clambake.toml") {
    ///     println!("Configuration file exists");
    /// }
    /// ```
    fn exists(&self, path: &str) -> bool;
    
    /// Execute a command and return its output
    /// 
    /// # Arguments
    /// * `program` - The program to execute
    /// * `args` - Command line arguments
    /// 
    /// # Examples
    /// ```rust,no_run
    /// # use my_little_soda::fs::{FileSystemOperations, StandardFileSystem};
    /// # #[tokio::main]
    /// # async fn main() -> anyhow::Result<()> {
    /// let fs_ops = StandardFileSystem;
    /// let output = fs_ops.execute_command("git", &["status".to_string()]).await?;
    /// println!("Git output: {}", String::from_utf8_lossy(&output.stdout));
    /// # Ok(())
    /// # }
    /// ```
    async fn execute_command(&self, program: &str, args: &[String]) -> Result<std::process::Output>;
}

/// Standard implementation that uses actual file system operations
/// 
/// This is the production implementation of `FileSystemOperations` that performs
/// real file system operations. Use this in production code and switch to
/// `MockFileSystemOperations` in tests.
/// 
/// # Examples
/// 
/// ```rust,no_run
/// use my_little_soda::fs::{FileSystemOperations, StandardFileSystem};
/// use std::sync::Arc;
/// 
/// let fs_ops: Arc<dyn FileSystemOperations> = Arc::new(StandardFileSystem);
/// // Use fs_ops for file operations in your application
/// ```
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