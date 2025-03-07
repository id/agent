# MQTT Agent Demo

This demo showcases a multi-agent system using MQTT for communication. It consists of:

1. **EMQX MQTT Broker**: Handles message routing and rule-based processing
2. **Data Processor Agent**: Analyzes and enriches incoming messages
3. **Response Generator Agent**: Creates personalized responses based on processed data

## Architecture

```
User → agent/user/query → [EMQX Rule Engine] → agent/data-processor/input → [Agent 1] 
→ agent/response-generator/input → [Agent 2] → agent/user/response → User
```

## Setup Instructions

### Prerequisites

- Docker and Docker Compose
- OpenAI API key (set as environment variable)

### Running the Demo

1. Set your OpenAI API key:
   ```bash
   export OPENAI_API_KEY=your_api_key_here
   ```

2. Run the setup script:
   ```bash
   chmod +x setup_demo.sh
   ./setup_demo.sh
   ```

   This script will:
   - Build the agent Docker image
   - Start the EMQX broker and agent containers
   - Set up the EMQX rules for message routing

3. Test the system:
   ```bash
   chmod +x test_message.sh
   ./test_message.sh
   ```

## Message Flow

1. User sends a JSON message to `agent/user/query`
2. EMQX rule engine adds metadata and forwards to `agent/data-processor/input`
3. Data Processor Agent analyzes the message and forwards to `agent/response-generator/input`
4. Response Generator Agent creates a response and sends it to `agent/user/response`

## Message Format

### User Query (Input)
```json
{
  "query": "What is the weather like today?",
  "user_id": "user123",
  "client_type": "mobile"
}
```

### Data Processor Output
```json
{
  "query": "What is the weather like today?",
  "user_id": "user123",
  "client_type": "mobile",
  "metadata_clientid": "mqtt-client-123",
  "metadata_username": "user",
  "metadata_peerhost": "192.168.1.100",
  "metadata_topic": "agent/user/query",
  "metadata_qos": 1,
  "metadata_timestamp": 1679012345678,
  "analysis": {
    "sentiment": "neutral",
    "category": "question",
    "processed_at": 1679012345789
  }
}
```

### Response Generator Output
```json
{
  "original_query": "What is the weather like today?",
  "response": "Based on your location (detected from your IP: 192.168.1.100), the weather today is sunny with a high of 75°F. Have a great day, user123!",
  "metadata": {
    "client_id": "mqtt-client-123",
    "client_type": "mobile",
    "query_category": "question",
    "query_sentiment": "neutral"
  },
  "response_id": "resp-8a7b6c5d",
  "timestamp": 1679012346012
}
```

## Docker Compose Configuration

The demo uses Docker Compose to set up the following services:

1. **EMQX**: MQTT broker with a dashboard for monitoring
2. **Agent Builder**: Builds the agent Docker image
3. **Agent 1 (Data Processor)**: Processes incoming user queries
4. **Agent 2 (Response Generator)**: Generates responses based on processed data

## EMQX Dashboard

Access the EMQX dashboard at http://localhost:18083 with:
- Username: admin
- Password: public

You can view active connections, message flow, and rule engine statistics.

## Agent Configuration

Each agent uses a separate configuration file:

- `agent1_config.yaml`: Configuration for the Data Processor agent
- `agent2_config.yaml`: Configuration for the Response Generator agent

These files specify the MQTT topics, model settings, and system messages for each agent.

## Cleaning Up

To clean up the demo environment:

```bash
chmod +x cleanup_demo.sh
./cleanup_demo.sh
```

This will stop and remove all containers, delete the agent image, and remove EMQX data directories.

## Customizing the Demo

You can customize the demo by:

1. Modifying the agent configuration files
2. Updating the EMQX rule in `emqx_rule.json`
3. Adding more agents to the system by extending the Docker Compose file
4. Implementing custom processing logic in the agents

## Troubleshooting

If you encounter issues:

1. Check the logs of each container:
   ```bash
   docker logs emqx
   docker logs data-processor-agent
   docker logs response-generator-agent
   ```

2. Verify the EMQX rules are properly set up by checking the dashboard

3. Ensure your OpenAI API key is correctly set in the environment

4. Check network connectivity between containers using the MQTT network 