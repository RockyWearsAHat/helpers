# OS Process Management — Lifecycle, PCB, Scheduling & IPC

## Overview

A **process** is an independent, executable instance of a program running on an operating system. Processes are the abstraction that enables multitasking: the illusion that many programs run simultaneously (true parallelism on multicore, or rapid context switching on single core). Understanding process lifecycle, scheduling algorithms, and inter-process communication (IPC) is foundational for systems programming, debugging, and performance analysis.

## Process Lifecycle

A process transitions through states managed by the kernel scheduler:

```
         +--------+
         |   New  |  (Process created; not yet admitted to ready queue)
         +---+----+
             |
             v
         +--------+
    +--> | Ready  |  (Runnable, waiting for CPU time)
    |    +--------+
    |        |
    |        v
    |    +--------+
    |    | Running|  (Currently executing on CPU)
    |    +---+----+
    |        |
    +--------+    (preemption or yield)
    |
    |    +----------+
    +----| Blocked  |  (Waiting for I/O, lock, etc.)
         +----------+
             |
             v
         +--------+
         | Exited |  (Process terminated; kernel waiting to parent to reap)
         +--------+
```

**States:**
- **New**: Process created but not yet added to scheduler queues. Kernel allocates PCB, memory, file descriptors.
- **Ready**: Process is runnable and in ready queue, but CPU not allocated.
- **Running**: Process is executing on CPU.
- **Blocked**: Process is waiting for an event (I/O completion, lock acquisition, sleep timeout). Cannot run until event occurs.
- **Exited (Zombie)**: Process has terminated but parent process has not called `wait()` to retrieve exit status. PCB remains in kernel.

## Process Control Block (PCB)

The **Process Control Block** is a kernel data structure that stores all information about a process:

```
PCB:
  - Process ID (PID): Unique identifier
  - Parent PID (PPID): Creating process
  - State: Running, Ready, Blocked, etc.
  - Priority: Scheduler priority (lower number = higher priority, or inverted)
  - CPU Registers: Program counter, stack pointer, general-purpose registers
  - Memory Pointers:
      - Text segment (code)
      - Data segment (global variables, heap)
      - Stack segment
  - I/O State: Open file descriptors (file table)
  - Signal Handlers: Actions for signals (SIGINT, etc.)
  - Accounting: CPU time used, start time, memory usage
  - Scheduling Info: Time quantum remaining, priority, queue pointers
```

When the kernel switches from one process to another, it **saves** the current process's register state into its PCB, then **loads** the next process's registers from its PCB. This is the essence of **context switching**.

## Context Switching

Context switching is the mechanism by which the kernel pauses one process and resumes another. Steps:

1. **Interrupt or System Call**: Timer interrupt (scheduler quantum expired) or system call triggers context switch.
2. **Save Current Context**: CPU registers (PC, SP, general registers, floating-point state, special registers) copied from CPU to current process's PCB.
3. **Select Next Process**: Scheduler algorithm chooses next process from ready queue.
4. **Load New Context**: Registers from PCB of next process copied to CPU.
5. **Resume Execution**: CPU jumps to program counter of new process.

**Context Switch Overhead**: Modern systems spend 1-10 microseconds per switch (varies by CPU). Costs include:
- CPU cache misses (new process's working set not in cache).
- TLB flushes (CPU cache of virtual-to-physical address mappings invalidated if address spaces differ).
- Kernel code execution.

Excessive context switching (thrashing) degrades performance; proper process/thread count and load balancing minimize it.

## CPU Scheduling Algorithms

### FCFS (First-Come-First-Served)

Simplest non-preemptive algorithm: process runs to completion or blocks.

**Pros**: No starvation; fair.
**Cons**: Long average waiting time; one long-running process blocks others (convoy effect).

Example: Processes A (24ms), B (3ms), C (3ms) arrive in order.
```
  A A A A A A A A B C
  0           24 27 30
Average waiting time: (0 + 24 + 27) / 3 = 17ms
```

### Round-Robin (Time Slicing)

Preemptive: Each process gets a time quantum (e.g., 10ms). If not finished, moved to back of queue.

**Pros**: Responsive; prevents single process from monopolizing CPU.
**Cons**: Context switch overhead; average waiting time worse than SJF.
**Quantum tuning**: Too small → excessive switching; too large → behaves like FCFS.

### SJF (Shortest Job First)

Non-preemptive (or preemptive variant SRTF). Run process with shortest remaining time first.

**Pros**: Minimizes average waiting time.
**Cons**: Starvation risk (short jobs keep arriving, long jobs postponed); requires predicting burst time (hard).

Example: Same processes A (24ms), B (3ms), C (3ms).
```
  B C A A A A A A A A A A
  0 3 6                 30
Average waiting time: (0 + 3 + 6) / 3 = 3ms (much better!)
```

### Priority-Based Scheduling

Each process has a priority. CPU allocated to highest-priority ready process.

**Issue**: **Priority Inversion**. High-priority process H waiting for lock held by low-priority process L, while medium-priority process M keeps preempting L. H is blocked, L doesn't run, M runs. Solution: Priority inheritance (L's priority temporarily raised to H's while holding lock).

### Completely Fair Scheduler (CFS) — Linux

Modern algorithm targeting fairness: each process receives proportional CPU time. Implemented via red-black tree; selects process with "furthest behind" virtual time (`vruntime`).

**Key idea**: Track ideal CPU allocation; preempt process that has exceeded its fair share.
**Benefit**: Responsive; scales well with process count; avoids priority inversion by design.

### O(1) Scheduler (Pre-2.6 Linux)

Constant-time scheduling via per-priority queues. Degraded under heavy load (priority inversion, interactive task starvation). Replaced by CFS in newer kernels.

## Process vs Thread

| Aspect | Process | Thread |
|--------|---------|--------|
| **Memory** | Isolated address space; memory protection | Shared address space within process |
| **Context Switch Cost** | Expensive (TLB flush, cache miss) | Cheaper (shared memory, TLB hit likely) |
| **Creation Cost** | Expensive (allocate memory, open file descriptors) | Cheap (reuse memory, stack allocation) |
| **Communication** | IPC mechanisms (pipes, sockets, shared memory) | Direct shared memory access (race condition risk) |
| **Scalability** | 100s-1000s per system | 1000s-100,000s per system; thread pools |
| **Fault Isolation** | Fault in one process doesn't crash others | Thread fault can crash entire process |
| **Concurrency Model** | Processes + IPC; higher overhead, safer | Threads + synchronization; lower overhead, higher complexity |

**Threads** are lightweight; most modern concurrent systems (web servers, databases) use thread pools. **Processes** provide isolation; used for independent services, privilege separation (web server runs as unprivileged user).

## IPC (Inter-Process Communication)

### Pipes (Anonymous)

Unidirectional, in-memory channels connecting parent and child processes:

```bash
command1 | command2  # Output of command1 is input to command2
```

File descriptor-based: write end, read end. Data is buffered (typically 4-64KB); write blocks if full, read blocks if empty.

**Cons**: Unidirectional only; limited buffer; connects related (hierarchical) processes only.

### Named Pipes (FIFOs)

Like pipes but with a filesystem entry:

```bash
mkfifo /tmp/myfifo
echo "hello" > /tmp/myfifo  # Write end
cat < /tmp/myfifo            # Read end (blocks until data)
```

**Pros**: Named; any process can open (if permissions allow).
**Cons**: Still unidirectional; filesystem overhead.

### Shared Memory

Processes map the same physical memory region into their virtual address spaces. Fastest IPC (no data copying).

```c
// Process A
int *shared = shmat(shmid, NULL, 0);
*shared = 42;

// Process B
int *shared = shmat(shmid, NULL, 0);
printf("%d\n", *shared);  // 42 (same memory)
```

**Pros**: Speed; can share complex data structures.
**Cons**: Synchronization required (locks, semaphores); no built-in safety; memory protection issues possible on failur.

### Message Queues

FIFO queues of messages. Producer sends message; consumer receives.

```c
msgsnd(qid, &msg, sizeof(msg), 0);  // Send
msgrcv(qid, &msg, sizeof(msg), 0, 0); // Receive (blocks until available)
```

**Pros**: Decoupled; ordered; priority levels available.
**Cons**: Kernel overhead; limited message size; System V IPC complexity.

### Sockets

TCP/UDP sockets enable IPC over network or local domain sockets (Unix sockets):

```c
// Server
int sock = socket(AF_UNIX, SOCK_STREAM, 0);
bind(sock, &addr, sizeof(addr));
listen(sock, 5);
int client = accept(sock, NULL, NULL);
recv(client, buf, 1024, 0);

// Client
int sock = socket(AF_UNIX, SOCK_STREAM, 0);
connect(sock, &addr, sizeof(addr));
send(sock, "hello", 5, 0);
```

**Pros**: Flexible (local or network); widely used.
**Cons**: Connection-oriented overhead (not suitable for broadcast); socket family complexity.

### Signals

Asynchronous notifications. One process sends signal to another; process handles signal with registered handler.

```c
signal(SIGTERM, handler);  // Register handler
kill(pid, SIGTERM);         // Send SIGTERM to process pid
```

Common signals:
- `SIGTERM` (terminate)
- `SIGKILL` (kill; cannot be caught)
- `SIGSEGV` (segmentation fault; usually crashes)
- `SIGCHLD` (child exited; parent can reap status)
- `SIGUSR1,SIGUSR2` (user-defined)

**Pros**: Simple; asynchronous; interrupt-based.
**Cons**: No data payload (signal number only); unreliable in old Unix; 32 signal limit.

## Zombie & Orphan Processes

### Zombie

A process that has exited but whose parent has not called `wait()` or `waitpid()` to retrieve its exit status. The PCB persists in the kernel, consuming a process table slot.

```bash
./my_program &
ps aux | grep my_program
# Result: Z (defunct) in STAT column
```

Mitigation: Parent must reap children via `wait()` or `SIGCHLD` handler.

### Orphan

A process whose parent exited. The process is reparented to `init` (PID 1) by the kernel. Not a problem; `init` reaps orphans.

## See Also

- **concurrency-patterns** — Thread synchronization, race conditions, deadlock.
- **memory-management** — Virtual memory, paging, segmentation.
- **networking-tcp-ip** — Socket communication model.