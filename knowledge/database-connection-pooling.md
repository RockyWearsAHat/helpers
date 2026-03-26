# Database Connection Pooling — Architectures, Pool Sizing, Lifecycle, and Serverless Patterns

## Overview

**Connection pooling** maintains a cache of database connections, reused across client requests. Creating a new database connection is expensive:
- TCP handshake (network round-trip)
- TLS negotiation (if encrypted)
- Authentication (username, password verification)
- Database initialization (set session variables, load user context)

A single connection creation can consume 50–500ms. Pooling eliminates this overhead by keeping connections alive and passing them between clients.

**The fundamental tradeoff:** Reuse efficiency vs. resource overhead. Too few connections and clients wait; too many and the database runs out of memory/file descriptors.

---

## Pooling Architectures

### Application-Level Pooling (In-Process)

**Mechanism:** Each application instance maintains its own pool (e.g., HikariCP in Java, `sqlalchemy.pool` in Python).

**Diagram:**
```
App1 (Pool: 5 conns) ──┬─→ DB
App2 (Pool: 5 conns) ──┤
App3 (Pool: 5 conns) ──┴─→ DB
Total connections seen by DB: 15
```

**Characteristics:**
- **Low latency:** No network round-trip to acquire a connection.
- **Resource multiplier:** 10 apps × 10 connections = 100 connections on the database.
- **Isolation:** Each app's pool is independent; one app's misbehavior doesn't cull connections for others.

**When used:** Monolithic applications, microservices with moderate connection needs.

### Proxy-Level Pooling (Middleware)

**Mechanism:** Standalone proxy (PgBouncer, ProxySQL) sits between applications and database. Applications connect to the proxy; the proxy maintains a smaller pool to the database.

**Diagram:**
```
App1 ─→ ┐
App2 ─→ │ PgBouncer (Pool: 20 conns) ─→ DB
App3 ─→ ┴                                (sees only 20 connections)
100s of client connections possible
```

**Characteristics:**
- **Multiplexing:** Proxy can handle thousands of lightweight client connections using a small pool of real database connections.
- **Centralized control:** All applications share the same pool. Easy to tune globally.
- **Latency overhead:** Small (proxy is local or nearby).
- **Resilience:** Proxy is a single point of failure (mitigate with clustering).

**When used:** Large deployments, microservices with many short-lived clients, serverless (see below).

### Serverless Connection Management

**Challenge:** Serverless functions (Lambda, Cloud Functions) are ephemeral and can scale to thousands of concurrent invocations. Naive pooling leads to connection explosion.

**Patterns:**

1. **RDS Proxy (AWS) / Cloud SQL Auth proxy:** Lightweight proxy managed by the cloud provider. Functions authenticate to the proxy; the proxy handles multiplexing to the database.

2. **Connection pooling library with local cache:** Library (e.g., `pg` in Node.js) maintains a small local pool kept alive by the function container (if warm). Cold starts create new connections.

3. **Database native pooling:** Some databases (PostgreSQL with `pgbouncer`, MySQL with ProxySQL) accept and cache connections natively.

RPO/RTO considerations: Functions time out, connections drop. Pool cleans up stale connections.

---

## Connection Pool Modes

### Session Mode (Full Multiplexing)

Each client connection maps to exactly one database connection for the duration of the session.

**Characteristics:**
- **Simple semantics:** Connection owns all session state (temp tables, transactions, cursor state).
- **Efficiency:** 1:1 mapping. No multiplexing gains if connection idle.
- **Transaction safety:** Implicit transactions (e.g., `BEGIN`) are isolated per session.

**When used:** Legacy applications, applications with long idle connections, those that rely on session-scoped features.

### Transaction Mode

Connections are pooled at the transaction boundary. A client connection is allocated a database connection only for the duration of a transaction; once `COMMIT`/`ROLLBACK`, the connection is released back to the pool.

**Mechanics:**
1. Client sends query/transaction.
2. Proxy allocates a database connection (potentially different from the last transaction).
3. Transaction executes.
4. `COMMIT`/`ROLLBACK` releases the connection.
5. Next transaction may use a different database connection.

**Characteristics:**
- **High multiplexing:** Many clients can share few database connections if transactions are short.
- **Session state loss:** Each transaction gets a fresh connection. Session temp tables, cursors, or explicit transactions don't survive across client transactions.
- **Reduced connection count:** 100 clients, 10 transactions/sec, 1-sec transactions = ~10–20 database connections needed.

**Restrictions:**
- No distributed transactions (2PC) across clients.
- No explicit transaction control (`BEGIN` outside the proxy-managed transaction).
- No prepared statement handles (connection may change between transactions).

**Examples:** Optimal for microservices, APIs with short request-response cycles.

### Statement Mode (Least Common)

Connection released after each SQL statement (not even per transaction).

**Downsides:** Very restrictive (almost no real applications support this). Rarely used.

---

## Pool Sizing

Pool size is **the** critical tuning knob. Too small and clients wait for connections; too large and the database resource limit is exceeded.

### Formula (Postgres Literature)

A classic formula (from PgBouncer docs):

```
pool_size = ((core_count * 2) + effective_spindle_count)
```

Where:
- `core_count` = Database server CPU cores.
- `effective_spindle_count` = Storage devices (0 for SSD, 1-2 for spinning disk).

**Example:** 16-core database + SSD = `(16 * 2) + 0 = 32 connections`.

**Rationale:** 
- Database can effectively run 2 queries per core (one active, one I/O-bound).
- SSD I/O is much faster than disk seeks; fewer spindles = fewer blocked queries.

### Practical Tuning

1. **Start with the formula.** 20–40 connections for modern databases.
2. **Monitor connection wait time.** If clients frequently wait for a connection, increase pool size.
3. **Monitor database CPU/memory.** If increasing pool size causes CPU saturation or memory pressure, the pool is already too large.
4. **Measure end-to-end latency.** Pool size that minimizes total latency (client wait + query time) is optimal.

### Per-Pool Sizing (Multiple Pools)

Some applications maintain separate pools for different workload types:

**Example (HikariCP config):**
```java
// OLTP pool: many short transactions
hikari.maximumPoolSize = 32
hikari.minimumIdle = 10

// Analytics pool: few long-running queries
hikari.maximumPoolSize = 5  // Don't over-provision
hikari.minimumIdle = 1
```

---

## Connection Lifecycle and Tuning Parameters

### Idle Timeout

**Purpose:** Close connections idle for too long. Reclaims resources; detects stale connections.

**Typical setting:** 30 minutes (conservative, safe).

**Tradeoff:**
- **Shorter timeout:** Saves server resources, but frequent reconnection overhead.
- **Longer timeout:** Stale connections accumulate (network firewalls may silently drop idle TCP connections).

**Note:** Database-side idle timeouts may differ. Configure both application pool and database.

### Connection Validation

**Purpose:** Before handing a connection to a client, verify it's still alive.

**Methods:**
- **Ping query:** `SELECT 1` or driver-specific validation (fast, sub-millisecond).
- **Connection age check:** Validate every N minutes.
- **Lazy validation:** Only validate if connection not used recently.

**Cost:** Adds latency. Balance robustness vs. speed.

### Max Connection Age

**Purpose:** Rotate out very old connections to refresh state and clear any accumulated memory/resource leaks.

**Typical setting:** 10–30 minutes or unbounded (if connections are stable).

---

## Practical Pooling Solutions

### PgBouncer (PostgreSQL)

**Mode:** Proxy-level pooling for PostgreSQL.

**Features:**
- **Transaction vs. Session mode:** Configurable per-pool.
- **Lightweight:** Written in C, minimal overhead.
- **Cluster-aware:** Can route to replicas.
- **Configuration:** TOML-like file with pool sections.

**Example config:**
```ini
[databases]
mydb = host=localhost port=5432 dbname=mydb

[pgbouncer]
pool_mode = transaction
max_client_conn = 10000
default_pool_size = 25
reserve_pool_size = 5
reserve_pool_timeout = 3
```

**Common gotchas:**
- **Transaction mode state loss:** Prepared statements don't survive (`PREPARE` doesn't persist across transactions).
- **LISTEN/NOTIFY:** Only works in session mode (requires persistent connection).

### ProxySQL (MySQL/MariaDB)

**Mode:** Proxy-level, full middleware with query rewriting, sharding, and connection pooling.

**Features:**
- Complex rule-based query routing.
- Connection pooling with separate pools by target backend.
- Query caching.
- Load balancing across replicas.

**Connection pool controls:**
```
SET VARIABLE mysql-max_connections = 10000;
SET VARIABLE mysql-default_max_connections = 150;
SET VARIABLE mysql-connection_max_age_ms = 3600000;
```

### HikariCP (Java)

**Mode:** Application-level pooling, lightweight and opinionated.

**Features:**
- **Minimalist API:** `getConnection()` returns immediately if available.
- **Connection validation:** Configurable statement validation.
- **Metrics:** Built-in monitoring (connection wait, active count).

**Tuning:**
```java
HikariConfig config = new HikariConfig();
config.setMaximumPoolSize(32);
config.setMinimumIdle(10);
config.setConnectionTimeout(30000);  // 30 sec wait max
config.setIdleTimeout(600000);       // 10 min idle timeout
config.setMaxLifetime(1800000);      // 30 min connection age
```

---

## Common Problems and Solutions

### Connection Leak (Connections Gradually Exhausted)

**Symptom:** Over time, all connections in the pool are checked out and never returned.

**Causes:**
- Application forgot to close a connection (no try-finally or try-with-resources).
- Exception in the try block prevents `close()`.

**Solution:**
- Use language features (`try-with-resources` in Java, context managers in Python).
- Enable connection leak detection in pooling library.
- Monitor pool utilization; alert on sustained high usage.

### Connection Timeout Under Load

**Symptom:** Clients fail after timeout waiting for a connection, even though the database is responsive.

**Causes:**
- Pool too small for concurrent load.
- Long-running queries holding connections.
- Database-side query queueing causing blocking transactions.

**Solution:**
- Increase pool size (if database can sustain it).
- Reduce query latency (indexing, query optimization).
- Use connection limits per user/application.

### Stale Connections (Network Timeout or Database Restart)

**Symptom:** Pooled connections work, then database restarts; clients get "connection reset" errors.

**Solution:**
- Enable connection validation on every use (adds latency, but ensures freshness).
- Shorter idle timeout (max 10 minutes).
- Automated pool drain and recreate after database restart.

---

## When NOT to Pool

- **Very high throughput, single-purpose workload:** Direct connection-per-request may be simpler (e.g., HTTP request → single short query → response). Profiling required.
- **Interactive sessions with long idle periods:** Pooling wastes connections. Better to multiplex at a higher level (session management, WebSocket).
- **Embedded databases (SQLite):** No network overhead; pooling not needed.

See also: database-patterns, distributed-replication, infrastructure-resource-limits.