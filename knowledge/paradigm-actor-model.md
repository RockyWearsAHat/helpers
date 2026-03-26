# The Actor Model — Message-Passing Concurrency

## Core Concept

The actor model is a mathematical model of concurrent computation where the **actor** is the fundamental unit. An actor is an isolated entity that:

1. **Receives messages** via a mailbox (queue)
2. **Processes messages** one at a time, sequentially
3. **In response to a message**, can do exactly three things:
   - Send messages to other actors it knows about
   - Create new actors
   - Designate the behavior for the next message it receives

There is no shared mutable state between actors. All coordination happens through asynchronous message passing. This eliminates an entire category of concurrency bugs — data races, deadlocks from lock ordering, and the combinatorial complexity of shared-memory synchronization.

The model was formalized in 1973 and has influenced language design, distributed systems, and concurrent programming for decades.

## Actor Anatomy

```
┌─────────────────────────────┐
│           Actor              │
│                              │
│  ┌──────────┐               │
│  │ Mailbox  │ ← messages    │
│  │ (queue)  │   arrive here │
│  └────┬─────┘               │
│       │ dequeue one at a time│
│       ▼                      │
│  ┌──────────┐               │
│  │ Behavior │ current       │
│  │ function │ message       │
│  └────┬─────┘ handler       │
│       │                      │
│       ▼                      │
│  ┌──────────┐               │
│  │  State   │ private,      │
│  │          │ mutable only  │
│  │          │ by this actor │
│  └──────────┘               │
└─────────────────────────────┘
```

### Mailbox

The mailbox is a message queue that decouples sender from receiver. Senders deposit messages without blocking; the actor processes them sequentially. This sequential processing guarantee means an actor never needs internal locks — it handles one message at a time.

### Behavior

The behavior is the function that determines how the actor responds to the current message. Crucially, an actor can **change its behavior** for the next message — this is how actors maintain state without mutable variables in the traditional sense.

```
// Conceptual: a counter actor
behavior counter(n):
  on Increment → become counter(n + 1)
  on GetCount(replyTo) → send n to replyTo; become counter(n)
```

The `become` mechanism replaces external mutable state with behavior evolution. Each message processed potentially yields a new behavior for handling subsequent messages.

### State

Actor state is completely private. No other actor can read or modify it. The only way to observe an actor's state is to send it a message asking it to report its state back — and even then, the response reflects the state at the time of processing, not the time of asking.

## Message Passing Semantics

### Asynchronous and Non-Blocking

Sending a message is a fire-and-forget operation. The sender is not blocked waiting for the receiver to process the message. This asynchrony is fundamental — it enables actors to operate concurrently without coordination overhead at send time.

```
actorA sends Message1 to actorB  // returns immediately
actorA sends Message2 to actorC  // does not wait for B to process Message1
```

### Message Delivery Guarantees

The basic actor model provides **at-most-once delivery** — messages may be lost (especially across networks) but are never duplicated by the transport. Different implementations layer additional guarantees:

| Guarantee     | Meaning                                            | Cost                                           |
| ------------- | -------------------------------------------------- | ---------------------------------------------- |
| At-most-once  | Message delivered zero or one times                | Lowest overhead; no tracking                   |
| At-least-once | Message delivered one or more times; may duplicate | Requires retry and ack                         |
| Exactly-once  | Message delivered precisely once                   | Requires deduplication on top of at-least-once |

Exactly-once semantics in distributed systems are achievable only through idempotent message handling or deduplication at the application level — the network cannot guarantee it alone.

### Message Ordering

Between a specific pair of actors, messages are typically delivered in the order sent. But when multiple actors send to the same recipient, no global ordering is guaranteed:

```
A sends M1 to C
A sends M2 to C
B sends M3 to C

C receives: M1 before M2 (guaranteed — same sender)
            M3 position relative to M1, M2 (undefined)
```

This partial ordering is sufficient for most actor interactions but requires attention when coordinating multi-actor protocols.

## Tell Pattern vs Ask Pattern

Two fundamental communication styles:

### Tell (fire-and-forget)

```
actorA.tell(actorB, DoWork(data))
// A continues immediately, doesn't expect a response
```

- Non-blocking, fully asynchronous
- Highest throughput — no response tracking overhead
- Suitable when the sender doesn't need a result
- The natural actor communication style

### Ask (request-response)

```
future = actorA.ask(actorB, GetStatus(), timeout=5s)
// Creates a temporary actor to receive the response
// Returns a future that completes when response arrives or timeout expires
```

- Creates a temporary mailbox for the response
- Introduces timeout semantics — what happens if the response never comes?
- Adds overhead: temporary actor creation, future management, timeout scheduling
- Necessary when the caller needs a result to proceed

The ask pattern is convenient but introduces coupling and resource overhead that tell avoids. A common recommendation is to prefer tell and restructure protocols to be message-driven rather than request-response, though this is not universally applicable — sometimes ask is the pragmatic choice.

## Location Transparency

A defining feature of the actor model: the programming model is identical whether actors are in the same process, on different threads, on different machines, or in different data centers.

```
// Same code works regardless of actor location
actorRef.tell(ProcessOrder(order))
// actorRef might be:
//   - local (same JVM/process)
//   - remote (different machine, different continent)
//   - clustered (automatically placed by the runtime)
```

This is possible because:

- Communication is always through messages (serializable data)
- Actors are referenced by addresses/references, not direct pointers
- The runtime handles routing, serialization, and transport

Location transparency enables deployment flexibility — actors can be redistributed across nodes for scaling or fault tolerance without code changes. However, it can also mask performance realities: a "local" method call taking nanoseconds and a "remote" message taking milliseconds are not equivalent, even if the programming model treats them identically.

## Supervision Hierarchies

### Parent-Child Relationships

When an actor creates another actor, it becomes that actor's **supervisor**. This creates a tree structure:

```
        /system
           │
        /user
       /    \
    orderMgr  inventoryMgr
    /    \         |
  order1  order2  stock
```

### The "Let It Crash" Philosophy

Rather than defensive programming with try-catch at every level, the actor model embraces failure:

1. If an actor encounters an unrecoverable error, it crashes
2. Its supervisor is notified
3. The supervisor decides what to do based on a **supervision strategy**

### Supervision Strategies

| Strategy     | Behavior                                             | Use case                                                |
| ------------ | ---------------------------------------------------- | ------------------------------------------------------- |
| **Resume**   | Ignore failure, actor continues with current state   | Transient errors that don't corrupt state               |
| **Restart**  | Stop actor, create fresh instance with initial state | State corruption where a clean start resolves the issue |
| **Stop**     | Permanently terminate the failed actor               | Unrecoverable conditions, or intentional shutdown       |
| **Escalate** | Pass failure up to own supervisor                    | Supervisor cannot handle this failure class             |

Supervision strategies can be applied selectively based on exception type:

```
// Conceptual supervision strategy
strategy:
  ArithmeticException → Resume
  DatabaseException   → Restart (max 3 times in 60 seconds)
  FatalException      → Stop
  Unknown             → Escalate
```

### Why Supervision Works

- **Separation of concerns**: the actor doing work doesn't handle its own failure recovery — its supervisor does
- **Hierarchical containment**: failures are contained at the lowest possible level before escalating
- **Clean restart semantics**: restarted actors get fresh state, eliminating corrupted-state bugs
- **Recursive resilience**: the supervision tree means every actor (except the root) has a failure handler

This is fundamentally different from exception-based error handling, where the code that encounters the error must also decide how to recover from it.

## Actor Lifecycle

```
Created → Started → [Processing Messages] → Stopping → Stopped
                          │        ▲
                          │        │
                      Restarted ───┘
                     (by supervisor)
```

### Lifecycle Hooks

| Hook            | When                                    | Purpose                                    |
| --------------- | --------------------------------------- | ------------------------------------------ |
| **preStart**    | After creation, before first message    | Initialize resources, subscribe to events  |
| **postStop**    | After actor stops processing            | Release resources, deregister              |
| **preRestart**  | Before restart (after failure)          | Save state for recovery if needed          |
| **postRestart** | After restart, before resuming messages | Reinitialize with fresh or recovered state |

### Death Watch

Actors can watch other actors for termination:

```
actorA watches actorB
// If B terminates (for any reason):
//   A receives Terminated(actorB) message
```

This enables cleanup, failover, and dependency tracking without coupling to the supervision hierarchy.

## Mailbox Strategies

The mailbox implementation significantly affects actor behavior:

| Strategy           | Ordering                     | Behavior                                                          |
| ------------------ | ---------------------------- | ----------------------------------------------------------------- |
| **Unbounded FIFO** | First-in, first-out          | Default; simple; risk of OOM under sustained load                 |
| **Bounded FIFO**   | First-in, first-out          | Drops or blocks when full; provides backpressure                  |
| **Priority**       | By message priority          | Urgent messages processed first; starvation risk for low-priority |
| **Stash-capable**  | FIFO with temporary deferral | Actor can stash messages for later processing when in wrong state |

### Stashing

Stashing allows an actor to temporarily defer messages it cannot handle in its current behavioral state:

```
behavior waitingForInit:
  on InitComplete(config) →
    become ready(config)
    unstash all  // reprocess deferred messages
  on other →
    stash  // save for later
```

This pattern is valuable for actors that go through initialization phases or state transitions where certain messages are only meaningful in certain states.

## Actor Model vs Thread-Based Concurrency

| Aspect           | Threads + Locks                                     | Actor Model                                   |
| ---------------- | --------------------------------------------------- | --------------------------------------------- |
| State sharing    | Shared mutable state, protected by locks            | No shared state; private to each actor        |
| Communication    | Method calls, shared variables                      | Asynchronous messages                         |
| Synchronization  | Explicit (mutex, semaphore, monitor)                | Implicit (sequential mailbox processing)      |
| Failure handling | Try-catch, often ad-hoc                             | Supervision hierarchies                       |
| Scaling          | Thread pools, limited by OS threads                 | Millions of lightweight actors per VM         |
| Deadlock risk    | Lock ordering mistakes                              | No locks, but possible message-level livelock |
| Reasoning        | Requires reasoning about all possible interleavings | Each actor reasons sequentially               |
| Debugging        | Race conditions are non-deterministic               | Message ordering issues are more reproducible |

The actor model trades the fine-grained control of shared-memory concurrency for a constrained model that eliminates entire categories of bugs. However, actors introduce their own complexity — message protocol design, mailbox overflow management, and distributed coordination challenges.

## Distributed Actors — Challenges

Location transparency makes distribution _possible_ but not _easy_. Distributed actor systems face fundamental challenges:

### Network Partitions

When network connectivity between actor nodes is lost, the system must decide:

- Are remote actors dead or just unreachable?
- Should local actors continue processing, potentially creating split-brain state?
- How are actors reassigned when nodes rejoin?

### Message Ordering Across Nodes

Local message ordering (A→B, A→C: order preserved per pair) is straightforward. Across network hops, ordering guarantees weaken. Multi-hop routing, retries, and failover can reorder messages that were ordered at send time.

### Consistency vs Availability

Distributed actors face the same CAP theorem constraints as any distributed system. Actor-based distributed systems typically favor availability and partition tolerance (AP) with eventual consistency, though this is an architectural choice, not an inherent constraint.

### Cluster Membership

Determining which nodes are part of the cluster, detecting node failures, and rebalancing actor placement when membership changes requires consensus protocols or gossip-based failure detection — each with their own trade-offs in detection speed, false positive rate, and network overhead.

## Virtual Actors / Grains

Virtual actors (sometimes called "grains") extend the actor model with automatic lifecycle management:

| Aspect            | Traditional Actors               | Virtual Actors                           |
| ----------------- | -------------------------------- | ---------------------------------------- |
| Creation          | Explicit (`system.actorOf(...)`) | Implicit (first message creates)         |
| Destruction       | Explicit or supervised           | Automatic (garbage collected after idle) |
| Location          | Fixed or manually migrated       | Runtime-placed, transparently migrated   |
| Addressing        | Runtime-generated references     | Stable identity (type + key)             |
| State persistence | Application-managed              | Framework-managed (auto-persisted)       |
| Single-activation | Not guaranteed across cluster    | Guaranteed: one instance per identity    |

Virtual actors simplify distributed stateful programming by eliminating explicit lifecycle management, placement decisions, and state persistence code. The trade-off is less control over placement, activation timing, and resource management. A critical property: for any given virtual actor identity, at most one instance exists in the cluster at any time — eliminating distributed state conflicts but introducing reactivation challenges during host failures.

## Patterns in Actor Systems

### Router Pattern

Distribute messages across a pool of worker actors:

```
       ┌─→ worker1
router ┼─→ worker2
       └─→ worker3

Strategies: round-robin, random, smallest-mailbox, consistent-hashing
```

### Saga / Process Manager

Coordinate multi-step distributed operations: each step sends a message to the responsible actor, and on any failure, compensating actions undo completed steps.

### Aggregate Pattern

An actor encapsulating a domain aggregate (in DDD terms), ensuring all state changes go through the actor's sequential mailbox — providing natural consistency without distributed locks.

### Circuit Breaker

An intermediary actor monitoring downstream health: Closed (passing messages) → Open (rejecting after N failures) → Half-Open (testing recovery).

### Event Sourcing with Actors

Actors naturally pair with event sourcing:

- Messages trigger state changes
- State changes are recorded as events
- Actor state is recoverable by replaying the event log
- The sequential mailbox guarantees event ordering per actor

## When Actors Are a Good Fit

**Stateful concurrent systems**: When many independent stateful entities must operate concurrently — chat rooms, game entities, user sessions, IoT devices — actors model the natural entity boundaries with built-in concurrency safety.

**Distributed systems**: Location transparency and supervision hierarchies provide a programming model that extends naturally across nodes. Systems that need to scale horizontally while maintaining per-entity state benefit from the actor abstraction.

**Real-time and event-driven systems**: Systems processing high volumes of events where different event types route to different processing logic — financial trading, real-time analytics, multiplayer game servers.

**Fault-tolerant systems**: The supervision hierarchy and let-it-crash philosophy provide structured fault tolerance that is difficult to achieve with traditional try-catch error handling.

## When They Add Unnecessary Complexity

**Simple CRUD applications**: If the workload is stateless request-response — read from database, transform, return — actors add an indirection layer (message serialization, mailbox queueing, async response handling) without corresponding benefit. A straightforward service layer with connection pooling may suffice.

**Highly sequential workflows**: When operations are inherently sequential and single-threaded — batch processing a file line by line, a synchronous pipeline — the actor overhead of async messaging adds latency without enabling parallelism.

**When shared state is actually simple**: If the shared state is a single counter or a bounded cache, a concurrent data structure or a read-write lock may be simpler than an actor wrapping that state.

**Small teams or codebases**: The conceptual overhead of designing message protocols, supervision strategies, and actor hierarchies may outweigh the concurrency benefits in small systems with modest concurrency requirements.

**Synchronous integration requirements**: When integrating with systems that are inherently synchronous (traditional database transactions, synchronous RPC), the async nature of actors requires bridging patterns (ask + await) that can negate the non-blocking benefits.

## Design Trade-Offs

### Single Responsibility vs Message Overhead

Fine-grained actors (one per entity) provide clean isolation but increase message-passing overhead. Coarser actors reduce messages but concentrate state and reintroduce some complexity within the actor. The granularity decision depends on the ratio of inter-entity communication to intra-entity computation.

### Mailbox as Hidden Queue

Every actor mailbox is effectively an unbounded queue (by default). Under sustained load, mailboxes grow, increasing memory usage and processing latency. This is the actor model's backpressure problem — unlike reactive streams with explicit demand signaling, actors accept all messages and buffer indefinitely unless bounded mailboxes or work-pulling patterns are employed.

### Testing Actors

Testing actor-based systems requires different patterns than testing object-oriented code — state is not directly observable and must be inferred from message responses or side effects. Common approaches include synchronous test execution, test probe actors that collect messages for assertion, and message log assertion.

### Debugging Distributed Actors

Debugging actor systems is challenging because execution is non-deterministic, state is distributed across many actors, traditional step-through debuggers are less useful for async message processing, and causal relationships between messages may span many actors. Structured logging with correlation IDs and message tracing infrastructure are essential for production actor systems.

## Relationship to Other Concurrency Models

| Model                                        | Shared State           | Communication             | Granularity     |
| -------------------------------------------- | ---------------------- | ------------------------- | --------------- |
| **Actors**                                   | None (isolated)        | Async messages            | Per-entity      |
| **CSP (Communicating Sequential Processes)** | None                   | Synchronous channels      | Per-process     |
| **Threads + Locks**                          | Shared memory          | Method calls, shared vars | Per-thread      |
| **Software Transactional Memory**            | Shared (transactional) | Memory transactions       | Per-transaction |
| **Coroutines / async-await**                 | Shared (cooperative)   | Direct calls + yield      | Per-task        |

CSP and actors are philosophically similar (no shared state, message-based communication) but differ in synchronization: CSP channels are synchronous (sender blocks until receiver is ready), while actor messages are asynchronous (sender never blocks). This makes actors more naturally suited to distributed systems where synchronous rendez-vous across a network is impractical.
