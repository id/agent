pub mod stdin;
pub mod stdout;
pub mod webhook;

use anyhow::Result;
use async_trait::async_trait;
use std::any::Any;

#[async_trait]
pub trait InputSource: Send + Sync {
    /// Get the name of the input source
    fn name(&self) -> &str;
    
    /// Read a message from the input source
    async fn read_message(&mut self) -> Result<Option<String>>;
    
    /// Convert to Any for downcasting
    fn as_any(&self) -> &dyn Any;
}

#[async_trait]
pub trait OutputDestination: Send + Sync {
    /// Get the name of the output destination
    fn name(&self) -> &str;
    
    /// Write a message to the output destination
    async fn write_message(&self, role: &str, content: &str) -> Result<()>;
    
    /// Convert to Any for downcasting
    fn as_any(&self) -> &dyn Any;
}

/// Factory function to create input sources
pub fn create_input_source(source_name: &str) -> Result<Box<dyn InputSource>> {
    match source_name {
        "stdin" => Ok(Box::new(stdin::StdinSource::new())),
        "webhook" => Ok(Box::new(webhook::WebhookSource::new())),
        _ => Err(anyhow::anyhow!("Unknown input source: {}", source_name)),
    }
}

/// Factory function to create output destinations
pub fn create_output_destination(destination_name: &str, webhook_url: Option<&str>) -> Result<Box<dyn OutputDestination>> {
    match destination_name {
        "stdout" => Ok(Box::new(stdout::StdoutDestination::new())),
        "webhook" => {
            if let Some(url) = webhook_url {
                Ok(Box::new(webhook::WebhookDestination::with_url(url.to_string())))
            } else {
                Ok(Box::new(webhook::WebhookDestination::new()))
            }
        },
        _ => Err(anyhow::anyhow!("Unknown output destination: {}", destination_name)),
    }
} 