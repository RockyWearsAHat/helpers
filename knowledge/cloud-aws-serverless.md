# AWS Serverless Services

## Lambda + API Gateway

### API Gateway Types

| Feature       | REST API                                              | HTTP API                             |
| ------------- | ----------------------------------------------------- | ------------------------------------ |
| Cost          | ~$3.50/million                                        | ~$1.00/million                       |
| Latency       | Higher                                                | 60% lower                            |
| Auth          | IAM, Cognito, Lambda authorizer, API keys             | IAM, Cognito, JWT, Lambda authorizer |
| Features      | Request validation, WAF, caching, canary, usage plans | Simpler, fewer features              |
| Protocols     | REST                                                  | REST, WebSocket via separate         |
| Output format | Full control                                          | Simpler payload format v2.0          |

HTTP API is the default choice unless you need REST API-specific features (WAF, request validation, caching, API keys, usage plans).

### Integration Types

| Type               | Behavior                                                           |
| ------------------ | ------------------------------------------------------------------ |
| Lambda proxy       | API GW passes full request to Lambda, Lambda returns full response |
| Lambda (non-proxy) | Request/response mapping templates (VTL), API GW transforms        |
| HTTP proxy         | Pass-through to HTTP endpoint                                      |
| AWS service        | Direct integration (Step Functions, SQS, DynamoDB) without Lambda  |
| Mock               | Return hardcoded response (testing, CORS preflight)                |

Direct AWS service integrations avoid Lambda entirely:

```yaml
# API Gateway → SQS (no Lambda needed)
x-amazon-apigateway-integration:
  type: aws
  uri: arn:aws:apigateway:us-east-1:sqs:action/SendMessage
  requestParameters:
    integration.request.querystring.QueueUrl: "'https://sqs.us-east-1.amazonaws.com/123/queue'"
    integration.request.querystring.MessageBody: method.request.body
```

### Lambda Authorizers

- **Token-based**: Receives bearer token, returns IAM policy. Cached by token.
- **Request-based**: Receives full request context (headers, query params). Cached by identity sources.
- Authorization caching: 0-3600 seconds. Cached policy applies to all methods sharing the authorizer.

### Stage Variables and Deployments

- Stage variables: Environment-specific configuration (`dev`, `staging`, `prod`)
- Canary deployments: Route percentage of traffic to new deployment
- Usage plans + API keys: Rate limiting and quotas per client

## Step Functions

### Amazon States Language (ASL)

```json
{
  "StartAt": "ProcessOrder",
  "States": {
    "ProcessOrder": {
      "Type": "Task",
      "Resource": "arn:aws:lambda:us-east-1:123:function:process-order",
      "Next": "CheckInventory",
      "Retry": [
        {
          "ErrorEquals": ["ServiceUnavailable"],
          "IntervalSeconds": 2,
          "MaxAttempts": 3,
          "BackoffRate": 2.0
        }
      ],
      "Catch": [
        {
          "ErrorEquals": ["States.ALL"],
          "Next": "HandleError"
        }
      ]
    },
    "CheckInventory": {
      "Type": "Choice",
      "Choices": [
        {
          "Variable": "$.inventory",
          "NumericGreaterThan": 0,
          "Next": "FulfillOrder"
        }
      ],
      "Default": "OutOfStock"
    },
    "FulfillOrder": {
      "Type": "Parallel",
      "Branches": [
        {
          "StartAt": "ChargeCard",
          "States": {
            "ChargeCard": { "Type": "Task", "Resource": "...", "End": true }
          }
        },
        {
          "StartAt": "ShipItem",
          "States": {
            "ShipItem": { "Type": "Task", "Resource": "...", "End": true }
          }
        }
      ],
      "Next": "Complete"
    },
    "Complete": { "Type": "Succeed" },
    "OutOfStock": {
      "Type": "Fail",
      "Error": "OutOfStock",
      "Cause": "Item not available"
    },
    "HandleError": { "Type": "Fail" }
  }
}
```

### State Types

| State    | Purpose                                                          |
| -------- | ---------------------------------------------------------------- |
| Task     | Execute work (Lambda, SDK integration, ECS, Fargate, Glue, etc.) |
| Choice   | Branching logic (if/else)                                        |
| Parallel | Execute branches concurrently                                    |
| Map      | Iterate over array (Inline for small, Distributed for millions)  |
| Wait     | Delay (seconds or timestamp)                                     |
| Pass     | Transform data, inject fixed result                              |
| Succeed  | Terminal success                                                 |
| Fail     | Terminal failure with error/cause                                |

### Standard vs Express Workflows

| Feature           | Standard                                    | Express                                    |
| ----------------- | ------------------------------------------- | ------------------------------------------ |
| Duration          | Up to 1 year                                | Up to 5 minutes                            |
| Execution model   | Exactly-once                                | At-least-once (async), at-most-once (sync) |
| Execution history | Full, in console + API                      | CloudWatch Logs only                       |
| Pricing           | Per state transition ($0.025/1000)          | Per execution + duration + memory          |
| Max rate          | 2000/sec start, sustained higher            | 100,000/sec                                |
| Use case          | Long-running, orchestration, human approval | High-volume, short, event processing       |

SDK integrations: Step Functions can call 220+ AWS services directly without Lambda. Use `.sync` suffix for synchronous (wait for completion), `.waitForTaskToken` for callback pattern.

### Distributed Map

Process millions of items from S3:

- Input: S3 objects, S3 inventory, JSON array
- Up to 10,000 concurrent child executions
- Each child is a separate Standard or Express workflow
- Built-in batching, error handling, result aggregation
- Use for: ETL, data validation, bulk operations

## EventBridge

### Rules

Match events and route to targets:

```json
{
  "source": ["aws.ec2"],
  "detail-type": ["EC2 Instance State-change Notification"],
  "detail": {
    "state": ["terminated", "stopped"]
  }
}
```

Up to 300 rules per event bus. Each rule can have up to 5 targets. Targets: Lambda, Step Functions, SQS, SNS, Kinesis, API destinations (HTTP), and more.

Content-based filtering:

- Prefix matching: `{"prefix": "prod-"}`
- Numeric matching: `{"numeric": [">", 0, "<=", 100]}`
- Exists: `{"exists": true}` or `{"exists": false}`
- Anything-but: `{"anything-but": ["error", "fatal"]}`
- Suffix: `{"suffix": ".png"}`

### EventBridge Pipes

Point-to-point integration with optional filtering, enrichment, and transformation:

```
Source → (Filter) → (Enrichment) → Target
```

Sources: SQS, Kinesis, DynamoDB Streams, Kafka, MQ.
Enrichment: Lambda, Step Functions, API Gateway, API destinations.
Targets: 15+ targets including Lambda, Step Functions, SQS, ECS.

Use case: Replace Lambda glue code. Instead of SQS → Lambda → process → send to Step Functions, use SQS → Pipe (filter + enrich) → Step Functions.

### EventBridge Scheduler

Cron and one-time scheduled events:

- Cron: `cron(0 12 * * ? *)` — every day at noon UTC
- Rate: `rate(5 minutes)`
- One-time: Specific datetime
- Flexible time window: Spread invocations to reduce spikes
- Retry policy: Max attempts + max event age
- Dead-letter queue for failed deliveries
- Timezone-aware schedules

## SQS

### Standard vs FIFO

| Feature    | Standard                        | FIFO                                                        |
| ---------- | ------------------------------- | ----------------------------------------------------------- |
| Throughput | Unlimited                       | 300 msg/sec (batch: 3000) or high-throughput mode: 70K+/sec |
| Ordering   | Best-effort                     | Strict within message group                                 |
| Delivery   | At-least-once (rare duplicates) | Exactly-once (5-minute dedup window)                        |
| Naming     | Any                             | Must end in `.fifo`                                         |

### Dead Letter Queues (DLQ)

Capture messages that fail processing after N attempts:

```json
{
  "RedrivePolicy": {
    "deadLetterTargetArn": "arn:aws:sqs:us-east-1:123:my-queue-dlq",
    "maxReceiveCount": 3
  }
}
```

DLQ redrive: Move messages from DLQ back to source queue for reprocessing (console or API).

### Lambda Triggers (Event Source Mapping)

- Batch size: 1-10,000 messages
- Batch window: 0-300 seconds (wait to accumulate batch)
- Concurrent batches: Up to 1000 (scales automatically for standard, limited by message groups for FIFO)
- Report batch item failures: Return partial failures instead of failing entire batch
- Filtering: FilterCriteria to process only matching messages (saves Lambda invocations)

```json
{
  "FilterCriteria": {
    "Filters": [{ "Pattern": "{\"body\": {\"type\": [\"order\"]}}" }]
  }
}
```

### Key Settings

| Setting            | Default | Notes                                             |
| ------------------ | ------- | ------------------------------------------------- |
| Visibility timeout | 30 sec  | Set to 6x Lambda timeout                          |
| Message retention  | 4 days  | Max 14 days                                       |
| Max message size   | 256 KB  | Use S3 for larger (Extended Client Library)       |
| Delay queue        | 0 sec   | Up to 15 min delay before visible                 |
| Long polling       | 0 sec   | Set to 20 sec to reduce empty responses and costs |

## SNS

### Message Filtering

Filter policies on subscriptions reduce unnecessary processing:

```json
{
  "store": ["electronics"],
  "price_usd": [{ "numeric": [">", 100] }],
  "event": [{ "anything-but": "test" }]
}
```

Without filter: All subscribers get all messages. With filter: Each subscriber gets only matching messages.

Filter policy scope: `MessageAttributes` (default) or `MessageBody` (filter on message content directly).

### Fanout Pattern

SNS → multiple SQS queues:

```
Order Placed (SNS Topic)
  ├── SQS: Inventory Service
  ├── SQS: Payment Service
  ├── SQS: Notification Service
  └── Lambda: Analytics
```

Each subscriber processes independently. Failed processing in one doesn't affect others. SQS provides buffering and retry.

### Delivery Features

- FIFO topics: Ordered, deduplicated delivery to FIFO SQS subscribers
- Message archiving: Archive to S3 via Kinesis Data Firehose
- SMS, Email, HTTP/S, Lambda, SQS, Kinesis Data Firehose targets
- Delivery retry policies: Custom retry backoff per subscription protocol

## AppSync

Managed GraphQL API service:

- Real-time subscriptions via WebSocket
- Offline support (Amplify clients)
- Resolvers: Lambda, DynamoDB, RDS, HTTP, OpenSearch, EventBridge, Step Functions
- Pipeline resolvers: Chain multiple data sources
- JavaScript resolvers (replaced VTL for most use cases)
- Caching: Full API or per-resolver, up to 1 hour TTL
- Merged APIs: Combine multiple source APIs into one endpoint
- Authorization: API key, Cognito, IAM, OIDC, Lambda

## SAM and CDK

### SAM (Serverless Application Model)

CloudFormation extension for serverless:

```yaml
AWSTemplateFormatVersion: "2010-09-09"
Transform: AWS::Serverless-2016-10-31

Globals:
  Function:
    Runtime: python3.12
    Timeout: 30
    MemorySize: 256
    Architecture: arm64

Resources:
  ApiFunction:
    Type: AWS::Serverless::Function
    Properties:
      Handler: app.handler
      CodeUri: src/
      Events:
        Api:
          Type: Api
          Properties:
            Path: /items
            Method: get
      Policies:
        - DynamoDBReadPolicy:
            TableName: !Ref ItemsTable

  ItemsTable:
    Type: AWS::Serverless::SimpleTable
```

`sam local invoke` — test Lambda locally. `sam local start-api` — local API Gateway.

### CDK Patterns

Higher-level constructs for common patterns:

```typescript
// API Gateway + Lambda + DynamoDB
const table = new dynamodb.Table(this, "Items", {
  partitionKey: { name: "id", type: dynamodb.AttributeType.STRING },
  billingMode: dynamodb.BillingMode.PAY_PER_REQUEST,
  removalPolicy: RemovalPolicy.DESTROY,
});

const handler = new lambda.Function(this, "Handler", {
  runtime: lambda.Runtime.NODEJS_20_X,
  handler: "index.handler",
  code: lambda.Code.fromAsset("lambda"),
  environment: { TABLE_NAME: table.tableName },
  architecture: lambda.Architecture.ARM_64,
});

table.grantReadWriteData(handler);

const api = new apigw.LambdaRestApi(this, "Api", { handler });
```

CDK vs SAM: SAM is simpler for pure serverless. CDK is better for complex infrastructure, reusable patterns, and multiple services beyond serverless. They can be used together (`sam build` supports CDK apps).
