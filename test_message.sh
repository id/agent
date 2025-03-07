#!/bin/bash

# Check if mosquitto-clients is installed
if ! command -v mosquitto_pub &> /dev/null; then
    echo "mosquitto-clients is not installed. Installing..."
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        sudo apt-get update && sudo apt-get install -y mosquitto-clients
    elif [[ "$OSTYPE" == "darwin"* ]]; then
        brew install mosquitto
    else
        echo "Please install mosquitto-clients manually for your OS."
        exit 1
    fi
fi

echo "Sending test message to MQTT broker..."

# Generate a random user ID
USER_ID="user_$(date +%s)"

# Send a test message
mosquitto_pub -h localhost -t "agent/user/query" -m "{\"query\":\"What can you tell me about MQTT?\",\"user_id\":\"$USER_ID\",\"client_type\":\"test-script\"}"

echo "Message sent! Listening for responses..."

# Listen for responses
mosquitto_sub -h localhost -t "agent/user/response" -v &
SUB_PID=$!

echo "Press Ctrl+C to stop listening"

# Wait for user to press Ctrl+C
trap "kill $SUB_PID; echo 'Stopped listening.'; exit 0" INT
wait 