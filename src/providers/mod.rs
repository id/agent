pub mod openai;
pub mod anthropic;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub function: Option<FunctionCall>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    #[serde(rename = "type")]
    pub type_: String,
    pub function: Function,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub message: Message,
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[async_trait]
pub trait Provider: Send + Sync {
    /// Get the name of the provider
    fn name(&self) -> &str;
    
    /// Get the available models for this provider
    #[allow(dead_code)]
    fn available_models(&self) -> Vec<String>;
    
    /// Send a chat completion request to the provider
    async fn chat_completion(
        &self,
        model: &str,
        messages: &[Message],
        tools: Option<&[Tool]>,
    ) -> Result<ChatCompletionResponse>;
}

pub fn get_provider(provider_name: &str, api_key: &str) -> Result<Box<dyn Provider>> {
    match provider_name.to_lowercase().as_str() {
        "openai" => Ok(Box::new(openai::OpenAIProvider::new(api_key))),
        "anthropic" => Ok(Box::new(anthropic::AnthropicProvider::new(api_key))),
        _ => anyhow::bail!("Unsupported provider: {}", provider_name),
    }
} 