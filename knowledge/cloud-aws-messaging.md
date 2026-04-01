# AWS Messaging & Event Services

## SQS (Simple Queue Service)

### Queue Types

| Feature       | Standard                            | FIFO                                    |
| ------------- | ----------------------------------- | --------------------------------------- |
| Throughput    | Unlimited                           | 3,000 msg/s with batching (300 without) |
| Ordering      | Best-effort                         | Strict per message group                |
| Delivery      | At-least-once (possible duplicates) | Exactly-once                            |
| Deduplication | None                                | Content-based or explicit ID            |
| Name          | Any                                 | Must end in `.fifo`                     |

### Message Lifecycle

```
Producer → Send → Queue → Receive → Visibility Timeout → Delete
                                  ↓ (timeout expires, not deleted)
                              Re-queued → Retry
                                  ↓ (maxReceiveCount exceeded)
                              Dead Letter Queue
```

**Visibility timeout**: 0s–12h (default 30s). While a message is "in flight" (received but not deleted), it's invisible to other consumers. If the consumer crashes, the message reappears after the timeout.

### Batching

```python
import boto3

sqs = boto3.client('sqs')

# Batch send (up to 10 messages, 256KB total)
response = sqs.send_message_batch(
    QueueUrl=queue_url,
    Entries=[
        {'Id': '1', 'MessageBody': json.dumps(order1), 'MessageGroupId': 'orders'},
        {'Id': '2', 'MessageBody': json.dumps(order2), 'MessageGroupId': 'orders',
         'MessageDeduplicationId': order2['id']},
    ]
)
# Check response['Failed'] for per-message errors

# Batch receive (up to 10 messages)
messages = sqs.receive_message(
    QueueUrl=queue_url,
    MaxNumberOfMessages=10,
    WaitTimeSeconds=20,        # Long polling (0 = short polling)
    VisibilityTimeout=60,
    MessageAttributeNames=['All']
)

# Batch delete
sqs.delete_message_batch(
    QueueUrl=queue_url,
    Entries=[{'Id': str(i), 'ReceiptHandle': m['ReceiptHandle']}
             for i, m in enumerate(messages.get('Messages', []))]
)
```

**Long polling** (`WaitTimeSeconds` > 0): reduces empty responses and API calls. Long polling is generally preferred in production (20s recommended).

### FIFO Deduplication

Two methods:

1. **Content-based**: SQS hashes the message body — identical bodies within the 5-minute dedup window are dropped
2. **Explicit ID**: provide `MessageDeduplicationId` — you control what's considered a duplicate

**Message Group ID**: messages with the same group ID are delivered in order. Different group IDs are processed in parallel. For maximum throughput, use many distinct group IDs.

### Partial Batch Failure (Lambda)

```python
def handler(event, context):
    batch_item_failures = []

    for record in event['Records']:
        try:
            process_message(record)
        except Exception:
            batch_item_failures.append({
                'itemIdentifier': record['messageId']
            })

    return {'batchItemFailures': batch_item_failures}
```

Requires `FunctionResponseTypes: [ReportBatchItemFailures]` in the event source mapping. Without it, any single failure causes the entire batch to retry. With it, only failed messages return to the queue.

### Dead Letter Queue Configuration

```json
{
  "RedrivePolicy": {
    "deadLetterTargetArn": "arn:aws:sqs:us-east-1:ACCT:orders-dlq",
    "maxReceiveCount": 3
  }
}
```

**DLQ redrive**: move messages from DLQ back to source queue:

```bash
aws sqs start-message-move-task \
  --source-arn arn:aws:sqs:us-east-1:ACCT:orders-dlq \
  --destination-arn arn:aws:sqs:us-east-1:ACCT:orders \
  --max-number-of-messages-per-second 50
```

### Key Limits

- Message size: 256 KB (use S3 extended client for larger payloads)
- Retention: 1 minute to 14 days (default 4 days)
- In-flight messages: 120,000 (standard), 20,000 (FIFO)
- Delay: 0–15 minutes per message or per queue

## SNS (Simple Notification Service)

### Topic Types and Subscriptions

| Protocol         | Standard Topic | FIFO Topic             |
| ---------------- | -------------- | ---------------------- |
| SQS              | Yes            | Yes (FIFO queues only) |
| Lambda           | Yes            | Yes                    |
| HTTP/S           | Yes            | No                     |
| Email            | Yes            | No                     |
| SMS              | Yes            | No                     |
| Kinesis Firehose | Yes            | No                     |

FIFO topics: strict ordering, deduplication, must end in `.fifo`, can only deliver to FIFO SQS queues or Lambda.

### Message Filtering

Subscription filter policies — subscribers only receive matching messages:

```json
{
  "order_type": ["premium", "enterprise"],
  "amount": [{ "numeric": [">=", 100] }],
  "region": [{ "prefix": "us-" }],
  "status": [{ "anything-but": ["cancelled"] }],
  "metadata": {
    "priority": ["high"]
  }
}
```

Filter policy scope: `MessageAttributes` (default) or `MessageBody` (JSON payload filtering).

| Operator     | Example                                 | Matches                 |
| ------------ | --------------------------------------- | ----------------------- |
| Exact match  | `["premium"]`                           | Value equals "premium"  |
| Prefix       | `[{"prefix": "us-"}]`                   | Value starts with "us-" |
| Suffix       | `[{"suffix": ".jpg"}]`                  | Value ends with ".jpg"  |
| Anything-but | `[{"anything-but": ["test"]}]`          | Any value except "test" |
| Numeric      | `[{"numeric": [">=", 100, "<", 1000]}]` | Between 100 and 999     |
| Exists       | `[{"exists": true}]`                    | Attribute is present    |
| IP address   | `[{"cidr": "10.0.0.0/8"}]`              | IP in CIDR range        |

Without a filter, the subscriber receives all messages. With a filter, SNS evaluates every message against the filter policy and delivers only matches.

### Fanout Patterns

```
Producer → SNS Topic ─→ SQS Queue (processing)
                      ├→ SQS Queue (analytics)
                      ├→ Lambda (notification)
                      └→ Kinesis Firehose (archive to S3)
```

**SNS + SQS fanout**: the core AWS async pattern. Decouple producer from consumers, each consumer processes independently at its own rate. Adding a consumer doesn't affect the producer.

### Message Delivery

- **Retries**: HTTP/S endpoints get 3 retries with backoff (configurable delivery policy). Lambda gets 3 attempts. SQS is durable (no retry needed).
- **Dead-letter queue**: configure per subscription for failed deliveries
- **Raw message delivery**: skip SNS JSON wrapper, deliver raw payload to SQS/HTTP subscribers
- **Message size**: 256 KB (same as SQS)

## EventBridge

### Event Pattern Matching

```json
{
  "source": ["aws.ec2"],
  "detail-type": ["EC2 Instance State-change Notification"],
  "detail": {
    "state": ["stopped", "terminated"],
    "instance-id": [{ "prefix": "i-0" }]
  }
}
```

Events are JSON. Rules match events by pattern — only matched events invoke the target. Default event bus receives all AWS service events. Custom event buses isolate application events.

### Custom Events

```python
import boto3

events = boto3.client('events')

events.put_events(Entries=[{
    'Source': 'myapp.orders',
    'DetailType': 'OrderCreated',
    'Detail': json.dumps({
        'orderId': 'ord-123',
        'customerId': 'cust-456',
        'amount': 99.99,
        'items': ['SKU-A', 'SKU-B']
    }),
    'EventBusName': 'orders-bus'
}])
```

### Archive and Replay

```bash
# Create archive
aws events create-archive \
  --archive-name order-events-archive \
  --event-source-arn arn:aws:events:us-east-1:ACCT:event-bus/orders-bus \
  --event-pattern '{"source": ["myapp.orders"]}' \
  --retention-days 90

# Replay events (e.g., reprocess last hour after bug fix)
aws events start-replay \
  --replay-name fix-processor-bug \
  --event-source-arn arn:aws:events:us-east-1:ACCT:event-bus/orders-bus \
  --destination '{"Arn": "arn:aws:events:us-east-1:ACCT:event-bus/orders-bus"}' \
  --event-start-time 2024-01-15T10:00:00Z \
  --event-end-time 2024-01-15T11:00:00Z
```

Replayed events have `replay-name` header — your consumers should handle idempotently or detect replays.

### API Destinations

Call external HTTP APIs as EventBridge targets:

```bash
# Create connection (auth credentials)
aws events create-connection \
  --name stripe-api \
  --authorization-type API_KEY \
  --auth-parameters '{"ApiKeyAuthParameters":{"ApiKeyName":"Authorization","ApiKeyValue":"Bearer sk_live_xxx"}}'

# Create API destination
aws events create-api-destination \
  --name stripe-webhook \
  --connection-arn arn:aws:events:us-east-1:ACCT:connection/stripe-api \
  --invocation-endpoint https://api.stripe.com/v1/events \
  --http-method POST \
  --invocation-rate-limit-per-second 10
```

Built-in retry, rate limiting, and connection management. Supports OAuth, API key, and basic auth.

### EventBridge Pipes

Point-to-point integrations with optional filtering, enrichment, and transformation:

```
Source → Filter → Enrichment → Target
(SQS)   (pattern) (Lambda/    (Step Functions/
                   API GW/     Lambda/SNS/
                   Step Fn)    SQS/Kinesis)
```

Supported sources: SQS, Kinesis, DynamoDB Streams, Managed Kafka, self-managed Kafka, MQ. Replaces glue Lambda functions.

### EventBridge Scheduler

Cron and rate-based scheduling with one-time schedules:

```bash
# Recurring schedule
aws scheduler create-schedule \
  --name daily-report \
  --schedule-expression "cron(0 9 * * ? *)" \
  --schedule-expression-timezone "America/New_York" \
  --target '{"Arn":"arn:aws:lambda:...:report-fn","RoleArn":"arn:aws:iam::...:role/scheduler"}' \
  --flexible-time-window '{"Mode":"FLEXIBLE","MaximumWindowInMinutes":15}'

# One-time schedule (e.g., send reminder in 24h)
aws scheduler create-schedule \
  --name order-reminder-123 \
  --schedule-expression "at(2024-01-16T14:00:00)" \
  --target '{"Arn":"arn:aws:lambda:...:reminder-fn","Input":"{\"orderId\":\"123\"}"}'
```

Flexible time window distributes invocations to avoid thundering herd.

## Kinesis

### Data Streams

```bash
aws kinesis create-stream \
  --stream-name clickstream \
  --stream-mode-details StreamMode=ON_DEMAND  # Auto-scales, no shard management
```

| Mode        | Capacity                                   | Pricing                          |
| ----------- | ------------------------------------------ | -------------------------------- |
| On-demand   | Auto-scales, up to 200 MB/s write          | Per GB ingested + per shard-hour |
| Provisioned | Manual shard count, 1 MB/s write per shard | Per shard-hour                   |

**Partition key** determines which shard receives the record. Hot partition = all traffic hitting one shard. Use high-cardinality keys (user IDs, UUIDs).

### Enhanced Fan-Out

| Consumer Type       | Throughput                                   | Latency | Model                   |
| ------------------- | -------------------------------------------- | ------- | ----------------------- |
| Shared (GetRecords) | 2 MB/s per shard shared across all consumers | 200ms+  | Pull                    |
| Enhanced fan-out    | 2 MB/s per shard per consumer                | ~70ms   | Push (SubscribeToShard) |

```python
# Register enhanced fan-out consumer
kinesis.register_stream_consumer(
    StreamARN='arn:aws:kinesis:us-east-1:ACCT:stream/clickstream',
    ConsumerName='analytics-consumer'
)
```

Up to 20 enhanced fan-out consumers per stream. Each gets dedicated throughput.

### Kinesis Data Firehose

Zero-admin delivery to destinations:

```
Source → Buffer → Transform (optional Lambda) → Destination
                                                 ├→ S3
                                                 ├→ Redshift (via S3 COPY)
                                                 ├→ OpenSearch
                                                 ├→ Splunk
                                                 ├→ HTTP endpoint
                                                 └→ 3rd party (Datadog, etc.)
```

**Buffer hints**: 1–900 seconds OR 1–128 MB (whichever comes first). S3 delivery: prefix patterns with `!{timestamp:yyyy/MM/dd}` for partitioned output.

**Dynamic partitioning**: route records to different S3 prefixes based on content — replaces Lambda pre-processing for many ETL patterns.

### Key Kinesis Limits

- Record size: 1 MB
- Retention: 24 hours (default) to 365 days
- Write: 1,000 records/s or 1 MB/s per shard (provisioned)
- Read: 2 MB/s per shard (shared), 2 MB/s per consumer per shard (enhanced)

## Amazon MQ

Managed Apache ActiveMQ and RabbitMQ:

### When to Use MQ vs SQS/SNS

| Scenario                                   | Use                     |
| ------------------------------------------ | ----------------------- |
| New cloud-native app                       | SQS/SNS                 |
| Migrating from on-prem message broker      | MQ                      |
| Need JMS, AMQP 1.0, MQTT, STOMP, OpenWire  | MQ                      |
| Need message routing, header-based routing | MQ (RabbitMQ exchanges) |
| Simple queue/topic pattern                 | SQS/SNS                 |
| Ultra-high throughput (>100K msg/s)        | SQS/Kinesis             |

### RabbitMQ on Amazon MQ

```python
import pika

connection = pika.BlockingConnection(
    pika.URLParameters('amqps://user:pass@b-xxxx.mq.us-east-1.amazonaws.com:5671')
)
channel = connection.channel()

# Topic exchange with routing key pattern
channel.exchange_declare(exchange='orders', exchange_type='topic')
channel.queue_declare(queue='premium-orders')
channel.queue_bind(exchange='orders', queue='premium-orders', routing_key='order.premium.#')

channel.basic_publish(exchange='orders', routing_key='order.premium.us', body=json.dumps(order))
```

Deployment: single instance (dev) or cluster (prod, 3 brokers across AZs). Storage: EBS (persistent) or EFS (shared across cluster). Access via private VPC endpoints — not publicly accessible by default.
