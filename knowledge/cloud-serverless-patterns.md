# Serverless Patterns — Function Composition, Event Flow & Practical Constraints

## Overview

Serverless architectures decompose applications into stateless, independently-invoked functions. Beyond single-function design, serverless patterns address **orchestration** (how functions invoke each other), **event handling** (asynchronous triggers), **state management** (stateless by default), and **cost modeling** (pay-per-execution models impact design choices). Each pattern trades latency, cost, operational complexity, and vendor coupling.

## Function Composition Patterns

### Sequential Chaining

Functions invoke the next function synchronously. Simplest model but creates tight coupling and linear latency.

```
Function A → (waits) → Function B → (waits) → Function C
Total latency: sum of all latencies
If any step fails, retry from Function A (entire chain restarts)
```

**Tradeoffs:**
- Pro: Simple state propagation (output → input chain).
- Con: Tail latency amplification (slowest function bottlenecks entire chain), high failure blast radius (single failure restarts whole chain), expensive (all functions stay warm waiting).

**Use case:** Small sequential workflows under 1 minute; not suitable for operations where intermediate steps differ in duration.

### Fan-Out / Fan-In

One function spawns multiple parallel branches, then waits for all to complete. Implemented via AWS Step Functions (Parallel state), GCP Workflows, or orchestration libraries.

```
Function A (fan-out)
  ├─ Branch 1: Process Customer Data
  ├─ Branch 2: Process Orders
  └─ Branch 3: Process Inventory
Function B (fan-in, aggregates results)
Total latency: max(Branch 1, 2, 3), not sum
```

**Tradeoffs:**
- Pro: Parallelism reduces wall-clock time; independent branches can use different logic.
- Con: Orchestrator (Step Functions) adds cost and latency; coordination state (partial results) must persist; debugging parallel failures is harder; step function state transitions have overhead.

**Cost consideration:** Each Step Functions execution transition costs $0.000025 (on AWS). High-frequency parallel jobs (thousands/second) accumulate orchestration costs. Serverless platforms that hide orchestration (e.g., internal concurrency models) avoid this.

**Use case:** Independent data processing (ETL fan-out), batch report generation, microservice fan-in.

### Saga Pattern

Long-running transactions decomposed into local transactions + compensations. If step N fails, execute compensations N-1, N-2, ..., 1 to undo side effects.

```
Function A: Reserve ticket (compensation: release ticket)
  ↓ success
Function B: Process payment (compensation: refund payment)
  ↓ success
Function C: Send confirmation (no compensation needed)
  ↓ failure → run compensations B, A
```

**Variants:**
- **Choreography:** Each function publishes events; next function subscribes and triggers. Decoupled but non-obvious flow (event sources scattered).
- **Orchestration:** Central coordinator (Step Functions, Temporal) manages transitions and compensations. Centralized flow logic but operational burden.

**Tradeoffs:**
- Pro: Supports long-running workflows (minutes to hours) in serverless without holding open connections.
- Con: Compensations are not true transactions (no rollback semantics); partial failures leave system in inconsistent state (eventual consistency only); debugging requires correlation IDs and centralized logging.

**Serverless constraint:** Typical Lambda timeouts (15 minutes max). Sagas exceeding this require checkpointing (save progress to database, resume from checkpoint on timeout).

**Use case:** Multi-step order processing, reservation systems, workflow automation.

## Event-Driven Patterns on Serverless

### Push (Invocation Triggered)

Function invoked directly when event occurs. Examples: S3:ObjectCreated → Lambda, CloudWatch Events → Lambda, API Gateway → Lambda.

**Latency:** Milliseconds to seconds (platform-dependent).
**Scaling:** Automatic based on event rate; platform scales concurrency as needed.
**Cost:** Pay per invocation + per-ms duration; no cost when idle.

**Constraint:** Concurrency limits (AWS Lambda default 1000 concurrent executions per account per region) can be a bottleneck if events arrive faster than platform can start functions. Requires either account limit increase or queue buffer.

### Pull (Consumer Polling)

Function polls event sources (SQS queue, Kinesis stream, or DynamoDB Streams). Platform manages polling + batch invocation.

**Latency:** Seconds (batch polling interval, typically 1 second).
**Scaling:** Platform polls in proportion to queue depth; batch size (typically 10-100 items) balances throughput vs. latency.
**Cost:** Pay per invocation (fewer invocations = fewer batches) + duration; more efficient than push for bursty workloads.

**Advantage over push:** Decoupling; producers emit to queue without knowing function exists. Backpressure: queue accumulates if function is slow; automatic error handling (DLQ for failed batches).

**Tradeoff:** Higher latency than push; polling adds slight cost (unused polls); requires queue infrastructure.

### Event Streaming (Kafka, Kinesis)

Push per record; guaranteed ordering per shard/partition; exactly-once or at-least-once delivery.

**Ordering guarantee:** Critical for workflows where sequence matters (e.g., audit logs, financial transactions). Records in same partition/shard always processed in order.

**State management:** Consumer group offset tracks progress; offset management handled by platform (e.g., Lambda with Kinesis) or explicitly (Kafka consumer groups).

**Tradeoff:** Ordering creates serialization bottleneck (shard processes one record at a time); high throughput requires many shards (operator burden).

## Serverless Databases & State Management

Serverless functions are stateless; state persists in external stores.

### DynamoDB / Firestore Pattern

**Pros:** Serverless (auto-scaling), pay per request, millisecond latency.
**Cons:** Limited query flexibility (key-value + simple range queries), expensive at scale (cost per RCU/WCU). DynamoDB: $1.25/million WCU, $0.25/million RCU (on-demand).

**Cost model:** Billed per request, not instance hour. For uniform traffic, predictable. For bursty, on-demand pricing avoids waste (pay only for requests used, not reserved capacity).

**Tradeoff:** No complex joins; queries often duplicate data across tables (denormalization). Requires careful design to avoid N+1 queries.

### Relational + Connection Pooling

Traditional RDS (Postgres, MySQL) + connection pooler (RDS Proxy, PgBouncer) as middleware.

**Challenge:** Each Lambda invocation is isolated; concurrent invocations would exhaust database connections without pooling. Pooler multiplexes many Lambda connections onto fewer database connections.

**Cost model:** Pay per database instance hour (not serverless). Cheaper at high utilization; more expensive for low, spiky workloads (idle instance costs remain).

**Tradeoff:** Lower cost at scale but requires baseline instance size committed; harder to scale down quickly; adds latency (pooler hop).

### Polyglot: Each Function's Datastore

Different functions use optimized stores: Lambda A uses DynamoDB (fast key-value), Lambda B uses S3 (bulk storage), Lambda C uses Elasticsearch (search). Complexity in data sync (eventual consistency).

## Cold Start Mitigation

AWS Lambda cold start times by runtime:

| Runtime | Cold Start | Warm Reuse |
| ------- | ---------- | ---------- |
| Go      | 50-100ms   | 1-5ms      |
| Python  | 200-500ms  | 5-10ms     |
| Node.js | 200-400ms  | 5-10ms     |
| Java    | 1-5s       | 10-50ms    |
| .NET    | 500-2s     | 10-50ms    |

**Strategies:**

- **Language choice:** Prefer Go/Rust (compiled, fast startup) over Java/Python for latency-sensitive endpoints.
- **Provisioned Concurrency:** Pre-warm N instances; cost ~$0.015/hour per concurrent execution. Worth it if cold starts cause unacceptable latency spikes.
- **Lazy initialization:** Move heavy initialization (DB connections, SDK setup) outside handler to module-level code (reused across warm invocations).
- **Graduated load:** API Gateway could queue requests during spike instead of triggering many cold starts.
- **SnapStart (Java):** AWS Lambda captures post-init snapshot; restore skips JVM startup (~200ms savings).

## Vendor Lock-In Considerations

Serverless patterns create coupling to vendor services:

- **Step Functions:** AWS proprietary orchestration language; migrating to GCP Workflows requires rewrite.
- **Event sources:** SNS, SQS, Kinesis are AWS-specific; Pub/Sub (GCP) has different API.
- **Databases:** DynamoDB schema (no joins, max 4KB item) differs from Firestore (less strict) differs from Cloud Spanner (joint queries).

**Mitigation:**
- Abstract orchestration: Use Temporal or durable-task libraries (multi-cloud).
- Standardize events: CloudEvents format (vendor-neutral event structure).
- Query abstraction: GraphQL federation (query layer independent of underlying database).

## Serverless Containers: AWS Fargate, Cloud Run

Function-as-a-Service (FaaS: Lambda, Cloud Functions) requires custom runtime support for new languages/frameworks. **Serverless containers** allow arbitrary Docker images, simplifying deployment but losing some optimization.

### AWS Fargate

Run ECS/Kubernetes tasks without managing EC2. Billing: per vCPU-hour + memory-hour (no function invocation model).

**Use case:** Longer-running processes (> 15 mins, Lambda's max), frameworks requiring specific binary dependencies, batch jobs.

**Cost model:** More predictable than Lambda (fixed vCPU-hour rate) but less efficient for short, infrequent tasks (baseline cost even if idle).

**Compared to Lambda:** Fargate: $0.04544/vCPU-hour. Lambda: $0.0000166667/GB-sec. For a 512MB function running 1 second: Lambda ~$0.000008, Fargate (minimum 256MB) ~$0.000126/sec = much cheaper. But Fargate includes more features (volumes, networking), justifying cost.

### Google Cloud Run

Similar to Fargate but simpler model: one container, auto-scales to zero (no idle cost). Pricing: per request + per vCPU-second.

**Advantage:** True serverless (scale to zero), simpler mental model (single service, not task/pod coordination).

**Disadvantage:** Limited to 1 hour max execution; less flexible networking (service-to-service only, no direct VPC peering).

## Cost Modeling

### Function Invocation Costs Formula

**AWS Lambda:**
```
Cost = (Invocations × $0.20 + (GB-seconds × $0.0000166667)) × discount
GB-seconds = (Memory/1024) × DurationSeconds
Discount: 10% for Compute Savings Plans, 15% for 1-year commitment
```

Example: 100 invocations/day, 256MB, 5 seconds each:
```
GB-sec per invocation = (256/1024) × 5 = 1.25
Monthly GB-sec = 100 invocations × 30 days × 1.25 = 3750
Monthly cost ≈ (100×30×$0.20 + 3750×$0.0000166667) ≈ $600 + $0.06 ≈ $600
```

Invocation cost dominates; optimization target: reduce invocation count and memory.

### Common Optimization

- **Batch processing:** Instead of 1000 individual function invocations, batch into 100 larger invocations (1 million items each). Reduces invocation cost 10×.
- **Reserved capacity/Savings Plans:** Lock in 20-30% discount if baseline usage is stable.
- **Right-size memory:** Higher memory = faster execution (CPU correlated). Find sweet spot: 512MB may run in 2 seconds; 256MB may run in 8 seconds. If $0.0000166667/GB-sec, doubling memory costs 2×, but halving duration saves more.

## See Also

- **architecture-serverless:** Foundational FaaS patterns
- **architecture-event-driven:** Event topic and consumer design
- **architecture-saga-pattern:** Detailed saga implementation
- **cloud-aws-lambda-patterns:** AWS Lambda specifics
- **database-connection-pooling:** Serverless database practices