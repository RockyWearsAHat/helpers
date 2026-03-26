# DuckDB — In-Process OLAP, Columnar Storage & Vectorized Execution

DuckDB is in-process OLAP database engine optimized for analytical queries on structured data. Written in C++, it embeds in applications (Python, Node.js, Rust, Java, Go) with zero network overhead, handling columnar data via vectorized execution while supporting zero-copy reads from Parquet, CSV, and JSON.

## Positioning: Not a Replacement, a Complement

DuckDB is **not** a general-purpose database. SQLite targets OLTP (transactional workloads); PostgreSQL handles OLTP+OLAP with transactions. DuckDB is purpose-built for **analytical queries on static or append-only data**, typically within a single application or data science workflow.

**Use DuckDB when:**
- Running analytics on static snapshots or logs (no ongoing write conflicts)
- Acting as a query layer over data lakes (Parquet, CSV, S3)
- Building ETL pipelines with SQL instead of custom code
- Embedding analytics inside Python/R/Node applications
- Interactive data exploration without external database

**Do not use DuckDB when:**
- You need distributed query across multiple machines (single-machine only)
- Application requires ACID transactions with concurrent writes
- Data cannot fit in available memory (though out-of-core execution exists)

## Storage Model: Column-Oriented with Block-Based Vectors

Unlike row-oriented (SQLite, PostgreSQL), DuckDB stores data column-by-column. More importantly, it processes data in **vectors** (dense arrays of values), enabling SIMD operations and cache efficiency.

### Columns for Compression & Selectivity

Storage layout:
```
Column: customer_id  [1, 2, 3, 4, ...]
Column: purchase_amount [99.99, 149.50, 29.95, ...]
Column: date [2025-01-01, 2025-01-02, ...]
```

Query `SELECT customer_id FROM orders WHERE purchase_amount > 100`:
- Load only **purchase_amount** and **customer_id** columns
- Row-oriented: load all columns (wasted memory)
- Compression: repeated values (e.g., timestamps, categories) compress via dictionary or delta encoding; numeric sequences compress via bit-packing

Result: analytical queries load 10-100x less data than row databases.

### Vectorized Execution

Queries execute on **vectors** (typically 2048 values per vector), not row-by-row. Operations:
- Filter: `WHERE purchase_amount > 100` applies to entire vector (one CPU instruction, multiple values via SIMD)
- Aggregate: `SUM(purchase_amount)` vectorized across thousands of values
- Join: Probe hash tables with vector batches

Contrast: SQLite executor processes one row, steps to next row. DuckDB processes 2048 rows, releases CPU, processes next vector.

## Data Source Integration: Zero-Copy Reading

### Parquet Direct Scan

DuckDB reads Parquet files directly without deserialization. Parquet column chunks align with DuckDB's vector layout:
- Parquet stored as compressed blocks → DuckDB decompresses into vectors → query engine
- No intermediate representation (e.g., Arrow serialization)
- Can push filters into Parquet reader: `SELECT * FROM 'data.parquet' WHERE year = 2024` — only reads blocks matching year

```python
import duckdb
result = duckdb.sql("SELECT * FROM 'sales.parquet' WHERE region = 'US'")
```

### CSV & JSON Streaming

CSV/JSON are textual, so no zero-copy, but DuckDB parallelizes parsing (multiple chunks → separate threads) and integrates directly:

```python
duckdb.sql("SELECT DATE, SUM(amount) FROM 'logs.csv' GROUP BY DATE")
duckdb.sql("SELECT user_id, category FROM 'events.json' WHERE event_type = 'purchase'")
```

### Multi-File & Data Lake Querying

Query across files without staging:
```python
duckdb.sql("SELECT COUNT(*) FROM 'data/2024/*.parquet'")
duckdb.sql("SELECT * FROM read_parquet('s3://my-bucket/data/*.parquet')")
```

## SQL Dialect & Extensions

DuckDB supports standard SQL (SELECT, JOIN, aggregates, window functions, CTEs). Notable extensions:

| Feature          | Notes                                      |
|------------------|--------------------------------------------|
| **Vectors**      | `ARRAY_LENGTH(arr)`, `UNNEST()`, multidimensional arrays |
| **Time Series**  | `ASOF JOIN`, `date_trunc()`, window functions with frame clauses |
| **JSON**         | `json_extract()`, `json_transform()`, automatic parsing |
| **Struct/Map**   | Nested types for semi-structured data      |
| **Variables**    | `@variable` syntax for parameterized queries |

## Extensions & Plugins

DuckDB has an official extension system (similar to PostgreSQL):

- **httpfs**: S3, GCS, Azure, HTTP read/write
- **json**: Enhanced JSON processing
- **excel**: Read `.xlsx` directly
- **postgres_scanner**: Remote PostgreSQL tables via foreign data wrapper
- **iceberg**: Apache Iceberg table format
- **vss**: Vector similarity search

Extensions are community-contributed and version-locked per DuckDB release.

## Execution: In-Process, No Network

Running DuckDB:

```python
import duckdb
conn = duckdb.connect(':memory:')  # In-memory, process-local
result = conn.execute("SELECT COUNT(*) FROM large_table")
```

Single-threaded execution by default for simplicity; multi-threaded with explicit `threads` parameter. No server, no network overhead, no authentication—all query planning and execution happens in-process.

This makes DuckDB ideal for:
- **Data science notebooks** (Python/R): Exploratory analysis on local files
- **ETL inside applications**: Parse, transform, load without external services
- **OLAP layer in backend services**: Analytical queries served from application memory

## Comparison to Related Systems

### vs SQLite

| Aspect          | SQLite          | DuckDB            |
|-----------------|-----------------|-------------------|
| **Workload**    | OLTP (row queries) | OLAP (columnar analytics) |
| **Execution**   | Row-at-a-time   | Vectorized        |
| **Transactions**| ACID, concurrent | Not designed for high concurrency |
| **Compression** | Minimal         | Column-based (10-100x) |
| **Aggregate speed** | Slow on billions | Fast on billions |

Choose SQLite for application data; choose DuckDB for analytics on that data.

### vs ClickHouse

ClickHouse is a **distributed** analytics database (Yandex-built, deployed as cluster). DuckDB is **single-machine, in-process**. ClickHouse scales to petabytes across clusters; DuckDB scales to available RAM.

ClickHouse is a deployed service (network I/O, ops overhead); DuckDB is a library (embed in app, query local files).

### vs Polars / DataFrames

Polars is a **DataFrame library** (in-memory analytics). DuckDB is a **SQL engine** on structured data. Trade-off:
- Polars: Ergonomic, idiomatic Python/R
- DuckDB: SQL for complex queries, less memory overhead, better streaming

Many workflows use both: DuckDB to load/filter, Polars to transform.

## Key Limitations

1. **Single-machine only.** Cannot query across multiple nodes. Not a replacement for distributed data warehouses.
2. **Memory-bound by default.** Out-of-core (spill to disk) exists but degrades performance.
3. **No transaction isolation across multiple connections.** Designed for single-threaded or workers without conflicts.
4. **ANSI SQL SQL, not PostgreSQL dialect.** Window functions are newer; some PostgreSQL extensions not available.

## Use Case: Analytics in Data Science

Typical DuckDB workflow:

1. **Load data** from Parquet/CSV/S3, push filters down
2. **Aggregate** with GROUP BY, window functions
3. **Export** to CSV, Parquet, or NumPy arrays for downstream analysis
4. **Iterate** on queries without rebuilding intermediate files

Example:
```python
import duckdb

# Query logs from S3
events = duckdb.sql("""
    SELECT DATE(timestamp) as day, event_type, COUNT(*) as cnt
    FROM 's3://logs/2025-03-*.parquet'
    WHERE event_type IN ('purchase', 'view')
    GROUP BY day, event_type
    ORDER BY day DESC
""")

# Export for visualization
events.to_df().to_csv('events.csv')
```

No Spark cluster, no database server, no network—pure SQL executed on single machine with vectorization.