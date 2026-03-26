# Serverless Patterns

## FaaS Design

### Cold Starts

When a function hasn't been invoked recently, the platform must provision a new execution environment: download code, start runtime, initialize dependencies. This is the cold start.

| Runtime | Typical Cold Start (AWS Lambda) |
| ------- | ------------------------------- |
| Python  | 200-500ms                       |
| Node.js | 200-400ms                       |
| Go      | 50-100ms (compiled binary)      |
| Java    | 1-5s (JVM startup)              |
| .NET    | 500ms-2s                        |
| Rust    | 50-100ms (compiled binary)      |

### Reducing Cold Starts

| Strategy                    | How                                        | Trade-off                  |
| --------------------------- | ------------------------------------------ | -------------------------- |
| **Provisioned concurrency** | Pre-warm N instances                       | Cost (always-on instances) |
| **Smaller packages**        | Tree-shake, minimize dependencies          | Development effort         |
| **SnapStart** (Java/Lambda) | Snapshot after init, restore from snapshot | Some state constraints     |
| **Compile to native**       | GraalVM native image, Go, Rust             | Build complexity           |
| **Keep functions warm**     | Scheduled ping (hacky)                     | Cost, doesn't scale        |
| **Lazy initialization**     | Defer non-critical init to first request   | First-request latency      |

### Function Design Patterns

```python
# Initialize OUTSIDE the handler (reused across invocations)
import boto3
dynamodb = boto3.resource('dynamodb')
table = dynamodb.Table('orders')

def handler(event, context):
    # Handler logic only — no heavy init
    order_id = event['pathParameters']['orderId']
    response = table.get_item(Key={'id': order_id})
    return {
        'statusCode': 200,
        'body': json.dumps(response.get('Item', {}))
    }
```

### Warm Pools

After a cold start, the execution environment stays alive for reuse. Subsequent invocations are "warm" — no init overhead. AWS Lambda keeps environments for ~15 minutes of inactivity.

## Event-Driven Architectures

Serverless is inherently event-driven. Functions trigger on events:

| Event Source      | Trigger                | Use Case                      |
| ----------------- | ---------------------- | ----------------------------- |
| API Gateway       | HTTP request           | REST/GraphQL APIs             |
| S3                | Object created/deleted | Image processing, ETL         |
| SQS               | Message available      | Async work processing         |
| SNS               | Notification           | Fan-out to multiple consumers |
| EventBridge       | Event pattern match    | Cross-service event routing   |
| DynamoDB Streams  | Table change           | Change data capture           |
| Kinesis           | Stream record          | Real-time data processing     |
| CloudWatch Events | Scheduled              | Cron jobs, periodic tasks     |
| Cognito           | Auth event             | Post-signup actions           |

## API Patterns

### API Gateway + Lambda

```
Client → API Gateway → Lambda → DynamoDB
                          │
                    (authorizer Lambda)
```

```yaml
# serverless.yml (Serverless Framework)
functions:
  getOrder:
    handler: orders.get
    events:
      - httpApi:
          path: /orders/{id}
          method: get
  createOrder:
    handler: orders.create
    events:
      - httpApi:
          path: /orders
          method: post
```

### Function URLs (Lambda)

Direct HTTPS endpoint without API Gateway. Simpler, cheaper for single-function APIs, but no built-in throttling, request validation, or multiple routes.

```
Client → Lambda Function URL → Lambda
```

Best for webhooks, simple callbacks, and internal services where API Gateway features aren't needed.

### API Design Considerations

- **One function per route**: Fine-grained, independent scaling and deployment
- **Monolithic handler**: One function routes all requests. Simpler deployment, bigger cold start.
- **Recommended**: Start with one function per route (or small group), avoid the Lambda monolith

## Data Patterns

### DynamoDB Single-Table Design

The dominant pattern in serverless: model all entities in one table with composite keys.

```
PK              SK                  Data
USER#123        PROFILE             {name, email, ...}
USER#123        ORDER#456           {status, items, total}
USER#123        ORDER#789           {status, items, total}
ORDER#456       METADATA            {timestamp, shipping}
ORDER#456       ITEM#001            {product, quantity, price}
```

**Access patterns drive the design**: Define all queries before designing the table. Use GSIs for alternative access patterns.

### Aurora Serverless

Relational database that scales to zero. Use when:

- You need SQL/relational queries
- Infrequent or bursty traffic (scales down to zero ACU)
- Existing SQL-based applications moving to serverless

**Caveat**: Cold start can be 25-30s when scaling from zero. Not suitable for latency-sensitive APIs unless minimum capacity is configured.

### S3 Event Processing

```
S3 upload → S3 Event Notification → Lambda → Process file → Write results
                                       │
                                 (image resize, CSV parse, ML inference)
```

Use for ETL, media processing, log analysis. Fan out with SQS for high volume.

## Orchestration

### AWS Step Functions

Visual workflow orchestrator for serverless:

```json
{
  "StartAt": "ValidateInput",
  "States": {
    "ValidateInput": {
      "Type": "Task",
      "Resource": "arn:aws:lambda:...:validate",
      "Next": "Parallel Processing"
    },
    "Parallel Processing": {
      "Type": "Parallel",
      "Branches": [
        {
          "StartAt": "ProcessPayment",
          "States": {
            "ProcessPayment": { "Type": "Task", "Resource": "...", "End": true }
          }
        },
        {
          "StartAt": "ReserveInventory",
          "States": {
            "ReserveInventory": {
              "Type": "Task",
              "Resource": "...",
              "End": true
            }
          }
        }
      ],
      "Next": "Confirmation"
    },
    "Confirmation": {
      "Type": "Task",
      "Resource": "arn:aws:lambda:...:confirm",
      "End": true
    }
  }
}
```

Patterns: sequential, parallel, choice (branching), map (iterate), wait, retry, catch.

### Azure Durable Functions

Code-first orchestration in C#/JS/Python:

```csharp
[FunctionName("OrderOrchestrator")]
public static async Task RunOrchestrator(
    [OrchestrationTrigger] IDurableOrchestrationContext context)
{
    var order = context.GetInput<Order>();

    await context.CallActivityAsync("ValidateOrder", order);
    await context.CallActivityAsync("ProcessPayment", order);

    // Fan-out/fan-in
    var tasks = order.Items.Select(
        item => context.CallActivityAsync("FulfillItem", item));
    await Task.WhenAll(tasks);

    await context.CallActivityAsync("SendConfirmation", order);
}
```

## Practical Limits (AWS Lambda)

| Limit                       | Value                           |
| --------------------------- | ------------------------------- |
| Max execution time          | 15 minutes                      |
| Memory                      | 128 MB – 10,240 MB              |
| Ephemeral storage           | 512 MB – 10,240 MB              |
| Invocation payload (sync)   | 6 MB                            |
| Invocation payload (async)  | 256 KB                          |
| Concurrent executions       | 1,000 (default, can increase)   |
| Deployment package (zipped) | 50 MB direct, 250 MB unzipped   |
| Container image             | 10 GB                           |
| Environment variables       | 4 KB total                      |
| Layers                      | 5 layers, 250 MB total unzipped |

## Cost Model

### Pay-Per-Invocation

```
Cost = (Number of requests × $0.20/million)
     + (GB-seconds × $0.0000166667)
     + (API Gateway requests × $1.00/million)
```

### When Serverless Is Cheaper

```
         Cost
          │
          │    ╱ Always-on server
          │   ╱
          │  ╱
          │ ╱────── Serverless
          │╱
          ├─────────────────── Traffic
          │
     Low traffic = serverless wins
     High steady traffic = server wins
     Crossover ≈ 1-5M requests/month (depends on duration)
```

**Breakeven**: Roughly $20-50/month equivalent. If a Lambda function runs constantly at high concurrency, a container or VM is cheaper.

### Hidden Costs

- API Gateway: $1-3.50/million requests
- Data transfer: $0.09/GB out
- CloudWatch Logs: $0.50/GB ingested
- Step Functions: $0.025/1000 state transitions
- DynamoDB: Read/write capacity units

## Testing

### Local Emulation

| Tool                   | What It Emulates                                      |
| ---------------------- | ----------------------------------------------------- |
| **SAM CLI**            | Lambda, API Gateway, DynamoDB (local)                 |
| **Serverless Offline** | API Gateway + Lambda (Serverless Framework)           |
| **LocalStack**         | AWS services (broad but imperfect fidelity)           |
| **DynamoDB Local**     | DynamoDB only (official AWS)                          |
| **SST Dev**            | Live Lambda dev mode (code syncs, real AWS resources) |

### Integration Testing

```python
# Test against real AWS (staging environment)
def test_create_order_integration():
    response = requests.post(
        f"{API_URL}/orders",
        json={"items": [{"id": "prod-1", "qty": 2}]},
        headers={"Authorization": f"Bearer {test_token}"}
    )
    assert response.status_code == 201
    order = response.json()

    # Verify downstream effects (async, may need polling)
    eventually(lambda: get_order(order["id"])["status"] == "pending")
```

### Unit Testing

Test business logic independently of Lambda handler:

```python
# Separate business logic from handler wiring
def process_order(order_data, db_client, event_bus):
    """Pure business logic — testable without Lambda."""
    validate_order(order_data)
    order = create_order(order_data)
    db_client.save(order)
    event_bus.publish(OrderCreated(order.id))
    return order

# Handler is thin — just wiring
def handler(event, context):
    body = json.loads(event['body'])
    order = process_order(body, dynamodb_client, eventbridge_client)
    return {'statusCode': 201, 'body': json.dumps(order.to_dict())}
```

## Frameworks

| Framework                | Focus                         | Strengths                            |
| ------------------------ | ----------------------------- | ------------------------------------ |
| **Serverless Framework** | Multi-cloud, plugin ecosystem | Largest community, most plugins      |
| **SST**                  | AWS, TypeScript-first         | Live Lambda dev, CDK constructs      |
| **SAM**                  | AWS official                  | CloudFormation-native, testing tools |
| **CDK**                  | AWS infrastructure-as-code    | Programmatic, composable             |
| **Pulumi**               | Multi-cloud, real languages   | No YAML/JSON, general-purpose IaC    |

## Anti-Patterns

### Lambda Monolith

One function handling all routes with internal routing. Defeats the purpose: no independent scaling, large cold start, big blast radius.

### Synchronous Chains

Lambda → Lambda → Lambda → Lambda. Each function waiting for the next. Use Step Functions or async messaging instead.

### No Timeout Strategy

Functions that call external services without timeouts, running until the 15-minute limit. Always set timeouts shorter than the Lambda timeout.

### Over-Orchestration

Step Functions for simple linear workflows that could be a single function. Use orchestration when you need branching, retries, parallel execution, or human-in-the-loop.

### Ignoring Idempotency

Lambda can retry on failure. If your function isn't idempotent, retries cause duplicate side effects (double charges, duplicate emails). Use idempotency keys or conditional writes.
