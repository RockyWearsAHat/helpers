# Database Storage Engines — B-tree vs LSM-tree, WAL, and Durability

## B-tree: The Read-Optimized Workhorse

B-trees (more precisely B+ trees in practice) organize data in a balanced tree structure. Keys are stored in sorted order across internal nodes and leaf nodes, with leaf nodes containing pointers to actual row data.

**B-tree write path**: When you insert or update a row, the database locates the correct page in the tree, modifies it in-place, and marks the page as dirty in the buffer pool. The actual disk write happens asynchronously during buffer eviction or explicit checkpoint.

**Read efficiency**: B-trees excel at point lookups and range scans because data on disk is already sorted. A single index lookup requires a logarithmic number of page fetches. Range queries traverse adjacent leaf nodes sequentially.

**Write amplification**: The cost of updates can be high. Inserting a single row may require multiple page rewrites: if a page splits, parent pointers must be updated, potentially cascading splits up the tree. A single logical write can trigger many physical disk writes — this is write amplification.

**Who uses it**: PostgreSQL, MySQL, SQLite. Any RDBMS prioritizing fast reads and transactions on structured, mutable data.

## LSM-tree: The Write-Optimized Alternative

An LSM-tree (Log-Structured Merge-tree) inverts B-tree trade-offs. Instead of updating data in-place on disk, all writes are initially appended to an in-memory buffer called a **memtable**. When the memtable fills, it's flushed as an immutable **SSTable** (Sorted String Table) to disk. Multiple SSTables accumulate on disk, periodically merged during **compaction**.

**Write path**: Write hits memtable in RAM (microseconds). Durability ensured by a write-ahead log (separate WAL file). When memtable fills (~100MB), flush to disk as immutable SSTable. Sequential disk writes are near-optimal performance.

**Compaction**: Background process merges SSTables to maintain query efficiency. Naive compaction reads all SSTables sequentially, writes merged result to new SSTable, discards old files. Cost: both read and write amplification during compaction.

**Read efficiency**: Depends on LSM structure. Must search multiple SSTables (first memtable, then recent SSTables, then older ones). **Bloom filters** attached to each SSTable accelerate negative lookups (key not found). Tiered compaction (level-based vs tiered) affects read cost and space usage.

**Who uses it**: RocksDB, Cassandra, LevelDB, DynamoDB. Systems prioritizing write throughput, immutability, and fault tolerance.

## Trade-off Dimensions

| Dimension | B-tree | LSM-tree |
|---|---|---|
| **Write Throughput** | Good (~10-100K writes/sec) | Excellent (~100K-1M writes/sec) |
| **Write Amplification** | 5-10x typical | 10-100x during compaction, amortized |
| **Read Performance** | Optimal (balanced tree) | Good (bloom filters + tiering help) |
| **Space Amplification** | ~1x (mostly exact) | 2-5x (multiple SSTables, compaction overhead) |
| **Point Lookups** | 1 tree traversal | Multiple SSTable searches + bloom filters |
| **Range Scans** | Sequential leaf traversal | Must span multiple SSTables |
| **Compaction Pauses** | None (in-place updates) | Occasional (background merges stall writes) |

## Write-Ahead Logging (WAL): Guaranteeing Durability

Both B-tree and LSM-tree systems use WAL to ensure **atomicity** and **durability** (the A and D in ACID).

**The WAL protocol**:
1. Client initiates write (insert/update)
2. Database writes a **log entry** to WAL buffer in RAM
3. On commit, WAL buffer is **flushed to disk** (fsync), ensuring durability
4. Changes then applied to data structure (cached in buffer pool)
5. Dirty pages eventually written to disk (ordering enforced by LSN — Log Sequence Number)

Data pages can only be written to disk if their log entries have already been flushed. This ordering constraint ensures that if a system crashes, the WAL can replay all committed transactions.

**WAL in LSM-trees**: LSM-trees require especially tight WAL integration. Every memtable write must be preceded by a WAL entry. When memtable is flushed to SSTable, the corresponding WAL segment can be discarded. WAL size is bounded by memtable size.

**WAL in B-trees**: B-tree WAL entries record page modifications. During recovery, the database replays the log to restore consistent state.

## Buffer Pool Management

The buffer pool is a cache of disk pages held in RAM. Not all data fits in memory; the pool must evict old pages to make room for new ones.

**Dirty page flushing**: When a page is modified, it's marked dirty. During buffer eviction, dirty pages must be flushed to disk. **WAL order** must be respected: flushed LSN (highest LSN written to disk) must always be >= modified page LSN, ensuring that log entries exist for all pending changes.

**Eviction policies**: LRU (Least Recently Used) or Clock eviction removes oldest-touched pages. Under write-heavy workloads, the buffer pool may become a bottleneck.

## SSTable, Bloom Filters, and Compression

**SSTable format**: An immutable sorted file containing key-value pairs. Typically structured as:
- **Index block**: Sparse index mapping keys to offsets
- **Data blocks**: Compressed key-value entries
- **Bloom filter**: Probabilistic set membership (fast negative lookups)
- **Footer**: Metadata (offsets, checksums)

**Bloom filters**: Used to prevent unnecessary SSTable reads. Checking a Bloom filter is O(k) where k is hash functions (typically 3-5). May have false positives (key might exist) but never false negatives (key definitely doesn't exist).

**Compression**: SSTables compress data blocks (Snappy, LZ4, zstd) to reduce disk I/O. Decompression happens in buffer pool. Compression ratio typically 2-5x for structured data.

## Row vs Columnar Storage

**Row-oriented** (traditional): Stores complete rows together on disk. Optimized for OLTP (Online Transaction Processing) — single-row writes/reads are fast. Full row scans include unwanted columns.

**Columnar** (column-oriented): Stores column values together. Optimized for OLAP (analytics) — selective columns queries faster, compression better (same types). Requires reconstructing full rows for OLTP.

LSM-trees typically use row-oriented; columnar stores (Parquet, ORC) common in data warehouses.

## Key Takeaways for Practitioners

- **Read-heavy workload** (OLTP, ACID): B-tree (PostgreSQL, MySQL)
- **Write-heavy workload** (timeseries, logs, streams): LSM-tree (RocksDB, Cassandra)
- **Both depend on WAL** for durability; WAL overhead can dominate on slow storage
- **Compaction is invisible but expensive**; LSM write amplification is front-loaded (memtable) and backend-loaded (compaction)
- **Bloom filters and compression** are critical LSM optimizations
- **Buffer pool eviction policy** deeply affects performance; LRU misses under skewed access patterns
- **Page size** (typically 4-16KB) balances tree branching factor vs I/O overhead

## See Also

- concurrency-patterns (MVCC vs locking)
- database-query-optimization (using indexes effectively)
- memory-management (buffer pool eviction algorithms)