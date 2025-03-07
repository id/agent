use anyhow::Result;
use async_trait::async_trait;

pub mod mqtt;
pub mod stdin;
pub mod stdout;

// Re-export the source and destination types
pub use mqtt::{MqttDestination, MqttSource};
pub use stdin::StdinSource;
pub use stdout::StdoutDestination;

#[async_trait]
pub trait InputSource: Send + Sync {
    /// Get the name of the input source
    fn name(&self) -> &str;

    /// Read a message from the input source
    async fn read_message(&mut self) -> Result<Option<String>>;
}

#[async_trait]
pub trait OutputDestination: Send + Sync {
    /// Get the name of the output destination
    fn name(&self) -> &str;

    /// Write a message to the output destination
    async fn write_message(&self, role: &str, content: &str) -> Result<()>;
}

/// Factory function to create input sources
pub async fn create_input_sources(config: &crate::config::Config) -> Vec<Box<dyn InputSource>> {
    let mut sources = Vec::new();

    for source in &config.inputs_vec {
        match source.as_str() {
            "mqtt" => {
                let mqtt_source = MqttSource::new(
                    config.mqtt_input_topic.clone(),
                    config.mqtt_broker.clone(),
                    config.mqtt_port,
                    Some(config.agent_name.clone()),
                )
                .await
                .expect("Failed to create MQTT source");
                sources.push(Box::new(mqtt_source) as Box<dyn InputSource>);
            }
            "stdin" => {
                let stdin_source = StdinSource::new();
                sources.push(Box::new(stdin_source) as Box<dyn InputSource>);
            }
            _ => {
                tracing::error!("Unknown input source: {}", source);
            }
        }
    }

    sources
}

/// Factory function to create output destinations
pub async fn create_output_destinations(
    config: &crate::config::Config,
) -> Vec<Box<dyn OutputDestination>> {
    let mut destinations = Vec::new();

    for dest in &config.outputs_vec {
        match dest.as_str() {
            "mqtt" => {
                let mqtt_dest = MqttDestination::new(
                    config.mqtt_output_topic.clone(),
                    config.mqtt_broker.clone(),
                    config.mqtt_port,
                    Some(config.agent_name.clone()),
                )
                .await
                .expect("Failed to create MQTT destination");
                destinations.push(Box::new(mqtt_dest) as Box<dyn OutputDestination>);
            }
            "stdout" => {
                let stdout_dest = StdoutDestination::new();
                destinations.push(Box::new(stdout_dest) as Box<dyn OutputDestination>);
            }
            _ => {
                tracing::error!("Unknown output destination: {}", dest);
            }
        }
    }

    destinations
}
