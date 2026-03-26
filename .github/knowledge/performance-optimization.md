# Performance Optimization

## Fundamental Rules

### Knuth's Law

"Premature optimization is the root of all evil" — Donald Knuth. The full quote adds: "Yet we should not pass up our opportunities in that critical 3%."

Translation: Write clear code first. Profile to find the actual bottleneck. Optimize only the measured hot spots. Most code doesn't need optimization.

### Measure, Don't Guess

Always profile before optimizing. Your intuition about what's slow is usually wrong.

- **CPU profiling**: Where is time being spent? (flame graphs, sampling profilers)
- **Memory profiling**: What's consuming RAM? Where are allocations?
- **I/O profiling**: Which queries, network calls, or disk reads are slowest?

Tools by language:

- **JavaScript**: Chrome DevTools, `node --prof`, clinic.js
- **Python**: cProfile, py-spy, memory_profiler, line_profiler
- **Java**: JFR (Flight Recorder), async-profiler, VisualVM
- **Go**: pprof (built-in), `go tool trace`
- **Rust**: flamegraph, cargo-bench, criterion
- **C/C++**: perf, Valgrind/Callgrind, gprof, VTune

## Algorithmic Complexity (Big O)

Choose the right algorithm and data structure first — this dominates all micro-optimizations.

| Operation       | Array  | Hash Map | Sorted Array | BST/Tree |
| --------------- | ------ | -------- | ------------ | -------- |
| Access by index | O(1)   | —        | O(1)         | O(log n) |
| Search          | O(n)   | O(1) avg | O(log n)     | O(log n) |
| Insert          | O(n)\* | O(1) avg | O(n)         | O(log n) |
| Delete          | O(n)   | O(1) avg | O(n)         | O(log n) |

\*O(1) amortized for append at end.

**Common complexity classes** (best to worst):
O(1) → O(log n) → O(n) → O(n log n) → O(n²) → O(2ⁿ) → O(n!)

**Red flags:**

- Nested loops over the same data → O(n²). Can often be replaced with hash maps for O(n).
- Repeated linear searches → Add an index or use a set.
- Sorting for every query → Sort once, binary search after.

## Memory Optimization

- **Reduce allocations**: Reuse objects, use object pools, prefer stack allocation.
- **Avoid memory leaks**: Close resources, remove event listeners, watch for circular references.
- **Use appropriate data structures**: `BitSet` over `Set<Integer>` for flags. Packed arrays over sparse structures.
- **Lazy loading**: Don't load data until it's needed.
- **Streaming**: Process large datasets in chunks, not all in memory.

## I/O Optimization

I/O (disk, network, database) is almost always the bottleneck in real applications.

- **Batch operations**: One query returning 100 rows beats 100 queries returning 1 row.
- **Connection pooling**: Reuse database/HTTP connections instead of creating new ones per request.
- **Async I/O**: Don't block threads waiting for I/O. Use async/await, event loops, non-blocking I/O.
- **Caching**: Cache at every level:
  - **L1**: In-memory (process-local) — fastest, limited size.
  - **L2**: Shared cache (Redis, Memcached) — fast, shared across processes.
  - **L3**: CDN, browser cache, HTTP cache headers — for static assets and API responses.
- **Compression**: Compress data in transit (gzip, brotli) and at rest. Trade CPU for bandwidth.

## Database Performance

- **Index the right columns**: Columns used in WHERE, JOIN, ORDER BY.
- **Avoid N+1 queries**: Load related data in one query (JOIN, eager loading). ORMs are notorious for this.
- **Use EXPLAIN/ANALYZE**: Read query execution plans to find full table scans.
- **Denormalize when justified**: Sometimes duplicating data avoids expensive JOINs. Measure first.
- **Connection pooling**: Always use a connection pool. Never open/close connections per query.
- **Pagination**: Never `SELECT *` without LIMIT. Use cursor-based pagination for large datasets.

## Frontend Performance

- **Minimize bundle size**: Tree-shaking, code splitting, lazy loading routes/components.
- **Optimize images**: WebP/AVIF, responsive `srcset`, lazy loading below the fold.
- **Reduce layout shifts**: Reserve space for dynamic content (CLS metric).
- **Cache aggressively**: Immutable assets with content hashes, long cache headers.
- **Minimize main thread work**: Move computation to Web Workers. Break up long tasks.

## General Optimization Patterns

1. **Do less work**: The fastest code is code that doesn't run. Skip unnecessary computation.
2. **Do it once**: Memoize/cache expensive computations. Precompute what can be precomputed.
3. **Do it later**: Defer non-critical work (lazy evaluation, background jobs, queues).
4. **Do it in bulk**: Batch I/O, batch API calls, batch database writes.
5. **Do it closer**: Move computation closer to the data (database stored procedures, edge computing, CDNs).

---

_Sources: Donald Knuth (Art of Computer Programming), Brendan Gregg (Systems Performance), Google Web Vitals, High Performance Browser Networking (Ilya Grigorik)_
