# I/O Models — Blocking, Non-Blocking, Async, and Multiplexing

## Overview

**I/O models** describe how programs coordinate with I/O operations (reading from network sockets, files, devices). Different models trade off simplicity, throughput, and latency. The evolution from blocking I/O → select/poll → epoll/kqueue → io_uring reflects decades of addressing the **C10K problem**: how to handle thousands of concurrent connections efficiently. Understanding each model's semantics and performance trade-offs is essential for building scalable servers.

## I/O Operation Latency

I/O is **slow** compared to CPU:

```
CPU cycle:               ~0.3 ns
L1 cache load:          ~0.5 ns
L2 cache load:          ~7 ns
L3 cache load:          ~40 ns
Main memory load:       ~100 ns
SSD read:               ~1,000,000 ns (1 ms)
Network round-trip:     ~10,000,000 ns (10 ms)
Disk seek + read:       ~50,000,000 ns (50 ms)
```

A CPU could execute billions of instructions while waiting for a network read. Blocking on a single I/O request wastes CPU.

## Blocking I/O (1-to-1 Model)

**Blocking I/O** is the simplest model: the thread waits until the operation completes.

```
Thread 1:
  Client connects: socket = accept(listen_fd)
  read(socket)     ← Blocks until client sends data (10 ms+)
  process_request()
  send_response(socket)
  close(socket)

  Meanwhile: CPU idle, thread blocked, cannot serve other clients
```

**For N concurrent connections:**
- Require N threads (one per client, plus some overhead)
- Kernel maintains N stacks (typically 1-8 MB each) → 10,000 clients = 10-80 GB memory (before request processing)
- Context switching overhead: N threads → N scheduling decisions

**Scale limit (C10K problem)**: ~10,000 connections hit memory and scheduling overhead limits. Older web servers (Apache's 1:1 model) struggled here.

**Advantages:**
- Simple logic: read → process → write (sequential)
- Familiar to most developers
- Works for small client counts (< 100)

## Non-Blocking I/O

Set file descriptor to non-blocking mode (`fcntl(fd, F_SETFL, O_NONBLOCK)`). Operations return immediately:

```
Socket operations (non-blocking):
  read(socket)  → Returns immediately, -1 if no data (errno=EAGAIN)
  write(socket) → Returns immediately, partial write if buffer full
  accept(listen_fd) → Returns immediately, NULL if no client waiting

Example:
  fd = socket(...)
  fcntl(fd, F_SETFL, O_NONBLOCK);
  
  // Attempt to read
  int n = read(fd, buf, 256);
  if (n < 0 && errno == EAGAIN) {
    // No data available; try again later
    // CPU is now free to handle other clients
  }
```

**Problem**: Polling loop burns CPU. Busy-waiting without knowing when data is ready:

```
while (1) {
  for (int i = 0; i < N; i++) {
    read(sockets[i], buf, 256);  // Try each client
  }
}
// Each iteration: N syscalls, N return-immediately results
// 100% CPU usage even if no clients are sending
```

## I/O Multiplexing: select() & poll()

**I/O multiplexing** asks the kernel: "Wait until one of these file descriptors is ready. Tell me which one."

### select(2)

```
int select(int nfds, fd_set *readfds, fd_set *writefds,
           fd_set *exceptfds, struct timeval *timeout);
```

Blocks until file descriptor(s) are ready:

```
fd_set readfds, writefds;
FD_ZERO(&readfds);
FD_ZERO(&writefds);

// Monitor socket 3 for reading, socket 5 for writing
FD_SET(3, &readfds);
FD_SET(5, &writefds);

// Block until 3 readable, 5 writable, or 10 seconds elapses
select(6, &readfds, &writefds, NULL, &timeout);

// Check which are ready
if (FD_ISSET(3, &readfds))   read(3, ...);
if (FD_ISSET(5, &writefds))  write(5, ...);
```

**Limitations:**
- **O(n) iteration**: Kernel scans all registered FDs on each call
- **Limited FD count**: fd_set is fixed-size bitmap (typically 1024 FDs max on many systems)
- **Re-register every call**: Rebuild fd_set after each select

**Complexity**: For 10,000 connections, select must scan all 10,000 to find which completed. Multiple calls per second = poor scaling.

### poll(2)

```
struct pollfd {
  int fd;           // File descriptor
  short events;     // Requested events (POLLIN, POLLOUT, etc.)
  short revents;    // Returned events (set by kernel)
};

poll(fds, nfds, timeout);
```

Advantages over select:
- No fd_set bitmap limit (can poll up to `nfds`)
- Uses array instead of bitmask (bit manipulation avoided)

**Disadvantages**: Still O(n) — kernel must iterate all entries to find ready FDs.

## Level-Triggered vs. Edge-Triggered

**Level-Triggered (LT)**: Notification whenever FD is in a "ready" state.

```
read(socket) returns 5 bytes, 10 bytes available in buffer
epoll_wait() returns: "socket is readable"
If you don't read those 10 bytes:
epoll_wait() again returns: "socket is still readable"

Simple to use; may miss: If unread data accumulates, subsequent calls fire repeatedly
```

**Edge-Triggered (ET)**: Notification only when state **changes** from "not ready" to "ready".

```
Data arrives on socket (not-ready → ready): epoll fires
Kernel does NOT fire again until:
  - More data arrives (not-ready again → ready again)
  - Or FD is cleared (all data read)

read() returns 5 bytes; 10 bytes remain
epoll_wait() does NOT return again (edge already fired)
Must drain all bytes in one read or loop until EAGAIN
```

**ET trade-off:**
- **Pro**: Lower event delivery overhead (no spurious notifications); more control
- **Con**: More complex logic; risk of starving a connection if not fully drained

## epoll() (Linux, scales to 100k+ connections)

**epoll** maintains a kernel-side interest list (eliminating re-register overhead) and returns only ready FDs:

```
// Create epoll instance
int ep = epoll_create(1);

// Register interest in socket 5, readable events, edge-triggered
struct epoll_event ev;
ev.events = EPOLLIN | EPOLLET;
ev.data.fd = 5;
epoll_ctl(ep, EPOLL_CTL_ADD, 5, &ev);

// Wait for events (no re-register)
struct epoll_event events[128];
int n = epoll_wait(ep, events, 128, timeout);

for (int i = 0; i < n; i++) {
  int fd = events[i].data.fd;
  if (events[i].events & EPOLLIN)   read(fd, ...);
  if (events[i].events & EPOLLOUT)  write(fd, ...);
}
```

**Scalability:**
- **O(1) registration**: epoll_ctl adds/removes FD from kernel set (once)
- **O(k) wait**: epoll_wait returns k ready events; iterate only ready ones (not all FDs)

**Complexity**: For 100,000 connections with 100 active, you iterate 100, not 100,000.

**Level-triggered (default):** Most beginner-friendly.  
**Edge-triggered:** Slightly more efficient, more error-prone.

## kqueue() (BSD/macOS, similar to epoll)

macOS, BSD, and some other systems use **kqueue** instead of epoll:

```
// Create kqueue
int kq = kqueue();

// Register interest in socket 5 for reading
struct kevent ev;
EV_SET(&ev, 5, EVFILT_READ, EV_ADD, 0, 0, NULL);
kevent(kq, &ev, 1, NULL, 0, NULL);

// Wait for events
struct kevent events[128];
int n = kevent(kq, NULL, 0, events, 128, NULL);

for (int i = 0; i < n; i++) {
  int fd = events[i].ident;
  if (events[i].filter == EVFILT_READ)   read(fd, ...);
}
```

**Similar to epoll:**
- O(1) registration (kevent sets up filter)
- O(k) iteration (return k ready events)

**Differences:**
- More general: Not I/O-specific; can monitor timers, signals, process events
- More portable across BSD variants
- No separate edge-trigger flag; automatic edge-trigger semantics for some filters

## The C10K Problem & Event-Driven Architecture

**The C10K Problem** (Dan Kegel, 1999): How to serve 10,000+ concurrent connections on a single server?

**Root cause**: Blocking or select-based models scale poorly.

**Solution**: Event-driven architecture.

```
Traditional (thread-per-client):
  Client 1 → Thread 1 (blocked on read)
  Client 2 → Thread 2 (blocked on read)
  ...
  Client 10000 → Thread 10000 (blocked on read)
  
  Overhead: 10000 stacks, context switching load, memory waste

Event-driven (single thread + epoll):
  epoll (non-blocking)
    ├─ Client 1 (readable)  → process
    ├─ Client 2 (readable)  → process
    ├─ Client 3 (writable)  → process
    └─ ...
  
  Overhead: Single stack, minimal context switching, CPU busy-working
```

Modern servers (Nginx, Node.js, Tokio, Netty): Event-driven + event loop.

## Reactor vs. Proactor Patterns

**Reactor** (epoll/kqueue model):
- Application waits on multiplexer (epoll_wait)
- When FD is ready, application calls I/O function (read, write)

```
while (1) {
  epoll_wait()  // Blocks until event
  read() or write()  // Application initiates I/O
}
```

**Proactor** (io_uring/IOCP model):
- Application submits I/O operations in advance
- Kernel completes them asynchronously
- Application polls for completions

```
while (1) {
  io_uring_get_cqe()  // Poll completion queue
  // Process completed I/O results
  io_uring_submit()   // Submit next batch of operations
}
```

Proactor avoids redundant context switches between application and kernel.

## io_uring (Linux, async I/O, modern approach)

**io_uring** submits I/O operations **in batch**, kernel processes them asynchronously.

```
// One-time setup
struct io_uring ring;
io_uring_queue_init(64, &ring, 0);  // Queue size 64

// Application loop
while (have_work) {
  // Prepare submission (multiple operations)
  for (int i = 0; i < batch; i++) {
    struct io_uring_sqe *sqe = io_uring_get_sqe(&ring);
    io_uring_prep_read(sqe, fds[i], buffers[i], 256, 0);
    sqe->user_data = i;  // Tag for tracking
  }
  
  // Submit all at once (single syscall)
  io_uring_submit(&ring);
  
  // Wait for completions
  struct io_uring_cqe *cqe;
  io_uring_wait_cqe(&ring, &cqe);
  
  // Process all completed operations
  process_result(cqe->user_data, cqe->res);
  io_uring_cqe_seen(&ring, cqe);
}
```

**Advantages over epoll/kqueue:**
- **Batching**: Multiple I/O submissions in one syscall (lower syscall overhead)
- **Kernel-driven**: Kernel schedules I/O, not application re-trying
- **Proactor model**: Avoid ping-pong between app and kernel

**Emerging standard** on modern Linux. Adoption growing in web frameworks.

## Zero-Copy I/O & Sendfile

Naive server: read(2) → userspace buffer → write(2) → socket. Data copied multiple times.

**Zero-copy approaches:**

### sendfile(2)
Transfer file directly to socket without entering userspace:
```
sendfile(socket_fd, file_fd, offset, nbytes);
// Kernel: read file block → NIC buffer (no userspace copy)
```

### splice(2) (Linux)
General-purpose: splice bytes between FDs (pipes, sockets, files).

### Mmap + Write
Map file into address space; writes go to page cache → socket → NIC.

## Memory-Mapped I/O

Map a device (e.g., NIC hardware buffer, GPU VRAM) directly into process address space. Reads/writes directly interact with hardware:

```
// Map GPU VRAM
int fd = open("/dev/gpu0", O_RDWR);
uint32_t *gpu_mem = mmap(NULL, 256 MB, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0);

gpu_mem[0] = some_value;  // Write directly to GPU

// Or, kernel driver polls GPU registers mapped into address space
```

Used for high-performance scenarios: GPUs, FPGAs, real-time I/O.

## DMA (Direct Memory Access)

**DMA** allows I/O devices to read/write memory directly without CPU intervention:

```
Without DMA (CPU-mediated):
  NIC sends byte → CPU reads from NIC → writes to RAM  (CPU busy)

With DMA (device-mediated):
  CPU programs DMA: "Copy 4 KB from NIC to address X"
  NIC writes directly to RAM
  When done, NIC raises interrupt
  CPU handles interrupt (minimal work)
```

**Benefit**: CPU freed for other tasks while bulk data transfers occur. Essential for high-throughput I/O.

## See Also

- [Web Event Loop](web-event-loop.md) — JavaScript event loop (similar concepts, different context)
- [Architecture Event-Driven](architecture-event-driven.md) — Larger architectural patterns
- [Systems Reasoning](systems-reasoning.md) — Why syscall overhead matters