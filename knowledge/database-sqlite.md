# SQLite

## Embedded Architecture

SQLite is a **serverless, zero-configuration** database engine. The entire database is a single file. The library (~600KB) links directly into the application process — no client-server protocol, no network overhead.

### How It Works

- Database is a single file (cross-platform binary format, stable since 2004)
- Uses a B-tree for tables and indexes (one B-tree per table/index)
- Page-based storage: default page size 4096 bytes (matches most OS/filesystem pages)
- All writes go through the pager, which manages caching and journaling
- Entire SQL engine compiles to a virtual machine (VDBE) that executes bytecode

### Limits

- Max database size: 281 TB (practical limit far lower)
- Max row size: 1 billion bytes (limited by `SQLITE_MAX_LENGTH`)
- Max columns per table: 2000
- Max attached databases: 10 (default, compilable to 125)
- Single-writer model: one write transaction at a time (unlimited concurrent readers in WAL mode)

## WAL Mode

Write-Ahead Logging mode dramatically improves concurrent access. **Enable it on every SQLite database** unless you have a specific reason not to.

```sql
PRAGMA journal_mode = WAL;
```

### How WAL Works

1. Readers read from the database file + WAL file (snapshot isolation)
2. Writers append to the WAL file (no blocking readers)
3. Periodically, WAL is checkpointed back to the database file

### WAL vs Rollback Journal (default)

| Aspect                      | WAL                | Rollback Journal       |
| --------------------------- | ------------------ | ---------------------- |
| Writer blocks readers       | No                 | Yes                    |
| Multiple concurrent readers | Yes                | Yes (but slower)       |
| Read performance            | Excellent          | Good                   |
| Write performance           | Good               | Good for single writer |
| File count                  | 3 (db, -wal, -shm) | 2 (db, -journal)       |
| Network filesystem          | Not recommended    | Works (barely)         |
| Crash recovery              | Automatic          | Automatic              |

```sql
-- Tune WAL checkpoint behavior
PRAGMA wal_autocheckpoint = 1000;  -- checkpoint every 1000 pages (default)
PRAGMA wal_checkpoint(TRUNCATE);   -- manual checkpoint + truncate WAL file
```

## PRAGMA Tuning

Essential PRAGMAs for production applications:

```sql
-- Performance (set at connection open)
PRAGMA journal_mode = WAL;          -- concurrent readers + writer
PRAGMA synchronous = NORMAL;        -- safe with WAL (FULL is overkill)
PRAGMA cache_size = -64000;         -- 64MB page cache (negative = KB)
PRAGMA foreign_keys = ON;           -- off by default (!)
PRAGMA busy_timeout = 5000;         -- wait 5s instead of instant SQLITE_BUSY
PRAGMA temp_store = MEMORY;         -- temp tables/indexes in RAM
PRAGMA mmap_io = 268435456;         -- memory-map up to 256MB of database

-- Optimize (run periodically or at connection close)
PRAGMA optimize;                    -- analyze tables that need it (3.18+)

-- Integrity
PRAGMA integrity_check;             -- full database consistency check
PRAGMA quick_check;                 -- faster, less thorough check
```

### Dangerous PRAGMAs (understand before using)

```sql
PRAGMA synchronous = OFF;           -- fastest, but database corrupts on power loss
PRAGMA journal_mode = OFF;          -- no crash protection at all
PRAGMA locking_mode = EXCLUSIVE;    -- single process access, avoids lock contention
```

## JSON Support (JSON1 Extension)

Built into SQLite since 3.38.0 (was a loadable extension before). Stores JSON as text, processes with dedicated functions.

```sql
-- Create table with JSON data
CREATE TABLE events (
  id INTEGER PRIMARY KEY,
  data TEXT NOT NULL  -- store JSON as text
);

INSERT INTO events (data) VALUES ('{"type":"click","user":{"id":42,"name":"Alice"},"tags":["ui","button"]}');

-- Extraction
SELECT json_extract(data, '$.type') FROM events;             -- 'click'
SELECT json_extract(data, '$.user.name') FROM events;        -- 'Alice'
SELECT data ->> '$.user.name' FROM events;                   -- 'Alice' (3.38+, returns text)
SELECT data -> '$.user.name' FROM events;                    -- '"Alice"' (returns JSON)

-- Modification
UPDATE events SET data = json_set(data, '$.processed', true);
UPDATE events SET data = json_insert(data, '$.priority', 'high');  -- only if not exists
UPDATE events SET data = json_replace(data, '$.type', 'tap');      -- only if exists
UPDATE events SET data = json_remove(data, '$.tags');

-- Array operations
SELECT json_array_length(data, '$.tags') FROM events;        -- 2
SELECT json_each.value FROM events, json_each(data, '$.tags');  -- 'ui', 'button'

-- Aggregation over JSON arrays
SELECT DISTINCT j.value
FROM events, json_each(events.data, '$.tags') j
WHERE json_extract(events.data, '$.type') = 'click';

-- JSON validation
SELECT json_valid('{"key": "value"}');  -- 1
SELECT json_valid('{bad json}');        -- 0

-- Build JSON
SELECT json_object('id', id, 'type', json_extract(data, '$.type')) FROM events;
SELECT json_group_array(json_extract(data, '$.type')) FROM events;
```

### Indexing JSON Fields

```sql
-- Generated column + index (3.31+)
ALTER TABLE events ADD COLUMN event_type TEXT GENERATED ALWAYS AS (json_extract(data, '$.type')) STORED;
CREATE INDEX idx_event_type ON events (event_type);

-- Expression index (alternative)
CREATE INDEX idx_user_id ON events (json_extract(data, '$.user.id'));
```

## Full-Text Search (FTS5)

High-performance full-text search. Creates a virtual table backed by an inverted index.

```sql
-- Create FTS table
CREATE VIRTUAL TABLE articles_fts USING fts5(
  title,
  body,
  content='articles',          -- external content table
  content_rowid='id',
  tokenize='porter unicode61'  -- stemming + unicode support
);

-- Populate from existing table
INSERT INTO articles_fts(rowid, title, body)
  SELECT id, title, body FROM articles;

-- Keep in sync with triggers
CREATE TRIGGER articles_ai AFTER INSERT ON articles BEGIN
  INSERT INTO articles_fts(rowid, title, body) VALUES (new.id, new.title, new.body);
END;
CREATE TRIGGER articles_ad AFTER DELETE ON articles BEGIN
  INSERT INTO articles_fts(articles_fts, rowid, title, body) VALUES('delete', old.id, old.title, old.body);
END;
CREATE TRIGGER articles_au AFTER UPDATE ON articles BEGIN
  INSERT INTO articles_fts(articles_fts, rowid, title, body) VALUES('delete', old.id, old.title, old.body);
  INSERT INTO articles_fts(rowid, title, body) VALUES (new.id, new.title, new.body);
END;

-- Search queries
SELECT * FROM articles_fts WHERE articles_fts MATCH 'database optimization';
SELECT * FROM articles_fts WHERE articles_fts MATCH 'title:database AND body:performance';
SELECT * FROM articles_fts WHERE articles_fts MATCH '"full text search"';  -- phrase
SELECT * FROM articles_fts WHERE articles_fts MATCH 'data*';              -- prefix

-- Ranking
SELECT *, rank FROM articles_fts WHERE articles_fts MATCH 'query' ORDER BY rank;
-- rank is built-in BM25 (negative values, closer to 0 = better)

-- Snippet and highlight
SELECT snippet(articles_fts, 1, '<b>', '</b>', '...', 20) FROM articles_fts WHERE articles_fts MATCH 'query';
SELECT highlight(articles_fts, 0, '<b>', '</b>') FROM articles_fts WHERE articles_fts MATCH 'query';
```

### FTS5 Tokenizers

| Tokenizer   | Behavior                                                  |
| ----------- | --------------------------------------------------------- |
| `unicode61` | Unicode-aware, case folding, diacritics removal (default) |
| `porter`    | English stemming (wraps another tokenizer)                |
| `ascii`     | ASCII-only, simple splitting                              |
| `trigram`   | Character trigrams (substring matching)                   |

## Common Table Expressions

```sql
-- Recursive CTE: generate a series
WITH RECURSIVE cnt(x) AS (
  SELECT 1
  UNION ALL
  SELECT x + 1 FROM cnt WHERE x < 100
)
SELECT x FROM cnt;

-- Date series
WITH RECURSIVE dates(d) AS (
  SELECT date('2024-01-01')
  UNION ALL
  SELECT date(d, '+1 day') FROM dates WHERE d < '2024-12-31'
)
SELECT d FROM dates;

-- Tree traversal
WITH RECURSIVE tree AS (
  SELECT id, name, parent_id, 0 AS depth
  FROM categories WHERE parent_id IS NULL
  UNION ALL
  SELECT c.id, c.name, c.parent_id, t.depth + 1
  FROM categories c JOIN tree t ON c.parent_id = t.id
)
SELECT * FROM tree ORDER BY depth, name;
```

## Concurrent Access

### Single-Process Applications

Easy: open one connection, enable WAL, set `busy_timeout`.

### Multi-Thread Applications

```
Connection mode: SQLITE_OPEN_FULLMUTEX (serialized, safest default)
Alternative: SQLITE_OPEN_NOMUTEX (multi-thread, one connection per thread)
```

Best practice: **write on a single dedicated connection, read on a pool**.

```python
# Python example — writer + reader pool
import sqlite3, threading

# Single write connection
write_conn = sqlite3.connect('app.db', check_same_thread=False)
write_lock = threading.Lock()

def write(sql, params):
    with write_lock:
        write_conn.execute(sql, params)
        write_conn.commit()

# Read pool — each thread gets its own connection
local = threading.local()
def get_reader():
    if not hasattr(local, 'conn'):
        local.conn = sqlite3.connect('file:app.db?mode=ro', uri=True)
        local.conn.execute('PRAGMA journal_mode=WAL')
    return local.conn
```

### Multi-Process Applications

WAL mode allows one writer + many readers across processes. The `busy_timeout` handles contention. For higher write throughput, consider a connection pool with a write queue.

**Network filesystems**: SQLite does **not** work reliably on NFS, SMB, or other network filesystems. The locking semantics are broken. Use a client-server database if you need network access.

## When SQLite Is the Right Choice

### Ideal Use Cases

- **Embedded applications**: mobile apps, desktop apps, IoT devices
- **Configuration storage**: replacing config files, registries, plists
- **Caches and temp stores**: faster than filesystem for structured data
- **Testing**: use SQLite for test DB, swap to PostgreSQL in production
- **Single-server web apps**: up to ~100K requests/day with proper WAL setup (often much more)
- **Data analysis**: load CSVs, run SQL, export results
- **Application file format**: `.sqlite` as the file format (replaces XML, JSON documents)
- **Edge computing**: bring the database to the data

### When to Use Something Else

- **High write concurrency**: >1 write per millisecond from multiple processes
- **Large datasets**: >100GB (works but other DBs handle this better)
- **Network access needed**: multiple machines reading/writing
- **Fine-grained access control**: no user/role system
- **Replication**: no built-in replication (Litestream and LiteFS exist as external tools)

### SQLite in Production Web Apps

A growing trend: [Litestream](https://litestream.io/) for streaming replication, [LiteFS](https://fly.io/docs/litefs/) for distributed SQLite, [Turso/libSQL](https://turso.tech/) for edge SQLite.

```
# Litestream — continuous backup to S3
litestream replicate /data/app.db s3://bucket/app.db
```

## Useful Patterns

### UPSERT

```sql
INSERT INTO kv (key, value) VALUES ('setting', 'dark')
  ON CONFLICT(key) DO UPDATE SET value = excluded.value;
```

### RETURNING Clause (3.35+)

```sql
INSERT INTO users (name, email) VALUES ('Alice', 'a@b.com') RETURNING id, name;
DELETE FROM events WHERE created_at < date('now', '-30 days') RETURNING *;
```

### Strict Tables (3.37+)

```sql
CREATE TABLE metrics (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  value REAL NOT NULL,
  ts INTEGER NOT NULL
) STRICT;
-- Enforces type checking — no silent coercion
```

### Generated Columns (3.31+)

```sql
CREATE TABLE products (
  price REAL NOT NULL,
  tax_rate REAL NOT NULL DEFAULT 0.08,
  total REAL GENERATED ALWAYS AS (price * (1 + tax_rate)) STORED
);
```

### Window Functions

```sql
SELECT
  date, revenue,
  SUM(revenue) OVER (ORDER BY date ROWS BETWEEN 6 PRECEDING AND CURRENT ROW) AS rolling_7d,
  AVG(revenue) OVER (ORDER BY date ROWS BETWEEN 29 PRECEDING AND CURRENT ROW) AS avg_30d,
  LAG(revenue, 7) OVER (ORDER BY date) AS prev_week_same_day,
  RANK() OVER (PARTITION BY strftime('%Y-%m', date) ORDER BY revenue DESC) AS monthly_rank
FROM daily_revenue;
```

### Backup While Running

```sql
-- Online backup API
VACUUM INTO '/backups/app_backup.db';  -- 3.27+ atomic snapshot
```
