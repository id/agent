#!/bin/bash

# Wait for EMQX to be ready
echo "Waiting for EMQX to be ready..."
until $(curl --output /dev/null --silent --fail http://localhost:18083/api/v5/status); do
  printf '.'
  sleep 5
done
echo "EMQX is ready!"

# Login to EMQX and get token
echo "Logging in to EMQX..."
TOKEN=$(curl -s -X POST -H "Content-Type: application/json" \
  -d '{"username": "admin", "password": "public"}' \
  http://localhost:18083/api/v5/login | jq -r '.token')

# Create rule for user queries
curl -s -X POST -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d @emqx_rule.json \
  http://localhost:18083/api/v5/rules
