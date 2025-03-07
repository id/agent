mod cli;
mod providers;
mod io;

use anyhow::{Context, Result};
use clap::Parser;
use dotenv::dotenv;
use std::env;
use uuid;
use tracing::{info, error};

use cli::Args;
use providers::{Message, Tool, Function};

// Function to evaluate mathematical expressions
fn evaluate_expression(expression: &str) -> f64 {
    // This is a simple implementation that handles basic operations
    // In a real-world scenario, you might want to use a more robust expression evaluator
    
    // Remove whitespace
    let expr = expression.replace(" ", "");
    
    // Try to parse as a simple number first
    if let Ok(num) = expr.parse::<f64>() {
        return num;
    }
    
    // Handle addition
    if let Some(idx) = expr.find('+') {
        let left = &expr[0..idx];
        let right = &expr[idx+1..];
        return evaluate_expression(left) + evaluate_expression(right);
    }
    
    // Handle subtraction
    if let Some(idx) = expr.rfind('-') {
        // Make sure it's not a negative number
        if idx > 0 {
            let left = &expr[0..idx];
            let right = &expr[idx+1..];
            return evaluate_expression(left) - evaluate_expression(right);
        }
    }
    
    // Handle multiplication
    if let Some(idx) = expr.find('*') {
        let left = &expr[0..idx];
        let right = &expr[idx+1..];
        return evaluate_expression(left) * evaluate_expression(right);
    }
    
    // Handle division
    if let Some(idx) = expr.find('/') {
        let left = &expr[0..idx];
        let right = &expr[idx+1..];
        let right_val = evaluate_expression(right);
        if right_val != 0.0 {
            return evaluate_expression(left) / right_val;
        } else {
            return f64::NAN; // Division by zero
        }
    }
    
    // Handle square root
    if expr.starts_with("sqrt(") && expr.ends_with(")") {
        let inner = &expr[5..expr.len()-1];
        let inner_val = evaluate_expression(inner);
        if inner_val >= 0.0 {
            return inner_val.sqrt();
        } else {
            return f64::NAN; // Negative square root
        }
    }
    
    // Handle power
    if let Some(idx) = expr.find('^') {
        let left = &expr[0..idx];
        let right = &expr[idx+1..];
        return evaluate_expression(left).powf(evaluate_expression(right));
    }
    
    // Handle parentheses
    if expr.starts_with("(") && expr.ends_with(")") {
        let inner = &expr[1..expr.len()-1];
        return evaluate_expression(inner);
    }
    
    // If we can't parse the expression, return NaN
    f64::NAN
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();
    
    // Load environment variables from .env file
    dotenv().ok();
    
    // Parse command-line arguments
    let args = Args::parse();
    
    // Handle daemon mode if requested
    if args.daemon {
        use daemonize::Daemonize;
        
        // Create a daemonize object
        let daemonize = Daemonize::new()
            .pid_file("/tmp/agent.pid")
            .working_directory(".")
            .user("nobody")
            .group("daemon")
            .umask(0o027)
            .stdout(std::fs::File::create("/tmp/agent.out").unwrap())
            .stderr(std::fs::File::create("/tmp/agent.err").unwrap());
            
        match daemonize.start() {
            Ok(_) => println!("Success, daemonized"),
            Err(e) => eprintln!("Error, {}", e),
        }
    }
    
    // Get API key from environment variable based on provider
    let api_key_env_var = match args.provider.as_str() {
        "openai" => "OPENAI_API_KEY",
        "anthropic" => "ANTHROPIC_API_KEY",
        _ => "OPENAI_API_KEY", // Default to OpenAI
    };
    
    let api_key = env::var(api_key_env_var)
        .context(format!("{} environment variable not set", api_key_env_var))?;
    
    // Get provider based on command-line argument
    let provider = providers::get_provider(&args.provider, &api_key)?;
    
    println!("Using provider: {} with model: {}", provider.name(), args.model);
    println!("Available models for {}: {:?}", provider.name(), provider.available_models());
    
    // Define any tools if needed
    let tools = if args.enable_tools {
        Some(vec![
            // Weather tool
            Tool {
                type_: "function".to_string(),
                function: Function {
                    name: "get_current_weather".to_string(),
                    description: "Get the current weather in a given location".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "location": {
                                "type": "string",
                                "description": "The city and state, e.g. San Francisco, CA"
                            },
                            "unit": {
                                "type": "string",
                                "enum": ["celsius", "fahrenheit"]
                            }
                        },
                        "required": ["location"]
                    }),
                },
            },
            // Calculator tool
            Tool {
                type_: "function".to_string(),
                function: Function {
                    name: "calculate".to_string(),
                    description: "Perform a mathematical calculation".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "expression": {
                                "type": "string",
                                "description": "The mathematical expression to evaluate, e.g. '2 + 2', '5 * 10', 'sqrt(16)'"
                            }
                        },
                        "required": ["expression"]
                    }),
                },
            },
        ])
    } else {
        None
    };
    
    // Create a conversation history
    let mut messages: Vec<Message> = Vec::new();
    
    // Add system message if provided
    if !args.system_message.is_empty() {
        messages.push(Message {
            role: "system".to_string(),
            content: args.system_message,
            tool_calls: None,
            tool_call_id: None,
        });
    }
    
    // Create input sources
    let input_sources: Vec<String> = args.inputs.split(',').map(|s| s.trim().to_string()).collect();
    let mut inputs = Vec::new();
    let mut webhook_ports = Vec::new();
    let has_webhook_input = input_sources.iter().any(|s| s == "webhook");
    
    for source_name in &input_sources {
        let input_source = io::create_input_source(&source_name)?;
        
        // If this is a webhook source, get and display the port
        if source_name == "webhook" {
            if let Some(webhook_source) = input_source.as_any().downcast_ref::<io::webhook::WebhookSource>() {
                let port = webhook_source.port();
                webhook_ports.push(port);
                println!("Webhook server listening on http://localhost:{}", port);
                println!("You can send messages with: curl -X POST http://localhost:{} -H 'Content-Type: application/json' -d '{{\"message\":\"Hello\"}}'", port);
            }
        }
        
        inputs.push(input_source);
    }
    
    // Create output destinations
    let output_destinations: Vec<String> = args.outputs.split(',').map(|s| s.trim().to_string()).collect();
    let mut outputs = Vec::new();
    for dest_name in output_destinations {
        outputs.push(io::create_output_destination(&dest_name, args.webhook_url.as_deref())?);
    }
    
    // Main conversation loop
    loop {
        // Read messages from all input sources
        let mut received_message = false;
        let mut all_inputs_done = true;
        
        for input in &mut inputs {
            if let Some(content) = input.read_message().await? {
                // Log the received message
                info!("Received message: {}", content);
                
                // Check for exit command
                if content.to_lowercase() == "exit" {
                    // Send goodbye message to all outputs
                    for output in &outputs {
                        output.write_message("system", "Goodbye!").await?;
                    }
                    return Ok(());
                }
                
                // Add user message to history
                messages.push(Message {
                    role: "user".to_string(),
                    content: content.clone(),
                    tool_calls: None,
                    tool_call_id: None,
                });
                
                // Send user message to all outputs
                for output in &outputs {
                    output.write_message("user", &content).await?;
                }
                
                received_message = true;
                all_inputs_done = false;
                
                // Log that we're sending the message to the AI
                info!("Sending message to AI: {}", content);
                
                // Send a message to all outputs indicating that the request is being processed
                for output in &outputs {
                    output.write_message("system", "Processing your request...").await?;
                }
                
                // Get chat completion
                let tools_ref = tools.as_deref();
                let response = provider.chat_completion(&args.model, &messages, tools_ref).await?;
                
                // Log the AI's response
                if let Some(tool_calls) = &response.tool_calls {
                    info!("AI responded with tool calls: {:?}", tool_calls);
                } else {
                    info!("AI responded: {}", response.message.content);
                }
                
                // Handle tool calls if any
                if let Some(tool_calls) = &response.tool_calls {
                    // Create a new message history for the follow-up request
                    // This ensures we have the correct structure for the OpenAI API
                    let mut follow_up_messages = messages.clone();
                    
                    // Add the assistant's response with tool calls to the history
                    follow_up_messages.push(Message {
                        role: "assistant".to_string(),
                        content: "".to_string(), // Empty content because we're using tool_calls
                        tool_calls: response.tool_calls.clone(),
                        tool_call_id: None,
                    });
                    
                    // Process each tool call
                    for tool_call in tool_calls {
                        if let Some(function) = &tool_call.function {
                            // Send tool call information to all outputs
                            for output in &outputs {
                                output.write_message("system", &format!("Tool call: {}", function.name)).await?;
                                output.write_message("system", &format!("Arguments: {}", function.arguments)).await?;
                            }
                            
                            // Here you would actually execute the tool function
                            // For now, we'll just simulate a response
                            let tool_response = match function.name.as_str() {
                                "get_current_weather" => {
                                    r#"{"temperature": 72, "unit": "fahrenheit", "description": "Sunny"}"#.to_string()
                                },
                                "calculate" => {
                                    // Parse the expression from the arguments
                                    let args: serde_json::Value = serde_json::from_str(&function.arguments).unwrap_or_default();
                                    let expression = args["expression"].as_str().unwrap_or("0");
                                    
                                    // Evaluate the expression
                                    let result = evaluate_expression(expression);
                                    
                                    // Return the result as JSON
                                    format!(r#"{{"result": {}}}"#, result)
                                },
                                _ => r#"{"error": "Tool not implemented"}"#.to_string(),
                            };
                            
                            // Generate a unique tool call ID if one wasn't provided
                            let tool_call_id = tool_call.id.clone().unwrap_or_else(|| format!("call_{}", uuid::Uuid::new_v4().to_string()));
                            
                            // Add the tool response message
                            follow_up_messages.push(Message {
                                role: "tool".to_string(),
                                content: tool_response,
                                tool_calls: None,
                                tool_call_id: Some(tool_call_id),
                            });
                        }
                    }
                    
                    // Get a follow-up completion with the tool response
                    let follow_up = provider.chat_completion(&args.model, &follow_up_messages, None).await?;
                    
                    // Add the assistant's response to the conversation
                    messages.push(Message {
                        role: "assistant".to_string(),
                        content: follow_up.message.content.clone(),
                        tool_calls: None,
                        tool_call_id: None,
                    });
                    
                    // Send the assistant's response to all outputs
                    for output in &outputs {
                        output.write_message("assistant", &follow_up.message.content).await?;
                    }
                    
                    // Log the follow-up response
                    info!("AI follow-up response: {}", follow_up.message.content);
                } else {
                    // Add the assistant's response to the conversation
                    messages.push(Message {
                        role: "assistant".to_string(),
                        content: response.message.content.clone(),
                        tool_calls: None,
                        tool_call_id: None,
                    });
                    
                    // Send the assistant's response to all outputs
                    for output in &outputs {
                        output.write_message("assistant", &response.message.content).await?;
                    }
                }
                
                break; // Process one message at a time
            } else {
                // This input source is done, but others might still have messages
                continue;
            }
        }
        
        // If all input sources are done and we don't have webhook inputs, exit gracefully
        if all_inputs_done && !received_message && !has_webhook_input {
            return Ok(());
        }
        
        if !received_message {
            // No messages received, wait a bit before checking again
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            continue;
        }
    }
} 