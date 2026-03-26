# AWS Lambda Patterns — Cold Starts, Event Sources, Layers, and Orchestration

## Cold Start Latency

Lambda functions experience initialization latency when the platform must provision a new execution environment. This overhead includes downloading function code, starting the runtime, initializing global scope code outside the handler, and loading dependencies. For some workloads, this "cold start" is negligible. For others — latency-sensitive APIs, user-facing endpoints, real-time data processing — it is a critical failure mode.

Cold start latency varies by runtime. Interpreted languages (Node.js, Python) typically start faster (50-500ms). Compiled languages (Java, .NET) can take 1-3 seconds due to compilation and class loading. Custom runtimes (Go, Rust) are faster if properly optimized.

### SnapStart (Java 11+, Python 3.12+, .NET 8+)

SnapStart reduces initialization latency by capturing a Firecracker microVM snapshot when a function version is published. The snapshot freezes the memory and disk state of the initialized environment. On invocation, Lambda resumes new execution environments from the cached snapshot instead of initializing from scratch — resulting in sub-second startup in optimal scenarios. Cost includes caching fees (per MB of allocated memory, minimum 3 hours) plus restoration charges per snapshot resume.

SnapStart works best at scale; infrequently-invoked functions may not see improvements. It requires that the function code is resilient to snapshot state (e.g., handling uniqueness of state, validating network connections, refreshing ephemeral data like credentials or timestamps).

SnapStart does not support provisioned concurrency, Amazon EFS, or ephemeral storage >512 MB. It is available only on published function versions and aliases, never on `$LATEST`.

### Provisioned Concurrency

Pre-initialize execution environments in advance and keep them ready. Invoking a provisioned function returns a response in double-digit milliseconds. Provisioned concurrency is charged separately from invocation duration (hourly fee per configured environment). It is useful for applications with strict cold start requirements that SnapStart cannot adequately address (e.g., functions invoked at predictable high volume but requiring guaranteed sub-100ms latency).

Tradeoff: Higher baseline cost, but predictable latency and no surprise cold starts. Not suitable for bursty, unpredictable workloads.

### Reserved Concurrency

Reserved concurrency sets both the minimum and maximum concurrent instances for a function, preventing other functions from using that capacity and preventing the function from scaling beyond the limit. It is useful for preventing thunder herd effects on downstream services (e.g., database connection pools). Reserved concurrency incurs no additional charge — you only pay for what executes.

## Event Sources

Lambda can be invoked by many AWS services, each providing different delivery semantics:

### Synchronous Sources

**API Gateway** — HTTP request triggers Lambda and returns the response synchronously. Latency and cold starts directly impact user experience. SnapStart or provisioned concurrency are common optimizations.

**ALB (Application Load Balancer)** — Routes HTTP requests to Lambda targets. Similar semantics to API Gateway.

### Asynchronous Sources

**SQS** — Lambda polls the queue (batch processing). If processing fails, the message returns to the queue for retry. Dead-letter queues (DLQs) capture permanently failed messages. No throughput or latency SLA — Lambda scales polling concurrency automatically based on queue depth.

**SNS** — Lambda subscribed to a topic receives notifications asynchronously. No retry on Lambda failure unless a DLQ is configured.

**S3** — Object creation/deletion events trigger Lambda. Delivery is eventually consistent (may see duplicate events).

**DynamoDB Streams / Kinesis Streams** — Lambda polls stream shards. Respects shard ordering and supports batch processing. Failures trigger retries with exponential backoff (up to 24 hours by default).

**EventBridge** — Event pattern matching with optional dead-letter queue (DLQ) if Lambda returns an error.

### Key Tradeoffs

Synchronous sources require low latency from Lambda. Asynchronous sources tolerate higher latency but must handle retries, duplicates, and idempotency. Polling-based sources (SQS, Kinesis) couple Lambda's scaling to queue/stream depth; they may lag under burst traffic.

## Lambda Layers and Code Organization

Layers package shared code (libraries, runtimes, extensions) for reuse across functions. A layer is a .zip file extracted under `/opt` in the execution environment. Multiple layers can be combined; code is available under `/opt/nodejs`, `/opt/python`, `/opt/extensions`, etc.

Layers are useful for managing dependencies separately from function code (e.g., PowerTools library), custom runtimes, or shared utilities. Tradeoff: Layers add deployment complexity and must be versioned independently.

## Lambda Extensions

External extensions run as independent processes in the execution environment and continue after function invocation completes. Internal extensions run as part of the runtime process (e.g., via JAVA_TOOL_OPTIONS). Extensions share function resources (CPU, memory, storage) and count against deployment package size limits (250 MB unzipped total).

Extensions are used for monitoring, observability, security, and governance integration. They must complete initialization before Lambda invokes the handler, so heavy initialization work increases invocation latency. AWS and partner extensions are available; custom extensions can be written in any language.

PostRuntimeExtensionsDuration and MaxMemoryUsed metrics help measure extension overhead. Extensions with significant init time or memory footprint can meaningfully degrade function performance.

## Lambda Execution Lifecycle

The execution environment goes through phases: **Init** (extension init → runtime init → function init), **Restore** (SnapStart only), **Invoke** (function handler), **Shutdown** (no invocations for period of time).

With SnapStart, before-checkpoint and after-restore hooks allow code to run before and after snapshot respectively. The after-restore hook must complete within 10 seconds (total runtime must load + hook must complete).

## Custom Runtimes

For unsupported languages (Go, Rust, etc.), implement a custom runtime that calls the AWS Lambda Runtime HTTP API. Custom runtimes accept invocation events, invoke your handler, and return responses. They can be packaged as a layer or included in the deployment package. Custom runtimes have similar cold start characteristics to interpreted languages if written efficiently.

## PowerTools and Observability

AWS Lambda PowerTools (libraries for Python, Java, Node.js, .NET) provide utilities for logging, tracing, metrics, and configuration management. They reduce boilerplate and encourage best practices (structured logging, distributed tracing). PowerTools have negligible overhead when included as a layer.

## Step Functions Orchestration

AWS Step Functions coordinates multi-step workflows by invoking Lambda functions, waiting for results, implementing retries and error handling, and managing state. Step Functions decouples Lambda from business process logic, allowing complex workflows (fan-out, parallel branches, conditional logic, loops) without embedding orchestration in Lambda code itself.

Step Functions can invoke Lambda synchronously and wait for completion, or asynchronously via job tokens. Integration with Lambda is the primary use case for event-driven data processing pipelines and business process automation.

## See Also

Related: `architecture-serverless`, `architecture-event-driven`, `cloud-aws-serverless`, `cloud-aws-messaging`