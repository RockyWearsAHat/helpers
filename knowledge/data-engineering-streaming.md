# Stream Processing — Concepts, Patterns & Trade-offs

## Batch vs Stream Processing

Batch processing operates on bounded datasets — a known beginning and end. Stream processing operates on unbounded datasets — events arrive continuously without a defined endpoint.

| Dimension        | Batch                                     | Stream                                               |
| ---------------- | ----------------------------------------- | ---------------------------------------------------- |
| Latency          | Minutes to hours                          | Milliseconds to seconds                              |
| Completeness     | Full dataset available at processing time | Data arrives incrementally, completeness is eventual |
| Complexity       | Simpler — operate on static snapshots     | More complex — handle time, ordering, late data      |
| Resource model   | Burst compute, then idle                  | Continuous resource allocation                       |
| Error recovery   | Rerun the batch                           | Checkpoint and resume from last known good state     |
| State management | Typically stateless per batch             | Stateful — must maintain running aggregations        |

The choice between batch and stream is less about which is "better" and more about where on the latency-complexity spectrum the use case falls. Many systems run both — batch for correctness and stream for timeliness — converging on the same results through different paths.

## The Stream Processing Conceptual Model

### Events, Streams, and Tables

An **event** is an immutable fact — something happened at a point in time. A **stream** is an unbounded, ordered sequence of events. A **table** is a materialized view of a stream at a point in time — the accumulated state.

### Stream-Table Duality

Streams and tables are two representations of the same underlying data:

- A **stream** can be turned into a **table** by replaying events and accumulating state (applying inserts, updates, deletes in order)
- A **table** can be turned into a **stream** by capturing every change as an event (change data capture)

```
Stream of changes          Table (materialized state)
─────────────────          ──────────────────────────
{user:1, name:"Alice"}  →  | user | name  | balance |
{user:2, name:"Bob"}    →  |  1   | Alice |   100   |
{user:1, balance:100}   →  |  2   | Bob   |    50   |
{user:2, balance:50}    →  └──────┴───────┴─────────┘
```

This duality means the architecture choice between "process a stream" and "query a table" is about the access pattern, not the data itself. Systems that internalize this duality can support both operational and analytical workloads from the same data foundation.

## Windowing Concepts

Because streams are unbounded, any aggregation over a stream requires defining a finite scope — a **window**. Windows group events for collective processing.

### Window Types

| Window type       | Behavior                                                                                         | Suited for                                                          |
| ----------------- | ------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------- |
| Tumbling          | Fixed-size, non-overlapping. Each event belongs to exactly one window.                           | Regular interval reporting (per-minute counts, hourly aggregations) |
| Sliding (hopping) | Fixed-size, overlapping. Windows advance by a hop interval smaller than the window size.         | Smoothed metrics (5-minute average updated every 30 seconds)        |
| Session           | Dynamic size, defined by inactivity gap. Window closes when no events arrive for a gap duration. | User session analysis, interaction grouping                         |
| Global            | Single window encompassing the entire stream.                                                    | Full-stream aggregations (running totals, lifetime metrics)         |

### Window Assignment Challenges

- **Late data** — Events that arrive after their window has been finalized. The system must decide whether to reopen the window, update results, or discard the late event.
- **Window alignment** — Tumbling windows must agree on boundaries. "Hourly windows" could start at :00, :15, or :30 depending on configuration — affecting which events fall into which window.
- **Session gap tuning** — Session windows depend on the gap parameter. Too short → sessions fragment. Too long → unrelated activity merges into one session. The right value is domain-specific.

## Time Semantics

### Event Time vs Processing Time

**Event time** is when the event actually occurred (embedded in the event payload). **Processing time** is when the event reaches the processing system.

| Time basis      | Advantages                                                               | Challenges                                                              |
| --------------- | ------------------------------------------------------------------------ | ----------------------------------------------------------------------- |
| Event time      | Deterministic results regardless of processing delays; correct windowing | Requires handling out-of-order events; needs watermark mechanism        |
| Processing time | Simple — wall clock, no coordination needed                              | Non-deterministic — results vary with processing speed and reprocessing |

For repeatable results, event-time processing is necessary. For low-latency approximations where exact correctness is secondary, processing-time semantics reduce complexity.

### Watermarks

A watermark is a declaration: "all events with timestamp ≤ W have (probably) arrived." Watermarks allow the system to decide when a window's input is complete enough to emit results.

```
Event stream with event times:

  t=1  t=3  t=2  t=5  t=4  t=7  t=6
  ─────────────────────────────────────→ processing time
              ^
              Watermark at t=3:
              "Confident all events with t ≤ 3 have arrived"
```

Watermark strategies represent a trade-off:

- **Aggressive watermarks** (advance quickly) → Lower latency, but more late data that arrives after its window closes
- **Conservative watermarks** (advance slowly) → Higher latency, but less late data
- **Heuristic watermarks** — Based on observed event patterns, source-specific knowledge, or statistical models of delay distributions

No watermark strategy guarantees zero late data. The question is how much late data the system tolerates and what happens when late events do arrive.

### Handling Late Data

Strategies for events that arrive after their window's watermark has passed:

- **Drop** — Discard late events. Simplest approach; acceptable when late data has diminishing value.
- **Allowed lateness** — Keep the window open for an additional grace period beyond the watermark. Events within the grace period trigger updated results; events after are dropped.
- **Side output** — Route late events to a separate stream for offline reprocessing or manual handling.
- **Accumulation modes** — When a window emits updated results, it can emit the full recomputed result (accumulating), just the delta since the last emission (accumulating and retracting), or replace the previous emission.

## Exactly-Once Semantics

"Exactly-once processing" means each event affects the output state exactly once, even in the presence of failures. This is one of the most nuanced concepts in stream processing.

### What Exactly-Once Actually Means

Strictly, messages may be _delivered_ more than once (networks are unreliable), but the _effect_ on output state appears as though each message was processed exactly once. This is achieved through a combination of:

1. **At-least-once delivery** — Ensure every event is processed (retry on failure)
2. **Idempotent processing** — Ensure reprocessing the same event doesn't change the result

### Challenges

| Challenge             | Why it's hard                                                                                                                                                         |
| --------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Distributed state     | Processing state and output state may live in different systems — coordinating atomic updates across them is a distributed transaction problem                        |
| External side effects | If processing sends an email or calls an API, replaying the event causes the side effect to repeat. Exactly-once semantics apply to _state_, not to external effects. |
| End-to-end guarantees | Exactly-once within the processing framework doesn't guarantee exactly-once from source to sink unless all three components coordinate                                |
| Performance cost      | Transactional writes, deduplication tracking, and checkpoint coordination all add latency and reduce throughput                                                       |

### Implementation Approaches

- **Transactional writes** — Bundle state updates and output commits into a single transaction. Effective when the state store supports transactions, less so across heterogeneous systems.
- **Idempotent sinks** — Design the output system to handle duplicate writes gracefully (upserts on primary key, deduplication layers).
- **Distributed snapshots** — Periodically snapshot the entire pipeline state consistently, creating known-good recovery points.

Exactly-once is a spectrum: some applications need strict guarantees (financial transactions), while others tolerate occasional duplicates (click analytics at scale where a few extra counts are noise).

## State Management in Stream Processing

Stateful operations — counts, joins, windowed aggregations — require the processor to maintain state across events. This creates challenges that stateless batch processing avoids.

### State Storage Approaches

| Approach                       | Characteristics                                                                          |
| ------------------------------ | ---------------------------------------------------------------------------------------- |
| In-memory / embedded           | Fast access, limited by node memory, requires checkpointing for durability               |
| Local disk (embedded database) | Larger capacity than memory, survives process restarts, still node-local                 |
| External state store           | Shared access, independent scaling, but introduces network latency on every state access |

### Checkpointing

Periodic snapshots of processing state allow recovery without replaying the entire stream from the beginning. Checkpointing involves:

1. Pausing or coordinating in-flight processing
2. Persisting current state of all operators
3. Recording current position in the input stream (offsets/positions)

The checkpoint interval trades off between recovery time and overhead:

- Frequent checkpoints → Less data to reprocess on failure, but more I/O overhead during normal operation
- Infrequent checkpoints → Less overhead, but longer recovery with more reprocessing

### Fault Recovery

When a processor fails:

1. Restart the processor (or reassign to a healthy node)
2. Restore state from the latest checkpoint
3. Replay input events from the checkpoint's recorded position
4. Resume normal processing

This assumes the input source supports replay (reading from a specific offset). Sources that don't support replay limit the system's ability to recover without data loss.

### State Scaling and Redistribution

When processing scales horizontally (adding or removing nodes), state must be redistributed across the new topology. This involves:

- Repartitioning keyed state across the new node count
- Transferring state data between nodes
- Ensuring consistency during the transition period

State redistribution is one of the most operationally complex aspects of running stateful stream processors at scale.

## The Kappa Architecture

The Kappa architecture simplifies the Lambda architecture by eliminating the batch layer entirely. All data processing — both real-time and historical — flows through a single stream processing pipeline.

### Concepts

- Retain the raw event log indefinitely (or for a long retention period)
- For reprocessing, replay events from the log through an updated pipeline version
- Run old and new pipeline versions in parallel during migration
- Switch consumers to the new output once reprocessing completes

### Trade-offs

| Advantage                                                        | Limitation                                                               |
| ---------------------------------------------------------------- | ------------------------------------------------------------------------ |
| Single codebase — no maintaining parallel batch and stream logic | Reprocessing deep history through a stream pipeline can be slow          |
| Simpler operational model                                        | Requires the event log to be the system of record, with long retention   |
| Consistent processing semantics                                  | Some analytical queries are more naturally expressed as batch operations |
| Eliminates "serving layer" merge logic                           | Event log storage costs scale with retention depth                       |

The Kappa architecture works well when the event log is naturally the primary data store and when stream processing capabilities are sufficient for all analytical needs. It becomes strained when complex batch computations (large-scale graph analysis, model training) need to run over historical data.

## Backpressure

Backpressure is a flow control mechanism: when a downstream processor cannot keep up with upstream throughput, it signals the upstream to slow down.

### Why Backpressure Matters

Without backpressure, overwhelmed consumers either:

- Buffer events in memory until OOM → crash
- Drop events silently → data loss
- Spill to disk → latency spikes that cascade

### Backpressure Strategies

| Strategy              | Mechanism                                                                                                           |
| --------------------- | ------------------------------------------------------------------------------------------------------------------- |
| Credit-based          | Consumer tells producer how many events it can accept; producer sends no more than allocated                        |
| Rate limiting         | Fixed throughput cap regardless of consumer capacity                                                                |
| Buffer with spillover | Buffer in memory up to a threshold, then spill to disk or external storage                                          |
| Sampling/shedding     | Drop a fraction of events under load — acceptable for some analytics workloads, unacceptable for transactional data |
| Dynamic scaling       | Automatically add consumers to absorb increased throughput — addresses the cause rather than managing the symptom   |

Backpressure propagation through multi-stage pipelines is particularly challenging. If stage 3 slows down, the pressure must propagate through stage 2 back to stage 1 (and potentially to the source). Each hop in the chain adds response delay to the flow control signal.

## Join Patterns in Streaming

Joining two streams is conceptually similar to database joins but complicated by the unbounded nature of streams and the time dimension.

### Stream-Stream Joins

Joining two event streams requires defining a time window within which events from both streams can match:

```
Orders stream:   ──o1──────o2──────o3──────→
Payments stream: ────────p1──────p2────────→

Join window: ±5 minutes
o1 joins with p1 (within window)
o2 joins with p2 (within window)
o3 has no matching payment yet (window still open)
```

Challenges:

- Both streams must be buffered for the join window duration — memory scales with window size × throughput
- Deciding what to emit for unmatched events (inner join drops them, outer join emits nulls after window expiry)
- Clock skew between streams can cause missed matches

### Stream-Table Joins

Enriching a stream of events with data from a slowly-changing table (looking up customer details for each order event). Two approaches:

- **Lookup on event arrival** — Query the table for each event. Simple but introduces latency and load on the table.
- **Table changelog as stream** — Materialize the table locally by consuming its change stream. Join becomes local lookup. More complex setup but eliminates per-event network calls.

### Temporal Correctness

A subtle but important question: when a stream event joins with a table, should it use the table's _current_ value or the value _at the time of the event_? The answer depends on the semantics needed:

- Using current value is simpler but introduces temporal inconsistency — reprocessing historical events would join with today's table state, not the state that existed when the event occurred
- Using point-in-time value requires versioned table snapshots or a table changelog with timestamps

## The Lambda Architecture

The Lambda architecture addresses the tension between latency and accuracy by maintaining two parallel processing paths:

```
                    ┌─→ Batch Layer (accurate, slow) ──→ Batch View ─┐
Raw Events ─────────┤                                                 ├─→ Serving Layer
                    └─→ Speed Layer (approximate, fast) → Real-time  ─┘
```

- **Batch layer** — Periodically reprocesses the full dataset to produce accurate, complete views
- **Speed layer** — Processes recent events in real-time to produce approximate, low-latency views
- **Serving layer** — Merges batch and speed outputs, preferring batch results when available

### Why Lambda Emerged

Early stream processing systems struggled with exactly-once semantics and complex stateful operations. Batch systems were reliable but slow. Lambda combined both: stream for speed, batch for correctness, with the batch layer periodically "correcting" the speed layer's approximations.

### Criticisms

- **Dual codebase** — Maintaining the same logic in batch and stream frameworks doubles development and testing effort
- **Merge complexity** — The serving layer must reconcile potentially inconsistent results from two paths
- **Operational burden** — Two complete processing systems to monitor, debug, and maintain

As stream processing frameworks matured in their correctness guarantees, the motivation for Lambda's dual-path approach diminished for many use cases.

## Micro-Batching

Micro-batching processes events in small, frequent batches rather than one-at-a-time. It sits between pure batch and pure streaming on the latency-throughput spectrum.

| Characteristic          | True streaming                                 | Micro-batching                         | Traditional batch                    |
| ----------------------- | ---------------------------------------------- | -------------------------------------- | ------------------------------------ |
| Processing granularity  | Per event                                      | Small batches (ms to seconds)          | Large batches (minutes to hours)     |
| Latency                 | Lowest (sub-millisecond possible)              | Low (batch interval floor)             | High                                 |
| Throughput efficiency   | Lower per-event overhead but more coordination | Good — amortizes overhead across batch | Highest — full optimization possible |
| Exactly-once complexity | High — per-event coordination                  | Moderate — per-batch coordination      | Low — batch-level atomicity          |
| API model               | Event-at-a-time with state                     | Batch API applied to small windows     | Standard batch APIs                  |

Micro-batching can achieve near-streaming latency while leveraging batch-oriented optimizations (vectorized processing, bulk writes). The trade-off is a latency floor equal to the micro-batch interval — if the interval is 500ms, results cannot appear faster than 500ms regardless of event arrival time.

## Event Ordering Guarantees

### Why Ordering Matters

When events represent state changes (balance updates, status transitions), processing them out of order produces incorrect results:

```
Correct order:    set balance=100, add 50  → balance = 150
Reversed order:   add 50 (to what?), set balance=100  → balance = 100
```

### Ordering Levels

| Guarantee level  | What it means                                                                                 | Cost                                                 |
| ---------------- | --------------------------------------------------------------------------------------------- | ---------------------------------------------------- |
| No ordering      | Events may arrive in any order                                                                | Cheapest — maximum parallelism                       |
| Per-key ordering | Events with the same key arrive in order; different keys are unordered relative to each other | Moderate — partition by key                          |
| Total ordering   | All events arrive in a single global order                                                    | Expensive — limits parallelism to a single partition |
| Causal ordering  | If event A caused event B, A arrives before B; unrelated events are unordered                 | Complex — requires causal dependency tracking        |

Per-key ordering is the most common practical choice. It provides ordering where it matters (within a single entity's events) while allowing parallelism across entities. Total ordering is a throughput bottleneck and rarely necessary.

### Achieving Per-Key Ordering

Partition the stream by key, ensuring all events for a given key route to the same partition. Within a partition, events maintain their order. This works as long as:

- The partitioning key is stable (doesn't change for a given entity)
- Each partition is consumed by a single consumer thread at a time
- Repartitioning (when scaling consumers) is handled carefully to avoid temporary disorder

### The Cost of Ordering

Stronger ordering guarantees constrain parallelism. A system that guarantees total ordering across all events is limited to single-threaded processing — no horizontal scaling. Relaxing ordering to per-key allows partitioned parallelism proportional to the number of distinct keys.

Ordering is also at tension with exactly-once semantics during failure recovery: replaying events from a checkpoint may introduce temporary reordering relative to events already processed. The system must handle this gracefully, typically through idempotent operations or sequence number tracking.

## Operational Considerations

### Scaling Patterns

Stream processors scale by partitioning work across parallel consumers. Scaling decisions involve:

- **Partition count** — Determines the maximum parallelism. Too few partitions bottleneck throughput; too many create coordination overhead.
- **Consumer group management** — Adding or removing consumers triggers partition rebalancing, which temporarily disrupts processing.
- **Stateful scaling** — Scaling stateful processors requires redistributing state, which is significantly more complex than scaling stateless processors.

### Monitoring Stream Processing

Key metrics that indicate pipeline health:

| Metric                | What it reveals                                                                                          |
| --------------------- | -------------------------------------------------------------------------------------------------------- |
| Consumer lag          | How far behind the processor is from the latest event — growing lag indicates the consumer can't keep up |
| Processing latency    | Time between event arrival and result emission — spikes indicate processing bottlenecks                  |
| Checkpoint duration   | How long state snapshots take — growing duration suggests state size is increasing unsustainably         |
| Rebalance frequency   | How often partition assignment changes — frequent rebalances indicate instability                        |
| Watermark progression | How fast the watermark advances — stalled watermarks indicate missing or delayed events from a source    |

### The Exactly-Once Cost in Practice

The operational cost of exactly-once semantics is often underestimated:

- Throughput typically decreases 20-50% compared to at-least-once, depending on implementation
- Transactional coordination adds tail latency
- Recovery time increases due to checkpoint coordination
- Debugging becomes harder when the system masks duplicates

For many analytics workloads, at-least-once with idempotent sinks achieves the same practical outcome at lower operational cost. Exactly-once is most justified when the cost of a duplicate or missed event is high relative to the performance overhead.
