use anyhow::Result;
use async_trait::async_trait;
use std::io::{self, BufRead};
use tokio::sync::mpsc;
use tokio::task;
use tracing::error;

use super::InputSource;

pub struct StdinSource {
    message_rx: mpsc::Receiver<String>,
    _shutdown_tx: tokio::sync::broadcast::Sender<()>, // Keep sender alive
}

impl StdinSource {
    pub fn new() -> Self {
        let (message_tx, message_rx) = mpsc::channel(100);

        // Create a shutdown channel that is Send
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::broadcast::channel::<()>(1);
        let shutdown_tx_clone = shutdown_tx.clone();

        // Spawn a task to read from stdin
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    // Check for shutdown signal
                    _ = shutdown_rx.recv() => {
                        tracing::info!("Stdin source shutting down");
                        break;
                    }
                    // Read from stdin
                    line_result = task::spawn_blocking(|| {
                        let mut line = String::new();
                        io::stdin().lock().read_line(&mut line).map(|_| line)
                    }) => {
                        let line = match line_result {
                            Ok(result) => result,
                            Err(e) => {
                                error!("Failed to spawn blocking task: {}", e);
                                break;
                            }
                        };

                        match line {
                            Ok(line) => {
                                let line = line.trim().to_string();
                                if !line.is_empty() {
                                    if message_tx.send(line).await.is_err() {
                                        error!("Failed to send message to channel");
                                        break;
                                    }
                                }
                            },
                            Err(e) => {
                                error!("Failed to read from stdin: {}", e);
                                break;
                            }
                        }
                    }
                }
            }

            tracing::info!("Stdin source task completed");
        });

        Self {
            message_rx,
            _shutdown_tx: shutdown_tx_clone, // Store sender to keep it alive
        }
    }
}

#[async_trait]
impl InputSource for StdinSource {
    fn name(&self) -> &str {
        "stdin"
    }

    async fn read_message(&mut self) -> Result<Option<String>> {
        match self.message_rx.recv().await {
            Some(message) => Ok(Some(message)),
            None => Ok(None),
        }
    }
}
