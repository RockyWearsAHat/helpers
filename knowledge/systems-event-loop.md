# Systems: Event Loop — Node.js, Browser, and OS-Level Mechanics

The **event loop** is the central mechanism through which event-driven runtimes (Node.js, browsers) coordinate asynchronous I/O, timers, callbacks, and rendering without blocking. Understanding its internals—phases, queue priorities, and OS integration—is essential for debugging performance issues, avoiding race conditions, and reasoning about execution order.

## Overview: Single-Threaded, Non-Blocking Concurrency

An event loop is a loop that repeatedly:
1. Checks for ready events (I/O completion, timers, callbacks)
2. Invokes handlers for those events
3. Processes any generated work (new I/O requests, callbacks)
4. Sleeps until the next event or timeout

This allows a single thread to appear to handle many concurrent operations by interleaving their execution.

## Node.js Event Loop (libuv)

Node.js delegates its event loop to **libuv**, a cross-platform C library that abstracts OS event mechanisms (epoll on Linux, kqueue on macOS/BSD, IOCP on Windows). The Node.js event loop runs in **phases**, each handling a specific category of callbacks:

### The Six Phases

1. **timers**: Execute callbacks scheduled by `setTimeout` and `setInterval` whose timers have expired
2. **pending callbacks**: Execute I/O callbacks deferred from the previous cycle (e.g., connection errors, TCP write errors)
3. **idle/prepare**: Internal to libuv; execute idle handles and prepare handles
4. **poll**: Retrieve new I/O events from the OS; execute most I/O callbacks (file read/write, network operations). This phase blocks waiting for OS events if no timers or setImmediate are pending
5. **check**: Execute callbacks scheduled by `setImmediate`
6. **close callbacks**: Execute close handlers for closed sockets/file descriptors

After each phase, microtasks (promises, `process.nextTick`) are drained before moving to the next phase.

### Microtask Queue vs. Macrotask Queue

- **Microtasks**: `process.nextTick`, Promise callbacks, `queueMicrotask`. Drained completely after every phase, before moving to the next phase
- **Macrotasks**: setTimeout, setInterval, setImmediate, I/O, file operations. One macrotask per phase

```
┌─────────────────────────────────────────────┐
│         Timers Phase (setTimeout)           │
│  [drain microtasks after]                   │
├─────────────────────────────────────────────┤
│    Pending Callbacks (deferred I/O)         │
│  [drain microtasks after]                   │
├─────────────────────────────────────────────┤
│         Poll Phase (I/O operations)         │
│  [blocks if no work, or until I/O ready]    │
│  [drain microtasks after]                   │
├─────────────────────────────────────────────┤
│    Check Phase (setImmediate)               │
│  [drain microtasks after]                   │
├─────────────────────────────────────────────┤
│      Close Callbacks                        │
│  [drain microtasks after]                   │
└─────────────────────────────────────────────┘
```

### I/O Polling and Blocking

The **poll phase** is where most I/O happens. If no events are ready, libuv calls `epoll_wait` (Linux), `kevent` (macOS), or `GetQueuedCompletionStatus` (Windows) with a timeout. The timeout is determined by:
- If timers are pending: timeout = milliseconds until next timer fires
- If no timers but `setImmediate` is queued: timeout = 0 (don't block, check immediately)
- If neither: timeout is indefinite (block until I/O event arrives)

This prevents busy-waiting and allows the OS to schedule other processes.

## Browser Event Loop

Browsers also use an event loop, but with different phases and closer coordination with rendering:

### Browser Event Loop Phases

1. **Macrotask (Task) Queue**: One task per iteration. Examples: setTimeout, setInterval, XMLHttpRequest, user events (click, scroll)
2. **Microtask Queue**: All microtasks drained after each macrotask. Examples: Promise callbacks, MutationObserver, `queueMicrotask`
3. **Rendering**: Update the DOM based on accumulated changes. Only if visual updates have occurred and there is time before the next frame deadline

```
┌──────────────────────────────────┐
│  Execute One Macrotask           │
├──────────────────────────────────┤
│  Drain All Microtasks            │
├──────────────────────────────────┤
│  Rendering (if needed for 60fps) │
│  - Paint                         │
│  - Composite                     │
└──────────────────────────────────┘
```

### Key Difference: Rendering Integration

Unlike Node.js, the browser event loop coordinates with rendering. If a microtask takes too long, it delays rendering and can cause jank (dropped frames). This is why expensive operations should be split into smaller chunks or moved to Web Workers.

### Frame Deadline

The browser targets 60 FPS (16.67ms per frame). If microtasks consume all available time, rendering is skipped for that frame. `requestAnimationFrame` callbacks are scheduled before rendering to allow animation updates.

## libuv Internals: OS Integration

### Platform Abstractions

libuv abstracts platform-specific I/O mechanisms:

| Platform | Mechanism     | Strength                 | Weakness                  |
|----------|---------------|--------------------------|---------------------------|
| Linux    | epoll         | Scales to millions of fds | Requires registration     |
| macOS    | kqueue        | Scalable, flexible       | Per-fd context (expensive)|
| Windows  | IOCP          | Async-friendly           | Complex API               |

### epoll (Linux)

```c
epoll_fd = epoll_create();  // Create epoll instance
epoll_ctl(epoll_fd, EPOLL_CTL_ADD, fd, ...);  // Register fd
epoll_wait(epoll_fd, events, maxevents, timeout);  // Wait for events
```

epoll uses an internal red-black tree to track registered file descriptors and a ready list to report events. It's efficient for thousands of connections.

### kqueue (macOS)

kqueue is similar to epoll but with some advantages:
- Single API for all event types (files, sockets, timers, signals, processes)
- Can set complex filters and event transformations
- Better integration with OS scheduler

### IOCP (Windows)

IOCP (I/O Completion Port) is fundamentally different:
- Callbacks are invoked when I/O completes (proactive)
- Integrates thread pools for efficient CPU utilization
- libuv-on-Windows uses thread pools to simulate Unix-like event handling

## Execution Order Guarantees

Understanding the order of execution is crucial for debugging:

```javascript
// Execution order example:
console.log('1: start');

setTimeout(() => console.log('2: setTimeout'));
setImmediate(() => console.log('3: setImmediate'));
Promise.resolve().then(() => console.log('4: promise'));
process.nextTick(() => console.log('5: nextTick'));

console.log('6: sync');

// Output:
// 1: start
// 6: sync
// 5: nextTick
// 4: promise
// [poll phase has nothing to do]
// 3: setImmediate
// [next iteration: timers phase]
// 2: setTimeout
```

## Performance Implications

1. **Long Microtask Queues**: If promises or nextTick callbacks accumulate, they block the poll phase, starving I/O handlers. On the browser, they block rendering.
2. **Blocking Poll Phase**: Synchronous operations in I/O handlers block all other I/O. Use `async`/`await` or split work.
3. **Timer Precision**: Timers are not guaranteed to fire at their specified time; they fire as soon as the timers phase runs. Heavy CPU use can cause delays.
4. **setImmediate vs. setTimeout(0)**: `setImmediate` runs in the check phase (after poll), while `setTimeout(0)` runs in the next iteration's timers phase. `setImmediate` is faster and preferred for deferring work.

## Related Concepts

- See [concurrency-patterns.md](concurrency-patterns.md) for event-driven architecture patterns
- See [os-io-models.md](os-io-models.md) for reactor/proactor patterns and I/O multiplexing details
- See [paradigm-concurrent-models.md](paradigm-concurrent-models.md) for how event loops compare to threads and actors
- See [language-javascript-runtime.md](language-javascript-runtime.md) for V8 heap and GC interaction with the event loop