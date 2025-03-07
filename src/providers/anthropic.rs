use anyhow::Result;
use async_trait::async_trait;
use reqwest::{Client, header};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::{ChatCompletionResponse, Message, Provider, Tool, ToolCall, FunctionCall};

pub struct AnthropicProvider {
    client: Client,
    #[allow(dead_code)]
    api_key: String,
}

impl AnthropicProvider {
    pub fn new(api_key: &str) -> Self {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            "x-api-key",
            header::HeaderValue::from_str(api_key).unwrap(),
        );
        headers.insert(
            "Content-Type",
            header::HeaderValue::from_static("application/json"),
        );
        headers.insert(
            "anthropic-version",
            header::HeaderValue::from_static("2023-06-01"),
        );

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .unwrap();

        AnthropicProvider { 
            client,
            api_key: api_key.to_string(),
        }
    }
}

#[async_trait]
impl Provider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    fn available_models(&self) -> Vec<String> {
        vec![
            "claude-3.7-sonnet".to_string(),
            "claude-3.5-sonnet".to_string(),
            "claude-3.5-haiku".to_string(),
        ]
    }

    async fn chat_completion(
        &self,
        model: &str,
        messages: &[Message],
        tools: Option<&[Tool]>,
    ) -> Result<ChatCompletionResponse> {
        // Convert our generic messages to Anthropic's format
        let anthropic_messages: Vec<AnthropicMessage> = messages
            .iter()
            .map(|msg| AnthropicMessage {
                role: msg.role.clone(),
                content: vec![AnthropicContent {
                    type_: "text".to_string(),
                    text: msg.content.clone(),
                }],
            })
            .collect();

        let mut request = json!({
            "model": model,
            "messages": anthropic_messages,
            "max_tokens": 1024,
        });

        if let Some(tools) = tools {
            // Convert our generic tools to Anthropic's format
            let anthropic_tools: Vec<AnthropicTool> = tools
                .iter()
                .map(|tool| AnthropicTool {
                    name: tool.function.name.clone(),
                    description: tool.function.description.clone(),
                    input_schema: tool.function.parameters.clone(),
                })
                .collect();

            request["tools"] = json!(anthropic_tools);
        }

        let response = self.client
            .post("https://api.anthropic.com/v1/messages")
            .json(&request)
            .send()
            .await?;

        let response_json: AnthropicResponse = response.json().await?;
        
        // Convert Anthropic's response to our common format
        let content = if let Some(content) = response_json.content.first() {
            content.text.clone()
        } else {
            String::new()
        };

        // Extract tool calls if any
        let tool_calls = if let Some(tool_use) = response_json.tool_use {
            Some(vec![ToolCall {
                id: Some(tool_use.id),
                type_: Some("function".to_string()),
                function: Some(FunctionCall {
                    name: tool_use.name,
                    arguments: serde_json::to_string(&tool_use.input)?,
                }),
            }])
        } else {
            None
        };

        Ok(ChatCompletionResponse {
            message: Message {
                role: "assistant".to_string(),
                content,
                tool_calls: None,
                tool_call_id: None,
            },
            tool_calls,
        })
    }
}

// Anthropic API request and response structs
#[derive(Debug, Serialize)]
struct AnthropicMessage {
    role: String,
    content: Vec<AnthropicContent>,
}

#[derive(Debug, Serialize)]
struct AnthropicContent {
    #[serde(rename = "type")]
    type_: String,
    text: String,
}

#[derive(Debug, Serialize)]
struct AnthropicTool {
    name: String,
    description: String,
    input_schema: Value,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    id: String,
    #[serde(rename = "type")]
    type_: String,
    role: String,
    content: Vec<AnthropicResponseContent>,
    model: String,
    stop_reason: Option<String>,
    tool_use: Option<AnthropicToolUse>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct AnthropicResponseContent {
    #[serde(rename = "type")]
    type_: String,
    text: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct AnthropicToolUse {
    id: String,
    #[serde(rename = "type")]
    type_: String,
    name: String,
    input: Value,
} 
