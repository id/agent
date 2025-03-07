use anyhow::Result;
use async_trait::async_trait;
use rand::Rng;
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::error;

use super::{InputSource, OutputDestination};

// MQTT message format
#[derive(Serialize, Deserialize)]
struct MqttMessage {
    role: String,
    content: String,
    timestamp: u64,
}

// MQTT input source implementation
pub struct MqttSource {
    message_rx: mpsc::Receiver<String>,
    _shutdown_tx: tokio::sync::broadcast::Sender<()>, // Keep sender alive
}

impl MqttSource {
    pub async fn new(
        topic: Option<String>,
        broker: Option<String>,
        port: Option<u16>,
        agent_name: Option<String>,
    ) -> Result<Self> {
        let agent_name = agent_name.unwrap_or_else(|| "agent".to_string());
        let default_topic = format!("agent/{}/input", agent_name);
        let topic = topic.unwrap_or_else(|| default_topic);
        let broker = broker.unwrap_or_else(|| "localhost".to_string());
        let port = port.unwrap_or(1883);

        // Generate a random client ID outside the async block
        let random_suffix: u16 = {
            let mut rng = rand::thread_rng();
            rng.gen()
        };
        let client_id = format!("{}-mqtt-input-{}", agent_name, random_suffix);

        // Create MQTT options with reconnection settings
        let mut mqtt_options = MqttOptions::new(&client_id, &broker, port);
        mqtt_options.set_keep_alive(std::time::Duration::from_secs(30));
        mqtt_options.set_clean_session(true);

        // Set manual reconnection parameters - we'll handle reconnection in the event loop

        // Create the MQTT client
        let (client, mut eventloop) = AsyncClient::new(mqtt_options, 10);

        // Create a channel for message passing
        let (message_tx, message_rx) = mpsc::channel(100);

        // Create a shutdown channel that is Send
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::broadcast::channel::<()>(1);
        let shutdown_tx_clone = shutdown_tx.clone();

        // Subscribe to the input topic
        match client.subscribe(&topic, QoS::AtLeastOnce).await {
            Ok(_) => tracing::info!("Successfully subscribed to topic: {}", topic),
            Err(e) => tracing::error!("Failed to subscribe to topic {}: {}", topic, e),
        }

        // Start the event loop in a separate task
        let topic_clone = topic.clone();
        let client_clone = client.clone();
        tokio::spawn(async move {
            let mut consecutive_errors = 0;

            loop {
                tokio::select! {
                    // Check for shutdown signal
                    _ = shutdown_rx.recv() => {
                        tracing::info!("MQTT input client shutting down");
                        break;
                    }
                    // Poll for MQTT events
                    event = eventloop.poll() => {
                        match event {
                            Ok(Event::Incoming(Packet::Publish(publish))) => {
                                // Reset error counter on successful message
                                consecutive_errors = 0;

                                if let Ok(message_str) = std::str::from_utf8(&publish.payload) {
                                    match serde_json::from_str::<MqttMessage>(message_str) {
                                        Ok(mqtt_message) => {
                                            if mqtt_message.role == "user" {
                                                if message_tx.send(mqtt_message.content).await.is_err() {
                                                    error!("Failed to send message to channel");
                                                }
                                            }
                                        },
                                        Err(e) => {
                                            error!("Failed to parse MQTT message: {}", e);
                                        }
                                    }
                                }
                            },
                            Ok(Event::Incoming(Packet::ConnAck(_))) => {
                                tracing::info!("MQTT connection established, subscribing to topic: {}", topic_clone);
                                // Resubscribe after reconnection
                                if let Err(e) = client_clone.subscribe(&topic_clone, QoS::AtLeastOnce).await {
                                    error!("Failed to resubscribe to topic {}: {}", topic_clone, e);
                                }
                            },
                            Ok(_) => {},
                            Err(e) => {
                                consecutive_errors += 1;
                                error!("MQTT input error (attempt {}): {}", consecutive_errors, e);

                                // Exponential backoff with maximum delay
                                let delay = std::cmp::min(
                                    std::time::Duration::from_millis(100 * 2u64.pow(consecutive_errors as u32)),
                                    std::time::Duration::from_secs(30)
                                );

                                tokio::time::sleep(delay).await;

                                // If we've had too many consecutive errors, log a warning
                                if consecutive_errors > 5 {
                                    tracing::warn!("Multiple consecutive MQTT errors, connection may be unstable");
                                }
                            }
                        }
                    }
                }
            }

            tracing::info!("MQTT input client task completed");
        });

        Ok(Self {
            message_rx,
            _shutdown_tx: shutdown_tx_clone, // Store sender to keep it alive
        })
    }
}

#[async_trait]
impl InputSource for MqttSource {
    fn name(&self) -> &str {
        "mqtt"
    }

    async fn read_message(&mut self) -> Result<Option<String>> {
        match self.message_rx.recv().await {
            Some(message) => Ok(Some(message)),
            None => Ok(None),
        }
    }
}

// MQTT output destination implementation
pub struct MqttDestination {
    client: AsyncClient,
    topic: String,
    _shutdown_tx: tokio::sync::broadcast::Sender<()>, // Keep sender alive
}

impl MqttDestination {
    pub async fn new(
        topic: Option<String>,
        broker: Option<String>,
        port: Option<u16>,
        agent_name: Option<String>,
    ) -> Result<Self> {
        let agent_name = agent_name.unwrap_or_else(|| "agent".to_string());
        let default_topic = format!("agent/{}/output", agent_name);
        let topic = topic.unwrap_or_else(|| default_topic);
        let broker = broker.unwrap_or_else(|| "localhost".to_string());
        let port = port.unwrap_or(1883);

        // Generate a random client ID outside the async block
        let random_suffix: u16 = {
            let mut rng = rand::thread_rng();
            rng.gen()
        };
        let client_id = format!("{}-mqtt-output-{}", agent_name, random_suffix);

        // Create MQTT options with reconnection settings
        let mut mqtt_options = MqttOptions::new(&client_id, &broker, port);
        mqtt_options.set_keep_alive(std::time::Duration::from_secs(30));
        mqtt_options.set_clean_session(true);

        // Set manual reconnection parameters - we'll handle reconnection in the event loop

        // Create the MQTT client
        let (client, mut eventloop) = AsyncClient::new(mqtt_options, 10);

        // Create a shutdown channel that is Send
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::broadcast::channel::<()>(1);
        let shutdown_tx_clone = shutdown_tx.clone();

        // Start the event loop in a separate task
        tokio::spawn(async move {
            let mut consecutive_errors = 0;

            loop {
                tokio::select! {
                    // Check for shutdown signal
                    _ = shutdown_rx.recv() => {
                        tracing::info!("MQTT output client shutting down");
                        break;
                    }
                    // Poll for MQTT events
                    event = eventloop.poll() => {
                        match event {
                            Ok(Event::Incoming(Packet::ConnAck(_))) => {
                                tracing::info!("MQTT output connection established");
                                consecutive_errors = 0;
                            },
                            Ok(_) => {},
                            Err(e) => {
                                consecutive_errors += 1;
                                error!("MQTT output error (attempt {}): {}", consecutive_errors, e);

                                // Exponential backoff with maximum delay
                                let delay = std::cmp::min(
                                    std::time::Duration::from_millis(100 * 2u64.pow(consecutive_errors as u32)),
                                    std::time::Duration::from_secs(30)
                                );

                                tokio::time::sleep(delay).await;

                                // If we've had too many consecutive errors, log a warning
                                if consecutive_errors > 5 {
                                    tracing::warn!("Multiple consecutive MQTT output errors, connection may be unstable");
                                }
                            }
                        }
                    }
                }
            }

            tracing::info!("MQTT output client task completed");
        });

        Ok(Self {
            client,
            topic,
            _shutdown_tx: shutdown_tx_clone, // Store sender to keep it alive
        })
    }
}

#[async_trait]
impl OutputDestination for MqttDestination {
    fn name(&self) -> &str {
        "mqtt"
    }

    async fn write_message(&self, role: &str, content: &str) -> Result<()> {
        if role == "assistant" {
            let message = MqttMessage {
                role: role.to_string(),
                content: content.to_string(),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            };

            let json = serde_json::to_string(&message)?;
            self.client
                .publish(&self.topic, QoS::AtLeastOnce, false, json)
                .await?;
        }
        Ok(())
    }
}
