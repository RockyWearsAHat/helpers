# Linux Performance Tools — perf, strace, ltrace, eBPF, sar, vmstat, iostat, USE Method & Brendan Gregg

## Overview

Linux provides a rich suite of performance analysis tools ranging from low-level profilers (perf, eBPF) to system-wide utilization monitors (sar, vmstat). Effective performance troubleshooting requires knowing which tool answers which question: Is the CPU bottleneck? Is I/O or memory? Are system calls the problem? The **USE (Utilization, Saturation, Errors) method** provides a systematic framework; **Brendan Gregg's tools map** visualizes the landscape. This note covers essential tools and methodologies for Linux performance analysis.

## Hardware Performance Counters and perf

### perf (Performance Events)

**perf** is the primary Linux profiling tool. It uses Hardware Performance Counters (HPCs)—event counters on modern CPUs that count specific hardware events (cache misses, branch mispredictions, cycles, instructions) with minimal overhead.

**Hardware events (sample architecture, x86-64):**
- `cycles`: CPU clock cycles (always available; baseline for all profiling)
- `instructions`: Retired instructions
- `cache-references` / `cache-misses`: L3 cache events
- `branch-misses`: Predicted branch directions that were wrong
- `page-faults`: TLB misses, page faults
- `context-switches`: Kernel switches between runnable tasks

**List available events:**
```bash
perf list
```

### CPU Profiling with perf

**Sampling approach:** perf records the call stack at regular intervals (e.g., every 1000 cycles or every 10ms). Low overhead (~1-5%) compared to instrumentation.

```bash
perf record -g -F 99 ./myapp
perf report  # Interactive report showing call tree
```

**Flags:**
- `-g`: Capture call stack (enable later visualizations like flame graphs)
- `-F 99`: Sample at 99 Hz (adaptive to avoid timing bias)
- `-e cycles:ppp`: Count cycles with multiplexing to capture more precise data
- `--call-graph dwarf`: Unwind stack using DWARF debug info (more accurate but slower)

### Flame Graphs

A **flame graph** is a stacked area chart visualizing profiling data. Each row represents a function; width represents total sample count at that function. Stacking shows call chains; wide flat regions are hot spots.

**Generate from perf:**
```bash
perf record -g ./myapp
perf script | stackcollapse-perf.pl | flamegraph.pl > profile.svg
# View profile.svg in browser
```

**Reading flame graphs:**
- **Wide boxes**: Function consumes lots of CPU (hot spot)
- **Tall towers**: Deep call chains (framework overhead)
- **Scattered boxes**: Function called from many call sites
- **Baseline**: Root functions at the bottom; traces go upward

Finding wide boxes at the top of towers identifies where optimization effort should go.

### Other perf Subcommands

- **perf stat**: Count hardware events for a process, report summary (no recording to disk). Quick overview: `perf stat ./myapp` shows total cycles, instructions, cache misses.
- **perf trace**: Similar to strace; shows system calls. Better integration with perf's event model.
- **perf sched**: Analyze CPU scheduler behavior (runnable time, migration, context-switch delays).
- **perf mem**: Analyze memory access patterns (works only on supported CPUs with Memory Instrumentation architecture).

## System Call Tracing: strace and ltrace

### strace

**strace** hooks into the ptrace API to intercept and log all system calls (and signals) made by a process:

```bash
strace ./myapp
strace -e trace=open,read,write ./myapp  # Filter specific syscalls
strace -c ./myapp  # Summary: count, time per syscall
strace -p 1234     # Attach to running process
```

**Output:**
```
open("/etc/host", O_RDONLY)              = -1 ENOENT (No such file or directory)
open("/etc/hosts", O_RDONLY)             = 3
read(3, "127.0.0.1 localhost\n", 4096)   = 20
close(3)                                 = 0
```

Each line shows: syscall name, arguments (registers or memory), return value, errno if failed.

**Use cases:**
- Diagnose file permission errors (which file was accessed?)
- Understand I/O patterns (which files are read, how many syscalls?)
- Find unexpected system calls (why is the app trying to load a library?)
- Identify system call bombs (one operation causing thousands of syscalls)

**Limitations:**
- Overhead: context switching to the tracing process for every syscall. Can slow app 10-100×.
- Does not capture return data (see ltrace for that).
- Not traceable on systems where ptrace is disabled (often in containers for security).

### ltrace

**ltrace** is like strace but intercepts library function calls (e.g., calls to libc, glibc functions) instead of raw syscalls:

```bash
ltrace ./myapp
ltrace -e 'malloc,free' ./myapp  # Filter specific functions
```

**Output:**
```
malloc(100)                                  = 0x7ffff0000000
strlen("hello")                              = 5
free(0x7ffff0000000)                         = 0
```

**Use cases:**
- Understand library usage (which malloc calls dominate?)
- Debug allocation issues (repeated free of same pointer → bug)
- Profile without recompiling (no instrumentation needed)

**Limitations:**
- Same ptrace overhead as strace
- Requires library symbols (ltrace hooks PLT entries)
- Not available on all architectures

## eBPF Tools for Tracing and Profiling

### BCC (BPF Compiler Collection) and bpftrace

**eBPF** programs run in the kernel, avoiding the ptrace context-switch overhead. **BCC** wraps eBPF in Python; **bpftrace** provides a Domain-Specific Language (DSL) for concise tracing.

**bpftrace example: Track malloc calls by size:**
```bash
bpftrace -e 'uprobe:/lib/libc.so.6:malloc { @sizes[arg0] = count(); }'
```

**BCC example: Measure syscall latency:**
```python
# trace_syscalls.py
from bcc import BPF
bpf_code = """
int trace_openat(struct pt_regs *ctx, int dirfd, const char *pathname, int flags) {
    u64 ts = bpf_ktime_get_ns();
    syscall_start.update(&pid, &ts);
    return 0;
}
"""
# Attach to openat syscall, measure time on return
```

**Advantages over strace:**
- No overhead for context switching; kernel eBPF handler is lightweight
- Can aggregate data in kernel (count by syscall type) and report summary
- Dynamic instrumentation: attach/detach without recompiling

**Common eBPF tools (from bcc/bpftrace projects):**
- **funclatency**: Measure latency of kernel/user functions
- **offcputime**: Sample when process is off-CPU (blocked on I/O, lock, sleep)
- **biolatency**: Measure block I/O latency
- **vfsstat**: Count filesystem operations
- **tcplife**: Lifecycle of TCP connections

## System Utilization and Throughput Tools

### sar (System Activity Report)

**sar** collects and reports cumulative system statistics: CPU usage, memory, disk I/O, network.

```bash
sar 1 10          # Report every 1 second, 10 times
sar -u            # CPU utilization (user, system, idle, iowait, steal)
sar -r            # Memory (physical, swap, buffers, cache)
sar -d            # Disk I/O per disk
sar -n DEV        # Network throughput per interface
```

**Output example:**
```
12:00:01 PM   CPU    %user  %nice %system %iowait %idle
12:00:02 PM    all    25.0   0.0    10.0    15.0  50.0
12:00:03 PM    all    22.0   0.0    12.0    18.0  48.0
```

Sustained `%iowait > 20%` suggests disk bottleneck. Sustained `%system > 30%` suggests frequent syscalls or context switches.

### vmstat (Virtual Memory Statistics)

**vmstat** reports memory, process, and I/O statistics, updated every N seconds:

```bash
vmstat 1 10
```

**Output columns:**
```
 procs -----------memory---------- ---swap-- -----io---- -system-- ------cpu-----
 r  b   swpd   free   buff  cache   si   so    bi    bo   in   cs us sy id wa st
 2  0      0 3000000 100000 500000   0    0     5    10  100  20  60 15 20  5  0
```

- **r**: Runnable processes (queue length for CPU)
- **b**: Blocked processes (waiting for I/O)
- **swpd**: Virtual memory used
- **free**: Free physical memory
- **si / so**: Swap in / out (paging from disk; bad sign if sustained)
- **bi / bo**: Block I/O in / out (read/write blocks per second)
- **in / cs**: Interrupts and context switches per second
- **us / sy / id / wa / st**: User, system, idle, I/O wait, steal (for virtualized guests)

High **r** and low **id** → CPU bottleneck. High **b** and **wa** → I/O bottleneck.

### iostat (I/O Statistics)

**iostat** focuses on disk I/O per device:

```bash
iostat -x 1 10   # Extended output with latency metrics
```

**Output columns:**
```
Device     r/s     w/s   rMB/s   wMB/s svctm  %util
sda       100.0    50.0   50.0   20.0   5.0    75.0
```

- **r/s, w/s**: Read/write operations per second
- **rMB/s, wMB/s**: Megabytes read/written per second
- **svctm**: Service time (how long the disk takes to process a request)
- **%util**: Percentage of time the disk was busy (saturation)

High **%util** (>80%) on multiple devices → disk bottleneck. High **svctm** → disk slow or overloaded.

### mpstat and pidstat

**mpstat** reports per-CPU statistics (useful for identifying uneven load or CPU affinity issues):

```bash
mpstat -P ALL 1 10
```

Shows %user, %system, %idle per CPU core.

**pidstat** reports per-process statistics (CPU, memory, I/O, context switches):

```bash
pidstat -u 1 10  # CPU usage per process
pidstat -r 1 10  # Memory per process
pidstat -d 1 10  # I/O per process (requires CONFIG_TASK_IO_ACCOUNTING)
```

## USE Method: Utilization, Saturation, Errors

The **USE method** (Brendan Gregg) systematizes performance analysis. For each resource (CPU, memory, disk, network), measure:

1. **Utilization**: Is the resource being used? (0-100%)
2. **Saturation**: Is the resource oversubscribed? (queue depth, wait time)
3. **Errors**: Are there error conditions? (dropped packets, page faults, retransmissions)

**Resource checklist:**

| Resource | Utilization | Saturation | Errors |
|----------|--------------|------------|--------|
| **CPU** | `top` %cpu | `vmstat r`, `perf sched` | `perf stat` lost-cycles (NMI) |
| **Memory** | `free` used % | `vmstat si, so` (paging) | `dmesg` OOM killer |
| **Disk** | `iostat %util` | `iostat await, svctm` | `dmesg` I/O errors |
| **Network** | `sar -n DEV` %rxutil | `netstat | grep Drop` | Interface errors `ifconfig` |

**Analysis flow:**
1. For each resource, collect the three metrics
2. High utilization + is performance bad? If no → workload is fine
3. High saturation → queueing, throttling, slow responses (fix via tuning, scaling, or redesign)
4. Errors → packets dropped, connections refused → infrastructure issue

## Brendan Gregg's Cloud Performance Checklist and Tools Map

### Cloud Performance Checklist (60 seconds)

If you have 60 seconds to diagnose performance using standard tools:

```bash
uptime                    # Load average, system up time
dmesg | tail -20          # Kernel errors
vmstat 1 5                # Memory, I/O, context switches
mpstat -P ALL 1 5         # Per-CPU utilization
pidstat -u 1 5            # Top CPU processes
pidstat -r 1 5            # Top memory processes
free -h                   # Memory utilization
iostat -xz 1 5            # Disk I/O (latency, saturation)
netstat -an | head -20    # Network connections and listen queue
ss -tan                   # Listen sockets, backlog depth
ps aux                    # Process tree, memory usage
```

Quick scan: high load + high iowait + few processes = disk or network I/O bottleneck. High load + multiple processes + low iowait = CPU bottleneck.

### Tools Map (Brendan Gregg)

Gregg's **Linux Perf Analysis Tools Map** visualizes relationship between tools:

- **Top**: CPU usage
- **Middle layers**: Memory, I/O (vmstat, iostat, pidstat)
- **Bottom layers**: Latency profiling (perf, strace, bpftrace)
- **Special**: Network (ss, netstat), processes (ps)

For production systems, start at the top (uptime, top, ps) to identify the resource bottleneck, then drill down with specialized tools (perf for CPU, iostat for I/O, etc.).

## Workflow: Top-Down Analysis

1. **Understand the problem.** What's slow? Response time? Throughput?
2. **Determine the metric** (latency, throughput, CPU, memory, I/O).
3. **Measure with monitoring tools** (top, sar, vmstat, iostat). Identify bottleneck type.
4. **Drill down with profilers**. If CPU bottleneck → perf record + flame graph. If I/O → iostat -x + bpftrace biolatency.
5. **Correlate with business logic.** Profile the hot path; optimize algorithms or caching.
6. **Validate fix.** Re-measure the same metrics; confirm improvement.

## See Also

- **performance-profiling.md** — General profiling concepts and tools
- **performance-optimization.md** — Optimization strategies and Knuth's law
- **systems-debugging-tools.md** — Debuggers (GDB, LLDB) and instrumentation
- **linux-networking.md** — Network performance tools (tc, tcpdump, eBPF XDP)