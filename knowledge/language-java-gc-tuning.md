# Java Garbage Collection Tuning — G1GC, ZGC, Shenandoah, and Pause Control

## Overview

Java garbage collection (GC) balance three goals: throughput (minimize GC time), latency (minimize pause duration), and memory footprint. Different GC algorithms prioritize these differently. G1GC is the default (Java 9+); ZGC and Shenandoah target ultra-low pauses; Epsilon trades latency for throughput.

Understanding GC tuning is essential for server applications sensitive to pause times (financial systems, real-time analytics) or resource-constrained environments.

## GC Algorithms Comparison

| Algorithm | Pause Time | Throughput | Memory Overhead | Use Case |
|-----------|-----------|-----------|-----------------|----------|
| G1GC | ~100-500 ms | High | Low | Default, balanced |
| ZGC | <10 ms | Medium | Higher (colored pointers) |Ultra-low latency, 4 GB+ |
| Shenandoah | 10-100 ms | Medium | Higher (load barriers) | Low latency |
| Epsilon | None (never GCs) | Highest | Large heap required | Batch jobs, testing |
| Serial | Pauses full heap | Good | Low | Small heaps, single-threaded |

## G1GC (Garbage-First)

G1 (default in Java 9+) divides the heap into regions and collects regions incrementally, reducing pause time:

### Region-Based Heap

The heap is divided into 2000-4000 fixed-size regions (~2 MB each):

```
Heap: | R1 | R2 | R3 | R4 | R5 | R6 | R7 | R8 |
      Young Gen        |    Old Gen         |
```

Regions are classified:
- **Young**: Contain only young objects; collected frequently
- **Old**: Contain old objects; collected less frequently
- **Humongous**: Single objects >50% region size (special handling)

### Collection Phases

1. **Young collection**: Evacuates young regions. Pause time is predictable (~target 200 ms).
2. **Mixed collection**: Collects young + old regions with high garbage density. Pause time remains bounded.
3. **Full collection**: Fallback (blocking, slow). Indicates tuning problem.

### Tuning Parameters

```bash
# Heap sizing
-Xms4g -Xmx4g                          # Min/max heap
-XX:+UseG1GC                            # Enable G1 (default in 9+)

# Pause time target (default 200 ms)
-XX:MaxGCPauseMillis=150                # G1 attempts to stay under 150 ms

# Region size (default auto)
-XX:G1HeapRegionSize=16m                # Explicit region size

# Young generation target
-XX:G1NewGenPercentSize=5               # Percentage of heap for young gen
```

### Young Gen Sizing

G1 dynamically adjusts young gen based on pause time goal. Larger young gen = fewer young collections but longer pauses.

```bash
-XX:G1ReservePercent=10  # Reserve for survivor/humongous allocation
```

### Humongous Objects

Objects >50% region size are allocated as humongous, directly in old gen, and collected at mixed collection time. Allocating many humongous objects is inefficient:

```java
// Inefficient: 100 MB objects in 32 MB regions (3+ regions each)
byte[][] arrays = new byte[1000][100 * 1024 * 1024];

// Better: use smaller regions or design data differently
```

### Full Collection Trigger

Full collection (blocking) occurs if:
- Allocation failure during young collection (heap exhausted)
- Explicit `System.gc()` call
- Concurrent marking cycle cannot keep up with allocation

Avoid:
- Oversized humongous objects
- Rapid allocation patterns under low heap utilization
- Calling `System.gc()` in production

## ZGC (Z Garbage Collector)

ZGC maintains sub-millisecond pause times through concurrent marking and moving. Introduced in Java 11 (production-ready in 15+).

### Colored Pointers

ZGC uses **colored pointers** to track object state in the pointer bits themselves (64-bit only):

```
Object pointer: | Unused | Marked0 | Marked1 | Remapped | Finalizable | ... | Address |
                   bits      bit      bit       bit        bit           obj pointers
```

Bits indicate:
- **Marked**: Object marked during concurrent phase
- **Remapped**: Object moved and pointer updated

The JVM checks these bits without dereferencing memory, enabling efficient concurrent GC.

### Load Barriers

Whenever a reference is loaded, a **load barrier** checks the pointer bits:

```
// Pseudo-code of load barrier
Object* ref = obj->field;  // Load barrier executed here
if (pointer_bit(ref) != current_phase) {
    ref = fixup_pointer(ref);  // Update reference if needed
}
```

Load barriers are small and JIT-compiled to fast inline code (~4 instructions).

### Phases

1. **Roots marking** (~1 ms pause): Scan thread stacks and roots
2. **Concurrently mark**: Scan object graph without stopping mutators
3. **Remap** (~1 ms pause): Fix forwarding pointers
4. **Concurrent move**: Relocate objects

Pause times are independent of heap size.

### Tuning Parameters

```bash
-XX:+UseZGC
-Xms4g -Xmx4g                          # Min/max heap (must be equal)
-XX:ZUncommitDelay=300                 # Delay before returning unused heap (default 300s)
-XX:ConcGCThreads=<n>                  # Concurrent GC threads (auto-tuned)
```

### Limitations

- 64-bit only (pointer colors require 64-bit addresses)
- Larger heap overhead (colored pointers, load barrier checks)
- Less effective on small heaps (<4 GB)
- May have longer GC pause than G1 on light workloads (but maximum pauses are sub-1ms)

## Shenandoah

Shenandoah is a low-pause collector similar to ZGC but with different trade-offs. Developed initially outside OpenJDK, now part of certain distributions (OpenJDK builds, Red Hat).

### Load Barriers

Like ZGC, Shenandoah uses load barriers, but they are heavier (9-15 instructions). Enables broader platform support (32-bit capable earlier, though now 64-bit focused).

### Pause Times

Shenandoah pause times are typically 10-100 ms, not as aggressive as ZGC (<1 ms) but still acceptable for most interactive systems.

### Tuning

```bash
-XX:+UseShenandoahGC
-Xms4g -Xmx4g
-XX:ShenandoahTargetNumRegions=2048     # Number of heap regions
-XX:ShenandoahGuaranteedGCInterval=10000 # Periodic concurrent cycle (ms)
```

## Epsilon GC

Epsilon never collects; it throws `OutOfMemoryError` when heap is full. Used for testing, short-lived applications, or workloads that produce negligible garbage.

```bash
-XX:+UseEpsilonGC
-Xms4g -Xmx4g

# Allocation fails immediately when heap full; no GC
```

Use cases:
- Batch jobs that allocate briefly then exit
- Performance testing (isolate allocation from GC cost)
- Embedded systems with predictable allocation

## GC Logging and Diagnostics

### JVM Unified Logging (-Xlog:gc)

```bash
# Basic GC logging
-Xlog:gc

# Detailed GC logging
-Xlog:gc*

# Output to file with rotation
-Xlog:gc:file=gc.log:filecount=5:filesize=10m

# Trace specific phases (G1)
-Xlog:gc+heap+region=trace

# Include timestamps and process ID
-Xlog:gc:level=info:filecount=5:filesize=100m:time,level,tags,pid
```

### Analyzing Logs

```
[2025.123s][info][gc] GC(5) Pause Young (G1 Evacuation Pause) 150M->80M(2000M) 125.456 ms
[2025.234s][info][gc] GC(6) Pause Young (G1 Evacuation Pause) 150M->80M(2000M) 120.123 ms
[2025.500s][info][gc] GC(7) Pause Full (System.gc()) 1500M->800M(2000M) 2500.456 ms
```

Fields:
- **GC(n)**: GC cycle number
- **Pause type**: Young, Mixed, Full
- **Before->After(Max)**: Heap usage
- **Duration**: Pause time

ZGC logs are different:
```
[2025.123s][info][gc] GC(0) Pause Mark End 0.123 ms
[2025.124s][info][gc] GC(0) Pause Relocate Start 0.456 ms
```

### JFR (Java Flight Recorder)

```bash
# Record with GC events
java -XX:StartFlightRecording=filename=recording.jfr,duration=10s,gc=enabled App

# Analyze
jfr dump recording.jfr

# Extract to CSV for graphing
jfr print --csv recording.jfr > gc.csv
```

JFR events: `GCPauseL4`, `GCAllocationFailure`, `GCConcurrentPhase`.

## Heap Sizing

### Dynamic vs Fixed

```bash
# Fixed (predictable, no resizing overhead)
-Xms4g -Xmx4g

# Dynamic (uses extra CPU during resizing)
-Xms1g -Xmx8g  # Avoid unless you need flexibility
```

For production, set `-Xms` and `-Xmx` equal to prevent GC pauses during heap expansion.

### Rule of Thumb

- **Max pause time**: Aim for 50-200 ms (depends on SLA)
- **Heap size**: 2-4x live data size (to reduce collection frequency)
- **Young gen**: ~25% of heap (for G1)

```bash
# Measure live data:
# Run under steady load, take heap dump during full GC
# Live data ~= heap size after full GC

# Example: 100 MB live data
-Xms400m -Xmx400m  # 4x live data
```

## GC Ergonomics

The JVM auto-tunes GC based on behavior if you don't set tuning flags. Ergonomics:
- Adjusts live data estimates
- Resizes young gen to hit pause time goals
- Selects GC based on platform

To disable ergonomics (explicit tuning):

```bash
-XX:-UseAdaptiveSizePolicy  # Disable automatic young gen sizing
```

## Common Tuning Patterns

### High-throughput batch processing

```bash
-Xms16g -Xmx16g
-XX:+UseG1GC
-XX:MaxGCPauseMillis=500        # Accept longer pauses
-XX:ParallelGCThreads=16
-XX:ConcGCThreads=4
```

### Ultra-low latency (trading resources)

```bash
-Xms8g -Xmx8g
-XX:+UseZGC
-XX:ConcGCThreads=8
# Expect higher CPU/memory overhead
```

### Memory-constrained

```bash
-Xms512m -Xmx512m
-XX:+UseG1GC
-XX:G1HeapRegionSize=8m         # Smaller regions for less heap
-XX:MaxGCPauseMillis=100
```

## Monitoring in Production

Key metrics:
- **GC pause time**: p50, p95, p99 (not just average)
- **GC frequency**: Young collections per second
- **Heap utilization**: Current vs capacity
- **Full GC rate**: Should be ~0 or very rare

Tools:
- **Prometheus + JMX exporter**: Long-term trending
- **JFR profiler**: Per-session diagnostics
- **Arthas**: Live heap, GC, thread analysis without restart

## Anti-Patterns

1. **Calling `System.gc()` in production** — Forces full GC, disruptive
2. **Over-sized humongous objects** — Inefficient G1 collection
3. **Misconfigured pause time goal** — Unachievable targets cause full GC fallback
4. **Heap too small relative to allocation rate** — Constant collection
5. **Mixed GC disabled** — G1 falls back to full collection

## See Also

- G1GC tuning guide (Oracle docs)
- ZGC design (JEP 333, 377, 414)
- Shenandoah design (JEP 189)
- Concurrent algorithms (Dijkstra, tri-color marking)
- Memory management and allocation