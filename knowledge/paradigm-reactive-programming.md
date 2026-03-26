# Reactive Programming — Streams, Observables & Dataflow

## Core Concept

Reactive programming models computation as the propagation of change through data streams. Rather than imperatively fetching data when needed (pull), reactive systems push data to interested consumers as it becomes available. This inversion of control — from "ask for data" to "react to data" — fundamentally changes how asynchronous and event-driven code is structured.

The paradigm treats everything as a stream: mouse clicks, HTTP responses, database changes, sensor readings, timer ticks. Once data sources are unified under the stream abstraction, the same composition and transformation tools apply regardless of origin.

## The Observer Pattern as Foundation

Reactive programming extends the classic observer pattern with three critical additions:

| Aspect            | Classic Observer           | Reactive Streams                    |
| ----------------- | -------------------------- | ----------------------------------- |
| Data delivery     | `onNext(value)`            | `onNext(value)`                     |
| Error propagation | Typically absent or ad-hoc | `onError(throwable)` — first-class  |
| Completion signal | Rarely formalized          | `onComplete()` — explicit lifecycle |
| Composition       | Manual wiring              | Operator chains                     |
| Backpressure      | Not addressed              | Demand-based flow control           |

The classic observer notifies subscribers of events but provides no standard mechanism for error handling, completion signaling, or flow control. Reactive extensions formalize these into a contract that producers and consumers both honor.

## Push vs Pull — The Control Inversion

**Pull-based (iterator) model:**

```
Consumer decides when to request next item
  consumer.next() → producer yields value
  consumer.next() → producer yields value
  consumer.next() → producer signals done
```

**Push-based (reactive) model:**

```
Producer decides when to emit next item
  producer emits → consumer.onNext(value)
  producer emits → consumer.onNext(value)
  producer signals → consumer.onComplete()
```

The pull model suits bounded, synchronous data — iterating a collection, reading a file line by line. The push model suits unbounded, asynchronous data — user interactions, WebSocket messages, system events. Some reactive implementations support a hybrid pull-push model where consumers signal demand (backpressure) while producers retain control over emission timing.

## The Observable/Subscriber Model

The observable represents a stream of values over time. Subscribers attach to observables to receive emissions. The contract between them:

```
Observable<T>
  ├── subscribe(Subscriber<T>) → Subscription
  │     ├── onNext(T)       // 0..N data items
  │     ├── onError(E)      // terminal: exactly once, mutually exclusive with onComplete
  │     └── onComplete()    // terminal: exactly once, mutually exclusive with onError
  └── Subscription
        ├── request(n)      // demand signaling (backpressure)
        └── cancel()        // unsubscribe, release resources
```

Key properties of this contract:

- **Sequential delivery**: `onNext` calls are serialized — no concurrent invocations on the same subscriber
- **Terminal signals are final**: after `onError` or `onComplete`, no further emissions occur
- **Subscription lifecycle**: resources are acquired on subscribe, released on cancel or terminal signal
- **Lazy by default**: no work happens until subscription occurs (for cold observables)

## Operators as Composable Stream Transformations

Operators transform, combine, and control streams without breaking the observable contract. They form a declarative pipeline:

### Transforming

| Operator  | Behavior                                           |
| --------- | -------------------------------------------------- |
| `map`     | Transform each emitted item: `A → B`               |
| `flatMap` | Transform each item into a stream, merge results   |
| `scan`    | Accumulate values, emit each intermediate result   |
| `buffer`  | Collect items into batches, emit batch as list     |
| `window`  | Like buffer but emits sub-streams instead of lists |
| `groupBy` | Partition stream by key into grouped sub-streams   |

### Filtering

| Operator   | Behavior                                                  |
| ---------- | --------------------------------------------------------- |
| `filter`   | Emit only items matching a predicate                      |
| `distinct` | Suppress duplicate consecutive (or all) items             |
| `take(n)`  | Emit first N items, then complete                         |
| `skip(n)`  | Suppress first N items                                    |
| `debounce` | Emit item only after a quiet period with no new emissions |
| `throttle` | Emit at most one item per time window                     |

### Combining

| Operator         | Behavior                                                            |
| ---------------- | ------------------------------------------------------------------- |
| `merge`          | Interleave emissions from multiple streams                          |
| `concat`         | Emit from streams sequentially, one after another                   |
| `zip`            | Pair items from multiple streams by index                           |
| `combineLatest`  | Emit when any stream emits, using latest from each                  |
| `switchMap`      | On new emission, cancel previous inner stream, subscribe to new one |
| `withLatestFrom` | Combine emission with most recent value from another stream         |

### Error handling

| Operator                | Behavior                                    |
| ----------------------- | ------------------------------------------- |
| `retry(n)`              | Resubscribe on error, up to N times         |
| `onErrorResumeNext`     | Switch to fallback stream on error          |
| `catch / onErrorReturn` | Replace error with default value            |
| `timeout`               | Signal error if no emission within duration |

The power of operators lies in their composability — complex async coordination reduces to a pipeline of simple, testable transformations.

## Hot vs Cold Observables

This distinction is one of the most consequential and frequently misunderstood aspects of reactive programming.

**Cold observables** produce data on subscription. Each subscriber gets its own independent execution:

```
coldObservable.subscribe(A)  // A gets items 1, 2, 3
coldObservable.subscribe(B)  // B gets items 1, 2, 3 (independent execution)
```

Analogies: reading a file, making an HTTP request, iterating a collection. Each subscriber triggers fresh production.

**Hot observables** produce data regardless of subscribers. Subscribers receive emissions from the point of subscription onward:

```
hotObservable emits: 1, 2, 3, 4, 5
  subscriber A joins at item 2 → sees 2, 3, 4, 5
  subscriber B joins at item 4 → sees 4, 5
```

Analogies: mouse events, stock tickers, WebSocket messages. Data exists independently of observation.

### The Subscription Timing Problem

Hot observables create race conditions between subscription and emission. Late subscribers miss data. Strategies for mitigation:

- **Replay**: buffer N recent items, deliver to late subscribers
- **Behavior/Latest**: always hold the most recent value for immediate delivery
- **Publish + connect**: multicasting that decouples subscription from activation
- **Share**: convert cold to hot with reference counting — first subscriber triggers, last unsubscribe cleans up

The choice between hot and cold has resource implications. Cold observables are safe (no missed data) but may duplicate expensive work. Hot observables share work but require careful subscriber lifecycle management.

## Backpressure — When Producers Outpace Consumers

Backpressure occurs when a producer emits data faster than a consumer can process it. Without mitigation, this causes unbounded buffer growth, memory exhaustion, or dropped data.

### Strategies

| Strategy             | Mechanism                                   | Trade-off                                           |
| -------------------- | ------------------------------------------- | --------------------------------------------------- |
| **Buffering**        | Accumulate excess items in memory           | Solves short bursts; unbounded buffers risk OOM     |
| **Dropping**         | Discard items when consumer is busy         | No memory growth; data loss                         |
| **Latest**           | Keep only most recent item, drop older ones | Useful for UI state; ignores intermediate values    |
| **Error**            | Signal error when buffer exceeds threshold  | Fail-fast; consumer must handle                     |
| **Demand signaling** | Consumer requests N items at a time         | Cooperative flow control; requires producer support |

Demand-based backpressure (the `request(n)` model) is the foundation of the Reactive Streams specification. The consumer controls the pace:

```
Subscriber: request(10)      // "I can handle 10 items"
Publisher:  onNext × 10      // sends exactly 10
Subscriber: request(5)       // "ready for 5 more"
Publisher:  onNext × 5       // sends 5
```

This creates cooperative flow control without buffering at the boundary — the producer slows down rather than the consumer drowning.

## The Reactive Streams Specification

The Reactive Streams specification defines a minimal contract for asynchronous stream processing with non-blocking backpressure. Four interfaces:

```
Publisher<T>    — source of elements, accepts Subscribers
Subscriber<T>  — receives elements and signals
Subscription   — link between Publisher and Subscriber, enables demand signaling
Processor<T,R> — both Subscriber and Publisher, acts as transformation stage
```

The specification deliberately avoids prescribing operators, threading models, or implementation strategies. It defines only the interop contract — the wire protocol between components. This allows different implementations to interoperate at stream boundaries.

Key rules from the specification:

- A Publisher must respect Subscriber demand — never emit more than requested
- Signals (`onNext`, `onError`, `onComplete`) must be serialized — no concurrent calls
- `Subscription.cancel()` must be idempotent and thread-safe
- Both Publisher and Subscriber must behave correctly even if the other misbehaves

## Functional Reactive Programming (FRP)

FRP, as originally formulated, models two concepts:

- **Behaviors**: continuous, time-varying values (e.g., the current mouse position at any instant)
- **Events**: discrete occurrences at specific points in time (e.g., a mouse click)

This differs from reactive streams, which model only discrete events. In FRP, a behavior has a value at every point in time — it can be sampled. An event stream has values only at specific moments.

The distinction matters conceptually:

```
Behavior: temperature = f(t) — has a value at every instant
Event:    buttonClick — value exists only at click moments
```

Most practical reactive libraries implement event-stream semantics rather than continuous FRP. The FRP label is often applied loosely to any reactive approach, though purists maintain the distinction. Continuous-time FRP remains primarily an academic construct, while discrete event-stream reactive programming has seen broad industrial adoption.

## The Marble Diagram

Marble diagrams are the standard visualization for reactive streams:

```
source:    --1--2--3--4--5--|
filter(odd):--1-----3-----5--|
map(*10):  --10----30----50--|

Legend:
  --  time axis (left to right)
  1   emitted value
  |   completion signal
  X   error signal
```

For operators combining multiple streams:

```
stream A: --1----3----5---------|
stream B: ----2----4----6-------|
merge:    --1-2--3-4--5-6-------|
zip:      ----[1,2]-[3,4]-[5,6]-|
```

Marble diagrams are particularly valuable for understanding timing-sensitive operators like `debounce`, `throttle`, `combineLatest`, and `switchMap`, where the relative timing of emissions determines output behavior.

## Scheduling and Threading

Reactive pipelines are agnostic to threading by default — operators execute on whatever thread triggers the emission. Schedulers provide explicit control:

| Scheduler type  | Typical use                                        |
| --------------- | -------------------------------------------------- |
| Computation     | CPU-bound work, parallel processing                |
| I/O             | Blocking operations, network calls, file access    |
| Single-threaded | Sequential processing, UI thread operations        |
| Immediate       | Execute synchronously on current thread            |
| Trampoline      | Queue work on current thread, execute sequentially |

Two operators control where work happens:

- **subscribeOn**: determines which scheduler handles the subscription and initial emission
- **observeOn**: determines which scheduler handles downstream operators

This separation allows a pipeline to subscribe on an I/O scheduler (for network fetch), then observe on a UI scheduler (for rendering), without manual thread management.

## When Reactive Programming Shines

**Event-heavy user interfaces**: UI interactions generate numerous asynchronous event streams — clicks, text input, drag gestures, resize events. Reactive composition handles debouncing search input, combining form fields for validation, or coordinating complex gesture recognition naturally.

**Real-time data feeds**: Stock tickers, sensor networks, social media streams, monitoring dashboards — scenarios with continuous data arrival and multiple consumers benefit from the multicast and transformation capabilities of reactive streams.

**Complex asynchronous coordination**: When multiple async operations must be coordinated — retry with exponential backoff, race between timeout and response, combine results from parallel API calls, poll until condition met — reactive operators express these patterns more declaratively than nested callbacks or manually managed promises.

**Streaming data processing**: ETL pipelines, log processing, and data transformation workflows where backpressure-aware streaming prevents memory issues with large datasets.

## When It Becomes a Liability

**Simple request-response patterns**: A single HTTP call returning a result does not benefit from observable wrapping. The overhead of stream machinery for one-shot operations adds complexity without proportional benefit.

**Debugging complexity**: Reactive stack traces are notoriously opaque. An error in a deeply composed pipeline produces traces through operator internals rather than application code. The declarative nature that aids readability paradoxically hinders debugging — the code says _what_ but obscures _when_ and _where_.

**Learning curve and team adoption**: Reactive programming requires a mental model shift from sequential/imperative thinking. Teams unfamiliar with the paradigm may produce code that is harder to maintain than equivalent imperative alternatives. The operator vocabulary is large, and subtle differences between operators (e.g., `flatMap` vs `switchMap` vs `concatMap`) can introduce bugs that are difficult to diagnose.

**Testing challenges**: Time-dependent operators (`debounce`, `timeout`, `delay`) require virtual time schedulers for deterministic testing. Testing hot observable behavior and subscription timing adds test infrastructure complexity.

**Over-abstraction risk**: Wrapping simple synchronous operations in reactive streams "for consistency" introduces unnecessary indirection. Not every data flow benefits from being a stream.

## Relationship to Event Sourcing and CQRS

Reactive programming, event sourcing, and CQRS share the theme of event-centricity but operate at different architectural levels:

| Aspect     | Reactive Programming           | Event Sourcing             | CQRS                           |
| ---------- | ------------------------------ | -------------------------- | ------------------------------ |
| Level      | Code/library                   | Architecture/persistence   | Architecture/data access       |
| Core idea  | Compose async event streams    | Persist state as event log | Separate read and write models |
| Events are | Transient stream items         | Durable state records      | Commands and queries           |
| Replay     | Operator-level (replay buffer) | Full state reconstruction  | Read model projection          |

These patterns complement rather than compete. A system might use event sourcing for persistence, CQRS for separating read/write concerns, and reactive streams for propagating changes from the event store to read model projections and real-time subscriptions.

## The Ecosystem Landscape

Reactive programming has been adopted across language ecosystems, each with different emphases:

- **JVM ecosystem**: strong adoption in both Android UI programming and server-side streaming. The Reactive Streams specification originated here and is incorporated into the platform standard library.
- **JavaScript/TypeScript**: widely used for frontend event handling and increasingly for server-side stream processing. The observable proposal has had a long standardization journey.
- **.NET ecosystem**: one of the earliest reactive extensions implementations, with deep integration into the LINQ query model.
- **Systems languages**: reactive patterns appear in async runtime designs, though the emphasis tends toward lower-level async/await primitives rather than high-level observable abstractions.
- **Mobile platforms**: reactive patterns are deeply embedded in modern declarative UI frameworks, where UI is expressed as a function of reactive state.

## Common Anti-Patterns

- **Nested subscribes**: subscribing inside a subscribe callback defeats composition — use `flatMap`/`switchMap` instead
- **Ignoring disposal**: failing to unsubscribe creates memory leaks, especially with hot observables tied to long-lived sources
- **Side effects in operators**: operators like `map` should be pure; side effects belong in `doOnNext`/`tap` or the terminal subscriber
- **Overusing subjects**: subjects (both observable and observer) bypass the declarative pipeline model and can introduce hidden mutable state
- **Blocking in reactive pipelines**: synchronous blocking in an async pipeline defeats the non-blocking benefit and can cause thread starvation

## Design Considerations

When evaluating whether reactive programming fits a particular context:

- **Is the data naturally streaming?** Continuous or event-driven data suits reactive modeling better than request-response data.
- **Are there multiple consumers?** Multicasting and operator composition provide the most value when multiple downstream consumers need different views of the same data.
- **Is backpressure a concern?** If producer-consumer speed mismatch is possible, reactive streams' demand signaling is a significant advantage over ad-hoc buffering.
- **What is the team's familiarity?** The learning curve is real. Introducing reactive programming in a codebase maintained by a team without experience requires investment in education and code review practices.
- **What are the debugging requirements?** If rapid issue diagnosis is critical, the opacity of reactive stack traces should factor into the decision.
