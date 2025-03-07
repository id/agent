use anyhow::Result;
use async_trait::async_trait;
use reqwest::{Client, header};
use serde::Deserialize;
use serde_json::json;

use super::{ChatCompletionResponse, Message, Provider, Tool, ToolCall, FunctionCall};

pub struct OpenAIProvider {
    client: Client,
    #[allow(dead_code)]
    api_key: String,
}

impl OpenAIProvider {
    pub fn new(api_key: &str) -> Self {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            "Authorization",
            header::HeaderValue::from_str(&format!("Bearer {}", api_key)).unwrap(),
        );
        headers.insert(
            "Content-Type",
            header::HeaderValue::from_static("application/json"),
        );

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .unwrap();

        OpenAIProvider { 
            client,
            api_key: api_key.to_string(),
        }
    }
}

#[async_trait]
impl Provider for OpenAIProvider {
    fn name(&self) -> &str {
        "openai"
    }

    fn available_models(&self) -> Vec<String> {
        vec![
            "gpt-4o".to_string(),
            "o3-mini".to_string(),
            "o1".to_string(),
            "o1-mini".to_string(),
        ]
    }

    async fn chat_completion(
        &self,
        model: &str,
        messages: &[Message],
        tools: Option<&[Tool]>,
    ) -> Result<ChatCompletionResponse> {
        let mut request = json!({
            "model": model,
            "messages": messages,
        });

        if let Some(tools) = tools {
            request["tools"] = json!(tools);
            request["tool_choice"] = json!("auto");
        }

        let response = self.client
            .post("https://api.openai.com/v1/chat/completions")
            .json(&request)
            .send()
            .await?;

        // Check if the response is successful
        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("OpenAI API error: {}", error_text);
        }

        let response_json: OpenAIChatCompletionResponse = response.json().await?;
        
        // Extract the first choice from the response
        if let Some(choice) = response_json.choices.first() {
            let message = choice.message.clone();
            
            // Convert OpenAI's response format to our common format
            let tool_calls_converted = message.tool_calls.map(|calls| {
                calls.into_iter().map(|call| {
                    ToolCall {
                        id: Some(call.id),
                        type_: Some(call.type_),
                        function: Some(FunctionCall {
                            name: call.function.name,
                            arguments: call.function.arguments,
                        }),
                    }
                }).collect()
            });
            
            Ok(ChatCompletionResponse {
                message: Message {
                    role: message.role,
                    content: message.content.unwrap_or_default(),
                    tool_calls: tool_calls_converted.clone(),
                    tool_call_id: None,
                },
                tool_calls: tool_calls_converted,
            })
        } else {
            anyhow::bail!("No completion choices returned from OpenAI")
        }
    }
}

// OpenAI API response structs
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct OpenAIChatCompletionResponse {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<OpenAIChoice>,
    usage: OpenAIUsage,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    index: u32,
    message: OpenAIMessage,
    finish_reason: String,
}

#[derive(Debug, Deserialize, Clone)]
struct OpenAIMessage {
    role: String,
    content: Option<String>,
    tool_calls: Option<Vec<OpenAIToolCall>>,
}

#[derive(Debug, Deserialize, Clone)]
struct OpenAIToolCall {
    id: String,
    #[serde(rename = "type")]
    type_: String,
    function: OpenAIFunctionCall,
}

#[derive(Debug, Deserialize, Clone)]
struct OpenAIFunctionCall {
    name: String,
    arguments: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
} 
