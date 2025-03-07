use anyhow::Result;
use async_trait::async_trait;
use std::io::{self, BufRead, IsTerminal};
use tokio::sync::Mutex;
use tokio::task;
use std::sync::Arc;

use super::InputSource;

pub struct StdinSource {
    // Use a Mutex to make it thread-safe
    stdin_mutex: Arc<Mutex<()>>,
    // Flag to indicate if stdin is from a pipe (not a terminal)
    is_pipe: bool,
    // Flag to indicate if we've already read a message from a pipe
    has_read_from_pipe: bool,
}

impl StdinSource {
    pub fn new() -> Self {
        // Check if stdin is a terminal or a pipe
        let is_pipe = !io::stdin().is_terminal();
        
        StdinSource {
            stdin_mutex: Arc::new(Mutex::new(())),
            is_pipe,
            has_read_from_pipe: false,
        }
    }
}

#[async_trait]
impl InputSource for StdinSource {
    fn name(&self) -> &str {
        "stdin"
    }
    
    async fn read_message(&mut self) -> Result<Option<String>> {
        // If we're reading from a pipe and we've already read a message,
        // return None to indicate we're done
        if self.is_pipe && self.has_read_from_pipe {
            return Ok(None);
        }
        
        // Acquire the mutex to ensure only one thread is reading from stdin at a time
        let _lock = self.stdin_mutex.lock().await;
        
        // Use spawn_blocking because stdin operations are blocking
        let result = task::spawn_blocking(|| {
            let stdin = io::stdin();
            let mut line = String::new();
            match stdin.lock().read_line(&mut line) {
                Ok(0) => Ok(None), // EOF
                Ok(_) => {
                    // Trim the trailing newline
                    if line.ends_with('\n') {
                        line.pop();
                        if line.ends_with('\r') {
                            line.pop();
                        }
                    }
                    Ok(Some(line))
                },
                Err(e) => Err(anyhow::anyhow!("Failed to read from stdin: {}", e)),
            }
        }).await??;
        
        // If we got a message from a pipe, set the flag
        if self.is_pipe && result.is_some() {
            self.has_read_from_pipe = true;
        }
        
        Ok(result)
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
} 