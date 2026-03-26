# Union-Find (Disjoint Set) — Near-Optimal Equivalence Class Tracking

Union-Find is an elegant data structure for maintaining partitions (equivalence classes) of elements, supporting near-constant-time union and find operations. It solves classic problems: connected components, Kruskal's MST, and percolation theory.

## Concept: Partitions and Representatives

A partition divides a universe of $n$ elements into disjoint subsets. Union-Find maintains this partition and answers two queries:

- **Find(x)**: Which subset does element x belong to? Returns a representative.
- **Union(x, y)**: Merge the subsets containing x and y.

Initially, each element is in its own singleton set $\{x\}$. Operations gradually merge sets.

**Representative property**: Each set has a canonical element (representative). Find(x) returns this element; if Find(x) = Find(y), then x and y are in the same set.

## Naive Approach: Poor Performance

**Array-based labeling**:
```
parent[i] = i  (initially)

find(x):
    return parent[x]

union(x, y):
    rep_x = find(x)
    rep_y = find(y)
    for i = 0 to n-1:
        if parent[i] == rep_x:
            parent[i] = rep_y
```

- **Find**: $O(1)$
- **Union**: $O(n)$ (relabel all elements in one set)
- **Total for $m$ operations**: $O(m \cdot n)$ — unacceptable.

## Union by Rank: First Improvement

Observation: Instead of relabeling all elements, point the smaller set's root to the larger set's root. This keeps tree depth small.

```
rank[i] = 0 initially (tree height)

find(x):
    if parent[x] != x:
        parent[x] = find(parent[x])  ← Path compression (see below)
    return parent[x]

union(x, y):
    root_x = find(x)
    root_y = find(y)
    if root_x == root_y: return  (already united)
    if rank[root_x] < rank[root_y]:
        parent[root_x] = root_y
    elif rank[root_x] > rank[root_y]:
        parent[root_y] = root_x
    else:
        parent[root_y] = root_x
        rank[root_x] += 1
```

**Result**: With union by rank alone, $m$ operations cost $O(m \log n)$ because tree depth is $O(\log n)$.

## Path Compression: Second Improvement

When finding the representative of x, recursively compress the path: every node along the way points directly to the root.

```
find(x):
    if parent[x] != x:
        parent[x] = find(parent[x])  ← Recursive compression
    return parent[x]
```

First call to find(x) traverses $O(\text{depth})$ nodes. Subsequent calls traverse 1-2 nodes (direct to root).

**Example**: 
```
Before: x → a → b → c → root
After:  x → root, a → root, b → root, c → root
```

Path compression doesn't change rank; it only updates parent pointers to bypass intermediate nodes.

## Combined: Rank + Path Compression

Using both optimizations together achieves near-linear performance.

**Complexity**: $m$ find and union operations cost $O(m \cdot \alpha(n))$ where $\alpha(n)$ is the **inverse Ackermann function**.

$$\alpha(n) = \min \{ k : A_k(1) \geq n \}$$

Inverse Ackermann is **slower than $\log \log \log n$** for all practical values of $n$:
- $\alpha(10^6) \approx 4$
- $\alpha(10^{10}) \approx 5$
- $\alpha(10^{100}) \approx 6$

**Practical view**: $O(m \alpha(n))$ is effectively $O(m)$ for any problem size encountered in practice.

## Comparison of Approaches

| Approach | Find | Union | m operations |
|----------|------|-------|--------------|
| Naive | O(1) | O(n) | O(m·n) |
| Union by rank | O(log n) | O(log n) | O(m·log n) |
| Path compression | O(?)† | O(1) amortized | O(m·log n) amortized |
| Both | O(α(n)) | O(α(n)) | O(m·α(n)) |

† Path compression without rank: single worst-case chain still possible; amortized analysis shows average improves sharply.

## Weighted Quick-Union Variant

Instead of tracking rank (tree height), track **size** of each set. Unite the smaller set under the larger.

```
size[i] = 1 initially

union(x, y):
    root_x = find(x)
    root_y = find(y)
    if root_x == root_y: return
    if size[root_x] < size[root_y]:
        parent[root_x] = root_y
        size[root_y] += size[root_x]
    else:
        parent[root_y] = root_x
        size[root_x] += size[root_y]
```

**Guarantee**: Depth is always $O(\log n)$ because each unite roughly doubles the set size. Combined with path compression, also achieves near-linear performance.

**When to use weighted quick-union**: Easier to reason about than rank-based union; size naturally represents set cardinality (useful if you query set size).

## Applications

### Connected Components

Given graph vertices and edges, partition vertices into connected components. Union(u, v) for each edge (u, v); sets become components.

**Complexity**: $O((V + E) \cdot \alpha(V))$ — linear in graph size.

### Kruskal's Minimum Spanning Tree

Sort edges by weight. For each edge (u, v):
- If Find(u) ≠ Find(v): add edge to MST, union(u, v)
- Else: skip (would create cycle)

**Complexity**: $O(E \log E)$ for sorting; $O(E \cdot \alpha(V))$ for union-find operations.

### Percolation Theory

Given an $n \times n$ grid, mark cells as open or closed. Two cells are connected if both are open and adjacent. When a path connects top to bottom, the system "percolates."

Union-Find rapidly determines percolation: union(top_row, bottom_row) — if Find returns true, percolation exists.

Applications: fluid flow through porous media, disease spread, electrical networks.

### Equivalence Classes

Determine if statements like "$a = b$, $b = c$, $c = d$" are consistent and what equivalences hold. Union-Find partitions variables; find(a) = find(b) means variables are equivalent.

## Implementation Notes

**Path compression**: Recursive implementation is elegant but risks stack overflow for very deep trees (rare after rank + compression). Iterative version exists.

**Size vs. rank**: Rank is an upper bound (actual depth after compression). For code clarity, size often preferred.

**Initialization**: $O(n)$ to initialize parent and rank/size arrays.

**Immutable variant**: For functional programming, union-find is mutable. Persistent variants exist (store version history) but sacrifice efficiency.

## When to Use Union-Find

**Ideal for**:
- Determining connected components in dynamic graphs
- Checking for cycles in undirected graphs
- Grouping elements by equivalence relation
- Kruskal's MST algorithm
- Percolation problems
- Offline connectivity queries (preprocess operations then answer)

**Not ideal for**:
- Finding paths between vertices (use BFS/DFS instead)
- Maintaining dynamic connectivity in directed graphs (not supported)
- Weighted equivalence (if equivalence has degrees or costs)

## Relationship to Other Structures

Union-Find is complementary to:
- **Graph algorithms**: BFS/DFS answer reachability; union-find answers connectivity more efficiently for offline queries
- **Sorting**: Kruskal uses sorting (edges) + union-find (forest construction)
- **Segment trees**: Both are forest-like; segment trees support range queries, union-find supports equivalence

See also: [algorithms-graph.md](algorithms-graph.md), [algorithms-sorting.md](algorithms-sorting.md), [math-complexity-theory.md](math-complexity-theory.md)