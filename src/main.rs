mod cli;
mod config;
mod io;
mod providers;

use anyhow::{Context, Result};
use clap::Parser;
use serde_json::json;

use cli::Args;
use config::Config;
use providers::{Function, Message, Tool};

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
        let right = &expr[idx + 1..];
        return evaluate_expression(left) + evaluate_expression(right);
    }

    // Handle subtraction
    if let Some(idx) = expr.rfind('-') {
        // Make sure it's not a negative number
        if idx > 0 {
            let left = &expr[0..idx];
            let right = &expr[idx + 1..];
            return evaluate_expression(left) - evaluate_expression(right);
        }
    }

    // Handle multiplication
    if let Some(idx) = expr.find('*') {
        let left = &expr[0..idx];
        let right = &expr[idx + 1..];
        return evaluate_expression(left) * evaluate_expression(right);
    }

    // Handle division
    if let Some(idx) = expr.find('/') {
        let left = &expr[0..idx];
        let right = &expr[idx + 1..];
        let right_val = evaluate_expression(right);
        if right_val != 0.0 {
            return evaluate_expression(left) / right_val;
        } else {
            return f64::NAN; // Division by zero
        }
    }

    // Handle square root
    if expr.starts_with("sqrt(") && expr.ends_with(")") {
        let inner = &expr[5..expr.len() - 1];
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
        let right = &expr[idx + 1..];
        return evaluate_expression(left).powf(evaluate_expression(right));
    }

    // Handle parentheses
    if expr.starts_with("(") && expr.ends_with(")") {
        let inner = &expr[1..expr.len() - 1];
        return evaluate_expression(inner);
    }

    // If we can't parse the expression, return NaN
    f64::NAN
}

// Add this function to manage message history
fn manage_message_history(messages: &mut Vec<providers::Message>, max_messages: usize) {
    // Always keep the system message (first message)
    if messages.len() <= 1 || messages.len() <= max_messages {
        return;
    }

    // Keep the system message and the most recent messages
    let system_message = messages.remove(0);

    // If we have too many messages, trim the oldest ones (after the system message)
    while messages.len() > max_messages - 1 {
        messages.remove(0);
    }

    // Put the system message back at the beginning
    messages.insert(0, system_message);

    tracing::info!("Trimmed message history to {} messages", messages.len());
}

// Add this function to handle retries for API calls
async fn with_retries<F, Fut, T>(
    operation: F,
    max_retries: usize,
    operation_name: &str,
) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut retries = 0;
    let mut backoff = tokio::time::Duration::from_millis(1000);

    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                retries += 1;
                if retries > max_retries {
                    tracing::error!(
                        "Operation '{}' failed after {} retries: {}",
                        operation_name,
                        max_retries,
                        e
                    );
                    return Err(e);
                }

                tracing::warn!(
                    "Operation '{}' failed (attempt {}/{}): {}",
                    operation_name,
                    retries,
                    max_retries,
                    e
                );
                tokio::time::sleep(backoff).await;
                backoff = std::cmp::min(backoff * 2, tokio::time::Duration::from_secs(30));
            }
        }
    }
}

// Add this function to send messages to all outputs
async fn send_to_all_outputs(
    outputs: &[Box<dyn io::OutputDestination>],
    role: &str,
    content: &str,
    message_type: &str,
) {
    tracing::info!("Sending {} message to all outputs", message_type);

    let mut futures = Vec::new();
    for output in outputs {
        let output_name = output.name().to_string();
        let future = async move {
            match output.write_message(role, content).await {
                Ok(_) => tracing::info!(
                    "Successfully sent {} message to output: {}",
                    message_type,
                    output_name
                ),
                Err(e) => tracing::error!(
                    "Failed to send {} message to output {}: {}",
                    message_type,
                    output_name,
                    e
                ),
            }
        };
        futures.push(future);
    }

    // Execute all futures concurrently
    futures::future::join_all(futures).await;
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Setup logging with appropriate level
    let log_level = if args.verbose {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };

    tracing_subscriber::fmt().with_max_level(log_level).init();

    tracing::info!("Log level set to {}", log_level);

    // Create a shutdown channel
    let (shutdown_tx, _) = tokio::sync::broadcast::channel::<()>(1);
    let shutdown_tx_clone = shutdown_tx.clone();

    // Setup signal handlers for graceful shutdown
    tokio::spawn(async move {
        match tokio::signal::ctrl_c().await {
            Ok(()) => {
                tracing::info!("Received shutdown signal, initiating graceful shutdown...");
                let _ = shutdown_tx_clone.send(());
            }
            Err(err) => {
                tracing::error!("Failed to listen for shutdown signal: {}", err);
            }
        }
    });

    // Load configuration
    let config_path = args.config.as_deref().unwrap_or("config.yaml");
    let mut config = Config::from_yaml(config_path)?;

    // Update config with command line arguments
    if !args.provider.is_empty() {
        config.provider = args.provider;
    }

    if !args.model.is_empty() {
        config.model = args.model;
    }

    if !args.system_message.is_empty() {
        config.system_message = args.system_message;
    }

    // Only update inputs if explicitly provided via command line
    if let Some(inputs) = &args.inputs {
        tracing::info!("Updating inputs from CLI arguments: {}", inputs);
        config.inputs_vec = inputs.split(',').map(|s| s.trim().to_string()).collect();
    } else {
        tracing::info!("Keeping inputs from config file: {:?}", config.inputs_vec);
    }

    // Only update outputs if explicitly provided via command line
    if let Some(outputs) = &args.outputs {
        tracing::info!("Updating outputs from CLI arguments: {}", outputs);
        config.outputs_vec = outputs.split(',').map(|s| s.trim().to_string()).collect();
    } else {
        tracing::info!("Keeping outputs from config file: {:?}", config.outputs_vec);
    }

    // Update other config values if provided via command line
    if args.enable_tools {
        config.enable_tools = true;
    }

    if args.daemon {
        config.daemon = true;
    }

    if let Some(broker) = &args.mqtt_broker {
        config.mqtt_broker = Some(broker.clone());
    }

    if let Some(port) = args.mqtt_port {
        config.mqtt_port = Some(port);
    }

    if let Some(input_topic) = &args.mqtt_input_topic {
        config.mqtt_input_topic = Some(input_topic.clone());
    }

    if let Some(output_topic) = &args.mqtt_output_topic {
        config.mqtt_output_topic = Some(output_topic.clone());
    }

    if let Some(max_history) = args.max_history_messages {
        config.max_history_messages = Some(max_history);
    }

    // Print the final configuration
    tracing::info!("Final configuration:");
    tracing::info!("  Inputs: {:?}", config.inputs_vec);
    tracing::info!("  Outputs: {:?}", config.outputs_vec);

    // If daemon mode is requested, daemonize the process
    if config.daemon {
        tracing::info!("Starting in daemon mode...");
        let daemon = daemonize::Daemonize::new()
            .working_directory(".")
            .user(
                users::get_current_username()
                    .unwrap()
                    .to_string_lossy()
                    .as_ref(),
            )
            .group(
                users::get_current_groupname()
                    .unwrap()
                    .to_string_lossy()
                    .as_ref(),
            );

        match daemon.start() {
            Ok(_) => tracing::info!("Daemon started successfully"),
            Err(e) => tracing::error!("Error starting daemon: {}", e),
        }
    }

    // Initialize provider
    let api_key_env_var = format!("{}_API_KEY", config.provider.to_uppercase());
    let api_key = std::env::var(&api_key_env_var)
        .context(format!("{} environment variable not set", api_key_env_var))?;

    // Create the provider
    let provider = providers::get_provider(&config.provider, &api_key)?;

    // Print the selected provider and model
    tracing::info!(
        "Using provider: {} with model: {}",
        provider.name(),
        config.model
    );

    // Print the available models
    tracing::info!(
        "Available models for {}: {:?}",
        provider.name(),
        provider.available_models()
    );

    // Initialize tools if enabled
    let tools = if config.enable_tools {
        Some(vec![
            Tool {
                type_: "function".to_string(),
                function: Function {
                    name: "get_current_weather".to_string(),
                    description: "Get the current weather".to_string(),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "location": {
                                "type": "string",
                                "description": "The location to get weather for, e.g. 'San Francisco, CA'"
                            }
                        },
                        "required": ["location"]
                    }),
                },
            },
            Tool {
                type_: "function".to_string(),
                function: Function {
                    name: "calculate".to_string(),
                    description: "Evaluate a mathematical expression".to_string(),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "expression": {
                                "type": "string",
                                "description": "The mathematical expression to evaluate, e.g. '2 + 2'"
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

    // Create input sources using the new function
    tracing::info!("Creating input sources: {:?}", config.inputs_vec);
    let inputs = io::create_input_sources(&config).await;
    tracing::info!("Successfully created {} input sources", inputs.len());

    // Create output destinations using the new function
    tracing::info!("Configuring output destinations: {:?}", &config.outputs_vec);
    let outputs = io::create_output_destinations(&config).await;
    tracing::info!("Successfully created {} output destinations", outputs.len());

    // Initialize conversation history
    let mut messages = vec![Message {
        role: "system".to_string(),
        content: config.system_message.clone(),
        tool_calls: None,
        tool_call_id: None,
    }];

    // First, create proper channels for input sources
    tracing::debug!("Setting up message channels...");
    let (tx, mut rx) = tokio::sync::mpsc::channel::<(usize, String)>(10);

    // Spawn tasks for each input source
    let mut input_tasks = tokio::task::JoinSet::new();
    for (i, input) in inputs.iter().enumerate() {
        let input_tx = tx.clone();
        let input_name = input.name().to_string();
        let mut shutdown_rx = shutdown_tx.subscribe();

        tracing::debug!("Starting listener for input source {}: {}", i, input_name);

        // Clone the config values we need
        let mqtt_input_topic = config.mqtt_input_topic.clone();
        let mqtt_broker = config.mqtt_broker.clone();
        let mqtt_port = config.mqtt_port;

        // Create a task to monitor this input
        input_tasks.spawn(async move {
            tracing::debug!("Starting listener task for input source {}: {}", i, input_name);

            // Clone the input for this task - we'll create a new instance with the same type
            let mut input_source = match input_name.as_str() {
                "mqtt" => {
                    // Import directly from the mqtt module
                    let mqtt_source = crate::io::mqtt::MqttSource::new(
                        mqtt_input_topic,
                        mqtt_broker,
                        mqtt_port,
                    ).await.expect("Failed to create MQTT source");
                    Box::new(mqtt_source) as Box<dyn crate::io::InputSource>
                },
                "stdin" => Box::new(crate::io::stdin::StdinSource::new()) as Box<dyn crate::io::InputSource>,
                _ => panic!("Unknown input source: {}", input_name),
            };

            // Implement exponential backoff for error recovery
            let mut backoff = tokio::time::Duration::from_millis(100);

            loop {
                tokio::select! {
                    // Check for shutdown signal
                    _ = shutdown_rx.recv() => {
                        tracing::info!("Shutting down input source {}: {}", i, input_name);
                        break;
                    }
                    // Try to read a message
                    result = input_source.read_message() => {
                        match result {
                            Ok(Some(msg)) => {
                                tracing::debug!("Input {}: Received message: {}", i, msg);
                                // Send the message to the main loop
                                if let Err(e) = input_tx.send((i, msg)).await {
                                    tracing::error!("Failed to forward message from input {}: {}", i, e);
                                    // If the channel is closed, we should exit
                                    break;
                                }
                                // Reset backoff on success
                                backoff = tokio::time::Duration::from_millis(100);
                            },
                            Ok(None) => {
                                // No message, wait a bit before checking again
                                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                            },
                            Err(e) => {
                                tracing::error!("Error reading from input {}: {}", i, e);
                                // Use exponential backoff with a maximum delay
                                tokio::time::sleep(backoff).await;
                                backoff = std::cmp::min(backoff * 2, tokio::time::Duration::from_secs(30));
                            }
                        }
                    }
                }
            }

            tracing::info!("Input source task {} completed", i);
        });
    }

    // Main event loop - truly event-driven
    tracing::info!("Starting event-driven message processing...");
    let mut shutdown_rx = shutdown_tx.subscribe();

    loop {
        tokio::select! {
            // Check for shutdown signal
            _ = shutdown_rx.recv() => {
                tracing::info!("Main loop received shutdown signal, exiting...");
                break;
            }
            // Wait for a message from any input source
            msg = rx.recv() => {
                match msg {
                    Some((idx, content)) => {
                        tracing::info!("\n\n=== MESSAGE RECEIVED ===");
                        tracing::info!("From input source {}: {}", idx, content);
                        tracing::info!("==========================\n\n");

                        // Check for exit command
                        if content.to_lowercase() == "exit" {
                            tracing::info!("Received exit command, shutting down");
                            for output in &outputs {
                                let _ = output.write_message("system", "Goodbye!").await;
                            }
                            // Trigger shutdown
                            let _ = shutdown_tx.send(());
                            break;
                        }

                        // Process the message - dereference the provider to get &dyn Provider
                        if let Err(e) = process_message(idx, content, provider.as_ref(), &config, &mut messages, &outputs, tools.as_deref()).await {
                            tracing::error!("Error processing message: {}", e);
                        }
                    },
                    None => {
                        tracing::info!("All input channels closed, exiting");
                        break;
                    }
                }
            }
        }
    }

    // Wait for all input tasks to complete
    tracing::info!("Waiting for input tasks to complete...");
    let shutdown_timeout = tokio::time::Duration::from_secs(5);
    let wait_for_tasks = async {
        while let Some(res) = input_tasks.join_next().await {
            if let Err(e) = res {
                tracing::error!("Error joining input task: {}", e);
            }
        }
    };

    // Add a timeout to prevent hanging
    match tokio::time::timeout(shutdown_timeout, wait_for_tasks).await {
        Ok(_) => tracing::info!("All input tasks completed successfully"),
        Err(_) => tracing::warn!("Timed out waiting for some input tasks to complete"),
    }

    // Force exit after a short delay to ensure all logs are flushed
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    tracing::info!("Agent shutdown complete");
    std::process::exit(0);
}

// Update the process_message function to fix the tool_call structure and provider type
async fn process_message(
    _input_idx: usize,
    content: String,
    provider: &dyn providers::Provider,
    config: &Config,
    messages: &mut Vec<providers::Message>,
    outputs: &[Box<dyn io::OutputDestination>],
    tools: Option<&[providers::Tool]>,
) -> Result<()> {
    // Add user message to history
    messages.push(providers::Message {
        role: "user".to_string(),
        content: content.clone(),
        tool_calls: None,
        tool_call_id: None,
    });

    // Send user message to all outputs
    send_to_all_outputs(outputs, "user", &content, "user").await;

    // Send processing message to all outputs
    send_to_all_outputs(
        outputs,
        "system",
        "Processing your request...",
        "processing",
    )
    .await;

    // Get chat completion with retries
    tracing::info!("Getting chat completion from AI");
    let response = with_retries(
        || provider.chat_completion(&config.model, &messages, tools),
        3,
        "chat_completion",
    )
    .await?;

    // Log the AI's response
    if let Some(tool_calls) = &response.tool_calls {
        tracing::info!("AI responded with tool calls: {:?}", tool_calls);
    } else {
        tracing::info!("AI responded: {}", response.message.content);
    }

    // Handle tool calls if present
    if let Some(tool_calls) = &response.tool_calls {
        // Add the assistant's response to the conversation
        messages.push(providers::Message {
            role: "assistant".to_string(),
            content: response.message.content.clone(),
            tool_calls: response.tool_calls.clone(),
            tool_call_id: None,
        });

        // Process each tool call
        for tool_call in tool_calls {
            // Unwrap the function since it's an Option
            if let Some(function) = &tool_call.function {
                let function_name = &function.name;
                let function_args = &function.arguments;

                tracing::info!(
                    "Processing tool call: {} with args: {}",
                    function_name,
                    function_args
                );

                // Parse the arguments
                let args: serde_json::Value = serde_json::from_str(function_args)?;

                // Execute the function
                let result = match function_name.as_str() {
                    "get_current_weather" => {
                        let location = args["location"].as_str().unwrap_or("unknown");
                        format!("Weather in {}: Sunny, 72°F", location)
                    }
                    "calculate" => {
                        let expression = args["expression"].as_str().unwrap_or("0");
                        let result = evaluate_expression(expression);
                        format!("Result: {}", result)
                    }
                    _ => format!("Unknown function: {}", function_name),
                };

                // Add the tool result to the conversation
                messages.push(providers::Message {
                    role: "tool".to_string(),
                    content: result,
                    tool_calls: None,
                    tool_call_id: tool_call.id.clone(),
                });
            }
        }

        // Get a follow-up response from the AI with retries
        tracing::info!("Getting follow-up response from AI");
        let follow_up = with_retries(
            || provider.chat_completion(&config.model, &messages, None),
            3,
            "follow_up_chat_completion",
        )
        .await?;

        // Add the follow-up response to the conversation
        messages.push(providers::Message {
            role: "assistant".to_string(),
            content: follow_up.message.content.clone(),
            tool_calls: None,
            tool_call_id: None,
        });

        // Manage message history to prevent excessive memory usage
        manage_message_history(messages, config.max_history_messages.unwrap_or(50));

        // Send the assistant's response to all outputs
        send_to_all_outputs(
            outputs,
            "assistant",
            &follow_up.message.content,
            "assistant",
        )
        .await;

        tracing::info!("AI follow-up response: {}", follow_up.message.content);
    } else {
        // Add the assistant's response to the conversation
        messages.push(providers::Message {
            role: "assistant".to_string(),
            content: response.message.content.clone(),
            tool_calls: None,
            tool_call_id: None,
        });

        // Manage message history to prevent excessive memory usage
        manage_message_history(messages, config.max_history_messages.unwrap_or(50));

        // Send the assistant's response to all outputs
        send_to_all_outputs(outputs, "assistant", &response.message.content, "assistant").await;
    }

    Ok(())
}
