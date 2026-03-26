# Systems: I/O Models — Blocking, Async, and Multiplexing Patterns

**I/O models** describe how programs coordinate with operating systems to perform I/O (reading/writing sockets, files, devices). Different models trade off programming complexity, throughput, latency, and resource usage. The journey from synchronous blocking to modern asynchronous I/O reflects decades of solving the **C10K problem**: how to efficiently handle 10,000+ concurrent connections.

## The Core Challenge

When a program performs an I/O operation (e.g., `read()` from a socket), the kernel must wait for the device to be ready. During this wait:
- **Blocking I/O**: The calling thread sleeps; the kernel can schedule other processes
- **Non-blocking I/O**: Control returns immediately; the thread must check status later
- **Async I/O**: The kernel invokes a callback when the operation completes

Each approach has implications for concurrency, latency, and resource usage.

## Five I/O Models

### 1. Blocking I/O (Synchronous)

```
┌─────────────────────────────────────────┐
│ read() called on socket                 │
│ Thread sleeps; kernel polls device      │
│ Data arrives → kernel copies to buffer  │
│ read() returns; thread wakes            │
└─────────────────────────────────────────┘
```

**Characteristics:**
- Simplest to program: one thread, one connection
- Inefficient for many concurrent connections: requires one thread per connection (memory overhead, scheduling cost)
- Used in: Classic web servers (Apache with prefork), many educational examples

**Limitation:** To handle N connections, you need N threads. With N=10,000, you have 10,000 stack frames (8MB each = 80GB RAM), plus scheduler overhead.

### 2. Non-Blocking I/O

```
┌─────────────────────────────────────────┐
│ fcntl(fd, O_NONBLOCK)                   │
│ read() called; no data available        │
│ read() returns -1, errno=EAGAIN         │
│ Caller must retry later                 │
└─────────────────────────────────────────┘
```

**Characteristics:**
- Caller must poll repeatedly or use multiplexing to check readiness
- Single thread can handle many connections
- Requires state management and higher CPU usage (busy-waiting without multiplexing)

**Problem:** Calling `read()` in a tight loop to check 10,000 sockets wastes CPU cycles.

### 3. I/O Multiplexing (Reactor Pattern)

Multiple models exist:

#### **select()**
```c
fd_set readfds;
FD_ZERO(&readfds);
FD_SET(sock1, &readfds);
FD_SET(sock2, &readfds);
select(nfds, &readfds, NULL, NULL, timeout);
// Returns when ANY socket is ready
for (int i = 0; i < nfds; i++) {
    if (FD_ISSET(i, &readfds)) {
        // fd i is ready; read it
    }
}
```

**Limitations:**
- FD set is a bitmap; limited to ~1024 file descriptors on most systems
- O(n) scan of all fds even if only one is ready

#### **poll()**
```c
struct pollfd fds[10000];
for (int i = 0; i < 10000; i++) fds[i].fd = sockets[i];
poll(fds, 10000, timeout);
for (int i = 0; i < 10000; i++) {
    if (fds[i].revents & POLLIN) {
        // fds[i] is ready
    }
}
```

**Advantages over select():**
- No FD limit (except system limit)
- Portable

**Disadvantage:**
- Still O(n) to scan results

#### **epoll() (Linux)**
```c
int epfd = epoll_create1(0);
struct epoll_event ev = {.events = EPOLLIN, .data.fd = sock};
epoll_ctl(epfd, EPOLL_CTL_ADD, sock, &ev);
int nready = epoll_wait(epfd, events, maxevents, timeout);
// events contains only READY fds
for (int i = 0; i < nready; i++) {
    int fd = events[i].data.fd;
    // fd is guaranteed ready; read it
}
```

**Advantages:**
- O(k) where k = number of ready fds (not total fds)
- No FD limit
- **Level-triggered** (default): returns every time fd is ready
- **Edge-triggered**: returns only on state transitions (more complex, but better for batch processing)

#### **kqueue() (BSD/macOS)**
```c
int kq = kqueue();
struct kevent kev = {.ident = sock, .filter = EVFILT_READ, .flags = EV_ADD};
kevent(kq, &kev, 1, events, maxevents, NULL);
// Single API for sockets, timers, signals, child processes, file changes
```

**Advantages:**
- Unified event source (sockets, timers, signals, fs changes)
- Edge-triggered by default
- Fewer syscalls

#### **IOCP (Windows)**
```c
HANDLE iocp = CreateIoCompletionPort(...);
GetQueuedCompletionStatus(iocp, &BytesTransferred, &CompletionKey, &Overlapped, timeout);
```

**Model:** Proactive completion-based (see below), not polling.

### 4. Signal-Driven I/O

```c
signal(SIGIO, handler);
fcntl(fd, F_SETOWN, getpid());
fcntl(fd, F_SETFL, O_ASYNC);
```

When data arrives, the kernel sends SIGIO to the process. The signal handler can read the data.

**Disadvantages:**
- Signals are low-priority, unreliable for high throughput
- Limited information in signal handler
- Rarely used in practice

### 5. Asynchronous I/O (Proactive Model)

Instead of asking "is the socket ready?", the program says "read this socket; call me when done."

#### **io_uring (Linux 5.1+)**
```c
struct io_uring ring;
io_uring_queue_init(QUEUE_DEPTH, &ring, 0);

// Submit: read from fd into buffer
struct io_uring_sqe *sqe = io_uring_get_sqe(&ring);
io_uring_prep_read(sqe, fd, buffer, size, offset);
io_uring_submit(&ring, 1);

// Wait for completion
struct io_uring_cqe *cqe;
io_uring_wait_cqe(&ring, &cqe);
int result = cqe->res;  // bytes read or error
io_uring_cqe_seen(&ring, cqe);
```

**Advantages:**
- Zero-copy: kernel buffers directly to user-provided memory
- Batch submissions and completions (low syscall overhead)
- Works with all I/O types: sockets, files, pipes, timers
- Supports cancellation

#### **POSIX AIO (deprecated on Linux)**
```c
struct aiocb cb;
memset(&cb, 0, sizeof(cb));
cb.aio_fildes = fd;
cb.aio_buf = buffer;
cb.aio_nbytes = size;
aio_read(&cb);
// Later: aio_error(&cb), aio_return(&cb)
```

**Disadvantages:**
- Thread pool-based on many systems (no kernel support)
- Not scalable

## Architectural Patterns

### Reactor Pattern (Event-Driven)
- Single thread or event loop
- Multiplexing (epoll/kqueue) monitors many fds
- When event occurs, dispatch handler
- Examples: Node.js, nginx, libevent

### Proactor Pattern (Completion-Based)
- Kernel performs I/O operation
- Invokes callback when complete
- Examples: io_uring, IOCP, Boost.Asio

## Performance Comparison Matrix

| Model           | Concurrency | CPU Efficiency | Latency | Throughput | Complexity |
|-----------------|-------------|----------------|---------|-----------|-----------|
| Blocking        | Low (# threads) | Poor | Low | Medium | Low |
| Non-blocking    | High        | Poor (busy-wait) | Low | Medium | Medium |
| select/poll     | High        | Fair (O(n))    | Low | Medium | Medium |
| epoll/kqueue    | Very High   | Good (O(k))    | Low | High | Medium |
| io_uring        | Very High   | Excellent      | Very Low | Very High | High |
| Signal-driven   | High        | Fair           | Medium | Low | High |

## The C10K and C10M Problems

**C10K Problem (2003)**: How to efficiently handle 10,000 concurrent connections?
- Blocking I/O: impossible (10,000 threads)
- Answer: epoll/kqueue multiplexing

**C10M Problem (2014+)**: How to handle 10,000,000 connections?
- epoll/kqueue become bottlenecks due to syscall overhead
- Answer: io_uring (batching, zero-copy), kernel bypass (DPDK)

## Choice of I/O Model

1. **Blocking I/O**: Simple applications, low concurrency, legacy systems
2. **Multiplexing (epoll/kqueue)**: Modern servers, 1,000s of connections, cross-platform (use libraries like libevent, libev)
3. **io_uring**: Ultra-high throughput, extreme concurrency, Linux 5.1+
4. **Signal-driven**: Rarely (mostly historical interest)
5. **Asynchronous I/O (IOCP)**: Windows systems, strongly recommended

## Related Concepts

- See [systems-event-loop.md](systems-event-loop.md) for how event loops use multiplexing
- See [patterns-event-driven.md](patterns-event-driven.md) for reactor/proactor architecture patterns
- See [paradigm-concurrent-models.md](paradigm-concurrent-models.md) for how I/O multiplexing relates to threading and actors
- See [performance-profiling.md](performance-profiling.md) for measuring I/O performance