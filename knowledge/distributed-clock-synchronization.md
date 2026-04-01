# Distributed Clock Synchronization — NTP, PTP, TrueTime & Monotonic Clocks

## Overview

Distributed systems operate across multiple machines with independent hardware clocks that drift at different rates. Synchronizing these clocks is fundamental to ordering events, implementing timeouts, detecting failures, and providing strong database consistency. Clock synchronization algorithms differ in **accuracy** (how close to "true" time), **precision** (how closely clocks agree), **latency** (message round-trip overhead), and **cost** (hardware, infrastructure, operational).

No algorithm achieves perfect synchronization; all strategies manage the **uncertainty bounds** that remain after synchronization.

## NTP: Network Time Protocol

### Architecture and Stratum

NTP organizes time sources in a **stratum hierarchy**:
- **Stratum 0** — atomic clocks, GPS receivers, radio terminals (no network)
- **Stratum 1** — servers directly connected to stratum 0 sources
- **Stratum 2-15** — clients that sync from stratum 1/2/etc via network
- **Stratum 16** — unsynchronized (fallback/offline state)

Clients query multiple servers (typically 4-8) and use statistical algorithms to reject outliers and choose reliable sources. Higher stratum numbers reflect both distance and accumulated uncertainty; clients prefer lower strata.

### The Discipline Loop

NTP's **discipline algorithm** continuously adjusts the local clock through feedback:

1. **Poll** — Client queries server at exponentially-backed intervals (every 64–1024 seconds in steady state)
2. **Measure** — Record round-trip delay and offset; discard if jitter is too high
3. **Filter** — Keep 8 most-recent measurements; select sample with smallest dispersion
4. **Combine** — Merge results from multiple servers; weight by reliability and stratum
5. **Adjust** — Apply frequency correction (slew) to local oscillator rather than step jumps (except at startup or >> 1 second offset)

The **slew rate** is typically 500 ppm (parts per million), meaning adjustment happens gradually over seconds to minutes. This prevents application errors from time reversals.

### Trade-offs

NTP achieves **seconds to ~100 ms accuracy** in typical networks. It's **simple, widely deployed, and free**, but:
- Accuracy degrades over intercontinental distances and congested networks
- Surprise jumps when oscillators diverge significantly
- No protection against Byzantine (malicious) time servers without NTPsec extensions
- Assumes relatively stable network delays (breaks poorly on satellite links)

## PTP: Precision Time Protocol (IEEE 1588)

### Hardware Timestamping

Unlike NTP's software-measured round-trips, PTP leverages **hardware timestamping** — network interface cards and switches timestamp packets at the physical layer, eliminating OS jitter. Accuracy improves to **microseconds (1588-2008) or nanoseconds (1588-2019)**.

### Master-Slave Hierarchy

PTP uses a similar hierarchy but with tighter coupling:
- One **Master** sends **Sync** announcements
- **Slaves** measure round-trip delay via **Delay-Request/Response** exchange
- Slaves compute offset and forward it to their local hardware clock

### Boundary and Transparent Clocks

- **Boundary clocks** (stratum transition points) re-timestamp outgoing sync messages, reducing accumulated errors
- **Transparent clocks** (e.g., switches) add themselves to the delay but don't alter the clock source path

### Trade-offs

PTP achieves submicrosecond accuracy in **controlled LANs** (financial trading, manufacturing, power grids), but:
- Requires hardware support (not available on commodity cloud instances)
- Deployment complexity increases with WAN spans
- Standards revision (v1 vs v2) fragmentation caused adoption gaps
- Not viable for geographically distributed systems (lightspeed = 130 ms coast-to-coast)

## Google TrueTime: Hybrid Physical-Logical Clocks

### Design

TrueTime combines GPS receivers and atomic clocks to provide a **time interval** rather than a point estimate. Each clock operation returns `[earliest, latest]` bounds that tighten as measurements accumulate:

- **GPS instances** (geographically replicated) measure absolute time
- **Atomic clocks** fill gaps when GPS signals fade (underground, tunnels, jamming)
- **Uncertainty bound** (typically 1–10 ms depending on GPS health) expands while waiting for new measurements

### Application: External Consistency

Spanner's transactions use TrueTime to assign commit timestamps that respect **external consistency**: if transaction T1 commits before T2's start, T1's timestamp is strictly less than T2's. The uncertainty margin allows **commit-wait**: transactions block until `now_latest()` passes the assigned timestamp, ensuring the interval closes.

### Trade-offs

TrueTime achieves **millisecond-level bounds** globally, enabling strong consistency at geographic scale, but:
- **High operational cost** — GPS receivers, atomic clocks, monitoring for GPS blackout
- Works only for applications that can afford commit-wait latency
- Clock jumps still possible (rare GPS/atomic failures); systems must handle time reversals
- Privacy concerns with GPS-indexed location leakage

## Hybrid Logical Clocks (HLC)

### Motivation

Logical clocks (Lamport, vector clocks) order causality but lack connection to wall-clock time. HLC bridges this gap:

$$HLC = (l_t, c_t)$$

where $l_t$ is a wall-clock time and $c_t$ is a logical counter.

### Rules

- On local event: if wall-clock time advanced, reset counter; else increment counter
- On message receive: adopt max(sender_HLC, local_HLC), apply rules above
- Total order: compare $l_t$ first, then $c_t$ on tie

### Trade-offs

HLC provides **causal ordering without external synchronization**: systems with arbitrary clock skew can coordinate, but:
- Assumes eventual HLC convergence; unbounded skew causes unbounded counter drift
- Not a substitute for clock syncronization; adds overhead
- Useful for event sequencing in distributed logs, but weak for timeout/lease-based failure detection

## Clock Skew, Drift, and Monotonicity

### Skew vs. Drift

- **Skew**: instantaneous difference between two clocks at the same moment
- **Drift**: rate at which one clock diverges from another (ppm per second)

Silicon oscillators drift ~100 ppm without compensation; temperature and aging compound this. Synchronization must occur regularly; maximum interval depends on desired accuracy bound.

### Monotonic Clocks

A **monotonic clock** never jumps backward, even if NTP/PTP applies a negative adjustment. Monotonic clocks are essential for:
- Timeout calculations (`now + 30s`)
- Elapsed time measurements
- Database transaction queuing

Most systems provide both:
- **System clock** — may jump; used for external contracts
- **Monotonic clock** — always advances; used internally

### Leap Seconds

UTC inserts (rarely) leap seconds to keep mean solar time aligned with atomic time. Systems respond differently:
- **Step jumps** (most NTP implementations) — clock jumps forward 1 second, can break timers
- **Slew smearing** (Google, large internet services) — stretch the last hour of the day, avoiding user-visible jumps
- **Ignore and resync** — accept transient clock inaccuracy

## Practical Guidance

### For Typical Distributed Applications

NTP (via OS) provides sufficient accuracy for event ordering, failure detection, and user-visible timestamps. Use **monotonic clocks** for internal timing. Sync interval: every 64–1024 seconds.

### For Microsecond Accuracy (LAN)

PTP with hardware support and boundary clocks achieves 100 ns accuracy within a data center. Requires network switch support and careful cable routing.

### For Global Consistency (WAN)

Consider logical clocks, consensus-based ordering (Paxos/Raft), or weak consistency models that don't require tight clock sync. TrueTime is a specialized infrastructure choice.

## See Also

- **distributed-clocks-ordering** — Lamport & vector clocks, causality
- **database-distributed-sql** — Spanner's TrueTime application
- **distributed-consensus** — Consensus without perfect clocks
- **cloud-gcp-spanner** — TrueTime internals and design