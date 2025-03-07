use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Path to YAML configuration file
    #[arg(short, long)]
    pub config: Option<String>,

    /// Provider to use (e.g., openai, anthropic)
    #[arg(short, long, default_value = "openai")]
    pub provider: String,

    /// Model to use (e.g., gpt-4o, claude-3-opus-20240229)
    #[arg(short, long, default_value = "gpt-4o")]
    pub model: String,

    /// System message to set the behavior of the assistant
    #[arg(
        short,
        long,
        default_value = "You are a helpful AI assistant with access to tools for retrieving weather information and performing calculations. You can answer questions, provide information, and assist with various tasks. When asked about weather or calculations, use the appropriate tools to provide accurate responses. Be concise, helpful, and friendly in your interactions."
    )]
    pub system_message: String,

    /// Enable tool usage (e.g., functions)
    #[arg(short, long, default_value = "false")]
    pub enable_tools: bool,

    /// Input sources (comma-separated list: stdin, mqtt)
    #[arg(long, required = false)]
    pub inputs: Option<String>,

    /// Output destinations (comma-separated list: stdout, mqtt)
    #[arg(long, required = false)]
    pub outputs: Option<String>,

    /// Run as a daemon (fork to background)
    #[arg(long, default_value = "false")]
    pub daemon: bool,

    /// MQTT broker address (default: localhost)
    #[arg(long)]
    pub mqtt_broker: Option<String>,

    /// MQTT broker port (default: 1883)
    #[arg(long)]
    pub mqtt_port: Option<u16>,

    /// MQTT input topic (default: agent/input)
    #[arg(long)]
    pub mqtt_input_topic: Option<String>,

    /// MQTT output topic (default: agent/output)
    #[arg(long)]
    pub mqtt_output_topic: Option<String>,

    /// Maximum number of messages to keep in history (default: 50)
    #[arg(long)]
    pub max_history_messages: Option<usize>,

    /// Enable verbose logging (debug level)
    #[arg(short, long, default_value = "false")]
    pub verbose: bool,
}
