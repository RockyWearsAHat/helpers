# Tree Data Structures — BST, AVL, Red-Black, B-tree, Tries, Segment Trees

Trees are fundamental for maintaining sorted data, enabling range queries, and supporting dynamic updates. Each variant balances insertion/deletion cost against search guarantees and architectural constraints like memory layout.

## Binary Search Trees (BST)

Unbalanced BST: for each node, all left subtree keys are less than the node's key, all right subtree keys are greater.

**Properties**: Search, insert, delete are $O(h)$ where $h$ is tree height. In best case ($h = O(\log n)$), operations are $O(\log n)$. In worst case (linear chain), operations are $O(n)$.

**Issues**: without rebalancing, insertion of sorted data produces a linked-list tree. Applications require balancing strategies.

**Use cases**: Underlying structure for self-balancing variants; conceptual foundation for understanding more complex trees.

## AVL Trees

Self-balancing BST via **rotations**. Each node stores a balance factor: height of left subtree minus height of right subtree. The invariant: all balance factors are in $\{-1, 0, 1\}$.

**Height bound**: $h \leq 1.44 \log_2(n)$ — more tightly balanced than red-black trees.

**Rotations**: Four cases (left-left, left-right, right-left, right-right) restore balance after insertion or deletion. Each rebalancing involves at most $O(\log n)$ rotations; however, insertion typically triggers 0-1 rotations, deletion can require multiple.

**Trade-offs**:
- Search: $O(\log n)$ guaranteed
- Insertion/Deletion: $O(\log n)$ with frequent rotations; more overhead per operation than red-black
- Useful when **searches dominate** and tree structure rarely changes

## Red-Black Trees

Self-balancing BST with **color property**: each node is colored red or black; constraints maintain near-balance.

**Invariants**:
1. Root is black
2. All leaves (NIL) are black
3. If a node is red, its children are black
4. All paths from node to descendants have the same number of black nodes

**Height bound**: $h \leq 2 \log_2(n + 1)$; relaxed compared to AVL (looser balance) but sufficient for $O(\log n)$ operations.

**Rebalancing**: At most **3 rotations for insertion, 3 for deletion** (amortized). Fewer rebalances than AVL; more efficient for write-heavy workloads.

**Trade-offs**:
- Less tightly balanced than AVL; acceptable since $O(\log n)$ is preserved
- Fewer rotations per operation → lower insertion/deletion cost
- Standard in sorted containers: `std::map` (C++), `TreeMap` (Java), `SortedDict` (Python)

**AVL vs Red-Black**: AVL is tighter balance (better search) + more rebalancing (worse insertion/deletion). Red-black trades balanced height for fewer writes. Red-black typically wins in practice for mixed workloads.

## B-Trees

Multi-way balanced trees: each node has 0 to many children and multiple keys. Designed for disk-based storage (magnetic disk seeks are expensive; read large blocks efficiently).

**Properties**:
- All leaves at the same depth
- Each non-root node has at least $\lceil m/2 \rceil$ children (minimum branching factor)
- Number of keys = number of children - 1
- Degree $t$ (max children): non-root nodes have up to $2t - 1$ keys

**Search**: $O(\log_t n)$ where $t$ is the branching factor; fewer tree traversals than binary trees.

**Insertion/Deletion**: $O(\log n)$ with child node splitting/merging. Splitting a full node: move median key up, split children. Merging reduces depth gradually.

**Advantages**: Minimizes disk block reads/writes; heavily used in databases (B-tree indexes) and filesystems (ext4, NTFS). Branching factor tuned to disk block size (e.g., $t = 256$ for 4KB blocks).

**Disadvantage**: immense number of keys within a single node → larger code complexity vs binary trees.

## B+ Trees

Variant of B-tree: **internal nodes store keys only; all data is in leaf nodes**. Leaves form a linked list for range scan efficiency.

**Advantages**:
- All keys accessible via single leaf list traversal (full table scans efficient)
- Internal nodes purely for navigation, smaller overhead
- Better suited for databases (range queries common)
- Sequential access pattern optimizes disk cache

Used in most production databases (PostgreSQL B-tree indexes, MySQL, SQLite) over plain B-trees.

## Tries (Prefix Trees)

Tree structure for **storing and searching strings** efficiently. Each node represents a prefix; edges labeled with characters. Root represents empty prefix.

**Operations**:
- Search word: $O(m)$ where $m$ is word length (independent of number of keys)
- Insertion: $O(m)$
- Deletion: $O(m)$

**Space**: $O(k \cdot m \cdot \sigma)$ where $k$ is number of keys, $m$ is average length, $\sigma$ is alphabet size. Unlike hash tables, tries don't require hash function; support ordered traversal and prefix search.

**Use cases**: autocomplete, spell-checking, IP routing (longest prefix match), string matching within a dictionary.

## Compressed Tries (Patricia Trees, Radix Trees)

Standard tries can be sparse (many nodes with single children). **Compression** collapses chains of single-child nodes into edges labeled with multi-character strings.

**Benefit**: $O(k)$ space for $k$ keys (independent of length), compared to $O(k \cdot m)$ for uncompressed tries. Lookup still $O(m)$ in worst case (scanning the string).

**Trade-off**: insertion/deletion slightly complex (may split edge labels).

## Skip Lists

A probabilistic data structure: **sorted linked list with multiple levels of "express" links**. Each node has a random height; level $i$ pointers skip approximately $2^i$ nodes.

**Operations**: search, insert, delete all $O(\log n)$ **with high probability** (not deterministic). Similar to balanced BSTs in complexity but simpler to implement (no rotations).

**Advantages**:
- Concurrent access patterns friendlier than rotations (thread-safe Skip lists exist)
- Simplicity of implementation
- More stable performance than randomly-chosen AVL insertions

**Disadvantage**: randomized height → non-deterministic worst case (though $O(n)$ worst-case is vanishingly rare).

**Use case**: Redis sorted sets, concurrent data structures.

## Splay Trees

Self-adjusting BST: when a node is accessed (searched, inserted), **rotate it to the root** via a series of splaying operations.

**Amortized complexity**: $O(\log n)$ amortized for any sequence of operations (despite possible $O(n)$ individual operations).

**Advantage**: recently accessed items are near the root → good temporal locality. Competitive with balanced trees on realistic workloads.

**Disadvantage**: splaying is expensive; worst-case single operation $O(n)$. Unpopular in practice.

## Treaps (Treap = Tree + Heap)

Hybrid: BST with respect to keys; heap with respect to random priorities stored at each node. Insertion chooses random priority; rotations maintain heap property.

**Properties**:
- Search: $O(\log n)$ expected (BST structure)
- Insertion/Deletion: $O(\log n)$ expected
- Simpler to implement than red-black trees
- Randomization avoids adversarial worst-case inputs

**Trade-off**: non-deterministic (but expected $O(\log n)` suffices for most applications).

## Segment Trees

Binary tree structure over an array; each node represents a range $[l, r]$ and stores an aggregate (sum, min, max, etc.) over that range.

**Operations**:
- Build: $O(n)$
- Range query: $O(\log n)$
- Point update: $O(\log n)$
- Range update (with lazy propagation): $O(\log n)` amortized

**Use**: competitive programming, computational geometry (range minimum queries), dynamic array statistics.

## Fenwick Tree (Binary Indexed Tree)

Space-efficient alternative to segment trees for range sum queries and point updates. Uses implicit tree structure within an array.

**Operations**: point update $O(\log n)`, range query $O(\log n)`. Simpler to code than segment trees; lower constant factors.

**Limitation**: only supports semigroup operations (associative binary ops); less flexible than segment trees for range updates and complex aggregates.

## Interval Trees

BST augmented with interval endpoints: each node stores an interval and the maximum endpoint of all intervals in its subtree. Allows efficient **stabbing queries** ("find all intervals overlapping a point").

**Operations**: $O(\log n)$ search, insertion, deletion.

**Use cases**: calendar/scheduling systems, collision detection (game engines).

## Comparison Table

| Structure | Search | Insert | Delete | Space | Best For |
|-----------|--------|--------|--------|-------|----------|
| BST | $O(h)$ | $O(h)$ | $O(h)` | $O(n)$ | Educational only |
| AVL | $O(\log n)$ | $O(\log n)$ | $O(\log n)` | $O(n)$ | Search-heavy |
| Red-Black | $O(\log n)` | $O(\log n)` | $O(\log n)` | $O(n)` | Balanced inserts/searches |
| B-tree | $O(\log n)` | $O(\log n)` | $O(\log n)` | $O(n)` | Disk I/O (databases) |
| B+ tree | $O(\log n)` | $O(\log n)` | $O(\log n)` | $O(n)` | Databases, range scans |
| Trie | $O(m)` | $O(m)` | $O(m)` | $O(k \cdot m)$ | Prefix search, autocomplete |
| Radix | $O(m)` | $O(m)` | $O(m)` | $O(k)` | Compact prefix storage |
| Skip List | $O(\log n)$ prob | $O(\log n)` prob | $O(\log n)` prob | $O(n)` | Concurrent access |
| Splay | $O(\log n)$ amort | $O(\log n)` amort | $O(\log n)` amort | $O(n)$ | Temporal locality |
| Treap | $O(\log n)` exp | $O(\log n)` exp | $O(\log n)` exp | $O(n)` | Randomized structure |
| Segment | N/A (range query) | $O(\log n)` | N/A | $O(n)` | Range aggregation |
| Fenwick | $O(\log n)` | $O(\log n)` | N/A | $O(n)` | Efficient range sums |
| Interval | $O(\log n)` | $O(\log n)` | $O(\log n)` | $O(n)` | Stabbing queries |

## See Also

algorithms-graph (trees as graphs), algorithms-dynamic-programming (tree DP), database-internals-storage (B-trees in databases), algorithms-string (tries), data-structures-algorithms