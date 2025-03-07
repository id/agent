use anyhow::Result;
use async_trait::async_trait;
use axum::{
    routing::{post, get},
    Router,
    extract::State,
    response::IntoResponse,
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use tracing::{info, error};
use portpicker::pick_unused_port;
use tokio::net::TcpListener;
use reqwest;
use serde_json::json;
use std::time::SystemTime;

use super::{InputSource, OutputDestination};

// Message queue for webhook input
type MessageSender = mpsc::Sender<String>;
type MessageReceiver = Mutex<mpsc::Receiver<String>>;

// Shared state for the Axum server
#[derive(Clone)]
struct AppState {
    message_sender: MessageSender,
}

// Request and response structures
#[derive(Deserialize)]
struct WebhookRequest {
    message: String,
}

#[derive(Serialize)]
struct WebhookResponse {
    status: String,
    message: String,
}

// Webhook input source implementation
pub struct WebhookSource {
    receiver: MessageReceiver,
    server_handle: Arc<StdMutex<Option<JoinHandle<()>>>>,
    port: u16,
}

impl WebhookSource {
    pub fn new() -> Self {
        // Create a channel for message passing
        let (sender, receiver) = mpsc::channel(100);
        let receiver = Mutex::new(receiver);
        
        // Find an available port
        let port = pick_unused_port().expect("No available ports");
        
        // Create the server handle
        let server_handle = Arc::new(StdMutex::new(None));
        let server_handle_clone = server_handle.clone();
        
        // Start the server in a separate task
        let sender_clone = sender.clone();
        tokio::spawn(async move {
            // Start the HTTP server
            if let Err(e) = start_webhook_server(port, sender_clone, server_handle_clone).await {
                error!("Failed to start webhook server: {}", e);
            }
        });
        
        WebhookSource {
            receiver,
            server_handle,
            port,
        }
    }
    
    // Get the port the server is listening on
    pub fn port(&self) -> u16 {
        self.port
    }
}

#[async_trait]
impl InputSource for WebhookSource {
    fn name(&self) -> &str {
        "webhook"
    }
    
    async fn read_message(&mut self) -> Result<Option<String>> {
        // Try to receive a message from the channel
        let mut receiver = self.receiver.lock().await;
        match receiver.try_recv() {
            Ok(message) => Ok(Some(message)),
            Err(mpsc::error::TryRecvError::Empty) => {
                // No message available, wait a bit
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                Ok(None)
            },
            Err(mpsc::error::TryRecvError::Disconnected) => {
                // Channel is closed, this shouldn't happen
                Err(anyhow::anyhow!("Webhook message channel disconnected"))
            }
        }
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    
    fn clone_box(&self) -> Box<dyn InputSource> {
        Box::new(WebhookSource::new())
    }
}

impl Drop for WebhookSource {
    fn drop(&mut self) {
        // Abort the server task when the source is dropped
        if let Some(handle) = self.server_handle.lock().unwrap().take() {
            handle.abort();
        }
    }
}

// Start the webhook HTTP server
async fn start_webhook_server(
    port: u16, 
    sender: MessageSender,
    server_handle: Arc<StdMutex<Option<JoinHandle<()>>>>
) -> Result<()> {
    // Create the application state
    let state = AppState { message_sender: sender };
    
    // Build the router
    let app = Router::new()
        .route("/", post(handle_webhook))
        .route("/health", get(health_check))
        .with_state(state);
    
    // Create the socket address
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    
    info!("Webhook server listening on http://{}", addr);
    
    // Create a TCP listener
    let listener = TcpListener::bind(addr).await?;
    
    // Start the server
    let server = axum::serve(listener, app);
    
    // Store the server handle
    let handle = tokio::spawn(async move {
        if let Err(e) = server.await {
            error!("Webhook server error: {}", e);
        }
    });
    
    *server_handle.lock().unwrap() = Some(handle);
    
    Ok(())
}

// Handler for webhook POST requests
async fn handle_webhook(
    State(state): State<AppState>,
    Json(payload): Json<WebhookRequest>,
) -> impl IntoResponse {
    // Send the message to the channel
    match state.message_sender.send(payload.message).await {
        Ok(_) => {
            let response = WebhookResponse {
                status: "success".to_string(),
                message: "Message received".to_string(),
            };
            (StatusCode::OK, Json(response))
        },
        Err(_) => {
            let response = WebhookResponse {
                status: "error".to_string(),
                message: "Failed to process message".to_string(),
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
        }
    }
}

// Health check endpoint
async fn health_check() -> impl IntoResponse {
    StatusCode::OK
}

// Webhook output destination implementation
pub struct WebhookDestination {
    url: String,
}

impl WebhookDestination {
    /// Create a new webhook destination
    pub fn new(url: &str) -> Result<Self> {
        Ok(Self {
            url: url.to_string(),
        })
    }
    
    // Get the webhook URL
    pub fn url(&self) -> &str {
        &self.url
    }
}

#[derive(Serialize)]
struct WebhookOutgoingMessage {
    role: String,
    content: String,
    timestamp: u64,
}

#[async_trait]
impl OutputDestination for WebhookDestination {
    fn name(&self) -> &str {
        "webhook"
    }
    
    async fn write_message(&self, role: &str, content: &str) -> Result<()> {
        // If we have a webhook URL and the role is "assistant", send the message
        // You can modify this condition to include other roles if needed
        if role == "assistant" {
            info!("Sending webhook to URL: {}", self.url);
            
            // Create the JSON payload
            let json = json!({
                "role": role,
                "content": content,
                "timestamp": SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            });
            
            // Send the webhook
            let client = reqwest::Client::new();
            match client.post(&self.url)
                .header("Content-Type", "application/json")
                .json(&json)
                .send()
                .await 
            {
                Ok(response) => {
                    if response.status().is_success() {
                        info!("Webhook sent successfully");
                    } else {
                        error!("Failed to send webhook: HTTP {}", response.status());
                    }
                },
                Err(e) => {
                    error!("Failed to send webhook: {}", e);
                    return Err(anyhow::anyhow!("Failed to send webhook: {}", e));
                }
            }
        }
        
        Ok(())
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
} 