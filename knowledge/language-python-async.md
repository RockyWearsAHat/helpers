# Python Async: Asyncio, Event Loop, and Structured Concurrency

## Introduction

Python's `asyncio` module provides a single-threaded concurrency model based on an event loop and coroutines. Unlike Rust's Futures or Go's goroutines, Python async is simpler but less flexible: a single event loop runs in one thread, and truly parallel computation requires separate processes. Understanding the event loop, coroutines, and modern structured concurrency libraries is essential.

## The Asyncio Event Loop

The **event loop** is a single-threaded executor that repeatedly polls I/O and calls callbacks (or runs coroutines):

```python
import asyncio

async def main():
    await asyncio.sleep(1)
    print("done")

asyncio.run(main())
```

`asyncio.run()` is the high-level entry point (Python 3.7+). It creates an event loop, runs the coroutine, and closes the loop. Internally:

1. Loop waits for I/O events (using `select`, `epoll`, or `kqueue` on the OS)
2. When I/O ready or timer fires, the loop runs the corresponding callback  
3. Coroutines that `.await` an operation "yield" control back to the loop
4. Loop schedules the next coroutine

**Key property:** Only one coroutine runs at a time. True parallelism requires separate processes (via `multiprocessing`) or threading (via `concurrent.futures`). The GIL prevents multiple threads from running Python bytecode simultaneously.

## Coroutines and Tasks

A **coroutine** is an async function (defined with `async def`):

```python
async def fetch_data(url):
    # await something that yields control to the loop
    response = await http_client.get(url)
    return response.json()

coro = fetch_data("https://example.com")
# coro is not yet running — it's paused at the start
```

Calling an async function returns a coroutine object, not the result. To run it, you must `await` it or schedule it as a **Task**:

```python
# Method 1: await (must be in an async context)
result = await fetch_data(url)

# Method 2: create a task (fire and forget, collect results later)
task = asyncio.create_task(fetch_data(url))
result = await task  # Wait for completion

# Method 3: gather (wait for multiple)
results = await asyncio.gather(
    fetch_data(url1),
    fetch_data(url2),
    fetch_data(url3),
)
```

**Task** wraps a coroutine and schedules it on the event loop. Unlike `await`, creating a task doesn't block:

```python
async def main():
    # Start three tasks immediately (concurrent)
    t1 = asyncio.create_task(fetch_data(url1))
    t2 = asyncio.create_task(fetch_data(url2))
    t3 = asyncio.create_task(fetch_data(url3))
    
    # Wait for all three
    results = await asyncio.gather(t1, t2, t3)
```

## Gather and Wait

**`gather()`** runs coroutines concurrently and waits for all:

```python
results = await asyncio.gather(coro1, coro2, coro3, return_exceptions=False)
```

If `return_exceptions=True`, exceptions are returned as items; otherwise, the first exception is raised.

**`wait()`** returns `(done, pending)` sets, allowing partial collection:

```python
done, pending = await asyncio.wait(
    [coro1, coro2, coro3],
    return_when=asyncio.FIRST_COMPLETED  # or FIRST_EXCEPTION, ALL_COMPLETED
)
```

## Popular Async Libraries

### aiohttp

HTTP client/server for asyncio:

```python
import aiohttp

async def fetch(session, url):
    async with session.get(url) as response:
        return await response.json()

async def main():
    async with aiohttp.ClientSession() as session:
        data = await fetch(session, "https://api.example.com/data")
```

### httpx

Modern async HTTP client (also supports sync):

```python
import httpx

async def fetch():
    async with httpx.AsyncClient() as client:
        response = await client.get("https://api.example.com")
        return response.json()
```

## Structured Concurrency: Trio, AnyIO, TaskGroup

Standard asyncio lacks **structured concurrency** — no automatic cleanup or hierarchy of tasks. Modern libraries add nurseries:

### Trio

Purpose-built for structured concurrency:

```python
import trio

async def child_task(name):
    for i in range(3):
        print(f"{name}: {i}")
        await trio.sleep(0.1)

async def main():
    async with trio.open_nursery() as nursery:
        nursery.start_soon(child_task, "A")
        nursery.start_soon(child_task, "B")
    # Guaranteed all children finished here
```

A **nursery** ensures all spawned tasks complete before exiting the context. If a task raises, the nursery cancels others and re-raises. This prevents orphaned background tasks.

### AnyIO

Abstraction layer supporting both asyncio and Trio:

```python
import anyio

async def main():
    async with anyio.create_task_group() as tg:
        tg.start_soon(task1)
        tg.start_soon(task2)
    # All tasks complete before exit
```

`anyio` lets libraries work under both engines, useful for ecosystem independence.

### TaskGroup (Python 3.11+)

Built into asyncio:

```python
async def main():
    async with asyncio.TaskGroup() as tg:
        tg.create_task(task1())
        tg.create_task(task2())
    # All tasks complete here; exceptions collected
```

This brings Trio's nursery design to standard asyncio.

## Async Generators

Generators can be async, yielding values as they're produced:

```python
async def count_up(n):
    for i in range(n):
        yield i
        await asyncio.sleep(0.1)

async def main():
    async for i in count_up(5):
        print(i)
```

Useful for streaming data where each item requires async I/O.

## Async Context Managers

Resources can be asynchronously acquired/released:

```python
class AsyncResource:
    async def __aenter__(self):
        # Async setup
        print("acquiring")
        await asyncio.sleep(0.1)
        return self
    
    async def __aexit__(self, exc_type, exc, tb):
        # Async cleanup
        print("releasing")
        await asyncio.sleep(0.1)

async def main():
    async with AsyncResource() as res:
        print("using resource")
```

This is essential for async database connections, file handles, etc.

## GIL Interaction

The **GIL** (Global Interpreter Lock) in CPython allows only one thread to execute Python bytecode at a time. Asyncio single-threaded model avoids the GIL for concurrency but cannot exploit multiple cores:

- **Async I/O:** Works great (GIL released during I/O wait)
- **CPU-bound work:** Single-threaded, no speedup from asyncio

For CPU parallelism, use `multiprocessing` or `concurrent.futures`:

```python
import asyncio
from concurrent.futures import ProcessPoolExecutor

async def main():
    loop = asyncio.get_event_loop()
    with ProcessPoolExecutor() as executor:
        result = await loop.run_in_executor(executor, cpu_intensive_func)
```

## Uvloop

Drop-in replacement for asyncio's event loop, written in Cython, 2-4x faster:

```python
import uvloop

asyncio.set_event_loop_policy(uvloop.EventLoopPolicy())

# Now asyncio.run() uses uvloop under the hood
```

No API changes; use it for performance. Increasingly common in production.

## Event Loop Management

```python
# Default: run one async function
asyncio.run(main())  # Creates, runs, closes loop

# Advanced: reuse loop
loop = asyncio.new_event_loop()
asyncio.set_event_loop(loop)
task = loop.create_task(main())
loop.run_until_complete(task)
loop.close()
```

In most code, `asyncio.run()` is sufficient. Manual loop management is for frameworks or long-lived servers.

## Common Pitfalls

- **Forgetting `await`:** Calling an async function without `await` schedules nothing; the coroutine object sits unused
- **Blocking the loop:** CPU-heavy or blocking I/O (e.g., `requests.get()` instead of `aiohttp`) stalls all other tasks
- **Shared state without sync:** Modification without a Lock causes races (though GIL provides some safety)
- **Callback spaghetti:** Using `.add_done_callback()` instead of `await` makes code hard to follow
- **Orphaned tasks:** Starting tasks without awaiting/gathering them; they may be cancelled on loop shutdown

## See Also

- **concurrency-patterns** — general async patterns (timeouts, retries)
- **web-event-loop** — event loop architecture across languages
- **language-python** — Python idioms and conventions