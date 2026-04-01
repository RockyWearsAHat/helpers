# AWS EventBridge Patterns — Event Bus Architecture, Rules, Targets, and Integration

## Event Bus Architecture

EventBridge is a serverless event router that decouples event producers from consumers. Events published to an event bus are evaluated against rules; matching events are routed to targets (Lambda, SNS, SQS, Step Functions, HTTP endpoints, etc.) in parallel.

The default event bus is used for AWS service events, custom application events, and SaaS integrations. Custom event buses isolate events for multi-tenant applications, separate teams, or domain-driven architectures.

Event buses enable event-driven architectures where services publish domain events without knowledge of consumers. This decoupling allows services to scale and evolve independently.

## Events and Event Patterns

An event is a JSON object representing something that happened (e.g., order placed, user registered, file uploaded). The structure includes metadata (time, source, version, account) and custom data.

```json
{
  "version": "0",
  "id": "12345-67890",
  "detail-type": "Order Placed",
  "source": "order-service",
  "account": "123456789012",
  "time": "2025-03-26T10:00:00Z",
  "region": "us-east-1",
  "detail": {
    "orderId": "ORDER-001",
    "customerId": "CUST-001",
    "amount": 99.99,
    "items": [...]
  }
}
```

### Event Patterns (Matching Rules)

Rules use event patterns (JSON objects) to match incoming events. Patterns support exact matching, prefix matching, existence checks, and value ranges.

```json
{
  "source": ["order-service"],
  "detail-type": ["Order Placed"],
  "detail": {
    "amount": [{ "numeric": [">", 100] }]
  }
}
```

This pattern matches events from order-service with detail-type "Order Placed" and amount greater than 100. Patterns are efficient; EventBridge evaluates millions of events per second.

## Rules and Targets

A rule specifies an event pattern and one or more targets. When an event matches, EventBridge invokes all targets in parallel. No guarantee of order when multiple targets are specified.

Targets include:

### Compute Targets

- **Lambda** — Function is invoked synchronously (blocking) or asynchronously. Supports retry policies and dead-letter queues (DLQs) for failed invocations.
- **Step Functions** — State machine execution is started. Enables complex orchestration workflows.
- **ECS Tasks** — Container task is launched. Useful for more resource-intensive processing.

### Messaging Targets

- **SNS** — Event published to topic; topic subscribers (email, SMS, Lambda, SQS, HTTP) receive notification.
- **SQS** — Event written to queue for batch processing. Decouples producers from consumers; supports dead-letter queues.
- **Kinesis** — Event written to stream for ordered, scalable multi-consumer processing.

### Integration Targets

- **API Destination** — HTTP POST to external HTTP endpoint. Supports authentication (OAuth, API keys, basic auth).
- **HTTP Endpoint (via API Gateway)** — Direct invocation of HTTP API.
- **EventBridge Archive** — Events stored for later replay.

### Other Targets

- **CloudWatch Logs** — Event logged for monitoring and debugging.
- **CodeBuild, CodePipeline** — CI/CD pipeline invocations.

## Dead-Letter Queues (DLQs)

If a target fails (returns error, timeout, or throws exception), EventBridge retries exponentially (default: 2 retries, ~30 seconds). If all retries fail, the event is sent to the associated DLQ (if configured) or dropped.

DLQs are typically SQS queues. Events in DLQs can be reviewed, debugged, and potentially replayed.

## Content-Based Filtering

Rules support filtering on event content. This reduces unnecessary target invocations and downstream processing cost.

```json
{
  "source": ["ecommerce"],
  "detail-type": ["Order"],
  "detail": {
    "status": ["completed"],
    "shippingMethod": ["premium"]
  }
}
```

Only completed orders with premium shipping trigger the associated target. This is cheaper than sending all orders to the target and filtering inside the handler.

## Scheduled Rules and EventBridge Scheduler

**Scheduled Rules (Legacy)** — Create rules that fire on a cron or rate schedule. Targets are invoked at specified intervals (e.g., every 5 minutes).

**EventBridge Scheduler (Recommended)** — Modern alternative to scheduled rules. Supports:

- Cron expressions (e.g., `cron(0 10 * * ? *)` for 10 AM daily)
- Rate expressions (e.g., `rate(30 minutes)`)
- One-time invocations
- Flexible time windows (attempt delivery within a window)
- Custom retry policies and maximum ages for failed invocations
- Timezone support

Scheduler is more flexible, scalable, and feature-rich than scheduled rules. Scheduled rules are considered legacy; new workloads should use Scheduler.

## EventBridge Pipes

EventBridge Pipes connect event sources directly to targets with optional filtering and enrichment. Pipes are designed for point-to-point integrations (one source → one target with transformations).

**Supported Sources**: SQS, SNS, Kinesis streams, DynamoDB streams, SQS FIFO, Kafka topics.

**Enrichment Step**: Optional. Before sending to the target, the event can be enriched by calling an external service (Lambda, API destination, step function). The enrichment response is merged with the event data.

**Filtering**: Events can be filtered at the pipe level (same matching logic as rules).

**Targets**: Lambda, Step Functions, SQS, SNS, Kinesis, HTTP endpoint, API Gateway, CloudWatch Logs.

### Example: SQS to Lambda with Enrichment

1. Order message arrives in SQS queue.
2. Pipe triggers and filters (orders >$100 only).
3. Enrichment step calls external API to fetch customer VIP status.
4. Enriched event (order + VIP status) sent to Lambda handler.
5. Lambda processes order with VIP context.

Pipes reduce the need for boilerplate orchestration code. Tradeoff: Limited to one-to-one integrations; complex multi-target routing should use event buses and rules.

## Schema Registry

EventBridge Schema Registry stores event schemas (JSON Schema documents) and enables code generation (Python, Java, Node.js, Go, etc.) to represent events as strongly typed objects.

**Discovery**: EventBridge can detect and register event schemas automatically as events arrive (schema discovery mode).

**Custom Registries**: Create registries to organize schemas by domain or team.

**Versioning**: Each schema can have versions; clients can generate code against specific versions.

Schema Registry reduces the friction of event-driven development (event structure is documented and validated) but adds operational complexity if coupled tightly to code generation.

## Cross-Account Events

Events can be published to event buses in other AWS accounts using `PutEvents` API with explicit event bus ARN. The target account must have a resource-based policy allowing the source account to publish.

```
Source Account: 111111111111
Target Account: 222222222222

Source Account publishes to: arn:aws:events:us-east-1:222222222222:event-bus/partner-events

Target Account policy allows: Principal 111111111111 to PutEvents on partner-events event bus
```

Cross-account events enable multi-tenant SaaS platforms, partner integrations, and organizational event aggregation (all accounts → central account for compliance and audit).

## Archive and Replay

Events can be archived (stored) to an S3-compatible sink for compliance, debugging, or later replay. Replayed events are re-evaluated against rules and routed to targets as if they had just arrived.

Replay is useful for:

- **Correcting bugs** — If a rule or target was misconfigured, replay events through the corrected infrastructure.
- **Backfilling** — Reprocess events for a new consumer or historical analysis.
- **Compliance and audit** — Retain immutable records of all events.

Archives are eventually consistent (events may take time to appear in the archive).

## Event-Driven Architecture Patterns

### Choreography vs. Orchestration

**Choreography** — Services independently listen to events and react. Order service publishes `OrderCreated`; Inventory service listens and decrements stock; Shipping service listens and creates shipment. No central coordinator. Looser coupling, but harder to debug and understand.

**Orchestration** — Central orchestrator (Step Functions, custom service) publishes events and waits for responses. More visibility and control, but introduces a coordinator bottleneck.

EventBridge supports both. Use event buses for choreography; use Pipes or rules + Lambda for simpler orchestration.

### Temporal Decoupling

Events decouple services in time. Producer publishes and moves on; consumer processes asynchronously. Useful for systems where consumers may be temporarily unavailable or slow.

### Event Sourcing Integration

EventBridge works with event sourcing systems where all state changes are events. Events are stored in an event store and also published to EventBridge for real-time consumer notification.

## Cost Optimization

- Filter at the rule level to avoid unnecessary target invocations.
- Use SQS/Kinesis targets for batch processing rather than invoking hundreds of Lambda functions.
- Leverage Pipes' enrichment to avoid redundant API calls in multiple targets.
- Archive selectively (not all events need retention).

## Managed Rules and AWS Service Integration

AWS services create managed rules in your account to support features (e.g., EventBridge Pipes creates rules to forward events). These rules should not be deleted unless the feature is no longer needed; deleting them breaks the feature.

## See Also

Related: `architecture-event-driven`, `cloud-aws-messaging`, `cloud-aws-serverless`, `patterns-event-driven`