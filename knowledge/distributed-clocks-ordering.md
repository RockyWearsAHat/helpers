# Distributed Clocks & Ordering — Time, Causality, and Total Order

## Overview

Distributed systems lack a global clock. Nodes cannot synchronize physical time perfectly; message delays are unpredictable. Without shared time, establishing order of events, detecting causality, and reasoning about consistency becomes subtle. This space is dominated by logical clocks (Lamport, vector) and hybrid approaches that blend physical time with logical ordering.

## Physical Clocks and NTP

Physical clocks (wall-clock time) drift—even atomic clocks. The Network Time Protocol (NTP) synchronizes clocks across networks.

### NTP Basics
Nodes contact NTP servers in a hierarchical stratum. The primary stratum (atomic/GPS clocks) provides authoritative time. Secondary strata contact primaries or other secondaries. A client queries multiple servers, discards outliers, and selects a best server by round-trip latency and jitter.

**Interval estimation:** NTP computes a confidence interval. If client sends a request at time T₀ and receives a reply at T₁, the server's time is approximately (T₀ + T₁)/2, with uncertainty of roughly (T₁ - T₀)/2 (the round-trip latency).

**Limitations:**
- Network delays are variable; latency adds uncertainty to any estimate.
- Distributed system clocks can still skew by milliseconds or more within a small cluster.
- NTP synchronizes to ~milliseconds in good conditions, microseconds with PTP (Precision Time Protocol).

### Why Not Just Use Physical Time?
Tests like "did event A happen before event B?" require comparing timestamps. In a distributed system:
- Two events might have timestamps 100ms apart, but arrive at a replica out-of-order due to asynchronous networks.
- A clock can jump backward (NTP adjusts the system clock; this is bad for applications).
- Events at different machines with synchronized clocks can still have causality violations (clock skew introduces ambiguity).

Google Spanner partially solves this using TrueTime: GPS and atomic clocks in every datacenter, with explicit error bounds. Applications read the error interval and wait if needed to ensure strong consistency. This is expensive and not practical for most systems.

## Lamport Logical Clocks

Lamport (1978) proposed an algorithm that orders events without accurate physical clocks.

### The Idea
Each process maintains a logical clock (an integer). For every event (send, receive, local action):
1. Increment the process's logical clock.
2. Include the logical clock value in the event.

When receiving a message with clock value m, set your clock to max(your_clock, m) + 1.

**Result:** If event A happens before event B (causally), A's Lamport clock ≤ B's Lamport clock.

### Limitations
Two events with the same Lamport clock value are **concurrent**—neither happened before the other. Lamport clocks provide a **partial order** (some events are unordered), not a total order.

**Example:** Process 1 increments to 2 and sends. Process 2 independently increments to 2 and sends. Both sends have clock 2, but neither causally depends on the other.

## Vector Clocks

Vector clocks extend Lamport clocks to capture full causality.

### The Algorithm
Each process maintains a vector of N clocks (one per process). For each event:
1. Increment the local entry in the vector.
2. Include the vector in the message.

When receiving a message with vector v, merge: for each entry i, set vc[i] = max(vc[i], v[i]), then increment vc[local].

**Result:** Event A happened before B iff A's vector is less than B's vector (element-wise ≤, strict on at least one entry).

### Advantages and Drawbacks
**Advantages:** Vector clocks determine total causality; if A → B in the causal order, A's vector is strictly less than B's.

**Drawbacks:**
- Vector size grows with the number of processes. In a large cluster, every message carries an N-element vector.
- Not practical for very large systems (hundreds of processes).
- Causality detection is still not enough to order all events; concurrent events remain unordered.

## Hybrid Logical Clocks (HLC)

Hybrid Logical Clocks (Kulkarni et al., 2014) unify physical and logical time, reducing message size and maintaining correlation with real time.

### Structure
HLC is a tuple (pt, lt) where pt is the physical time (from the node's clock) and lt is a logical counter.

**On each local event:** The local process increments lt. If pt has advanced (wall-clock time moved forward), reset lt to 0.

**On receiving a message with (pt_msg, lt_msg):**
- If pt_msg < pt_local, the sender's clock is behind; use pt_local, set lt to lt_local + 1.
- If pt_msg = pt_local, set lt to max(lt_local, lt_msg) + 1.
- If pt_msg > pt_local, the sender is ahead; use pt_msg, set lt to lt_msg + 1.

**Result:** HLC timestamps are close to physical time (respecting clock synchronization within bounds) but also enforce causality like logical clocks. All concurrent events get different timestamps.

### Why Hybrid Clocks Matter
- Messages carry only a single (pt, lt) tuple instead of a vector.
- Timestamps correlate with real time; you can reason about "events 5 minutes ago."
- Space-efficient and practical for large systems.
- YugabyteDB, Cockroach Labs' systems, and others use HLC variants.

## Happens-Before Relation and Causal Consistency

The **happens-before** relation (→) captures causal dependencies:
- Within a process: if a precedes b in the process's execution, a → b.
- Across processes: if process A sends a message and B receives it, A's send → B's receive.
- Transitivity: if a → b and b → c, then a → c.

Two events are **concurrent** (||) if neither happened before the other: ¬(a → b) ∧ ¬(b → a).

### Causal Consistency
A system is **causally consistent** if:
- Reads return values written by processes in the causal order.
- If two operations are causally related, all processes observe them in that order.
- Causally unrelated operations can be observed in any order at different replicas.

**Example:** User A posts a comment (write), then User B likes the comment (read A's data, then write). User C should never see the like before the comment.

Causal consistency is weaker than linearizability (allows concurrent operations to be reordered) but stronger than eventual consistency. It's implemented using vector clocks or HLC.

## Chandy-Lamport Snapshots

Determining a consistent global state of an ongoing distributed system is hard. Snapshots must capture both process state and in-flight messages without stopping the system.

### Algorithm
An initiating process sends a **marker** message to all outgoing channels. When a process receives a marker:
1. Save its current state (if not already saved).
2. Record all messages it received on the incoming channel before the marker.
3. Forward the marker on all other outgoing channels.

All processes continue executing. The result is a **consistent cut**—a state such that if an event is in the cut, all causally prior events are also in it.

**Key property:** The recorded state and messages form a consistent point in the causal order. This is not a true "snapshot at time T" (impossible in an asynchronous system) but a valid snapshot that could have occurred during the system's execution.

### Applications
- Distributed deadlock detection.
- Rollback recovery (save snapshots periodically; on crash, restart from the latest snapshot).
- Distributed debugging and state inspection.

### Limitations
Determining which events belong in one snapshot vs. another is non-deterministic. Different marker initiations produce different snapshots of the same execution.

## Clock Synchronization Challenges

Even with synchronization, clocks diverge. The **clock skew** or **clock drift** is the rate at which one clock falls behind or runs ahead of another.

**Sources of drift:**
- Quartz oscillators age and change frequency.
- Temperature changes affect oscillator frequency.
- Relativistic effects (less relevant in Earth-based systems, critical for satellites).

**Challenges for consensus and ordering:**
- Assigning timestamps to events requires knowing the current time accurately.
- Forward jumps in time (e.g., NTP correction) violate monotonicity; a subsequent event can have an earlier timestamp.
- Backward jumps (rare but possible) break causality assumptions.

**Mitigations:**
- Use logical clocks (independent of physical time).
- Continuously synchronize via NTP and bound assumptions on drift.
- In critical systems, use atomic clocks and GPS (Spanner's approach).
- For most applications, accept approximate synchronization and design algorithms that tolerate small time uncertainty.

## Total Ordering with Logical Clocks

Lamport and vector clocks provide causality but don't create a total order (concurrent events remain unordered). To create a **total order** (needed for consensus or fault-tolerant state machines):

1. Assign each process a unique ID.
2. For totally unordered events, use (Lamport clock, process ID) as a tiebreaker.

Example: Event A at process 1 with Lamport clock 5 gets order (5, 1). Event B at process 2 with Lamport clock 5 gets order (5, 2). Since 1 < 2, A comes before B.

This approaches is simple but arbitrary; the exact process ID used doesn't matter as long as it's unique and consistent.

## Practical Takeaways

- **Use NTP for coarse synchronization:** Many systems run NTP and accept ~millisecond drift.
- **Use logical or hybrid clocks for ordering:** Lamport clocks are simple but provide only partial order. Vector clocks are precise but costly. HLC is a good middle ground.
- **Don't rely on physical time for critical ordering:** Use logical clocks for consensus, primary-backup failover, and causal consistency.
- **Treat clock skew as inevitable:** Design distributed algorithms to handle clock jumps gracefully or avoid depending on monotonic time entirely.

## See Also

- [distributed-consensus](distributed-consensus.md) — How clocks support consensus protocols
- [distributed-replication](distributed-replication.md) — Ordering writes in replicated systems
- [distributed-transactions](distributed-transactions.md) — Coordinating operations across distributed agents using timestamps