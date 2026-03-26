# Performance Profiling — CPU, Memory, Continuous, and Browser Tools

Performance profiling reveals WHERE time and resources are actually being spent, replacing guesswork with data. Different profiling techniques answer different questions and suit different scenarios.

## CPU Profiling: The Fundamentals

CPU profiling measures where the processor spends its time, answering "why is this function slow?" There are two primary sampling strategies.

### Sampling vs. Instrumentation

**Sampling profilers** pause execution at regular intervals (e.g., every 10ms) and record the current instruction/function. Low overhead because they don't intercept every call. Trade-off: may miss short-lived functions if the sample interval is too coarse.

**Instrumentation profilers** insert hooks before/after every function call, recording entry and exit. Captures everything but adds significant overhead (often 2-10× slower). Useful when accuracy matters more than speed.

Most modern profilers use **adaptive sampling**: sample at a rate that captures 99% of CPU time while keeping overhead below ~5%.

### Flame Graphs

A **flame graph** (pioneered by Brendan Gregg) visualizes profiling data as a stacked area chart. The x-axis is total sample count; the y-axis stacks functions from caller to callee (root at bottom). Flame graphs reveal:

- **Hot functions**: wide horizontal bars represent functions that consume lots of CPU
- **Call chains**: vertical stacks show which paths dominate
- **Tail calls**: long chains of narrow stacks indicate deep recursion or framework overhead
- **Unexpected callers**: discovering that a function is called from a place you didn't expect

Reading flame graphs requires spotting wide flat regions (the hot path) and investigating why they're hot: is it a tight loop, O(n²) algorithm, or lock contention?

### Tools: perf, pprof, py-spy

**perf** (Linux): Kernel-level profiler using CPU performance counters. Captures sampling data directly from hardware, with minimal overhead. Command: `perf record -g ./app` then `perf report` to inspect or `FlameGraph` scripts to visualize.

**pprof** (Go, C++, Python via Py-Spy): Google's profiling tool supports CPU profiling with interactive web UI, flame graph export, and differential profiling. Built into Go runtime; can profile any C/C++ binary if linked with libprofiler.

**py-spy** (Python): GIL-aware sampler that profiles Python without modifying code. Uses ptrace to peek at the Python stack. Safe for production; overhead ~1%.

## Memory Profiling: Allocation and Residual Use

Memory profiling tracks heap allocations and identifies memory leaks or excessive allocation patterns.

### Heap Snapshots

A **heap snapshot** captures all objects currently in memory at a point in time, with their size, reference count, and allocation stack. Comparing two snapshots shows what was allocated between them.

**Use cases:**
- Memory bloat: take snapshots over time, compare to see growth
- Leak detection: snapshot before and after a user interaction; retained objects that should be GC'd reveal leaks
- Object explosion: identify which type (string, array, object) is proliferating

Most browser DevTools and runtime profilers (V8 for Node.js, Java `jps`/`jcmd`) support heap snapshots.

### Allocation Tracking

Track every memory allocation, not just the current heap. Instead of snapshots, this produces a continuous log of allocations and deallocations, grouped by allocation stack and size.

**Pros:**
- Reveals allocation patterns (e.g., 1 million strings allocated in a loop)
- Detects excessive churn even if objects are later freed

**Cons:**
- Significant overhead (10-50×) — use sparingly in production
- Logs can become massive for long-running processes

### Tools and Patterns

**Java**: `jvmstat`, `YourKit` (commercial), `Async Profiler` (open source, works with JVM agents)

**JavaScript (Node.js)**: Built-in `--inspect` mode with DevTools, or third-party tools like Clinic.js

**Python**: `memory_profiler` decorator for line-by-line tracking, `objgraph` for object reference inspection

**C/C++**: `valgrind --tool=massif`, `gperftools tcmalloc`, compiler-instrumented sanitizers (`-fsanitize=address`, `-fsanitize=memory`)

## Continuous Profiling: Always-On Production Observability

Traditional profiling is on-demand and often intrusive. **Continuous profiling** samples production traffic at a low rate (e.g., 1% of requests) to build a steady picture of performance without stopping the system.

### Architecture

A continuous profiler runs an agent in each production service, continuously sampling CPU and memory at low overhead (~1-3%). Samples are sent to a backend, which aggregates them across instances and time. Users can query: "Show me the top 10 functions by CPU across production over the last 24 hours" or "Compare CPU profiles between v1.2 and v1.3 of my service."

### Pyroscope

Pyroscope is an open-source continuous profiling backend. It ingests profiling data from agents (distributed as libraries in Go, Python, Ruby, Node.js, Java) and stores them efficiently (compressed delta encoding). Web UI allows interactive exploration of flames graphs.

**Strengths:**
- Low overhead (designed for always-on use)
- Automatic diffing: compare time periods or versions
- Built-in for most languages

**Limitations:**
- Requires agent setup (not zero-configuration)
- Network roundtrips can add latency if the backend is distant

### Parca

Parca is a competitor with similar goals but different philosophy: it uses **eBPF for kernel-level profiling on Linux**, capturing data directly without language-specific agents. Single Parca instance can profile all processes on a machine.

**Strengths:**
- Polyglot (no agent installation per language)
- Kernel-level visibility (syscalls, kernel functions)
- Lower per-process overhead

**Tradeoffs:**
- Linux-only (eBPF is a Linux feature)
- Requires kernel 5.8+ and CAP_BPF capability
- Less language-specific optimization

## Browser Performance Profiling

### DevTools Performance Panel

Modern browsers (Chrome, Firefox, Safari) include DevTools with a Performance tab. Record a session, and inspect:

- **Frames**: visual FPS timeline, highlights dropped frames
- **Main thread**: JavaScript execution, layout, paint
- **Scripting**: per-function CPU time
- **Rendering**: paint costs, composite operations
- **Network waterfall**: critical path dependency tree
- **User timing marks**: custom markers inserted via `performance.mark()`

The Performance panel is the first-line tool for web-specific bottlenecks.

### Lighthouse

Lighthouse is a browser-based auditor that runs synthetic tests against a page. It scores performance, detects unoptimized images, accessibility issues, PWA requirements, and SEO. Results include opportunities and diagnostics with specific recommendations.

Lighthouse runs tests in a controlled environment (throttled network, simulated mobile), making results reproducible but less representative of real user traffic than RUM (Real User Monitoring).

## Low-Level Profiling: perf and eBPF

### perf (Linux)

The kernel's performance counter interface. Captures:

- **CPU cycles**: clock ticks (measures wall time / core time)
- **Instruction count**: useful for spotting inefficient instruction sequences
- **Cache misses**: L1, L2, L3 misses correlate with memory stalls
- **Page faults**: measure memory subsystem pressure
- **Context switches**: indicate contention or I/O stalls

Examples:
```
perf record -e cycles -g ./myapp       # Profile CPU cycles, capture call stacks
perf report                           # Interactive inspection
perf stat ./myapp                     # Summary stats (no recording)
```

### eBPF (Extended Berkeley Packet Filter)

Originally a packet filtering mechanism, eBPF evolved into a general-purpose in-kernel VM. You write small programs that run in the kernel without recompiling it, giving direct access to:

- Kernel function calls and returns (kprobes)
- Userspace function calls (uprobes)
- Tracepoints (predefined kernel events)
- System calls
- Network packets

**Advantages:**
- Near-zero overhead when not actively filtering
- Can inspect kernel structures and system calls without leaving the kernel

**Limitations:**
- Requires kernel 4.4+ (best on 5.11+)
- Learning curve (requires C or eBPF-specific languages like bpftrace)
- Limited program size and complexity (kernel safety constraints)

### Tools

**bpftrace**: high-level language for eBPF tracing. One-liners like `bpftrace -e 'tracepoint:syscalls:sys_enter_open { @[comm] = count(); }'` count opens by process.

**Flame Graphs with eBPF**: capture kernel-level stack traces, generate flame graphs showing kernel + userspace hot paths together.

## Profiling in Practice

1. **Establish baseline**: profile the unmodified system to understand current hot paths
2. **Reproduce issue**: profile under the specific condition (high load, specific user interaction)
3. **Inspect differences**: use flame graph diff tools to spot what changed
4. **Validate fix**: re-profile after changes; ensure hot path narrowed
5. **Monitor production**: set up continuous profiling to catch regressions

Profiling is most effective when combined with **hypothesis-driven testing**: form a hypothesis about the bottleneck, profile to confirm or refute, then apply targeted fixes.

## See Also

- systems-debugging-tools.md — GDB, LLDB, tracing foundations
- performance-optimization.md — when to profile, strategic thinking
- web-performance.md — browser-specific profiling and metrics