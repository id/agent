# AI Chat CLI Agent

A flexible Rust CLI application that interacts with various AI chat completion APIs.

## Features

- Modular design with support for multiple AI providers
- Currently supports OpenAI and Anthropic APIs
- Accepts user input from stdin or webhook
- Sends messages to the AI model
- Displays the AI's responses
- Support for function calling/tools:
  - Weather information tool
  - Calculator tool for mathematical expressions
- Configurable via command-line arguments
- Daemon mode for running in the background

## Prerequisites

- Rust and Cargo installed
- API keys for the providers you want to use:
  - OpenAI API key for OpenAI models
  - Anthropic API key for Claude models

## Setup

1. Clone this repository
2. Create a `.env` file in the root directory with your API keys:
   ```
   OPENAI_API_KEY=your_openai_api_key_here
   ANTHROPIC_API_KEY=your_anthropic_api_key_here
   ```
3. Build the project:
   ```
   cargo build --release
   ```

## Usage

Run the application with default settings:

```
cargo run --release
```

Or specify provider, model, and other options:

```
cargo run --release -- --provider openai --model gpt-4o --system-message "You are a helpful assistant with expertise in answering general knowledge questions and solving problems."
```

For a system message that emphasizes tool usage:

```
cargo run --release -- --provider openai --model gpt-4o --enable-tools --system-message "You are an assistant with access to tools. When asked about weather or calculations, always use the appropriate tool to provide accurate information."
```

To enable tool usage:

```
cargo run --release -- --enable-tools
```

### YAML Configuration

Instead of specifying all options on the command line, you can use a YAML configuration file:

```
cargo run --release -- --config config.yaml
```

Example `config.yaml`:

```yaml
# Agent Configuration

# Provider settings
provider: openai
model: gpt-4o

# System message
system_message: |
  You are a helpful AI assistant with access to tools for retrieving weather information 
  and performing calculations. You can answer questions, provide information, and assist 
  with various tasks. When asked about weather or calculations, use the appropriate tools 
  to provide accurate responses. Be concise, helpful, and friendly in your interactions.

# Tool settings
enable_tools: true

# Input/Output settings
inputs_vec:
  - mqtt
  - stdin

outputs_vec:
  - mqtt
  - stdout

# Webhook settings
# webhook_url: http://localhost:8000

# MQTT settings
mqtt_broker: broker.emqx.io
mqtt_port: 1883
mqtt_input_topic: agent/input
mqtt_output_topic: agent/output

# Daemon mode
daemon: false
```

Note that in the YAML configuration, inputs and outputs are specified as lists (`inputs_vec` and `outputs_vec`) rather than comma-separated strings. This makes the configuration more readable and easier to maintain.

All options that can be specified on the command line can also be specified in the YAML configuration file.

### Available Command-Line Options

- `--config` or `-c`: Path to YAML configuration file
- `--provider` or `-p`: AI provider to use (default: "openai", options: "openai", "anthropic")
- `--model` or `-m`: Model to use (default: "gpt-4o")
  - OpenAI models: "gpt-4o", "gpt-4o-mini", "gpt-4-turbo", "gpt-3.5-turbo"
  - Anthropic models: "claude-3-opus-20240229", "claude-3-sonnet-20240229", "claude-3-haiku-20240307"
- `--system-message` or `-s`: System message to set the behavior of the assistant (default provides instructions about available tools and expected behavior)
- `--enable-tools` or `-e`: Enable tool usage (functions)
- `--inputs`: Comma-separated list of input sources (default: "stdin", options: "stdin", "webhook", "mqtt")
- `--outputs`: Comma-separated list of output destinations (default: "stdout", options: "stdout", "webhook", "mqtt")
- `--daemon`: Run as a daemon (fork to background)
- `--webhook-url`: URL to send webhook output to (required when using webhook output)
- `--mqtt-broker`: MQTT broker address (default: "broker.emqx.io")
- `--mqtt-port`: MQTT broker port (default: 1883)
- `--mqtt-input-topic`: MQTT topic to subscribe to for input (default: "agent/input")
- `--mqtt-output-topic`: MQTT topic to publish to for output (default: "agent/output")

## Available Tools

When tools are enabled (`--enable-tools`), the following tools are available:

### Weather Tool
Provides simulated weather information for a given location.

Example: "What's the weather like in San Francisco?"

### Calculator Tool
Evaluates mathematical expressions.

Example: "Calculate 2 + 2" or "What is the square root of 16?"

Supported operations:
- Addition (+)
- Subtraction (-)
- Multiplication (*)
- Division (/)
- Power (^)
- Square root (sqrt())
- Parentheses for grouping

## Input and Output Options

The application supports multiple input sources and output destinations:

### Input Sources

- `stdin`: Read user input from the standard input (default)
- `webhook`: Start an HTTP server that accepts POST requests with JSON payloads
- `mqtt`: Subscribe to an MQTT topic for input messages

You can specify multiple input sources using the `--inputs` option:

```
cargo run --release -- --inputs "stdin,webhook,mqtt"
```

### Webhook Server

When the webhook input source is enabled, the application starts an HTTP server on a random available port. The server accepts POST requests to the root path (`/`) with a JSON payload containing a message:

```json
{
  "message": "Your message here"
}
```

You can send messages to the webhook server using curl:

```
curl -X POST http://localhost:<PORT> -H "Content-Type: application/json" -d '{"message":"What is 2+2?"}'
```

The application will display the port number when it starts.

### Output Destinations

- `stdout`: Write output to the standard output (default)
- `webhook`: Send assistant responses to a webhook URL specified with `--webhook-url`
- `mqtt`: Publish assistant responses to an MQTT topic

When using the webhook or MQTT output destinations, only messages with the "assistant" role (the AI's responses) will be sent. The payload format is the same for both:

```json
{
  "role": "assistant",
  "content": "The AI's response",
  "timestamp": 1741352595
}
```

You can specify multiple output destinations using the `--outputs` option:

```
cargo run --release -- --outputs "stdout,webhook,mqtt" --webhook-url "http://localhost:8000/webhook"
```

To receive webhook outputs, you can use a simple HTTP server like netcat:

```
nc -l 8000
```

Or for a more robust solution, you can use a tool like [webhook.site](https://webhook.site/) or set up your own HTTP server.

## Daemon Mode

You can run the application as a daemon (in the background) using the `--daemon` flag:

```
cargo run --release -- --inputs webhook --daemon
```

When running in daemon mode:
- The application detaches from the terminal
- Logs are written to `/tmp/agent.out` and `/tmp/agent.err`
- A PID file is created at `/tmp/agent.pid`

This is particularly useful when running the application with webhook input, as it allows the server to run in the background.

## How it works

The application follows these steps:
1. Initializes the selected AI provider
2. Sets up input sources and output destinations
3. Maintains a conversation history
4. Reads messages from input sources
5. Sends user messages to the AI model
6. Processes any tool calls from the AI
7. Sends the AI's responses to output destinations
8. Repeats steps 4-7 until you exit

## Architecture

The application uses a modular architecture:

- `providers` module: Contains traits and implementations for different AI providers
  - `openai.rs`: OpenAI provider implementation
  - `anthropic.rs`: Anthropic provider implementation
- `cli` module: Handles command-line argument parsing
- `io` module: Handles input and output
  - `stdin.rs`: Input source for standard input
  - `stdout.rs`: Output destination for standard output
  - `webhook.rs`: Input source and output destination for webhooks
- `main.rs`: Orchestrates the application flow

## Adding New Providers

To add a new provider:
1. Create a new module in `src/providers/`
2. Implement the `Provider` trait
3. Update the `get_provider` function in `src/providers/mod.rs`

## Error Handling

The application uses the `anyhow` crate for error handling. If any errors occur during API calls or processing, they will be displayed with appropriate context.

## MQTT Support

The application supports MQTT for both input and output. By default, it connects to the public MQTT broker at broker.emqx.io on port 1883 without authentication.

### MQTT Input

When using MQTT as an input source, the application subscribes to the topic specified by `--mqtt-input-topic` (default: "agent/input"). It accepts messages in two formats:

1. Plain text messages, which are treated as user input
2. JSON messages with the following format:

```json
{
  "role": "user",
  "content": "Your message here",
  "timestamp": 1741352595
}
```

You can publish messages to the MQTT topic using any MQTT client. For example, using the mosquitto_pub command-line tool:

```
mosquitto_pub -h broker.emqx.io -t agent/input -m "What is 2+2?"
```

Or with a JSON payload:

```
mosquitto_pub -h broker.emqx.io -t agent/input -m '{"role":"user","content":"What is 2+2?","timestamp":1741352595}'
```

### MQTT Output

When using MQTT as an output destination, the application publishes the assistant's responses to the topic specified by `--mqtt-output-topic` (default: "agent/output"). The messages are published in JSON format:

```json
{
  "role": "assistant",
  "content": "The AI's response",
  "timestamp": 1741352595
}
```

You can subscribe to the MQTT topic using any MQTT client. For example, using the mosquitto_sub command-line tool:

```
mosquitto_sub -h broker.emqx.io -t agent/output
```

### Custom MQTT Configuration

You can customize the MQTT connection using the following options:

- `--mqtt-broker`: MQTT broker address (default: "broker.emqx.io")
- `--mqtt-port`: MQTT broker port (default: 1883)
- `--mqtt-input-topic`: MQTT topic to subscribe to for input (default: "agent/input")
- `--mqtt-output-topic`: MQTT topic to publish to for output (default: "agent/output")

Example:

```
cargo run --release -- --inputs mqtt --outputs mqtt --mqtt-broker "mqtt.example.com" --mqtt-port 8883 --mqtt-input-topic "my/input/topic" --mqtt-output-topic "my/output/topic"
```
