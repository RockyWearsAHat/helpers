# Systems Reasoning — How Everything Connects

Software doesn't exist in isolation. It runs on hardware, communicates over networks, serves humans, and interacts with other software. Understanding these layers — how they work, how they fail, how they interact — is what separates script writers from systems thinkers.

---

## How Computers Actually Work

### The Execution Model

Every program, no matter how abstract, eventually becomes:

```
Fetch instruction → Decode → Execute → Write Result → Repeat

This happens billions of times per second on a modern CPU.
A 4 GHz processor does 4 billion cycles per second.
Most instructions take 1-4 cycles.
```

**Why this matters for programmers:**

- Branch misprediction (CPU guessed wrong about an `if` statement) costs ~15 cycles. That's why sorted data can be faster to process — branches become predictable.
- Cache misses (data not in L1/L2/L3 cache) cost 100-300 cycles. That's why iterating through an array is 100x faster than chasing linked list pointers through scattered memory.
- Context switches (OS switches between threads) cost thousands of cycles plus cache pollution. That's why async/event-driven is faster than thread-per-request for I/O workloads.

### Memory Hierarchy — The Speed Pyramid

```
            ┌──────────┐
            │ Registers │  < 1ns, ~1KB
            ├──────────┤
            │  L1 Cache │  ~1ns, 32-64KB per core
            ├──────────┤
            │  L2 Cache │  ~4ns, 256KB-1MB per core
            ├──────────┤
            │  L3 Cache │  ~10ns, 8-64MB shared
            ├──────────┤
            │    RAM    │  ~100ns, 16-512GB
            ├──────────┤
            │    SSD    │  ~100μs, 1-8TB
            ├──────────┤
            │    HDD    │  ~10ms, 1-20TB
            ├──────────┤
            │  Network  │  ~1-150ms, unlimited
            └──────────┘
```

Each level is roughly 10-100x slower than the one above it. Programs that keep their working data in cache are orders of magnitude faster. This isn't a micro-optimization — it's a fundamental design consideration.

**Practical implications:**

```c
// SLOW: Column-major traversal (cache-hostile)
for (int col = 0; col < N; col++)
    for (int row = 0; row < N; row++)
        sum += matrix[row][col];  // jumps N*sizeof(int) bytes each iteration

// FAST: Row-major traversal (cache-friendly)
for (int row = 0; row < N; row++)
    for (int col = 0; col < N; col++)
        sum += matrix[row][col];  // sequential memory access, hardware prefetcher loves this

// For N=10000, the fast version can be 10-50x faster. Same algorithm, same result,
// just different memory access pattern.
```

### Virtual Memory — The Great Illusion

Every process thinks it has the entire 64-bit address space to itself. The OS + hardware maintain this illusion.

```
Process A sees:     Physical RAM:        Disk:
┌────────────┐     ┌──────────────┐     ┌──────────┐
│ 0x00000000 │ ──► │ Frame 47     │     │          │
│ 0x00001000 │ ──► │ Frame 12     │     │          │
│ 0x00002000 │ ──────────────────────► │ Swap page │
│ 0x00003000 │ ──► │ Frame 88     │     │          │
└────────────┘     └──────────────┘     └──────────┘

Page table translates virtual → physical.
TLB (Translation Lookaside Buffer) caches recent translations.
Page fault: page not in RAM → OS loads it from disk (SLOW: ~10ms)
```

**Why this matters:**

- `malloc`/`new` doesn't allocate physical memory. It reserves virtual address space. Physical pages are allocated on first access (lazy allocation).
- Memory-mapped files: the OS treats a file as if it's memory. Read/write to memory, OS handles the I/O. How databases and large file processing work.
- When your process uses "4GB of memory" but only accesses 500MB, that's fine. Only accessed pages use physical RAM.
- Thrashing: when working set > physical RAM, pages constantly swap in/out. System grinds to a halt. The fix is more RAM or less working set.

---

## How the Network Actually Works

### The Request Journey (What Happens When You Type a URL)

```
1. Browser parses URL
2. DNS lookup: domain → IP address
   (Check: browser cache → OS cache → router → ISP DNS → recursive resolution)
3. TCP handshake: SYN → SYN-ACK → ACK (1.5 round trips)
4. TLS handshake: 1-2 more round trips (key exchange, certificate verification)
5. HTTP request sent
6. Server processes request
7. HTTP response sent
8. Browser renders content
```

**Total latency:** DNS (0-100ms) + TCP (1 RTT ≈ 20-150ms) + TLS (1-2 RTT) + Server processing + Transfer time.

For a server 100ms away: minimum ~350ms before the first byte of the response arrives. That's why CDNs exist (move content closer to users).

### TCP vs UDP — When to Use Which

```
TCP:  "I'll make sure every byte arrives, in order, exactly once."
      How: Sequence numbers, acknowledgments, retransmissions, flow control.
      Cost: Connection setup (handshake), head-of-line blocking, overhead.
      Use for: HTTP, database connections, file transfer, email — anything
               where correctness matters more than latency.

UDP:  "Here's a packet. Good luck."
      How: Just sends it. No connection, no ordering, no reliability.
      Cost: Packets can be lost, duplicated, reordered.
      Use for: DNS, video streaming, gaming, VoIP — anything where
               a dropped packet is better than a delayed packet.

QUIC: "UDP's speed + TCP's reliability, with built-in encryption."
      How: Multiplexed streams over UDP, no head-of-line blocking.
      Used by: HTTP/3, Google services. The future of web transport.
```

### HTTP Versions — Why We Keep Making New Ones

```
HTTP/1.1: One request per TCP connection (or pipelining, which nobody uses).
          Workaround: Open 6 connections per domain.
          Problem: Head-of-line blocking, connection overhead.

HTTP/2:   Multiplexed streams over ONE TCP connection.
          Binary framing, header compression (HPACK).
          Problem: TCP head-of-line blocking (one lost packet blocks all streams).

HTTP/3:   HTTP/2 but over QUIC (UDP-based).
          No head-of-line blocking (streams are independent).
          0-RTT connection resume for repeat visits.
          The current state of the art.
```

### DNS — The Internet's Phone Book

```
You:         "What's the IP for api.example.com?"
Resolver:    Checks cache. Miss.
Root (.):    "Ask .com nameserver"
.com NS:     "Ask example.com nameserver"
example.com: "api.example.com → 93.184.216.34, TTL=300 seconds"

Total: 4 round trips worst case. Usually cached after first lookup.
TTL (Time To Live): How long the answer is cached. Lower = faster propagation of changes.
                    Higher = less DNS traffic.
```

**DNS failures cascade.** If DNS is down, your app can't find ANY servers. It's the most critical piece of internet infrastructure that most people ignore.

---

## How Operating Systems Work (What You Need to Know)

### Processes vs Threads

```
PROCESS:  Own memory space, own file descriptors, isolated.
          Creating: expensive (fork → copy-on-write).
          Crashing: doesn't affect other processes.
          Communication: IPC (pipes, sockets, shared memory) — explicit.

THREAD:   Shared memory space within a process.
          Creating: cheap (just a new stack + register set).
          Crashing: takes down the entire process.
          Communication: Shared variables — implicit (and dangerous).

COROUTINE/GREEN THREAD:
          Scheduled by the runtime, not the OS.
          Even cheaper than threads (KB-sized stacks vs MB).
          Can't use multiple CPU cores directly (need OS threads for that).
          Examples: Go goroutines, Python asyncio tasks, Erlang processes.
```

**The GIL problem (Python, Ruby):** The Global Interpreter Lock allows only one thread to execute Python bytecode at a time. Threads still help for I/O-bound work (waiting for network/disk), but CPU-bound work needs multiprocessing.

### File I/O — Everyone Gets This Wrong

```
write() doesn't write to disk. It writes to the kernel's page cache.
The data is in RAM. The kernel will flush it to disk "eventually."

If the power goes out before the flush → data loss.

fsync() forces kernel to flush to disk. Expensive but safe.
  Databases: write-ahead log (WAL) → fsync → then acknowledge write.
  This is why databases are slow for writes — they MUST fsync.

O_DIRECT: Bypass the page cache entirely. You manage your own caching.
  Used by databases that have their own buffer pool (PostgreSQL, MySQL).
```

**Buffered vs unbuffered I/O:**

```python
# Unbuffered: one syscall per write (slow)
for line in million_lines:
    os.write(fd, line.encode())  # 1,000,000 system calls

# Buffered: accumulate, write in chunks (fast)
with open('output.txt', 'w') as f:
    for line in million_lines:
        f.write(line)  # writes to buffer, flushes when buffer full
# ~1,000 system calls (buffer flushes every ~1000 lines)
```

System calls are expensive (~1μs each). Batching I/O reduces syscall overhead by 100-1000x.

### Signals and Process Lifecycle

```
SIGTERM (15): "Please shut down gracefully."
              Default: terminate. You should catch this and clean up.
              Docker sends this first. Kubernetes sends this first.

SIGKILL (9):  "Die immediately. No cleanup. No catching."
              Sent after SIGTERM timeout. Uncatchable.

SIGHUP (1):   "Your terminal disconnected."
              Often repurposed to mean "reload configuration."

SIGINT (2):   "Ctrl+C was pressed."
              Default: terminate. Catch for graceful shutdown.
```

**Graceful shutdown pattern:**

```python
import signal, sys

def shutdown(signum, frame):
    print("Shutting down gracefully...")
    # Stop accepting new requests
    # Finish in-flight requests
    # Close database connections
    # Flush logs
    sys.exit(0)

signal.signal(signal.SIGTERM, shutdown)
signal.signal(signal.SIGINT, shutdown)
```

---

## How Databases Actually Work

### The B-Tree — Why Databases Are Fast

Every relational database uses B-trees (or B+ trees) for indexes. Understanding them explains 90% of database performance.

```
B-tree of order 4 (max 4 keys per node):

              [20, 40, 60]
             /    |     |    \
    [5, 10, 15] [25, 30, 35] [45, 50, 55] [65, 70, 75, 80]

Lookup for 50:
  Root: 40 < 50 < 60 → third child
  Leaf: [45, 50, 55] → found in 2 comparisons

For 1 million rows: B-tree depth ≈ 3-4 levels
  → 3-4 disk reads to find any row. Each read ~10ms HDD, ~0.1ms SSD.
  → Full table scan: 1,000,000 reads. B-tree: 3-4 reads.
```

**Why B-trees, not binary trees?** Disk reads are slow but read entire blocks (4KB). A B-tree node fills one block with dozens of keys. Binary tree nodes hold one key per block. B-trees minimize disk reads.

### The Write-Ahead Log (WAL) — How Databases Survive Crashes

```
Every write (INSERT, UPDATE, DELETE):
1. Write the change to the WAL (sequential write, fast)
2. fsync the WAL (ensure it's on disk)
3. Acknowledge the write to the client
4. Eventually, apply the change to the actual data pages (background)

If crash happens after step 2:
  → On restart, replay WAL to recover uncommitted changes.
  → No data loss, even mid-write.
```

This is why database writes are fast despite durability guarantees: sequential WAL writes (~0.1ms) instead of random data page writes (~10ms).

### MVCC — How Databases Handle Concurrent Reads and Writes

```
Transaction A:  UPDATE balance SET amount = 200 WHERE id = 1
Transaction B:  SELECT amount FROM balance WHERE id = 1

Without MVCC: B waits for A to commit (locks)
With MVCC:    B sees the OLD version (100), A writes the NEW version (200)
              Multiple versions of the same row exist simultaneously.
              Each transaction sees a consistent snapshot.

Cleanup: Old versions are garbage collected when no transaction can see them.
```

PostgreSQL, MySQL (InnoDB), Oracle all use MVCC. It's why reads don't block writes and writes don't block reads.

### Query Optimization — Why Your Query Is Slow

```sql
-- SLOW: Full table scan (reads every row)
SELECT * FROM orders WHERE customer_email = 'alex@example.com';

-- If orders has 10 million rows: scans all 10 million.
-- Add an index:
CREATE INDEX idx_orders_email ON orders(customer_email);
-- Now: B-tree lookup → 3-4 disk reads → microseconds instead of seconds.

-- SLOW: Index exists but not used
SELECT * FROM orders WHERE LOWER(customer_email) = 'alex@example.com';
-- LOWER() wraps the column → can't use the index.
-- Fix: CREATE INDEX idx_orders_email_lower ON orders(LOWER(customer_email));
-- Or: Store emails lowercase and search lowercase.
```

**The EXPLAIN command is your best friend:**

```sql
EXPLAIN ANALYZE SELECT * FROM orders WHERE customer_email = 'alex@example.com';

-- Shows: Seq Scan vs Index Scan, estimated vs actual rows, time per operation.
-- If you see "Seq Scan" on a large table → missing index or index not usable.
```

---

## How Authentication and Security Actually Work

### Password Hashing — Why bcrypt, Not SHA-256

```
SHA-256:  Designed to be FAST (hash billions per second on a GPU)
          Attacker can brute-force 10 billion passwords/second.
          For a 6-character password: cracked in < 1 second.

bcrypt:   Designed to be SLOW (configurable work factor)
          At cost factor 12: ~250ms per hash.
          Attacker can try ~4 passwords/second.
          For a 6-character password: takes centuries.
```

**The rule:** Use bcrypt, scrypt, or argon2 for passwords. NEVER use MD5, SHA-1, or SHA-256 for passwords. These are hash functions, not password storage functions.

### JWT — How Stateless Auth Works (and Its Traps)

```
Header:  { "alg": "HS256", "typ": "JWT" }
Payload: { "sub": "user123", "role": "admin", "exp": 1735689600 }
Signature: HMAC-SHA256(base64(header) + "." + base64(payload), secret_key)

Token: header.payload.signature (base64-encoded, NOT encrypted)
```

**Critical understanding:**

- JWTs are **signed, not encrypted.** Anyone can read the payload. Don't put secrets in it.
- JWTs are **stateless** — the server doesn't store sessions. The token itself IS the proof of identity.
- **Revocation is hard.** You can't "log out" a JWT without a blocklist (which reintroduces state). Set short expiry (15-60 min) and use refresh tokens.

### HTTPS / TLS — How Encryption Works in Practice

```
Client → Server: "Hello, I support these cipher suites"
Server → Client: "Let's use TLS 1.3 with AES-256-GCM. Here's my certificate."
Client:          Verifies certificate chain (trusts the Certificate Authority?)
                 Key exchange: Ephemeral Diffie-Hellman → shared secret
Both:            Derive session keys from shared secret
                 All further communication is encrypted with AES-256-GCM
```

**Why HTTPS is non-negotiable:**

- Without it: anyone on the network can read all data (passwords, cookies, PII)
- Without it: anyone on the network can modify data in transit (inject malware)
- Certificate pinning: your app trusts ONLY specific certificates (prevents rogue CA attacks)
- HSTS header: tells browsers "never use HTTP for this domain, even if the user types http://"

---

## How Containers and Orchestration Work

### Containers Are Not VMs

```
Virtual Machine:                     Container:
┌─────────────────────┐              ┌──────────────────┐
│    Your App         │              │    Your App      │
│    App Dependencies │              │    App Deps      │
│    Guest OS (Linux) │              │    (no OS!)      │
│    Hypervisor       │              │    Container RT  │
│    Host OS          │              │    Host OS       │
│    Hardware         │              │    Hardware      │
└─────────────────────┘              └──────────────────┘
   Startup: minutes                     Startup: seconds
   Size: GBs                            Size: MBs
   Isolation: strong (hardware)          Isolation: process-level
```

Containers use **Linux cgroups** (resource limits) and **namespaces** (isolation) to run processes in isolation WITHOUT a full OS. Every container shares the host kernel.

**Why this matters:**

- Containers start in seconds (no OS boot)
- Containers use less memory (no duplicate OS)
- Containers are reproducible (same image = same behavior everywhere)
- Containers are NOT secure isolation against malicious code (shared kernel = potential escape)

### Docker Image Layers — Why Build Order Matters

```dockerfile
# BAD: Any code change invalidates npm install cache
COPY . /app
RUN npm install

# GOOD: Dependencies cached unless package.json changes
COPY package.json package-lock.json /app/
RUN npm install
COPY . /app

# Each instruction creates a layer. Layers are cached.
# If package.json hasn't changed → npm install uses cache.
# Only the COPY . /app layer rebuilds on code changes.
```

This can reduce build times from minutes to seconds.

---

## How the Web Works (The Full Stack)

### Client-Server Communication Patterns

```
REST:       Request → Response. Stateless. HTTP methods (GET/POST/PUT/DELETE).
            Best for: CRUD operations, public APIs, simple interactions.

GraphQL:    Single endpoint. Client specifies exactly what data it needs.
            Best for: Complex data requirements, mobile apps (minimize data transfer).

WebSocket:  Bidirectional persistent connection. Server can push to client.
            Best for: Chat, real-time dashboards, collaborative editing, gaming.

SSE:        Server → Client only (one-directional). Simple, uses HTTP.
            Best for: Live feeds, notifications, progress updates.

gRPC:       Binary protocol (protobuf), HTTP/2, strongly typed.
            Best for: Service-to-service communication, low latency, streaming.
```

### The Browser Rendering Pipeline

```
HTML → DOM Tree
CSS  → CSSOM
         ↘
      Render Tree → Layout → Paint → Composite
         ↗
JavaScript (can modify DOM/CSSOM at any point)

Reflow (Layout):  Cost HIGH — triggered by changing size/position
Repaint:          Cost MEDIUM — triggered by changing color/visibility
Composite:        Cost LOW — triggered by transforms/opacity

Performance rule: Animate with transform and opacity (composite-only).
                  Avoid animating width, height, top, left (trigger reflow).
```

### Authentication Flows

```
Session-based:
  Login → Server creates session → Sends session cookie → Browser sends cookie with every request
  State: on server (Redis/DB)
  Logout: delete session on server

Token-based (JWT):
  Login → Server creates JWT → Client stores in memory/localStorage → Sends in Authorization header
  State: in the token itself
  Logout: client deletes token (server can't revoke easily)

OAuth 2.0 (delegated auth):
  User → Your App → Redirect to Google → User logs in → Google redirects back with code
  Your App → Exchanges code for access token → Uses token to call Google API on behalf of user
  You never see the user's Google password.
```

---

## How Version Control Actually Works

### Git Object Model — Why Git Is Fast

```
Every file is stored as a BLOB (content-addressed by SHA-1 hash).
Every directory listing is a TREE (maps names → blob/tree hashes).
Every commit is a COMMIT object (points to tree + parent commits + metadata).

Commit A (abc123)
  ├── tree (root directory)
  │   ├── blob → README.md (content: "Hello")
  │   └── tree → src/
  │       ├── blob → main.py (content: "print('hi')")
  │       └── blob → utils.py (content: "def helper():")
  └── parent → (none, first commit)

Commit B (def456)
  ├── tree (root directory)
  │   ├── blob → README.md (SAME hash → SAME blob, no duplication!)
  │   └── tree → src/
  │       ├── blob → main.py (NEW hash → new blob)
  │       └── blob → utils.py (SAME hash → no duplication)
  └── parent → Commit A (abc123)
```

**Key insight:** Git stores snapshots, not diffs. But identical files are automatically deduplicated because content-addressable storage means identical content = identical hash = same blob. This is why Git is both fast and space-efficient.

### Branches Are Pointers, Not Copies

```
A branch is a 41-byte file containing a commit hash. That's it.
Creating a branch: write one hash to one file. Instant.
Switching branches: update HEAD to point to new branch. Update working directory.

main →  C1 ← C2 ← C3
                     ↑
                   feature → C4 ← C5

"main" is a file containing C3's hash.
"feature" is a file containing C5's hash.
HEAD is a file containing "ref: refs/heads/feature".
```

---

## Emergent Behavior and Systems Thinking

### Complex Systems Fail in Complex Ways

Simple components interacting can produce surprising behavior:

```
- Each server individually handles 1000 req/s fine.
- Put 3 behind a load balancer: works great.
- One server gets slow (GC pause, disk I/O).
- Load balancer keeps sending requests to slow server.
- Requests queue up. Timeout. Retry.
- Retries double the load on the remaining 2 servers.
- They start getting slow too.
- More retries. More timeouts. Cascade failure.
- Everything is down.

No single component "broke." The failure is an EMERGENT PROPERTY
of the interaction between components.
```

**Defense:** Circuit breakers, bulkheads, backpressure, retry budgets, timeouts. These are not optional features — they're structural requirements for any distributed system.

### Feedback Loops

**Positive feedback (amplifying):** More users → more content → more users (network effects). Also: more load → more retries → more load (retry storm). Can be good or bad.

**Negative feedback (stabilizing):** Auto-scaling: more load → more servers → load per server decreases. TCP congestion control: packet loss → reduce send rate → less congestion.

**Engineering lesson:** Build negative feedback loops into your systems. Auto-scaling, rate limiting, backpressure, and circuit breakers are all negative feedback mechanisms that keep the system stable.

### The Law of Leaky Abstractions (Revisited)

Every abstraction hides complexity. When things work, you don't need to know the hidden details. When things break, you absolutely do.

```
Your code uses:    But breaks when:           Because underneath:
HTTP client        Requests take 30s          TCP retransmission + DNS timeout
ORM queries        JOIN takes 10 minutes      Missing index, full table scan
Docker container   Works locally, fails in CI Different base image, different libc
Async/await        Deadlock                   Thread pool exhaustion
Cloud function     Random timeouts            Cold start, memory limit, container recycling
```

**The rule:** Learn one layer below your daily abstraction. Web developer? Learn HTTP internals. Backend developer? Learn OS and networking. Database user? Learn B-trees and WAL.

You don't need to be an expert in every layer. You need to know enough to recognize when the abstraction is leaking and debug downward.
