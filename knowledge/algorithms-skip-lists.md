# Skip Lists — Probabilistic Balanced Search Structure

Skip lists are a randomized data structure that maintain sorted sequences with expected $O(\log n)$ search, insertion, and deletion time while avoiding the complexity of tree rebalancing. They provide a simpler alternative to balanced binary search trees with comparable performance characteristics.

## Core Concept

A skip list is a layered linked list structure. The base level is an ordinary sorted linked list. Each higher level "skips" elements randomly, creating shortcuts that reduce traversal distance during search.

**Structure**:
- Level 0: complete sorted linked list of all $n$ elements
- Level 1: approximately $n/2$ elements
- Level 2: approximately $n/4$ elements
- Level $h$: approximately $n/2^h$ elements

Each element has a random height sampled from a geometric distribution: probability $p$ of advancing to the next level (typically $p = 0.5$).

**Mental model**: Think of skip lists as multiple sorted layers where higher layers act as express lanes, avoiding the need to traverse every element at the base level. Search jumps down only when overshooting the target.

## Search Algorithm

```
search(target):
    current = list.header
    for level from (max_level - 1) down to 0:
        while current.next[level] != null AND 
              current.next[level].key < target:
            current = current.next[level]
        if level == 0:
            if current.next[0].key == target:
                return current.next[0]
    return null
```

**Complexity**: Expected $O(\log n)$. At each level, we skip approximately $n/2$ elements; $O(\log n)$ levels needed.

**Why it works**: Skipping through express lanes (higher levels) reaches a zone quickly, then descending layers narrows the search space. Unlike binary search, this is online—no preprocessing or tree structure required beyond layer assignment.

## Insertion and Deletion

**Insertion**:
1. Search for position where element belongs
2. Generate random height (geometric distribution)
3. Insert element at all levels up to its height
4. Update forward pointers of predecessor elements

Expected $O(\log n)$ since search phase dominates.

**Deletion**:
1. Search for element
2. Remove it from all levels where it appears
3. Update forward pointers

Expected $O(\log n)$.

**Key property**: No rebalancing needed. Height is determined at insertion time via randomization; future insertions/deletions don't require restructuring the hierarchy.

## Comparison to Balanced Binary Search Trees

| Aspect | Skip List | Red-Black Tree |
|--------|-----------|----------------|
| Search | $O(\log n)$ expected | $O(\log n)$ worst-case |
| Insert/Delete | $O(\log n)$ expected | $O(\log n)$ with rebalancing |
| Rebalancing | None required | O(log n) rotations per op |
| Code complexity | Simpler, fewer cases | Complex (color rules, 7 rotations) |
| Cache locality | Linked list traversal; poor cache | Tree structure; better cache |
| Range queries | Natural (linked list); $O(k + \log n)$ for k elements | Requires iterator; $O(k + \log n)$ |
| Worst-case degenerate | Rare but possible (all tall elements first) | Impossible; invariants prevent |

**Tradeoff**: Skip lists trade guaranteed worst-case for simpler code. Red-black trees guarantee $O(\log n)$ always; skip lists achieve it \`\`on average'' unless adversarial element ordering occurs.

## Concurrent Skip Lists

Skip lists adapt well to concurrent environments. In single-threaded code, rebalancing trees is already overhead; under contention, readers compete with tree restructuring.

**Concurrent approach**:
- Readers acquire element-level locks only while traversing
- Writers lock a contiguous region (insert/delete range) before modifying
- Height assignments remain random and independent; no coordination needed for levels

**Advantage over concurrent balanced trees**: Concurrent rebalancing (rotation + subtree height updates) requires complex locking. Skip list height is immutable; only horizontal pointers change.

**Example**: Java's `ConcurrentSkipListMap` and `ConcurrentSkipListSet` use skip lists for this reason. Lock-free variants exist, using atomic operations on pointer updates.

## Space Trade-offs

Each element stores $h$ forward pointers where $h$ is its height.

**Expected space**: $\sum_{i=1}^{n} E[h_i] = \sum_{i=1}^{n} \frac{1}{1-p} \approx 2n$ for $p = 0.5$.

This is $2n$ pointers plus $n$ values—roughly 3x the memory of a simple array, but acceptable for maintaining order.

**Optimization**: Use a single height array outside each node (array of heights per level) rather than per-node dynamic allocation. Reduces fragmentation.

## Redis Sorted Sets Implementation

Redis implements sorted sets as skip lists paired with hash tables:
- Skip list: maintains score order for range queries (ZRANGE, ZCOUNT)
- Hash table: $O(1)$ member lookup (ZSCORE)

Search for a member: $O(\log n)$ skip list traversal to find leader node, $O(1)$ hash table for verification.

**Why Redis chose this**: Sorted sets need both rank-based range access (scores) and membership queries (members). Skip lists efficiently support range queries in sorted order; hash tables handle point lookups. Combined, they leverage strengths of each.

## When to Use Skip Lists

**Advantages**:
- Simpler to implement than AVL/red-black trees
- Natural support for concurrent readers/writers
- Excellent for range queries (sequential traversal)
- No rebalancing overhead
- Probabilistic height can be tuned (larger $p$ → deeper; smaller $p$ → shallower)

**Disadvantages**:
- Expected $O(\log n)$, not guaranteed; pathological element insertion order could degrade performance
- Pointer-heavy; cache-unfriendly compared to B-trees or dense arrays
- Not standard in typical language libraries (Java exception: java.util.concurrent)

**Contexts**:
- Implementing a sorted container with concurrent access (databases, in-memory stores)
- Teaching randomized data structures (simpler than AVL/RB trees)
- Applications needing frequent range scan and dynamic updates (immutable historical versions)
- When code simplicity is worth more than worst-case guarantees

## Variations

**Deterministic skip lists**: Height is not random but derived from bit patterns of keys. Removes randomization but loses elegance.

**Probabilistic level generation**: Geometric distribution is most common ($p = 0.5$ or $p = 1/3$), but other distributions exist; affects height distribution and average traversals per level.

**Lockless skip lists**: Atomic operations replace locks for concurrent access. Complex to implement correctly; used in low-latency systems requiring minimal blocking.

## Relationship to Other Structures

Skip lists lie in a spectrum:
- **Simpler than**: red-black trees, AVL trees (no rotation logic)
- **Related to**: layered graphs (skip layers are similar to overlay networks); B-trees at a conceptual level (both build hierarchies for fast access)
- **Distinct from**: hash tables (order preserved; no hashing); tries (string-specific; different insertion model)

See also: [algorithms-trees.md](algorithms-trees.md), [algorithms-randomized.md](algorithms-randomized.md), [algorithms-hash-tables.md](algorithms-hash-tables.md)