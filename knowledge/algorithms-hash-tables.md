# Hash Tables — Functions, Collision Resolution, Load Factors & Advanced Variants

Hash tables provide near-constant average-case lookup, insertion, and deletion by mapping keys to array indices via a hash function. Performance depends critically on function quality, collision strategy, and load factor management.

## Hash Functions

A hash function $h(k)$ maps a key $k$ from universe $U$ to slot indices $0 \ldots m-1$, where $m$ is the table size. Poor functions cause clustering; good ones distribute keys uniformly.

### Division Method

$$h(k) = k \bmod m$$

Simple and fast. Quality depends heavily on $m$. Powers of 2 (e.g., $m = 2^p$) are **problematic**: the function uses only the lowest $p$ bits, discarding higher bits and potentially revealing patterns in key distributions. Choosing $m$ as a prime not near powers of 2 reduces collisions. Widely used in practice despite inferior theoretical properties compared to multiplication.

### Multiplication Method

$$h(k) = \lfloor m \cdot (k \cdot A \bmod 1) \rfloor$$

where $A$ is a constant, typically $0 < A < 1$ (e.g., $A \approx 0.618$ derived from the golden ratio). Less sensitive to $m$'s value (works with any $m$); the quality depends on $A$. More uniform distribution than division. Knuth recommended this for better empirical performance.

### Universal Hashing

Fixes the pathological case: a fixed hash function has keys that always collide (adversary can construct inputs to trigger worst-case behavior). Universal hash families select a function **uniformly at random** from a family of functions. For any two distinct keys $x, y$:

$$\Pr[h(x) = h(y)] \leq \frac{1}{m}$$

Reduces collision probability to random baseline regardless of input. Example: $h_a(k) = ((ak + b) \bmod p) \bmod m$ where $p$ is prime, $a \neq 0$, and $a, b$ chosen randomly. Adds randomization overhead but bounds worst-case behavior probabilistically.

### Cryptographic Hash Functions

Functions like SHA-256 produce "random-looking" outputs with avalanche properties: changing a single input bit changes (on average) half of output bits. Used when collision resistance matters (security, integrity). Expensive (many CPU cycles) relative to simple arithmetic functions. Overkill for in-memory hash tables; primarily used for fingerprinting and security.

## Collision Resolution: Chaining

Each table slot contains a pointer to a linked list (chain) of all keys mapping to that slot. On collision, insert at the head or tail of the chain; search may traverse the chain.

**Properties**:
- Simple to implement; handles load factor > 1 gracefully
- Search, insert, delete: $\Theta(1 + \alpha)$ average time, where $\alpha = n/m$ is the load factor (average chain length)
- $\alpha$ can exceed 1 without performance cliff
- Memory overhead from pointers per node; poor cache locality (following chains in memory)
- Deletion is natural (unlink node)

**Worst case**: all $n$ keys hash to one slot → $\Theta(n)$ per operation. Occurs with fixed bad hash functions or adversarial input; mitigated by universal hashing or randomized pivot selection in hash computation.

**Practical use**: Hash maps in Java, Python, Go use chaining (with variants like balanced trees for long chains in Java 8+).

## Collision Resolution: Open Addressing

All entries stored directly in the table array (no separate chains). Collision is resolved by probing for an alternative empty slot.

### Linear Probing

$$h(k, i) = (h(k) + i) \bmod m$$

If slot $h(k)$ is occupied, try $(h(k) + 1) \bmod m$, then $(h(k) + 2) \bmod m$, etc. Simple but suffers from **primary clustering**: sequences of occupied slots grow, causing long probe chains. Search time becomes $\Theta\left(\frac{1}{(1-\alpha)^2}\right)$ as $\alpha \to 1$; performance degrades rapidly above $\alpha = 0.5$.

### Quadratic Probing

$$h(k, i) = (h(k) + i^2) \bmod m$$

Reduces clustering compared to linear probing but causes **secondary clustering** (different keys traversing similar probe sequences). Requires $m$ to be prime for theoretical guarantees of probing all slots.

### Double Hashing

$$h(k, i) = (h_1(k) + i \cdot h_2(k)) \bmod m$$

Two independent hash functions; more uniform probe sequences. Requires $h_2(k)$ to be coprime to $m$ (e.g., $m$ prime enforces this). Better distribution than quadratic but more complex. Probing time remains $\Theta\left(\frac{1}{1-\alpha}\right)$ — performance degrades gracefully with load factor.

## Advanced Collision Strategies

### Robin Hood Hashing

An open-addressing variant that minimizes probe sequence length variance. Insertion "robs" entries: if a new key $x$ would probe farther than an already-stored key $y$, **evict $y$** and re-insert it (finding a closer slot), displacing $x$ further if needed.

**Benefits**: Reduces worst-case probe length; nearly uniform performance across load factors up to ~0.95. Better cacheable than linear probing at high load. Used in modern systems prioritizing latency predictability.

### Cuckoo Hashing

Uses two independent hash functions $h_1, h_2$. Each key has two possible slots. Insertion inserts at $h_1$; if occupied, attempts to move the resident key to its alternative slot (via $h_2$), possibly triggering a cascade of displacements. If a cycle is detected (key displaced multiple times), rebuild the table with new hash functions.

**Properties**: Worst-case $O(1)$ lookup and deletion (key is at one of two slots). Insertion may be slow (cascading displacements, rebuilds) but amortized complexity is good. Works well up to $\alpha \approx 0.5$. Variant: **bucket cuckoo hashing** uses small buckets per slot, increasing capacity.

### Bloom Filters

A space-efficient **approximate membership test** (not a general hash table). Uses $k$ independent hash functions; insert sets $k$ bits; lookup checks if all $k$ bits are set. False positives possible (a key not in the set may pass the test), but false negatives impossible.

Space: $O(n)$ bits for $n$ elements, independent of key/value size. Lookup: $O(k)$. Set operations and support for deletion (with variants) are limited. Practical use: caches, network routing, deduplication.

### Count-Min Sketch

A probabilistic data structure for frequency estimation on streaming data. Uses an $m \times k$ array of counters and $k$ independent hash functions. Increment: for each of $k$ functions, increment counter at [hash(key)]. Query: returns the minimum count across $k$ hashes (worst-case estimate).

Space: $O(mk)$ bits. Guarantees: if true frequency is $f$, estimate is at most $f + (\epsilon \cdot N)$ with probability $1 - \delta$, where $\epsilon, \delta$ are configurable trade-offs via $m, k$.

## Perfect Hashing

Constructs a hash function such that there are **no collisions** on a static key set (achieved in practice). Two-level approach: first-level hash function distributes keys to buckets; second-level hash functions handle each bucket independently. With $O(n)$ space and random hash functions, achieves $O(1)$ lookup.

Practical construction expensive (preprocessing); useful when query volume justifies setup cost or space budget is limited and queries dominate.

## Load Factor & Resizing

**Load factor** $\alpha = n / m$ (entries per slot) controls collision frequency and thus performance.

- **Chaining**: Performance degrades linearly with $\alpha$; $\alpha > 1$ is acceptable
- **Open addressing**: Should keep $\alpha < 0.5 - 0.75$ to avoid probe length explosion
- **Resizing strategy**: Common is to double table size when $\alpha$ exceeds a threshold. Rebuilds cost $\Theta(n)$ but are infrequent (amortized $O(1)$ per insertion)

Amortized analysis: if resize threshold is constant (e.g., $\alpha = 0.75$), the expected cost per insertion is $O(1)$ despite occasional $O(n)$ resizes.

## Concurrent Hash Maps

**Lock-free or fine-grained locking** enables safe concurrent access. Challenges:
- **Resizing**: Must not break concurrent readers; often deferred or staged (readers may use old table)
- **Deletion**: Mark-and-sweep or lazy deletion to avoid synchronization overhead
- Examples: Java `ConcurrentHashMap` (segments with bucket locks), C++ `std::unordered_map` (not thread-safe; user responsible)

## Amortized Analysis

Hash table operations (insert, delete, search) have **expected** $O(1)$ time with a good hash function and reasonable load factor. Resizing adds $O(1)$ amortized cost. In adversarial settings (weak hash function, adversarial input), worst-case degrades to $O(n)$; mitigated by universal hashing.

## Trade-offs

| Aspect | Chaining | Linear Probing | Double Hashing | Robin Hood | Cuckoo | Bloom Filter |
|--------|----------|---|---|---|---|---|
| Insertion | $O(1)$ avg | $O(1/(1-\alpha))$ | $O(1/(1-\alpha))$ | $O(1/(1-\alpha))$ | $O(1)$ amort, high worst | $O(k)$ |
| Lookup | $O(1)$ avg | $O(1/(1-\alpha))$ | $O(1/(1-\alpha))$ | $O(1/(1-\alpha))$ | $O(1)$ worst | $O(k)`, false pos possible |
| Space | $O(n + m)$ | $O(m)` | $O(m)$ | $O(m)$ | $O(n)` | $O(nk/m)` |
| Load factor | can >1 | keeps <1 | keeps <0.9 | keeps <0.95 | ~0.5 | N/A |
| Deletion | easy | complex (tombstones) | complex | complex | hard | no remove |
| Cache locality | poor (chains) | good | good | good | variable | good |

## See Also

data-structures-algorithms, algorithms-sorting, database-internals-storage (B-trees use hashing for page buffers)