#!/bin/bash

# Check if Docker is installed
if ! command -v docker &> /dev/null; then
    echo "Docker is not installed. Please install Docker and Docker Compose first."
    exit 1
fi

# Check if OpenAI API key is set
if [ -z "$OPENAI_API_KEY" ]; then
    echo "OpenAI API key is not set. Please set it with:"
    echo "export OPENAI_API_KEY=your_api_key_here"
    exit 1
fi

echo "Setting up MQTT Agent Demo..."

# Make scripts executable
chmod +x setup_emqx_rules.sh
chmod +x test_message.sh

# Build the agent image first
echo "Building the agent image..."
docker compose build agent-builder

# Start the services
echo "Starting services..."
docker compose up -d emqx agent1 agent2

# Wait for EMQX to start
echo "Waiting for EMQX to start (this may take a minute)..."
sleep 30

# Set up EMQX rules
echo "Setting up EMQX rules..."
./setup_emqx_rules.sh

echo "Demo setup complete!"
echo ""
echo "To test the demo, run:"
echo "./test_message.sh"
echo ""
echo "To view the EMQX dashboard, visit:"
echo "http://localhost:18083"
echo "Username: admin"
echo "Password: public"
echo ""
echo "To view logs from the agents:"
echo "docker logs data-processor-agent"
echo "docker logs response-generator-agent"
echo ""
echo "To clean up the demo:"
echo "./cleanup_demo.sh" 
