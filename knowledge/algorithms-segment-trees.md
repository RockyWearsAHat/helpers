# Segment Trees — Hierarchical Range Query Structure

Segment trees efficiently answer range queries (minimum, maximum, sum) over dynamic arrays and support point updates in $O(\log n)$ time. They trade space for speed, enabling applications impossible with naive array algorithms.

## Core Concept

A segment tree is a full binary tree where:
- **Leaf nodes** represent individual array elements
- **Internal nodes** store aggregate values (sum, min, max, etc.) of their children's ranges
- **Root** represents the aggregate over the entire array

Each node represents a range [L, R] and stores an aggregate value computed from its children.

**Mental model**: Think of building a hierarchy of partial results. To query a range, combine cached results from the tree rather than scanning the array.

## Structure

For array of size $n$:
- Tree has $n$ leaves (one per element)
- Tree has $n - 1$ internal nodes
- Total nodes: $2n - 1$ (approximately)
- Height: $O(\log n)$

Implementation uses array indexing:
```
For node at index i:
    left_child = 2*i
    right_child = 2*i + 1
    parent = i/2
```

Array size reserved: $2^{\lceil \log_2 n \rceil + 1} - 1$ (next power of 2 above $2n$).

## Range Query (Point Update)

**Query(L, R)**: Aggregate of elements [L, R]?

```
query(node, node_L, node_R, L, R):
    if node_L > R or node_R < L:
        return IDENTITY  (out of range)
    if L <= node_L and node_R <= R:
        return tree[node]  (completely covered)
    mid = (node_L + node_R) / 2
    left_sum = query(left_child, node_L, mid, L, R)
    right_sum = query(right_child, mid + 1, node_R, L, R)
    return combine(left_sum, right_sum)
```

**Complexity**: $O(\log n)$ — recurses down one path, visiting $O(\log n)$ nodes.

**Update(idx, value)**: Set array[idx] = value, update all affected ancestors.

```
update(node, node_L, node_R, idx, value):
    if node_L == node_R:
        tree[node] = value
        return
    mid = (node_L + node_R) / 2
    if idx <= mid:
        update(left_child, node_L, mid, idx, value)
    else:
        update(right_child, mid + 1, node_R, idx, value)
    tree[node] = combine(tree[left_child], tree[right_child])
```

**Complexity**: $O(\log n)$ — updates $O(\log n)$ ancestors.

## Lazy Propagation: Efficient Range Updates

Standard segment trees support point queries and updates efficiently. For **range updates** ("add 5 to all elements in [L, R]"), naive approach requires $O((R - L) \log n)$ updates—slow.

**Lazy propagation** defers updates: mark a node with a pending operation; propagate only when needed.

```
update_range(node, node_L, node_R, L, R, delta):
    if R < node_L or L > node_R:
        return  (out of range)
    if L <= node_L and node_R <= R:
        lazy[node] += delta  (mark for later)
        tree[node] += delta * (node_R - node_L + 1)
        return
    push_down(node)  (propagate lazy to children)
    mid = (node_L + node_R) / 2
    update_range(left_child, node_L, mid, L, R, delta)
    update_range(right_child, mid + 1, node_R, L, R, delta)
    tree[node] = combine(tree[left_child], tree[right_child])

push_down(node):
    if lazy[node] != 0:
        lazy[left_child] += lazy[node]
        lazy[right_child] += lazy[node]
        tree[left_child] += lazy[node] * (size_left)
        tree[right_child] += lazy[node] * (size_right)
        lazy[node] = 0
```

**Complexity**: Range update and query both $O(\log n)$.

**Tradeoff**: Adds complexity; only use when range updates are frequent.

## Fenwick Tree (Binary Indexed Tree)

A simpler alternative to segment trees for prefix sums and point updates.

**Structure**: Array where index $i$ stores sum of elements in range $[i - (i \& -i) + 1, i]$ (parent of $i$ is $i + (i \& -i)$).

```
update(idx, delta):
    while idx <= n:
        tree[idx] += delta
        idx += idx & (-idx)

prefix_sum(idx):
    sum = 0
    while idx > 0:
        sum += tree[idx]
        idx -= idx & (-idx)
    return sum

range_sum(L, R):
    return prefix_sum(R) - prefix_sum(L - 1)
```

**Complexity**: $O(\log n)$ per operation; same as segment trees.

**Advantages**:
- Simpler code; fewer edge cases
- Lower memory; array of size $n$ vs. array of size $2n$
- Faster in practice (cache locality; fewer pointer dereferencing)

**Disadvantages**:
- Only binary operation (sum); generalizes poorly to min/max
- Range updates require different logic (treat as range sum updates)
- Less intuitive structure

**When to use**: Prefix sums and point updates only. For min/max or complex aggregates, segment trees are clearer.

## Persistent Segment Trees

Immutable version: each update creates a new version without modifying the old.

**Structure**: Share unchanged subtrees between versions using structural sharing.

```
update_persistent(node, idx, value, version):
    if leaf:
        new_node = Node(value)
    else:
        new_node = Node()
        mid = (node.L + node.R) / 2
        if idx <= mid:
            new_node.left = update_persistent(node.left, idx, value, version)
            new_node.right = node.right  (share from old)
        else:
            new_node.left = node.left
            new_node.right = update_persistent(node.right, idx, value, version)
    return new_node
```

**Complexity**: $O(\log n)$ time and space per update (allocate $O(\log n)$ new nodes).

**Use cases**: Version control of arrays, historical queries ("sum at time t?"), branching timelines.

## Merge Sort Tree

Variant for answering "count elements in range [L, R] with value in [V1, V2]" (2D range counting).

Each segment tree node stores a sorted list of values in its range. Query combines sorted lists efficiently.

**Complexity**: $O(\log^2 n)$ per query (log for tree depth, log for binary search in each sorted list).

## Two-Dimensional Segment Trees

Extend segment trees to 2D grids: build tree on rows, then for each row node, build tree on columns.

**Structure**: Tree of trees. Each row node contains a segment tree over column ranges.

**Query(row1, row2, col1, col2)**: Aggregate over 2D rectangle.

**Complexity**: $O(\log^2 n)$ for $n \times n$ grid.

**Advanced**: Persistent 2D segment trees enable time-travel queries over grid histories.

## Competitive Programming Applications

Segment trees are ubiquitous in competitive programming due to generality and efficiency:

- **Range min/max/sum queries with point updates**: Direct segment tree
- **Range updates with range queries**: Lazy propagation
- **Inversion count**: Merge sort tree variant
- **Dynamic LCA (Lowest Common Ancestor)**: Auxiliary tree + segment tree
- **Offline queries**: Build tree once, answer many queries

**Caveat**: Time spent coding and debugging a segment tree can exceed solutions using simpler data structures. Understand the problem constraints before reaching for this tool.

## When to Use Segment Trees

**Ideal for**:
- Frequent range queries + occasional point updates ($O(\log n)$ required)
- Support both range queries and range updates
- Complex aggregates (min, max, sum, gcd, bit operations)
- Competitive programming involving dynamic interval queries

**Alternative structures**:
- **Square root decomposition**: $O(\sqrt{n})$ per operation; simpler to code
- **Fenwick tree**: If prefix sums only
- **Simple array**: If queries are infrequent

**Not ideal for**:
- Static data (precompute prefix sums once)
- Queries only on sorted subsequences (precompute sorted array)
- Extreme space constraints (segment trees use $O(n)$ memory)

## Execution Considerations

**Implementation pitfalls**:
- Off-by-one errors in range boundaries (inclusive vs. exclusive)
- Forgetting to initialize lazy flags
- Combining operation should be symmetric (addition, min, max okay; subtraction not)

**Debugging**:
- Verify combining operation is correct: tree[node] = combine(tree[left], tree[right])
- Test boundary cases: single-element ranges, full array, empty ranges
- Print tree structure during small examples

See also: [algorithms-trees.md](algorithms-trees.md), [math-complexity-theory.md](math-complexity-theory.md), [algorithms-sorting.md](algorithms-sorting.md)