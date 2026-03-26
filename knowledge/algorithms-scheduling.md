# Scheduling Algorithms — CPU, Deadline, Job Shop, and Distributed Task Scheduling

## Overview

Scheduling is the fundamental problem of **allocating resources (CPU, I/O, network time) to competing tasks** while optimizing for objectives: throughput, latency, fairness, energy efficiency, or responsiveness.

Different contexts have different scheduling approaches:
- **CPU scheduling:** Operating systems choose which process runs next.
- **Real-time scheduling:** Deadline-driven systems ensure tasks complete by fixed deadlines.
- **Job shop scheduling:** Manufacturing / batch processing schedules dependent tasks with resource contention.
- **Distributed scheduling:** Cluster managers (Kubernetes, Yarn) assign work to machines.

---

## CPU Scheduling (Operating Systems)

### FIFO (First In, First Out)

**Algorithm:** Tasks run to completion in the order they arrive.

**Pros:**
- Simplest possible implementation.
- No starvation (every task eventually runs).
- $O(1)$ per operation (push/pop from queue).

**Cons:**
- Long-running tasks block short tasks (convoy effect).
- Interactive response is poor; one slow task delays everything.
- Throughput is reasonable but latency variance is high.

**Example:** Uniprogrammed systems; batch processing.

**When suitable:** Batch compute where all tasks are known upfront and latency is not critical.

---

### Shortest Job First (SJF)

**Algorithm:** Always run the task with the smallest estimated remaining time. Preemptive SJF (SRTF) switches to a longer task if a shorter one arrives.

**Analysis (non-preemptive):** Minimizes average turnaround time among all deterministic schedules.

**Proof sketch:** Two tasks with times $a < b$. Ordering $a$ then $b$ costs $a + (a+b) = 2a + b$. Ordering $b$ then $a$ costs $b + (a+b) = a + 2b$. Difference is $a - b < 0$, so shorter first is better.

**Pros:**
- Minimizes average turnaround time.
- Interactive tasks (typically short) respond quickly.
- Energy efficient for many workloads.

**Cons:**
- Requires prior knowledge of task length (often unknown).
- Starvation risk: long tasks may never run if short tasks keep arriving.
- Unfair; long tasks are de-prioritized.
- Difficult to estimate task duration in practice.

**When suitable:** Batch systems where task times are known; adaptive systems that estimate execution time from history.

---

### Priority Scheduling

**Algorithm:** Each task has a priority. Run the highest-priority task. Ties broken by FIFO or other rules.

**Variants:**
- **Non-preemptive:** A task runs to completion once started.
- **Preemptive:** A higher-priority task arriving interrupts a lower-priority task.

**Pros:**
- Flexible; supports diverse workloads (critical tasks first, batch jobs lower).
- Real-time systems use priority scheduling with deadlines as the priority metric.

**Cons:**
- Starvation: low-priority tasks may never run.
- Priority inversion: high-priority task waits for low-priority task holding a lock (mitigated by priority inheritance protocols).
- Requires manual priority assignment; hard to get right.

**Example:** Kernel scheduling assigns higher priority to I/O-bound tasks (interactive), lower to CPU-bound (batch).

---

### Round-Robin

**Algorithm:** Each task gets a fixed time quantum $q$ (typically 10-100ms). After using its quantum, the task is preempted and moved to the back of the queue.

**Analysis:** 
- Average turnaround time is no better than FIFO (and typically worse).
- Responsiveness is good; no task waits more than $(n-1) \cdot q$ time to run.
- Fairness: all tasks get equal CPU time over long intervals.

**Quantum tuning:**
- Small $q$ (e.g., 1ms): responsive but high context-switch overhead.
- Large $q$ (e.g., 100ms): low overhead but less responsive.
- Adaptive systems adjust $q$ based on task behavior.

**Pros:**
- Fair; all tasks progress.
- Responsive; no starvation.
- Simple to implement.

**Cons:**
- Turnaround time can be worse than SJF.
- Context-switch overhead can be significant (cache invalidation, TLB flushes).

**When suitable:** General-purpose time-sharing operating systems (Linux, macOS, Windows).

---

### Multilevel Feedback Queue (MLFQ)

**Algorithm:** Dynamically adjust task priority based on behavior.

**Structure:**
- Multiple queues at different priority levels.
- Tasks start in the highest-priority queue.
- If a task uses its full time quantum (CPU-bound), it drops to a lower-priority queue.
- If a task yields before its quantum (I/O-bound or blocked), it stays or rises in priority.

**Example schedule:**
```
Queue 0 (q=8ms):  I/O-bound tasks
Queue 1 (q=16ms): CPU-bound (medium)
Queue 2 (q=32ms): CPU-bound (long)
```

**Boosting:** Periodically boost all tasks back to the highest priority queue to prevent long-running tasks from starving.

**Pros:**
- Adapts to task behavior; no prior knowledge needed.
- I/O-bound tasks get good responsiveness; CPU-bound tasks get long quanta to reduce overhead.
- Works well in practice; used in modern Linux (CFS with feedback).

**Cons:**
- More complex to implement and tune (queue levels, boost intervals, quantum sizes).
- Vulnerable to gaming: tasks can yield just before their quantum expires, staying in high-priority queues.

**When suitable:** General-purpose operating systems where task behavior is unknown and diverse.

---

## Real-Time Scheduling

### Deadline-Based: Earliest Deadline First (EDF)

**Algorithm:** Always run the task with the nearest deadline. Preempt if a task arrives with an earlier deadline.

**Analysis:** EDF is **optimal** for single-processor scheduling of independent tasks with deadlines. If any schedule can meet all deadlines, EDF does.

**Proof sketch:** Suppose a schedule $S$ meets all deadlines and is not EDF. At some point, $S$ runs task $B$ while task $A$ (deadline earlier than $B$) is ready. Swap them; $A$ still meets its deadline (finishes earlier), $B$ must finish before the next task that conflicts with $B$'s deadline. By induction, EDF reordering preserves feasibility.

**Quasi-static vs. dynamic:**
- **Off-line (quasi-static):** All tasks, deadlines known upfront; compile a fixed schedule.
- **On-line (dynamic):** Tasks arrive over time; EDF decides each moment.

**Pros:**
- Optimal for single processor; strong theoretical guarantees.
- Simple to implement.

**Cons:**
- Assumes independent tasks (no precedence constraints).
- Assumes deterministic execution times (in practice, tasks are variable-time).
- Doesn't handle CPU overload gracefully (misses deadlines if overloaded).
- Doesn't account for task priorities beyond deadline.

**Example:** Real-time video rendering, aircraft systems, hard deadlines.

---

### Rate-Monotonic (RM) Scheduling

**Algorithm:** Assign fixed priority inversely proportional to task period. Shortest period = highest priority.

**Context:** Periodic tasks (repeat every $T_i$ time units).

**Analysis:** RM is optimal among fixed-priority schedules for periodic tasks. Schedulability condition (utilization bound):

$$\sum \frac{C_i}{T_i} \leq n(\sqrt[n]{2} - 1)$$

where $C_i$ is execution time, $T_i$ is period, and $n$ is the number of tasks.

For large $n$, the bound approaches $\ln 2 \approx 0.69$. This means a system using RM can guarantee all deadlines if utilization is below this threshold.

**Pros:**
- Fixed priority simplifies implementation.
- Periodic tasks are common (sensors, displays, control loops).
- Optimal for fixed priorities.

**Cons:**
- Utilization bound <70% wastes CPU; EDF can go up to 100%.
- Requires tasks to be periodic with known periods.
- Priority inversion still possible.

**Example:** Industrial control systems, robotics.

---

## Job Shop Scheduling

**Problem:** $n$ jobs, $m$ machines, each job has a sequence of tasks on specific machines. Minimize completion time (makespan) or total latency.

**Constraints:** 
- Each job must visit machines in a fixed order.
- Each machine can handle one job at a time.
- No preemption (a job on a machine runs to completion).

**Complexity:** NP-hard in general; even the 2-machine case is solvable in $O(n \log n)$ (Johnson's algorithm).

### Johnson's Algorithm (2 machines)

**Algorithm:**
1. Partition jobs into two groups: those whose first operation is shorter vs. longer on machine 1.
2. Order: shorter-on-M1 jobs (sorted by max operation time ascending), then longer-on-M1 jobs (sorted by max operation time descending).

**Result:** Optimal makespan for 2 machines.

**Why it works:** Intuitively, this balances loading on machine 1 (which other jobs can start while machine 2 processes early jobs) and machine 2.

### Heuristics for $m > 2$

**Shortest Processing Time (SPT):**
- Sort tasks by duration (shortest first).
- Assign each task to the machine that will be free earliest.
- Greedy; often produces good results but not optimal.

**Critical Path Method (CPM):**
- Build a dependency graph of tasks.
- Identify the critical path (longest sequence of sequential tasks).
- Prioritize tasks on the critical path; schedule non-critical tasks flexibly.

**Local Search / Genetic Algorithms:**
- Start with a greedy solution.
- Iteratively swap job orderings or shift tasks between machines to improve makespan.
- No guarantee of optimality but often finds good solutions in practice.

---

## DAG Task Scheduling

**Problem:** Tasks have dependencies (a DAG — directed acyclic graph). Schedule tasks to minimize total time, respecting dependencies.

**Variants:**
- **Offline:** All tasks and dependencies known upfront.
- **Online:** Tasks arrive over time; dependencies may depend on runtime values.

### Critical Path

The **critical path** is the longest directed path through the DAG. This is a lower bound on total execution time (no parallelism can make this shorter).

**Algorithm:**
1. Topologically sort tasks.
2. Compute the earliest-finish-time (EFT) for each task: `EFT[i] = max(EFT[predecessor]) + duration[i]`.
3. The critical path is the set of tasks with `EFT[i] = EFT[final_task]`.

**Scheduling via list scheduling:**
1. Compute EFT for all tasks.
2. Assign priority inversely to EFT (higher priority = larger EFT, i.e., on the critical path).
3. At each scheduling decision, assign the highest-priority ready task to the earliest-available processor.

**Performance:** List scheduling is within $2 - \frac{1}{m}$ of optimal (where $m$ is the number of processors).

---

## Distributed Scheduling

### Kubernetes Scheduling

**Problem:** Given a cluster of machines with constraints (CPU, memory, I/O, affinity rules), assign pods (containerized tasks) to machines.

**Algorithm:**
1. **Filter phase:** Identify nodes that satisfy hard constraints (CPU, memory, label selectors).
2. **Score phase:** Rank remaining nodes by soft preferences (spread across nodes, avoid high-load nodes, etc.).
3. **Bind:** Assign pod to the highest-scoring node.

**Customization:** Users define schedulers for custom logic (e.g., GPU-aware scheduling, data locality).

**Pros:**
- Declarative; users specify constraints and preferences separately.
- Extensible; custom schedulers for specific workloads.

**Cons:**
- Greedy assignments; no global optimization.
- Does not account for jobs that haven't yet been assigned (future jobs may be starved).

### YARN (Apache Hadoop)

**Problem:** Schedule batch jobs and long-running services on a shared cluster.

**Algorithm:**
1. Jobs request containers (CPU + memory).
2. Resource manager tracks node capacity.
3. Scheduler assigns containers to nodes trying to balance workloads and respect locality (prefer scheduling on nodes that have the task's input data).

**Fairness:** Multiple scheduling policies (FIFO, Fair Scheduler, Capacity Scheduler).

### Mesos

**Similar to YARN:** A two-level scheduling system where the Mesos kernel offers resources to frameworks, and frameworks decide whether to accept.

**Advantage:** Distributed decision-making; frameworks can reject offers that don't meet their needs, reducing global optimization overhead.

---

## Scheduling Objectives: Trade-offs

| Objective | Definition | Trade-off |
|-----------|-----------|----------|
| Throughput | Tasks per unit time | Maximizing throughput may delay deadline misses. |
| Average latency | Mean time to completion | Variance may be high; some tasks may be starved. |
| Latency tail (P99) | 99th percentile completion time | Requires prioritizing bad-luck tasks; reduces average throughput. |
| Fairness | Equal CPU time to all tasks | Unfair to priority tasks (e.g., interactive vs. batch). |
| Energy | Minimize CPU time + idle power | May conflict with latency (fast task completion uses more power). |
| Predictability | Deterministic completion time | Difficult with bursty arrival patterns or variable execution time. |

---

## Practical Implementation Patterns

### Scheduler Design

1. **Choose the right timing:** Pre-emptive schedulers (quantum-based) need timers; non-preemptive need explicit yield/block points.
2. **Queue structures:** Use a priority queue for efficiency; FIFO is simpler but less flexible.
3. **Context switching:** Save/restore thread state (registers, memory context, TLB). Minimize frequency to reduce overhead.
4. **Starvation prevention:** Boost low-priority tasks periodically or use aging (increase priority over time waiting).

### Tuning

- **Quantum size:** 10-100ms for general-purpose; 1-10ms for interactive; 1-5s for batch.
- **Priority levels:** 2-4 levels usually sufficient; more levels add complexity without benefit.
- **Boost interval:** Every 100-200ms is typical to prevent indefinite starvation.

---

## When to Use Each Algorithm

| Algorithm | Best For | Avoid If |
|-----------|----------|----------|
| FIFO | Batch, known tasks | Interactive workloads, long tasks present |
| SJF | Known task lengths, batch | Tasks too unpredictable, no prior knowledge |
| Priority | Diverse workloads | Hard to set priorities correctly |
| Round-robin | Time-sharing, fair | Real-time deadlines, long tasks |
| MLFQ | Unknown task behavior | Cannot measure I/O wait, complex tuning |
| EDF | Deadline-driven, soft real-time | Many periodic tasks, overload conditions |
| RM | Periodic tasks, hard real-time | Aperiodic tasks, variable execution time |
| DAG scheduling | Parallel tasks, known DAG | Highly dynamic task arrival, unknown dependencies |
| Distributed (K8s) | Cloud workloads, elasticity | Tight coupling between tasks, strict deadlines |

---

## See Also

- os-process-management.md (OS scheduling in context)
- infrastructure-load-balancing.md (load balancer scheduling)
- algorithms-graph.md (DAG algorithms)
- math-complexity-theory.md (NP-hardness of job shop scheduling)