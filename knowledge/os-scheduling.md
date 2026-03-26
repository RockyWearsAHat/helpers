# OS Scheduling — CFS, EEVDF, Real-Time, Priority, and Resource Limits

## Overview

The OS scheduler is the arbiter of CPU time: it decides which runnable process executes next, for how long, and under what constraints. Choices here are fundamental: a fair scheduler ensures responsiveness for interactive tasks; a real-time scheduler guarantees worst-case latency; priority inversion bugs cause unpredictable stalls; and resource limits (cgroups, NUMA awareness, CPU pinning) govern performance under contention. Modern kernels juggle competing criteria: fairness, latency responsiveness, cache locality, and energy efficiency.

## Completely Fair Scheduler (CFS)

Linux's default scheduler (since 2.6.23) models the CPU as a "perfect multitasking system": ideally, N runnable processes each get 1/N of the CPU. CFS tries to approximate this using a **virtual runtime** concept.

### Virtual Runtime (vruntime)

Each process accumulates a `vruntime`, which represents how much wall-clock time the scheduler "owes" it:

```
vruntime = wall_clock_time_spent_running / weight

weight = NICE_0_LOAD / (1 + 2^(-nice/10))
```

A process with `nice = 0` has `weight = NICE_0_LOAD` (1024 on modern kernels). A `nice = 10` process gets roughly half the weight, so it accumulates vruntime twice as fast, making the scheduler run it less often.

**Scheduling decision**: Always run the process with the smallest vruntime. This ensures no runnable process falls too far behind.

### Design Trade-offs

**Strengths:**
- Fair across priorities (weighted by nice value)
- Scales logarithmically; finding next process is O(log N) via red-black tree
- Responsive: even low-priority tasks get frequent, small time slices so they never starve
- Handles CPU affinity and NUMA locality heuristics

**Weaknesses:**
- Not real-time: no latency guarantees; a system under load can delay interactive processes by tens of milliseconds
- Fairness is approximate; under contention, actual CPU share drifts from weight-based share
- wake-up preemption: when a sleeping process wakes, scheduler must decide whether to preempt current runner; incorrect decisions hurt latency

### Time Slice and Preemption

CFS doesn't use fixed time slices. Instead, it grants each process a slice proportional to its weight:

```
time_slice = sched_latency / N_runnable

sched_latency = min(max(N_runnable * min_granularity, target_latency), max_latency)
```

- `min_granularity ≈ 0.75 ms`: the smallest slice any process gets
- `target_latency ≈ 6–24 ms`: desired period between process switches
- When `time_slice` expires or a higher-priority process wakes, CFS preempts

### The march toward EEVDF

CFS's fairness is good but not provably optimal. **Earliest Eligible Virtual Deadline First (EEVDF)** is a newer algorithm (Linux 6.6+, optional via `CONFIG_SCHED_CLASS`) that improves on CFS:

- Assigns each process an eligibility time (when it can run) and deadline (when it should run to maintain fairness)
- Always picks process with earliest deadline among eligible tasks
- Provides tighter bounds on latency and fairness deviation
- Still O(log N) but with better worst-case guarantees

Trade-off: EEVDF is more theoretically sound but introduces slight CPU overhead. CFS remains the default for compatibility.

## Real-Time Scheduling

Not all tasks tolerate unpredictability. Real-time scheduling classes (RT-FIFO, RT-RR) guarantee that a higher-priority process always preempts lower-priority ones, regardless of wall-clock fairness.

### FIFO (RT-FIFO)

```
static struct task {
  priority: 1–99 (reserved for RT)
  policy: SCHED_FIFO
}

Schedule: Among all runnable RT-FIFO tasks:
  - Select highest priority
  - If tied, run process that has been ready longest (FIFO order)
  - Never preempt unless blocked
```

**Use case**: Hard deadlines where missing them causes failure (flight control, medical devices).

**Risk**: One CPU-bound RT-FIFO task monopolizes the system; nothing else runs. The kernel thread must be explicitly managed.

### Round-Robin (RT-RR)

Like FIFO, but adds a fixed time slice:

```
- Highest-priority runnable task runs
- If it uses up its time slice, moved to back of queue at same priority
- Lower-priority never runs unless all higher-priority are blocked
```

**Use case**: Hard real-time but want preemption within priority levels.

### Priority Inversion

**The problem:** A high-priority task waits for a lock held by a low-priority task, which is preempted by a medium-priority task. Medium runs, blocking high, while low is not scheduled. This violates the priority assumption.

```
High (blocked on lock held by Low) ──╮
                                     ├─ Medium preempts Low (higher priority!)
Medium (runnable)                    │
                                     ├─ Low (blocked, can't run)
Low (running, holds lock on High) ──╯
```

**Solutions:**
- **Priority inheritance**: When high waits on low's lock, low inherits high's priority. Once low releases, it drops back.
- **Priority ceiling**: Assign each lock a ceiling priority. Task acquiring lock temporarily raises to ceiling priority.
- **Avoid locks in high-priority code**: Eliminate the vulnerability.

## Nice Values and Weights

Unix/Linux `nice` values range –20 (highest priority) to +19 (lowest). Setting a process's nice changes its CFS weight.

```
# Shell: reduce nice (higher priority, needs privilege)
nice -n -5 myprocess

# Observe weight
weight[i] = NICE_0_LOAD / (1 << (-(nice - 0) / 10))
```

Between adjacent nice levels, ratio is roughly 1.25×. Going from nice 0 to nice 10 drops weight by ~2.6×, so process gets ~2.6× longer between preemptions.

**Trade-off**: Nice is coarse for many applications. Real-time scheduling is more precise but risks system lockup.

## Cgroups CPU Limits

Control groups (cgroups) partition the system into task groups with resource quotas. CPU cgroups limit total CPU time a group can consume.

### CPU Shares (v1: `cpu.shares`)

A soft guarantee: CPUs are distributed proportionally to shares.

```
cgroup_a.shares = 1024
cgroup_b.shares = 512

Under contention:
  cgroup_a gets 1024 / (1024 + 512) = 66.7% of CPUs
  cgroup_b gets 512 / (1024 + 512) = 33.3% of CPUs
```

If cgroup_b is idle, cgroup_a can burst beyond 66.7%.

### CPU Quota (v2: `cpu.max`)

A hard limit: cap CPU time per period.

```
echo "50000 100000" > cgroups/mycgroup/cpu.max
# 50ms of CPU time per 100ms wall-clock period = 50% of one core
```

Processes exceeding quota are throttled (moved to sleep) until next period. Hard limits prevent noisy neighbors but risk starvation if quota is too small.

## NUMA-Aware Scheduling

On **Non-Uniform Memory Architecture** systems, memory access speed depends on which NUMA node. The scheduler tries to keep processes on a single node to maximize cache hits and minimize remote memory access.

### Scheduling and Load Balancing

- **Preferred node**: Scheduler tracks which NUMA node a process last ran on; tries to stick with it
- **Load balancing**: If one node is idle, scheduler can pull tasks from overloaded nodes, but pays NUMA penalty
- **Memory affinity**: If process's allocated memory is on node A but running on node B, memory access is slow

### Configuration

```bash
# Bind process to specific nodes
numactl --cpunodebind=0 --membind=0 ./myapp

# View NUMA topology
numactl --hardware
```

Trade-off: NUMA affinity improves throughput; excessive cross-node traffic can halve performance. But perfect NUMA isolation starves light-loaded nodes.

## CPU Pinning and Affinity

Binding a process to specific CPUs improves cache locality and reduces migration overhead, but sacrifices load balancing.

### Affinity Mask

```
# Application level (POSIX)
cpu_set_t mask;
CPU_ZERO(&mask);
CPU_SET(0, &mask);
CPU_SET(1, &mask);
sched_setaffinity(pid, sizeof(mask), &mask);

# Kernel command-line (boot)
isolcpus=2,3   # isolate CPUs 2 and 3 from general scheduling
taskset -p -c 0,1 12345  # bind process 12345 to CPUs 0,1
```

**Use cases:**
- High-frequency trading: dedicate cores to minimize jitter
- Real-time: isolate CPUs for hard-deadline tasks
- Tuning: pin worker threads to NUMA nodes

**Risks:** Uneven load; idle CPU while others are overloaded.

## Scheduling in Containers and VMs

### Container Scheduling (Using cgroups)

The host kernel scheduler is unaware of container boundaries; it sees individual processes. Containers use cgroups limits to constrain resource consumption.

```
Docker container → cgroup with cpu.max, memory.max, etc.
Host scheduler schedules individual processes
Container's limiting is transparent: processes get throttled if group exceeds quota
```

**Trade-off:** Simple, but the kernel doesn't know priority relationships across containers. Two containers at different priorities still compete fairly within cgroups.

### VM Scheduling (Hypervisor)

Hypervisors (KVM, Xen) schedule virtual CPUs (vCPUs) on physical cores. A vCPU is just a kernel thread from the host's perspective, so the host CFS scheduler handles it. But overcommitting (more vCPUs than pCPUs) causes thrashing.

```
Host: 4 physical cores
VM1: 4 vCPUs
VM2: 4 vCPUs
│
Scheduler must map 8 vCPUs into 4 cores; each core gets 2 vCPUs
Result: threads inside VMs are context-switched frequently, cache-miss spike
```

**Mitigation:** Avoid overcommit; use NUMA pinning to align vCPU threads with memory.

## Debugging and Observability

### Tools

- **`ps -eo pid,cmd,%cpu,nice`**: View nice values and CPU usage
- **`chrt -p PID`**: Check real-time policy and priority
- **`taskset -p -c PID`**: View/set CPU affinity
- **`trace-cmd` / `perf`**: kernel scheduling events (context switches, preemptions)

### Scheduler Class Hierarchy

```
Priority (SCHED_DEADLINE > SCHED_FIFO/RR > SCHED_NORMAL > SCHED_BATCH > SCHED_IDLE)

SCHED_DEADLINE: Hard real-time, deadline-driven (rare)
SCHED_FIFO/RR:  Real-time, fixed priority
SCHED_NORMAL:   Interactive, CFS (default user processes)
SCHED_BATCH:    Long-running compute jobs, low interactivity
SCHED_IDLE:     Background, runs only when nothing else needs CPU
```

## See Also

- **os-concurrency-primitives** — Locks, mutexes, avoiding priority inversion
- **os-process-management** — Life cycle, PCB, process creation
- **iot-rtos** — Real-time kernels and determinism
- **system-design-distributed** — Distributed scheduling (cluster scheduling)