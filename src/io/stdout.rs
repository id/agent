use anyhow::Result;
use async_trait::async_trait;
use std::io::{self, Write};
use tracing::info;

use super::OutputDestination;

pub struct StdoutDestination;

impl StdoutDestination {
    pub fn new() -> Self {
        // Make sure stdout is not buffered
        io::stdout().flush().ok();
        StdoutDestination
    }
}

#[async_trait]
impl OutputDestination for StdoutDestination {
    fn name(&self) -> &str {
        "stdout"
    }
    
    async fn write_message(&self, role: &str, content: &str) -> Result<()> {
        // Format the message based on the role
        let formatted_message = match role {
            "assistant" => format!("\nAssistant: {}\n", content),
            "user" => format!("\nUser: {}\n", content),
            "system" => format!("\nSystem: {}\n", content),
            "tool" => format!("\nTool: {}\n", content),
            _ => format!("\n{}: {}\n", role, content),
        };
        
        // Log that we're writing to stdout
        info!("Writing to stdout: role={}, content={}", role, content);
        
        // Write the message to stdout and ensure it's flushed
        print!("{}", formatted_message);
        io::stdout().flush()?;
        
        // Add a small delay to ensure the output is visible
        tokio::task::yield_now().await;
        
        Ok(())
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
} 