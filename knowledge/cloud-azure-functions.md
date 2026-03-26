# Azure Functions — Triggers, Bindings, Durable Functions & Scaling

Azure Functions is Microsoft's serverless compute platform: pay-per-execution event-driven applications without managing infrastructure. Functions are **triggered** by events (HTTP requests, timers, queue messages) and can **bind** to external services (storage, databases, messaging) declaratively.

## Triggers and Bindings

### Triggers

A **trigger** is the event that invokes a function. Every function has exactly one trigger.

**HTTP Trigger:**
```csharp
[FunctionName("HttpFunction")]
public static async Task<IActionResult> Run(
    [HttpTrigger(AuthorizationLevel.Anonymous, "get", "post", Route = null)] 
    HttpRequest req,
    ILogger log)
{
    return new OkObjectResult("Hello!");
}
```

Invoked when HTTP request arrives at function URL. Authorization levels:
- `Anonymous`: No authentication
- `Function`: Function-specific key required
- `Admin`: Master key required (sensitive operations)

**Timer Trigger:**
```csharp
[FunctionName("TimerFunction")]
public static void Run(
    [TimerTrigger("0 */5 * * * *")] TimerInfo timer,  // Every 5 minutes
    ILogger log)
{
    log.LogInformation($"Timer fired: {timer.ScheduleStatus}");
}
```

CRON expression: `{second} {minute} {hour} {day} {month} {day-of-week}`

**Queue Trigger:**
```csharp
[FunctionName("QueueFunction")]
public static void Run(
    [QueueTrigger("myqueue")] CloudQueueMessage msg,
    ILogger log)
{
    log.LogInformation($"Message: {msg.AsString}");
}
```

Invoked when message arrives in Azure Storage Queue or Service Bus.

**Blob Trigger:**
```csharp
[FunctionName("BlobFunction")]
public static void Run(
    [BlobTrigger("mycontainer/{name}")] Stream blob,
    string name,
    ILogger log)
{
    log.LogInformation($"Blob uploaded: {name}");
}
```

Invoked when blob is uploaded to storage container. Binding parameter `{name}` extracts blob filename.

**Event Hubs Trigger:**
```csharp
[FunctionName("EventHubsFunction")]
public static async Task Run(
    [EventHubTrigger("myeventhub", Connection = "EventHubConnection")] 
    EventData[] events,
    ILogger log)
{
    foreach (var e in events) {
        log.LogInformation($"Event data: {Encoding.UTF8.GetString(e.Body.Array)}");
    }
}
```

Invoked for events from Azure Event Hubs or Apache Kafka.

### Bindings

Bindings are **declarative connections** to external services. Unlike triggers, functions can have zero or many input/output bindings.

**Input Binding (read external data):**
```csharp
[FunctionName("GetUser")]
public static IActionResult Run(
    [HttpTrigger(AuthorizationLevel.Anonymous, "get", Route = "users/{userId}")] 
    HttpRequest req,
    [Blob("users/{userId}.json")] string userBlob,  // Input: read blob
    string userId)
{
    return new OkObjectResult(userBlob);
}
```

**Output Binding (write to external service):**
```csharp
[FunctionName("LogEvent")]
public static IActionResult Run(
    [HttpTrigger(AuthorizationLevel.Anonymous, "post")] HttpRequest req,
    [Queue("events")] IAsyncCollector<string> eventQueue,  // Output: push to queue
    ILogger log)
{
    // Function logic
    await eventQueue.AddAsync("Event logged");
    return new OkResult();
}
```

**Common output bindings:**
- `Queue`: Azure Storage Queue or Service Bus
- `Blob`: Read/write from storage
- `ServiceBus`: Topic or queue
- `CosmosDB`: Document insert/update
- `Table`: Azure Table Storage
- `SendGrid`: Email
- `Twilio`: SMS

## Durable Functions

**Durable Functions** extend Azure Functions to support **long-running, stateful workflows** using orchestration patterns. They enable:
- Multi-step workflows (call multiple functions sequentially)
- Parallel execution (fan-out/fan-in)
- Timers and retries without polling
- Human approval workflows
- State machines

### Orchestrator Function

An **orchestrator** is the workflow controller; it defines the sequence of activities.

```csharp
[FunctionName("OrderProcessingOrchestrator")]
public static async Task RunOrchestrator(
    [OrchestrationTrigger] IDurableOrchestrationContext context,
    ILogger log)
{
    var orderId = context.GetInput<string>();
    
    // Step 1: Validate payment
    var payment = await context.CallActivityAsync<bool>("ValidatePayment", orderId);
    if (!payment) throw new Exception("Payment failed");
    
    // Step 2: Deduct inventory (parallel calls)
    var inventoryTasks = new[] { "item1", "item2" }
        .Select(item => context.CallActivityAsync("DeductInventory", (orderId, item)))
        .ToArray();
    await Task.WhenAll(inventoryTasks);
    
    // Step 3: Ship order
    await context.CallActivityAsync("ShipOrder", orderId);
    
    return "Order processed";
}
```

**Key constraints:**
- Cannot use DateTime.UtcNow (use context.CurrentUtcDateTime)
- Cannot use Task.Delay (use context.CreateTimer)
- Deterministic only (same input → same execution path, no randomness)
- Cannot use external I/O directly; call activities instead

### Activity Functions

**Activities** are the actual work: call APIs, access databases, send emails.

```csharp
[FunctionName("ValidatePayment")]
public static async Task<bool> ValidatePayment([ActivityTrigger] string orderId)
{
    // Hit payment gateway API
    var result = await paymentGateway.Charge(orderId);
    return result.Success;
}

[FunctionName("DeductInventory")]
public static async Task DeductInventory([ActivityTrigger] (string orderId, string item) inputs)
{
    await database.UpdateInventory(inputs.item, -1);
}

[FunctionName("ShipOrder")]
public static async Task ShipOrder([ActivityTrigger] string orderId)
{
    await shippingService.CreateLabel(orderId);
}
```

Activities **execute at least once**: retries on failure (configurable backoff).

### Entity Functions

**Entities** (stateful actors) maintain mutable state without database queries.

```csharp
[FunctionName("BankAccount")]
public static void BankAccount(
    [EntityFunctionInput] IDurableEntityContext ctx,
    ILogger log)
{
    var bankAccount = ctx.GetState(() => new { Balance = 0 });
    
    switch (ctx.OperationName.ToLowerInvariant())
    {
        case "deposit":
            bankAccount = new { Balance = bankAccount.Balance + (int)ctx.GetInput<int>() };
            break;
        case "withdraw":
            bankAccount = new { Balance = bankAccount.Balance - (int)ctx.GetInput<int>() };
            break;
        case "get":
            ctx.SetResult(bankAccount.Balance);
            break;
    }
    
    ctx.SetState(bankAccount);
}

// Usage from orchestrator
var accountId = "account123";
await context.CallEntityAsync(new EntityId("BankAccount", accountId), "deposit", 100);
var balance = await context.CallEntityAsync<int>(new EntityId("BankAccount", accountId), "get");
```

### Timer and Retry

Durable functions provide built-in retry and timing:

```csharp
var retryOptions = new RetryOptions(
    firstRetryInterval: TimeSpan.FromSeconds(5),
    maxNumberOfAttempts: 3)
{
    BackoffCoefficient = 2.0  // Exponential: 5s, 10s, 20s
};

try
{
    await context.CallActivityWithRetryAsync("SomeActivity", retryOptions, input);
}
catch (FunctionFailedException ex)
{
    log.LogWarning("Activity failed after retries");
}
```

Timers:
```csharp
var deadline = context.CurrentUtcDateTime.AddHours(1);
await context.CreateTimer(deadline, CancellationToken.None);

// Or delay
await context.CreateTimer(context.CurrentUtcDateTime.AddSeconds(30), CancellationToken.None);
```

## Scaling Behaviors

### Consumption Plan (Serverless)

**Execution:** Pay per invocation + execution time (millisecond bins).

Pricing: $0.20 per 1M executions + $0.000016 per GB-second.

**Scaling:** *Automatic to 1000 instances per region (single function or shared pool).*

**Characteristics:**
- **Cold starts:** 1-2 seconds on first invocation after idle (language-dependent)
- **Throughput:** Limited by instance max (typically 100-200 concurrent)
- **Latency:** Variable (cold starts, resource contention)

**Best for:** Sporadic, bursty workloads; prototypes; dev/test.

### Premium Plan

**Execution:** Reserved instances; pay per vCPU per hour.

Pricing: ~$0.10-0.16 per vCPU/hour (variable by region).

**Scaling:** *Automatic, but to pre-warmed instances.*

**Characteristics:**
- **No cold starts:** Instances continuously warm
- **Throughput:** Higher concurrency (250+ per instance)
- **Latency:** Consistent, low (no cold start penalty)
- **VNET integration:** Functions in private network (security requirement)

**Best for:** Latency-sensitive apps, high throughput, always-on patterns.

### Dedicated (App Service) Plan

**Execution:** Shared with other App Service apps on the same plan.

Pricing: Same as App Service VMs (~$10-100/month for a small VM).

**Scaling:** *Manual or auto-scale to defined limits.*

**Characteristics:**
- **No cold starts**
- **Predictable cost** (pay for VM, not invocations)
- **Full OS control** (install custom runtime, libraries)
- **Lower per-GB costs** at very high volume

**Best for:** High-volume continuous workloads; legacy app migration; custom runtime needs.

## Cold Start Mitigation

### Root Causes

1. **JIT compilation:** .NET, Java require runtime startup (1-3 seconds)
2. **Dependency loading:** Large libraries, connection pool initialization
3. **Application startup code:** Logging, config parsing

### Strategies

**1. Language choice:**
- Node.js, Python: ~100-500 ms cold start
- .NET 5+, Java: ~500-2000 ms cold start
- Go, Rust: ~50-100 ms (near-instant language startup)

**2. Premium or Dedicated plan:** No cold starts (instances kept warm).

**3. Keep-alive automation:** Ping function every N minutes to prevent idle termination.

```csharp
[FunctionName("KeepAlive")]
public static IActionResult Run(
    [TimerTrigger("0 */4 * * * *")] TimerInfo timer)  // Every 4 minutes
{
    return new OkResult();
}
```

**4. Lightweight entry point:** Defer heavy initialization (lazy load, dependency injection).

```csharp
static class Startup
{
    private static Lazy<HttpClient> _httpClient = 
        new Lazy<HttpClient>(() => new HttpClient(), isThreadSafe: true);
    
    public static HttpClient GetHttpClient() => _httpClient.Value;
}

// Called only on first use, not on first cold start
```

**5. Assembly trimming (.NET):** Remove unused code before deployment (PublishTrimmed: True).

**6. Container image optimization:** Minimal Docker image (Alpine Linux, lean runtime).

## Isolated Worker Model

The **isolated worker model** runs functions in a separate process from the host, enabling:
- Language version flexibility (Python 3.9 vs 3.10+ simultaneously)
- Longer execution times (up to 10 hours vs 10 minutes in in-process)
- Process isolation (custom runtime, no host interference)

```csharp
// Program.cs (isolated worker)
var host = new HostBuilder()
    .ConfigureFunctionsWorkerDefaults()
    .Build();

host.Run();

// Function.cs
[Function("HelloWorld")]
public static HttpResponseData Run(
    [HttpTrigger(AuthorizationLevel.Anonymous, "get")] HttpRequestData req)
{
    var response = req.CreateResponse(HttpStatusCode.OK);
    response.Headers.Add("Content-Type", "text/plain; charset=utf-8");
    response.WriteString("Hello!");
    return response;
}
```

vs **in-process:**
```csharp
[FunctionName("HelloWorld")]
public static IActionResult Run([HttpTrigger(AuthorizationLevel.Anonymous)] HttpRequest req)
{
    return new OkObjectResult("Hello!");
}
```

**Tradeoff:** Isolated = more control and flexibility, but +10-30% memory/startup overhead vs in-process.

## Language Coverage

| Language    | In-Process | Isolated | Notes                     |
|-------------|-----------|----------|--------------------------|
| C# (.NET)   | Yes       | Yes      | Mature, fastest           |
| JavaScript  | Yes       | Yes      | Node.js-based             |
| Python      | No        | Yes      | 3.9 through 3.12          |
| Java        | No        | Yes      | OpenJDK 11+               |
| Go          | No        | Yes      | Fastest cold start        |
| PowerShell  | No        | Yes      | Automation, DevOps use    |

## See Also

- `serverless-architecture` — Event-driven design patterns, trade-offs
- `cloud-aws-lambda` — AWS Lambda comparison (pricing, triggers, concurrency)
- `distributed-transactions` — Saga pattern for durable workflows
- `architecture-resilience` — Failure modes, retry strategies in serverless
- `cloud-azure-compute` — Azure Functions placement in compute hierarchy