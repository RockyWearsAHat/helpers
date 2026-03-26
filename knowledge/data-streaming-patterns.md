# Stream Processing Patterns — Windowing, State, Joins, and Exactly-Once Semantics

Stream processing patterns describe how to structure unbounded event flows, maintain mutable state, and guarantee correctness semantics. See also [Stream Processing](data-engineering-streaming.md) for foundational concepts; this note focuses on concrete patterns and tool trade-offs.

## Windowing Patterns

Windowing divides an infinite stream into finite, analyzable chunks. Window choice determines latency, state overhead, and mental model.

### Tumbling Windows

Non-overlapping, fixed-size windows. Each event falls into exactly one window.

```
Events:  [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
Window size = 3:
Window 1: [1, 2, 3] → emit at t=3
Window 2: [4, 5, 6] → emit at t=6
Window 3: [7, 8, 9] → emit at t=9
Window 4: [10]     → emit at t=10 (or later)
```

**Use when:**
- Computing hourly/daily aggregates (revenue, pageviews, errors)
- Checkpoint intervals align with window boundaries; recovery is straightforward

**Characteristics:**
- No overlap; minimal state overhead
- Complete results at window close; no partial updates
- Latency = window duration (emit only after last event in window arrives)

### Sliding Windows

Fixed-size window that advances by a smaller hop. Events fall into multiple, overlapping windows.

```
Events: [1, 2, 3, 4, 5, 6, 7, 8]
Window size = 3, hop = 1:
Window 1: [1, 2, 3] → emit
Window 2: [2, 3, 4] → emit
Window 3: [3, 4, 5] → emit
...
```

**Use when:**
- Detecting trends (moving average, rolling volatility)
- Alerting on anomalies (spike detection; evaluate every tick, not batch)
- Responsive user-facing dashboards

**Characteristics:**
- High overlap → high state overhead (each event contributes to many windows)
- Frequency of emission = hop size (e.g., emit every 1 second even if window is 60 seconds)
- Complexity: Evicting old events and adding new ones

### Session Windows

Dynamic, data-driven windows. Window closes after a period of inactivity (gap threshold).

```
Events (timestamps): [1, 2, 3, 25, 26, 27]
Gap threshold = 10ms:
Session 1: [1, 2, 3]       → close at t=13 (3 + gap)
Session 2: [25, 26, 27]    → close at t=37
```

**Use when:**
- User sessions (queries, clicks, interactions)
- Attack detection (burst of failed logins, then quiet)
- Natural event grouping by message arrival pattern

**Characteristics:**
- State: Multiple open sessions per key; memory scales with concurrent sessions
- Complexity: Merging sessions if events arrive out of order
- Latency: Unpredictable; depends on gap threshold and actual event distribution
- Challenge: Defining gap threshold per data domain (10ms for clicks, 30m for user logins)

### Custom Windows

Event-time or other attributes-based semantives. Example: "all events in the same geographic region."

- Implementation: Assign each event to window(s) via custom function. Framework buffers until window close.
- Rarely needed unless building specialized analytics (geographic windows, custom business logic)

---

## Watermarks and Late Data Handling

In reality, events arrive out of order: delays in transmission, retries, backpressure. The framework must decide when to close a window despite late arrivals.

### Watermarks

A **watermark** is a threshold marking progress through event time. Events earlier than the watermark are assumed complete; events after may still arrive (late).

$\text{watermark} = \max(\text{event_time}) - \text{allowed_lateness}$

Example:
```
allowed_lateness = 10 seconds
Event stream (event_time):  [1, 2, 3, 8, 9, 10]
Watermarks: [→ 1, → 2, → 3, → -2 (max=8, lateness=10), → -1, → 0 (max=10, lateness=10)]
Window [0, 5): Closes when watermark crosses 5 (i.e., max event time > 15)
```

**Default watermarks** (auto-generated): $\max(\text{event time so far}) - \text{system latency estimate}$.
- Simple but unreliable; needs tuning per pipeline

**Custom watermarks** (user-defined): Track idleness, backpressure, or application-specific timestamps.
- Fine-grained control; more complex to implement

### Triggered Behavior

Once a window fires (emits a result), decide what to do with late arrivals.

**No triggers (fire on watermark)** → Emit once, period. Late events are dropped.
- Predictable; no late updates; simplest operational model

**Allowed lateness** → Fire on watermark, then fire again for every late event (or fire on-demand).
- Result: Multiple firings per window (initial + updates)
- Challenge: Downstream must idempotently handle duplicates or merge updates

**Sessions with allowed lateness** → Merge sessions if gap threshold expires but an event arrives within allowed_lateness.

---

## State Management

Streams are stateless by default (map, filter). Stateful operations (aggregations, joins, windowing) require persistent state on the processing node.

### Local State

Framework maintains state on each task; typically in RocksDB (embedded key-value store). Fast; survives task restarts via snapshots.

**Checkpoint**: Periodically save state to durable storage (HDFS, S3, distributed cache). On failure, restore and resume from last checkpoint.
- Frequency trade-off: More checkpoints = lower recovery time but higher I/O overhead
- Flink default: 10 seconds; configurable

**Rescaling** (adding/removing tasks): State must be redistributed. Options:
- **Rebalance**: Re-hash all keys; may reprocess past events or skip some
- **Operator state**: Map-based custom routing; rare

### Remote State

External store (Redis, DynamoDB, database). Useful for:
- Sharing state across pipelines
- Querying state from outside the pipeline
- Reduced message coupling (each task reads/writes independently)

**Trade-off**: Latency (remote request) vs. simplicity (no local state synchronization). Usually slower than local state but enables complex topologies.

---

## Stream-Table Joins

Joining a stream to a table (or static dataset) is common: enrichment (add customer name to transaction stream).

### Stream-KV Store Join

Stream event arrives; look up key in external KV store (Redis, database). Emit enriched event.

```
Stream: {user_id: 123, action: "purchase"}
KV Store: {123 → {name: "Alice", tier: "gold"}}
Result: {user_id: 123, name: "Alice", tier: "gold", action: "purchase"}
```

**Latency**: One remote lookup per event; scale with store throughput and network round-trip
**Consistency**: If the store updates while joining an old event, result reflects current state (eventual consistency)
**Failure**: Store unavailability blocks pipeline; needs fallback/retry

### Stream-Stream Join (Co-Partitioned)

Two streams with same key partitioning. Join event pairs arriving in the same window.

```
Stream A: {user_id: 123, endpoint: "/api/login", timestamp: 1000}
Stream B: {user_id: 123, ip: "192.168.1.1", timestamp: 1005}
Join window: [1000, 2000)
Result (if matching): {user_id: 123, endpoint: "/api/login", ip: "192.168.1.1"}
```

**Requirements:**
- Both streams partitioned by join key (same machines see related events)
- Time window (events must arrive within allowed clock skew)
- State per key: hold events until window closes or matching event arrives

**Trade-offs:**
- No remote calls (lower latency)
- State overhead grows with window size and concurrency
- Out-of-order events may never join if window closes early

### Lookup Table (Broadcast)

Stream joined with a small, infrequently-updated table. Broadcast table to all tasks; stream events do lookup-join locally.

- Use: Master data (products, exchange rates, configurations)
- Latency: Minimal (local)
- Scaling: Table size must fit in task memory; update frequency manageable

---

## Exactly-Once Semantics

Exactly-once guarantees that each input event is processed and output exactly once, despite failures and retries.

### Deduplication

If source allows replay (Kafka offset tracking), idempotent processing (same input → same output every time) enables exactly-once.

**Mechanism:** Sender includes sequence number or content hash. Receiver deduplicates by checking if (id, content) was seen before.

```
Message: {seq: 100, data: "..."}
State: {seq: 100, seen: true, output: "result"}
Retry arrives: {seq: 100, data: "..."}
Action: Return cached output (don't reprocess)
```

**Challenge:** Infinite state (track all seen seq numbers). Solution: Bounded window (drop old state after TTL) or probabilistic sketching (Bloom filter for dedup; trade-off accuracy).

### Transactional Writes

Combine source offset tracking with atomic output writes.

```
Kafka:
  Read offset 100, process, write output
  Atomic: commit offset 100 only after output written
Failure: Offset not advanced; replay from last committed offset
```

**Systems:**
- **Kafka + KV Store**: Utilize transactions if KV store supports it (e.g., database transactions)
- **Kafka + Kafka**: Transactional produces (all-or-nothing) tied to offset commits
- **Kafka + external DB**: Use idempotent upserts (same key updated; de facto exactly-once if key is unique)

### Distributed Snapshots (Flink, Spark)

Framework checkpoints entire pipeline state at a consistent timestamp.

**Chandy-Lamport Algorithm**: 
- Inject barrier into stream; all tasks snapshot state when barrier crosses them
- Once all tasks barrier-crossed, global snapshot is complete
- On failure, rewind to snapshot; re-inject from source offset

**Exactly-once guarantee**: Barriers ensure no state loss; source replay ensures no duplication.

---

## Tool Comparison: Kafka Streams vs. Apache Flink vs. Spark Streaming

| Aspect                  | Kafka Streams | Flink | Spark Structured Streaming |
|------------------------|---|---|---|
| **Latency**             | Sub-second | Milliseconds | Seconds (~100ms micro-batch) |
| **Windowing**           | Tumbling, sliding, session, custom | All + grace period (late data) | All + watermarks |
| **Exactly-once**        | Depends on sink; idempotent writes | Checkpoint-based; transactional | Structured Streaming: at-least-once by default; end-to-end exactly-once via sink idempotence |
| **State management**    | Local (RocksDB) | Local (RocksDB) | Micro-batch state in memory + checkpoint |
| **Scalability**         | Partitioned by key; lightweight | Horizontal tasks; fine-grained recovery | Batch-oriented; scaling adds batch size |
| **Deployment**          | Embedded in app; stateless cluster | Separate cluster (JobManager, TaskManager) | Spark cluster; orchestrated by driver |
| **Late event handling** | `grace` period (suppress-then-emit) | Built-in; configurable allowed lateness | Watermarks; late arrival triggers new batch |
| **SQL support**         | Limited (KSQL, separate component) | High (Flink SQL, streaming dataframes) | Native (DataFrame API, streaming SQL) |
| **Fault tolerance**     | State stores + source offset replay | Chandy-Lamport snapshots | RDD lineage + checkpoint |
| **Best for**            | Kafka-centric pipelines, stateful apps embedded in services | Complex topologies, sub-second latency, fine-grained recovery | Batch-heavy, iterative algorithms, ML workloads |

**Kafka Streams**: Lightweight, ops-simple, but assumes Kafka source. Roughly exactly-once with idempotent sinks.

**Flink**: Most sophisticated stream semantics, millisecond latency, true exactly-once out-of-box. Operational complexity higher.

**Spark Streaming**: Unified batch/stream API appealing for ML; latency higher; exactly-once requires idempotence.

---

## See Also
- [Stream Processing](data-engineering-streaming.md) — Fundamentals
- [Apache Kafka](database-kafka.md) — Source system
- [Event-Driven Patterns](patterns-event-driven.md) — Semantics of events