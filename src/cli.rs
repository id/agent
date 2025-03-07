use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Provider to use (e.g., openai, anthropic)
    #[arg(short, long, default_value = "openai")]
    pub provider: String,

    /// Model to use (e.g., gpt-4o, claude-3-opus-20240229)
    #[arg(short, long, default_value = "gpt-4o")]
    pub model: String,

    /// System message to set the behavior of the assistant
    #[arg(short, long, default_value = "You are a helpful AI assistant with access to tools for retrieving weather information and performing calculations. You can answer questions, provide information, and assist with various tasks. When asked about weather or calculations, use the appropriate tools to provide accurate responses. Be concise, helpful, and friendly in your interactions.")]
    pub system_message: String,

    /// Enable tool usage (e.g., functions)
    #[arg(short, long, default_value = "false")]
    pub enable_tools: bool,
    
    /// Input sources (comma-separated list: stdin, webhook)
    #[arg(long, default_value = "stdin")]
    pub inputs: String,
    
    /// Output destinations (comma-separated list: stdout, webhook)
    #[arg(long, default_value = "stdout")]
    pub outputs: String,
    
    /// Run as a daemon (fork to background)
    #[arg(long, default_value = "false")]
    pub daemon: bool,
    
    /// Webhook output URL (for webhook output destination)
    #[arg(long)]
    pub webhook_url: Option<String>,
} 