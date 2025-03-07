use anyhow::{Context, Result};

/// Configuration for the agent
#[derive(Debug, Clone)]
pub struct Config {
    /// Provider to use (e.g., openai, anthropic)
    pub provider: String,

    /// Model to use (e.g., gpt-4o, claude-3-opus-20240229)
    pub model: String,

    /// System message to set the behavior of the assistant
    pub system_message: String,

    /// Enable tool usage (e.g., functions)
    pub enable_tools: bool,

    /// Input sources (list: stdin, mqtt)
    pub inputs_vec: Vec<String>,

    /// Output destinations (list: stdout, mqtt)
    pub outputs_vec: Vec<String>,

    /// Run as a daemon (fork to background)
    pub daemon: bool,

    /// MQTT broker address (default: localhost)
    pub mqtt_broker: Option<String>,

    /// MQTT broker port (default: 1883)
    pub mqtt_port: Option<u16>,

    /// MQTT input topic (default: agent/input)
    pub mqtt_input_topic: Option<String>,

    /// MQTT output topic (default: agent/output)
    pub mqtt_output_topic: Option<String>,

    /// Maximum number of messages to keep in history (default: 50)
    pub max_history_messages: Option<usize>,
}

impl Config {
    /// Load configuration from a YAML file
    pub fn from_yaml(path: &str) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .context(format!("Failed to read config file: {}", path))?;

        let config: serde_yaml::Value =
            serde_yaml::from_str(&contents).context("Failed to parse YAML config")?;

        // Extract values with defaults
        let provider = config["provider"].as_str().unwrap_or("openai").to_string();
        let model = config["model"].as_str().unwrap_or("gpt-4o").to_string();
        let system_message = config["system_message"].as_str().unwrap_or("").to_string();
        let enable_tools = config["enable_tools"].as_bool().unwrap_or(false);
        let daemon = config["daemon"].as_bool().unwrap_or(false);
        let mqtt_broker = config["mqtt_broker"].as_str().map(|s| s.to_string());
        let mqtt_port = config["mqtt_port"].as_u64().map(|p| p as u16);
        let mqtt_input_topic = config["mqtt_input_topic"].as_str().map(|s| s.to_string());
        let mqtt_output_topic = config["mqtt_output_topic"].as_str().map(|s| s.to_string());

        // Extract max_history_messages with default
        let max_history_messages = config["max_history_messages"].as_u64().map(|m| m as usize);

        // Parse inputs and outputs from YAML
        let mut inputs_vec = Vec::new();
        let mut outputs_vec = Vec::new();

        if let Some(inputs) = config["inputs_vec"].as_sequence() {
            inputs_vec = inputs
                .iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect();
        }

        if let Some(outputs) = config["outputs_vec"].as_sequence() {
            outputs_vec = outputs
                .iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect();
        }

        // If inputs_vec or outputs_vec are empty, use defaults
        if inputs_vec.is_empty() {
            inputs_vec = vec!["mqtt".to_string(), "stdin".to_string()];
        }
        if outputs_vec.is_empty() {
            outputs_vec = vec!["mqtt".to_string(), "stdout".to_string()];
        }

        Ok(Config {
            provider,
            model,
            system_message,
            enable_tools,
            inputs_vec,
            outputs_vec,
            daemon,
            mqtt_broker,
            mqtt_port,
            mqtt_input_topic,
            mqtt_output_topic,
            max_history_messages,
        })
    }
}
