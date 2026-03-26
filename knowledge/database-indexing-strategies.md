# Database Indexing Strategies: Structure, Types, and Maintenance

## B-tree: The Standard Index Structure

B-trees (technically B+ trees in modern databases) remain the dominant index structure for most OLTP workloads. They maintain sorted order across multiple levels and guarantee logarithmic lookup time.

**Structure**: The tree has a root node, internal nodes, and leaf nodes. Data is stored only in leaf nodes; internal nodes carry copy keys to guide navigation. All leaves are at the same depth (balanced), and keys within each node are sorted. Each node corresponds to a disk page, so a 3-level B-tree with 4KB pages can index millions of entries with only 3 disk seeks.

**Page Splits**: When a leaf page fills, it splits into two pages. The middle key is promoted to the parent. If the parent fills, it also splits, potentially cascading up to the root. This maintains balance but is expensive—a single insert can trigger multiple page writes. The cost is amortized over many inserts, but write-heavy workloads suffer from page split overhead.

**Range Queries**: B-trees excel at range scans. After finding the first matching row, subsequent rows are in adjacent leaf pages, which can be read sequentially. A query like `SELECT * FROM users WHERE age BETWEEN 18 AND 65` retrieves a contiguous range without random seeks.

**Limitations**: B-trees require comparison operators to be defined on the key. They're unsuitable for unstructured data (images, text) without preprocessing. Highly selective queries with many false positives (e.g., wildcard searches) devolve into sequential table scans if selectivity is poor.

## Hash Indexes

Hash indexes map keys to buckets via a hash function, offering O(1) average lookup for equality conditions. They're fast for exact-match queries but cannot be used for range queries or sorting.

**When Hash Indexes Help**: Very high cardinality columns with only equality lookups (e.g., user ID). PostgreSQL's hash indexes were historically unreliable and are rarely recommended; MySQL doesn't expose hash indexes directly (MEMORY and NDB engines use them internally). Most systems default to B-tree for safety.

**Trade-off**: As the hash table grows, collisions increase and buckets must be resized. Resizing is expensive (all entries rehashed). B-trees grow more gracefully via tree height.

## Specialized Indexes: GiST, GIN, BRIN

**Generalized Search Tree (GiST)**: A flexible framework for implementing custom index types. Useful for geometric data, ranges, and full-text search. GiST can index any data type with comparison operators defined. The downside: slower than specialized structures for specific data types, and query performance varies widely based on implementation.

**Generalized Inverted Index (GIN)**: Optimized for full-text search and JSONB data. An inverted index maps terms to document IDs. A query like `SELECT * FROM articles WHERE text_column @@ 'database & optimization'` (PostgreSQL full-text) uses the GIN index to rapidly retrieve matching documents. Cost: slower inserts (must update the inverted index); higher memory overhead.

**Block Range Index (BRIN)**: A lightweight index for large tables with natural ordering (e.g., time-series data). Instead of storing one index entry per row, BRIN divides the table into blocks (typically 128 pages) and stores min/max values per block. Queries outside the range skip entire blocks. Ideal for time-series and append-only tables; ineffective for random access on unordered data.

## Partial and Conditional Indexes

A *partial index* indexes only rows matching a WHERE condition. Example: `CREATE INDEX idx_active_users ON users(id) WHERE active = true`. For tables with many inactive rows, this reduces index size and insertion cost.

Trade-off: A query must also include the same condition for the partial index to be considered. The optimizer doesn't automatically use a partial index for queries without the filter.

## Expression Indexes

An *expression index* indexes a computed value, not a raw column. Example: `CREATE INDEX idx_user_email_lower ON users(LOWER(email))`. Queries must use the same expression: `SELECT * FROM users WHERE LOWER(email) = 'test@example.com'`. This avoids repeated computation at query time.

Cost: Expression computation happens at insert/update time, adding overhead. The index is opaque—you can't easily inspect what values it contains without recomputation.

## Covering Indexes and INCLUDE

A *covering index* (or *clustered index* in some systems) includes all columns needed for a query (SELECT, WHERE, JOIN conditions) so the database can satisfy the query from the index alone, avoiding heap lookups.

PostgreSQL allows `CREATE INDEX idx ON table(col1) INCLUDE (col2, col3)` to add non-key columns. The INCLUDE columns are stored at leaf nodes only, not used for navigation. Queries selecting col1-col3 can use index-only scans (IOS), which are faster because they don't require heap lookups.

Trade-off: Covering indexes use more disk space and slow down inserts/updates (more data to maintain). They're worthwhile for frequently accessed queries on predictable columns.

## Index-Only Scans

An *index-only scan* returns query results directly from the index without accessing the main table. For this to work, all necessary columns must be in the index (covering index) AND the index must be *visibility-optimized*—it must know whether a row is visible to the current transaction.

PostgreSQL's Index Visibility Map (VM) tracks which index pages contain only visible rows. When the VM says a page is fully visible, the database can skip heap lookups. This applies to covering indexes and GIN indexes on JSONB.

Benefit: Significant speedup for hot queries, especially on large tables. Downside: VM maintenance adds overhead; high update rates can degrade index-only scan effectiveness.

## Index Maintenance and Bloat

Indexes grow over time as rows are inserted. Deleted rows leave gaps (dead space). In B-tree indexes, pages can't shrink—deleted row slots remain. Over time, pages become sparse and cache-inefficient.

**VACUUM** (PostgreSQL) reclaims dead space in the index using index-only vacuuming (skipping the heap). Aggressive VACUUM aggressively removes dead rows and compacts indexes.

**Index Bloat** occurs when many rows are updated/deleted but the index isn't vacuumed frequently. Bloated indexes consume more cache and are slower. Monitoring tools can estimate bloat; high bloat indicates either insufficient vacuum frequency or problematic workload patterns.

**REINDEX** rebuilds an index from scratch, fully compacting it. It's expensive (locks the table or requires `CONCURRENTLY`) but sometimes necessary for severely bloated indexes.

## Index Selection Strategy

Most databases benefit from indexes on:
- Foreign key columns (for joins)
- Columns in WHERE clauses with high selectivity (cardinality > 10% of rows)
- Columns in ORDER BY clauses
- Columns in GROUP BY clauses on large result sets

Avoid indexes on:
- Low-cardinality columns (too many rows per value; full scan often wins)
- Columns in volatile computed expressions (recomputed at every update)
- Seldom-used columns (index maintenance cost exceeds query benefit)

Query execution plans (EXPLAIN ANALYZE) reveal which indexes are used. Unused indexes waste space and slow down writes; they should be removed or consolidated with other indexes.