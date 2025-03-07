#!/bin/bash

echo "Cleaning up MQTT Agent Demo..."

# Stop and remove containers
echo "Stopping and removing containers..."
docker compose down

# Remove the agent image
echo "Removing agent image..."
docker rmi mqtt-agent:latest

# Remove EMQX data directories
echo "Removing EMQX data directories..."
rm -rf emqx_data emqx_log emqx_etc

echo "Cleanup complete!" 