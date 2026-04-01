# Unix Signals and IPC — Async Notification, Message Passing, and Resource Sharing

## Overview

Unix signals are asynchronous notifications delivered to processes: mechanisms for handling events (interrupts, termination requests, child process state changes) that occur outside normal execution flow. **Inter-Process Communication (IPC)** mechanisms—pipes, sockets, shared memory, message queues, semaphores—enable processes to coordinate, exchange data, and synchronize. Together, signals and IPC form the substrate for multi-process systems, but both carry subtle semantics (signal safety, deadlock potential, ordering guarantees) that demand careful handling.

## Unix Signals: Asynchronous Events

A signal is a software interrupt. The kernel delivers it by interrupting the process's execution, invoking a handler, then resuming.

### Common Signals

| Signal | Default Action | Use Case |
|--------|-----------------|----------|
| SIGTERM | Terminate | Graceful shutdown (kill -TERM PID) |
| SIGKILL | Terminate (uncatchable) | Forceful kill; cannot be caught or ignored |
| SIGINT | Terminate | Keyboard interrupt (Ctrl+C) |
| SIGCHLD | Ignore | Child process exited; parent should reap |
| SIGPIPE | Terminate | Write to pipe/socket with no readers |
| SIGSTOP | Stop (uncatchable) | Suspend process; cannot be caught |
| SIGCONT | Continue | Resume stopped process |
| SIGUSR1, SIGUSR2 | Terminate | Application-defined signals |
| SIGALRM | Terminate | Timer expired (set via alarm() or setitimer()) |

### Signal Delivery and Handling

```
Process A runs...
Kernel receives async event (e.g., SIGTERM from kill)
│
├─ Kernel checks signal handlers in process A's PCB
├─ If handler installed: Save registers, call handler(signum)
└─ Handler runs (async context) → returns → kernel resumes process

Process A continues (interrupted state restored)
```

### Signal Mask and Atomicity

Signals can arrive at inconvenient moments. Processes manage this via signal masks:

```c
sigset_t mask;
sigemptyset(&mask);
sigaddset(&mask, SIGTERM);
sigaddset(&mask, SIGCHLD);

// Block these signals during critical section
sigprocmask(SIG_BLOCK, &mask, NULL);
  // Critical code: SIGTERM, SIGCHLD won't interrupt
sigprocmask(SIG_UNBLOCK, &mask, NULL);

// Or atomically wait for signal while blocking others
sigsuspend(&mask);  // Sleep until signal NOT in mask arrives
```

### Signal Safety: Limited Semantics in Handlers

Signal handlers run in an async context and can interrupt ANY instruction. The C standard library is not async-safe: malloc, printf, pthread functions may be reentrant-unsafe.

**Signal-safe functions** (POSIX guarantees atomicity):
- `write()`, `read()` (not `FILE* printf`)
- `kill()`, `raise()`, `pause()`
- `signal()`, `sigaction()`
- `exit()`, `_exit()`, `abort()`

**Unsafe in handlers:**
- `malloc()`, `free()` (not reentrant; may corrupt heap)
- `printf()` (calls malloc internally)
- `pthread_mutex_lock()` (if handler interrupts lock code)
- Anything in user code (unless proven signal-safe)

**Best practice**: Handlers set a volatile atomic flag; main loop polls it.

```c
volatile sig_atomic_t signal_received = 0;

void handler(int sig) {
    signal_received = 1;
}

// Main loop
while (1) {
    if (signal_received) {
        // Safe to do complex work here
        handle_signal();
        signal_received = 0;
    }
    // Normal work
}
```

### SIGCHLD and Process Reaping

When a child exits, the kernel sends SIGCHLD to the parent. Parent must call `wait()` or `waitpid()` to reap the child's PCB; otherwise, the child becomes a **zombie** (terminated but not yet reaped).

```c
void reaper(int sig) {
    pid_t pid;
    int status;
    // Loop: reap all exited children (use WNOHANG to non-block)
    while ((pid = waitpid(-1, &status, WNOHANG)) > 0) {
        // Process exit status
    }
}

int main() {
    signal(SIGCHLD, reaper);
    // ...
    waitpid(-1, &status, WUNTRACED);  // Reap on demand
}
```

**Trade-off:** Asynchronous reaping (SIGCHLD handler) vs. synchronous (waitpid() in main loop). Async is responsive; synchronous is easier to reason about but requires polling or blocking.

### SIGPIPE: Broken Pipe Convention

Writing to a pipe or socket after all readers have closed triggers SIGPIPE, which terminates the process by default. This is a recovery mechanism: prevents writer from continuing to produce for nobody.

```c
write(pipe_fd, data, size);  // All readers closed
// Kernel sends SIGPIPE; process dies unless caught

// Catch and handle
signal(SIGPIPE, handler);
write(pipe_fd, data, size);  // Now just returns -1 (EPIPE)
```

## Inter-Process Communication (IPC)

Processes are isolated; IPC mechanisms create connections for data exchange and synchronization.

### Pipes: Unnamed, Unidirectional

A pipe is a ring buffer: write on one end, read on the other. Anonymous pipes are created with `pipe()` and typically used between parent and child.

```c
int pfd[2];
pipe(pfd);  // pfd[0] = read end, pfd[1] = write end

if (fork() == 0) {
    // Child
    close(pfd[1]);  // Don't need write end
    read(pfd[0], buf, size);
} else {
    // Parent
    close(pfd[0]);  // Don't need read end
    write(pfd[1], data, size);
}
```

**Properties:**
- Unidirectional: one-way communication
- Atomic writes < PIPE_BUF (4 KB on Linux)
- Non-seekable: FIFO discipline
- File descriptor based: inherited across fork

**Limitation:** Small size (~64 KB default), no tags or message boundaries, one-way only.

### Named Pipes (FIFOs)

A pipe with a name in the filesystem. Unrelated processes can open it like a file.

```bash
mkfifo /tmp/myfifo
# Process A
cat > /tmp/myfifo &
# Process B
cat < /tmp/myfifo  # Blocks until A writes
```

**Trade-off:** Unidirectional, but not limited to parent-child relationships.

### Unix Domain Sockets

Sockets are a bidirectional, connection-based IPC. Unix domain sockets are local to a machine (unlike TCP sockets).

```c
// Server
int server = socket(AF_UNIX, SOCK_STREAM, 0);
bind(server, (struct sockaddr_un *)&addr, len);
listen(server, 5);
client = accept(server, NULL, NULL);  // Blocks until connection

// Client
int client = socket(AF_UNIX, SOCK_STREAM, 0);
connect(client, (struct sockaddr_un *)&addr, len);
send(client, data, size, 0);
```

**Advantages:**
- Bidirectional (full-duplex)
- Datagram (SOCK_DGRAM) or stream (SOCK_STREAM)
- Credential passing via SCM_CREDENTIALS
- Better performance than TCP for localhost

**Use case:** Daemon communication (systemd uses this extensively).

### Shared Memory: SysV and POSIX

Processes can map the same physical memory region, enabling fast data exchange (no copying through kernel).

#### SysV Shared Memory

```c
// Create and attach
int shmid = shmget(IPC_PRIVATE, 4096, IPC_CREAT | 0666);
char *addr = shmat(shmid, NULL, 0);
*addr = 'A';  // Write

// Another process
char *addr = shmat(shmid, NULL, 0);
printf("%c", *addr);  // Reads 'A'

shmdt(addr);
shmctl(shmid, IPC_RMID, NULL);
```

**Trade-offs:**
- Very fast (no kernel round-trips for data transfer)
- Manual synchronization needed (use semaphores)
- Survives process death; must explicitly remove
- Orphan segments accumulate if not cleaned up

#### POSIX Shared Memory

```c
int shmfd = shm_open("/myshm", O_CREAT | O_RDWR, 0666);
ftruncate(shmfd, 4096);
void *addr = mmap(NULL, 4096, PROT_READ|PROT_WRITE,
                  MAP_SHARED, shmfd, 0);
```

**Improvements:**
- File-based (persists in /dev/shm)
- Cleaner lifecycle (unlink() removes)
- Integrates with mmap

### Semaphores: Synchronization Primitive

Semaphore is a counter with two atomic operations:
- **wait()** (P): Decrement; block if counter is zero
- **signal()** (V): Increment; wake one waiter

```c
// Binary semaphore (mutex-like)
sem_t *sem = sem_open("/mysem", O_CREAT, 0666, 1);

sem_wait(sem);    // Acquire; block if taken
// Critical section
sem_post(sem);    // Release

// Counting semaphore (resource pool)
sem_t *pool = sem_open("/pool", O_CREAT, 0666, 10);  // 10 resources
sem_wait(pool);   // Acquire resource
// Use resource
sem_post(pool);   // Release
```

**Trade-off:** Semaphores are powerful but prone to priority inversion and deadlock if misused. Mutexes (with priority inheritance in real-time) are often safer.

### Message Queues (System V and POSIX)

Message queues enable sending discrete messages (with tags) between processes.

```c
// System V: send message
struct msgbuf { long mtype; char mtext[256]; };
int qid = msgget(IPC_PRIVATE, IPC_CREAT | 0666);
msgbuf msg = {1, "Hello"};
msgsnd(qid, &msg, sizeof(msg.mtext), 0);

// Receive by type
msgrcv(qid, &msg, sizeof(msg.mtext), 1, 0);
```

**Advantages:**
- Discrete messages (not byte streams)
- Type-based filtering
- Asynchronous: sender doesn't block

**Disadvantages:**
- Kernel buffer limited; sender may block if queue full
- SysV version: no modern lifecycle management (orphans persist)
- POSIX version (mq_open) is cleaner but less common

## D-Bus

D-Bus is a high-level message bus for system and session communication, widely used on Linux for inter-component messaging.

```c
// DBusConnection *conn = dbus_bus_get(DBUS_BUS_SESSION, &err);
// Send message to org.example.Service
DBusMessage *msg = dbus_message_new_method_call(
    "org.example.Service",  // Destination
    "/org/example/Object",  // Path
    "org.example.Interface",  // Interface
    "DoSomething"  // Method
);

dbus_connection_send_with_reply_and_block(conn, msg, 5000, &err);
```

**Model:**
- Named services (org.freedesktop.systemd1)
- Object paths (/org/freedesktop/systemd1/unit/sshd_2eservice)
- Methods, signals, properties
- Central daemon (dbus-daemon) routes messages

**Use case:** Desktop integration (notification service, power management, device discovery).

## IPC Trade-Offs Summary

| Mechanism | Speed | Atomicity | Bidirectional | Scalability | Use Case |
|-----------|-------|-----------|---------------|-------------|----------|
| Pipes | Very fast | Atomic < PIPE_BUF | No | Parent-child | Shell redirection |
| Unix sockets | Fast | Stream or datagram | Yes | Bidirectional | Daemon communication |
| Shared memory | Fastest | Manual (semaphores) | N/A (shared object) | Very high | High-frequency trading, shared data |
| Message queues | Fast | Per-message | One-way queues | Moderate | Task queues, logging |
| Semaphores | - | Yes | - | - | General synchronization |
| D-Bus | Moderate | Yes (via daemon) | Yes | Low-moderate | System services, desktop |

## See Also

- **os-process-management** — Process creation, fork semantics
- **os-concurrency-primitives** — Mutexes, condition variables, alternatives to IPC
- **systems-debugging-tools** — strace (trace signals, system calls)
- **devops-container-runtimes** — How containers isolate signals, IPC namespaces