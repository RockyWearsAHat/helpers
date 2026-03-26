# Columnar Data Formats — Encoding, Statistics & Query Optimization

## Overview

**Columnar formats** store data column-by-column instead of row-by-row, enabling compression, efficient filtering, and analytical query performance. They underpin modern data warehouses, data lakes, and streaming systems.

The landscape:
- **Parquet** (Apache): most common in data lakes; widely supported; mature
- **ORC** (Hive): optimized for Hive/Hadoop ecosystems; similar capabilities to Parquet
- **Arrow** (Apache): in-memory columnar; enables zero-copy data exchange between processes
- **Avro** (Apache): row-oriented baseline for comparison; streaming optimized

Each format makes different trade-offs between compression, encoding complexity, query speed, and indexing.

---

## Row vs. Column Storage

### Row-Oriented

```
CSV, JSON, Avro (typical), SQLite:
id,name,amount
1,Alice,100
2,Bob,200
3,Charlie,150
```

**Disk layout**:
```
[id=1][name="Alice"][amount=100][id=2][name="Bob"][amount=200]...
```

**Write cost**: Single pass through data; writes all columns of a row
**Read cost (full scan)**: Must read all columns even if only one queried
**Compression**: Values of different types interleaved; hard to compress (text id followed by binary amount)
**Use case**: OLTP (operational systems); frequent inserts/updates of entire rows

### Column-Oriented

```
Parquet, ORC, Arrow:
```

**Disk layout**:
```
[id_1][id_2][id_3]...[name_"Alice"][name_"Bob"]...[amount_100][amount_200]...
```

**Write cost**: Aggregate and write one column at a time (multiple passes)
**Read cost (selective scan)**: Only read queried columns (huge win for wide tables)
**Compression**: Same-typed data together; homogeneous batches compress well (5-10x)
**Use case**: OLAP (analytical); wide tables; queries touch few columns

---

## Parquet: The Standard Data Lake Format

Apache Parquet is the dominant columnar format for batch/lake data. Understanding its internals illuminates columnar design trade-offs.

### File Structure Hierarchy

```
Parquet File
├── Row Group 1 (e.g., rows 0-99,999)
│   ├── Column Chunk 1 (Column: id)
│   │   ├── Page 1 (rows 0-999) [compressed]
│   │   ├── Page 2 (rows 1000-1999) [compressed]
│   │   └── ...
│   ├── Column Chunk 2 (Column: name)
│   │   ├── Page 1
│   │   └── ...
│   └── Metadata (min/max id, null count, page offsets)
└── Row Group 2
    └── ...
Footer
├── Schema
├── Row group metadata (offsets, statistics)
└── Checksum
```

**Three levels of granularity**:

1. **File**: top-level container; contains multiple row groups
2. **Row Group**: horizontal partitioning (e.g., 100K rows per group)
   - Goal: fit one row group in memory for processing
   - Metadata tracked per row group (column statistics for pruning)
3. **Column Chunk**: one column's data for the row group
4. **Page**: vertical granule within column chunk (e.g., 10K values per page)
   - Compression applied per-page
   - Page offset index for random access

### Encoding Schemes

#### Dictionary Encoding

For low-cardinality columns (few unique values):
```
Original: ["US", "EU", "US", "APAC", "US", "EU"]
Dictionary: {0: "US", 1: "EU", 2: "APAC"}
Encoded: [0, 1, 0, 2, 0, 1]

Compression: "US" appears 3x; stored once + 3 indices (1 bit each)
Original: 6*2 bytes = 12 bytes
Encoded: 6 bytes (dictionary) + 6 bits (indices) ≈ 6.75 bytes (44% savings)
```

**When**: High cardinality columns (100k+ unique values) don't benefit; overhead exceeds savings

#### Run-Length Encoding (RLE)

For repeated values:
```
Original: [5, 5, 5, 5, 10, 10, 3, 3, 3]
RLE: <5, run_length=4> <10, run_length=2> <3, run_length=3>
```

Used for:
- Flagged bitmaps (presence/NULL indicators): many 0s means high compression
- Sorting keys (data often pre-sorted by date in daily batches)

#### Delta Encoding

For sequences of similar values (timestamps, IDs):
```
Original: [1000, 1001, 1000, 1003, 1005]
Deltas: [1000, +1, -1, +3, +2]  (delta from prior = differences)
```

First value stored fully; deltas stored as small integers (fit in fewer bits)

**Use case**: Time-series data (consecutive timestamps differ by ~1 second); ID sequences

#### Bit-Packing

Store integers using fewer bits:
```
Values: [0-3] (need 2 bits each)
Original: [3, 1, 2, 0, 1, 3]
Bit-packed: 11010001_10... (6 values in 12 bits = 1.5 bytes vs. naive 24 bytes)
```

Often combined with RLE or delta encoding.

### Page-Level Compression

After encoding, data compressed with general-purpose compressors:
- **Snappy**: fast decompression; moderate ratio (3-4x typical)
- **Gzip**: high ratio (5-8x on text); slow decompression
- **LZO**: fast; less common (licensing issues)
- **Zstd**: high speed, high ratio; modern choice (10x typical)

**Choice**:
- **Fast reads**: Snappy (analytical queries on warm cache)
- **Storage-optimized**: Gzip or Zstd (cold archive, network I/O bound)

### Statistics & Predicate Pushdown

Parquet stores **column statistics** at page and row group level:
```
Row Group 1 (Column: amount)
├── min: 1
├── max: 100,000
├── null_count: 0
├── distinct_count: 50,000 (approx., often not exact)
└── Page-level stats: similar

Row Group 2
├── min: 100,001
├── max: 200,000
└── ...
```

**Predicate pushdown**: Query `SELECT * FROM orders WHERE amount > 150,000`

1. **File scan**: row group 1 has max=100k; skip entirely
2. **Row group 2**: min=100k, max=200k; contains qualifying rows, read all pages
3. **Page-level**: within row group 2, pages with max < 150k skipped
4. **In-memory filtering**: read pages; filter rows

**Savings**: Without stats, all rows scanned; with stats, many row groups skipped (orders of magnitude faster)

### Schema & Nested Types

Parquet supports nested structures (JSON-like):
```
message Order {
    required int64 order_id;
    required string customer_id;
    optional group items {
        repeated group item_list {
            required string product_id;
            required int32 quantity;
            required double unit_price;
        }
    }
}
```

Nested fields columnar organized (product_ids stored together, then quantities):
- Enables efficient filtering: "orders with at least 1 item quantity > 10"

---

## ORC: Hive-Optimized Format

ORC (Optimized Row Columnar) is Hive's native columnar format. Similar capabilities to Parquet but tuned for Hadoop:

### Differences from Parquet

**Stripe-based hierarchy** (Hive terminology):
```
ORC File
├── Stripe 1 (e.g., 64MB compressed)
│   ├── Column Chunks
│   ├── Index Stream (block-level indexes)
│   └── Data Stream
├── Stripe 2
└── Footer + Metadata
```

Stripe ≈ row group but often larger (storage-optimized).

**Built-in **indexing**:
- Every 10K rows: bloom filter and value frequency index built
- Reduces false-positive row skipping
- Query engine can use indexes to skip blocks mid-stripe

**Compression defaults**: Default Zstd (vs. Snappy for Parquet); better compression assumed

**ACID support**: ORC integrates with Hive ACID transactions (insert/update/delete operations)

### Parquet vs. ORC Trade-offs

| Aspect | Parquet | ORC |
|--------|---------|-----|
| **Format Maturity** | Wider adoption; language-agnostic | Hadoop-ecosystem tight |
| **Compression** | Snappy default (lower ratio) | Zstd default (higher ratio) |
| **Read Speed** | Columnar access fast | Comparable |
| **Indexing** | Statistics-based | Value indexing + bloom filters |
| **Encoding Schemes** | Simpler (dictionary, RLE, delta) | More complex; Hive-tuned |
| **ACID** | Support via table formats (Delta, Iceberg) | Native Hive ACID |
| **Ecosystem** | Spark, DuckDB, Trino, Presto, all clouds | Primarily Hadoop/Hive |

---

## Apache Arrow: In-Memory Columnar

Arrow is not a storage format but an **in-memory columnar layout spec** enabling zero-copy data exchange.

### Memory Layout

```
Column: [1, 2, null, 4, 5]

Arrow Buffers:
├── Validity (null bitmap):  [11011]  (1=valid, 0=null)
├── Data: [int32_1, int32_2, xxx, int32_4, int32_5]
```

**Zero-copy principle**:
- C++ process reads buffer; passes pointer to Python process
- No serialization/copy; both share same memory region
- Language-agnostic (C++, Python, Java, Rust, Go all implement Arrow)

### Use Cases

**Interprocess Communication**:
```
Spark → Arrow IPC → DuckDB
(Spark produces Arrow buffers; DuckDB reads directly without copying)
```

**In-memory OLAP**:
- Compute engine (Datafusion, DuckDB using Arrow backend)
- Filters/aggregates operate on columnar buffers in-place

**Data Science Pipelines**:
```python
pandas_df = ...
arrow_table = pa.Table.from_pandas(pandas_df)  # Zero-copy if possible
query_result = duckdb.query("SELECT * FROM arrow_table WHERE x > 10").to_arrow()
# All in-memory; no serialization
```

### Arrow vs. Parquet/ORC

Arrow is **not a replacement**:
- **Arrow**: In-memory columnar; optimized for CPU cache and vectorized operations
- **Parquet/ORC**: Disk storage; optimized for compression and statistics

Workflow:
```
Disk: Parquet file (compressed)
  ↓
Memory: Arrow table (uncompressed columnar, cache-friendly)
  ↓
Query: Columnar operations (filter, project, group-by)
  ↓
Result: back to Parquet if persisting
```

---

## Avro: Row-Oriented Baseline

Apache Avro stores data row-by-row (for reference/comparison):

```
Avro record: {id: 1, name: "Alice", amount: 100.0}
Binary encoding: [varint_1][string_len_5][bytes_"Alice"][double_100.0...]
```

**Strengths**:
- Compact binary encoding (smaller than JSON, XML)
- Schema versioning; backward compatible evolution
- Streamable; no need to load entire file

**Weaknesses**:
- Row-oriented; slow for analytical queries (must scan all columns)
- No compression at value level (only file-level gzip)
- No statistics; can't prune data blocks

**Use case**: Kafka events, streaming; transactional systems where row-oriented access natural

---

## Query Optimization: Predicate Pushdown & Pruning

### Predicate Pushdown

Moving filter conditions as close to storage as possible:

```
High-level query:
SELECT product_id, SUM(amount)
FROM sales
WHERE year = 2024 AND region = 'US'
GROUP BY product_id

Pushed-down predicates:
1. File read: skip row groups where year statistics != 2024
2. Page read: skip pages where region != 'US'
3. In-memory: filter rows matching condition
```

**Engine responsibility**:
- Spark, BigQuery, DuckDB: parse WHERE clause
- Extract filterable columns (year, region)
- Check file statistics; skip partitions/row groups/pages
- Only read matched data

### Z-Ordering (Multi-Dimensional Clustering)

Data sorted in z-curve order to improve multi-column pruning:

```
2D data (x, y):
(1,1), (2,1), (1,2), (2,2), (3,1), (3,2), (4,1), (4,2)

Z-order curve:
(1,1) → (2,1) → (1,2) → (2,2) → (3,1) → (4,1) → (3,2) → (4,2)
# Spatially nearby points in (x,y) space → nearby in file

Query: x ∈ [1,3] AND y ∈ [1,2]
Result: Corresponding z-order blocks fetched; fewer false positives
```

**Libraries**: Delta Lake `ZORDER BY x, y`; Iceberg `.sortBy()` method

---

## Statistics-Based Optimizations

### Min/Max Pruning

```
Query: SELECT * WHERE amount > 500

Row Group 1: amount min=10, max=100 → skip
Row Group 2: amount min=200, max=600 → read
Row Group 3: amount min=50, max=1000 → read
```

### Null Count Pruning

```
Query: SELECT * WHERE nullable_column IS NOT NULL

If row_group.null_count == row_group.row_count → all rows null; skip
```

### Bloom Filters

ORC includes bloom filters (probabilistic ; false positives okay, no false negatives):
```
Query: WHERE customer_id = 'X123'

Row Group 1: Bloom filter doesn't match 'X123' → definitely skip
Row Group 2: Bloom filter might match → check further
```

---

## Encoding Trade-offs Summary

| Encoding | Ratios | Use Cases | CPU Cost |
|----------|--------|-----------|----------|
| **Dictionary** | 5-20x (low cardinality) | Countries, status flags, categories | Minimal (lookup) |
| **RLE** | 10x+ (highly repetitive) | Flags, sorted data, sparse columns | Minimal (count) |
| **Delta** | 3-5x (sequences) | Timestamps, IDs, measurements | Low (subtraction) |
| **Bit-packing** | 2-8x (small integers) | Integers 0-255 | Moderate (bit shifts) |
| **General (Snappy)** | 3-4x (compat.) | Heterogeneous data | Low (fast) |
| **General (Gzip)** | 5-8x (cold storage) | Archive, infrequent access | High (slow) |

---

## Modern Trends

### Nested/Semi-Structured Data Handling

Parquet/ORC/Arrow handle JSON and semi-structured types natively:
- BigQuery STRUCT, ARRAY types → columnar representation
- Parquet nested groups → preserves structure while staying columnar
- DuckDB, Polars: JSON as first-class column type

### Adaptive Encoding

Some modern systems (DuckDB, Datafusion) choose encoding dynamically:
- Analyze column statistics during write
- Dictionary encoding if cardinality < 1% of rows
- Otherwise, plain encoding + general compression

### Columnar for Streaming

Event systems now support columnar formats:
- Kafka with Arrow: batch events, send as arrow batches (efficient)
- Streaming tables (Kafka, Pulsar) expose columnar view on recent events

---

## See Also

- [Data Serialization Formats](data-serialization-formats.md) — broader serialization taxonomy
- [Data Warehousing](data-engineering-warehousing.md) — analytical query patterns
- [Data Lakehouse](data-engineering-lakehouse.md) — table formats build on columnar layouts
- [Database Internals: Storage](database-internals-storage.md) — lower-level storage trees