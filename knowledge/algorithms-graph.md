# Graph Algorithms — Traversal, Shortest Paths, Connectivity, Flow

Graph algorithms are among the most practical in computer science: they model networks, dependencies, reachability, and resource allocation. Most reduce to a small set of core techniques: depth-first and breadth-first search, then layered solutions atop them.

## Core Traversals

### Depth-First Search (DFS) — O(V + E)

Explores as far as possible along one branch before backtracking. Builds a DFS tree rooted at the source.

- **Stack-based** (explicit or via recursion)
- **Time**: O(V + E) where V = vertices, E = edges
- **Space**: O(V) for call stack depth
- **Applications**: topological sort, cycle detection, connected components, strongly connected components

Variant: **Iterative DFS** avoids recursion depth issues on dense graphs.

### Breadth-First Search (BFS) — O(V + E)

Explores all neighbors before moving deeper. Builds a BFS tree; finds **shortest path in unweighted graphs**.

- **Queue-based**
- **Time**: O(V + E)
- **Space**: O(V) for the queue
- **Applications**: shortest unweighted paths, multi-source BFS, level-order traversal

**Multi-source BFS**: Start from multiple source vertices simultaneously; finds distance to nearest source for each vertex.

## Shortest Path Algorithms

### Dijkstra's Algorithm — O((V + E) log V) with min-heap, O(V²) with array

Greedy algorithm for shortest paths from a source to all vertices. **Requires non-negative edge weights**.

```
dist[source] = 0
all others = infinity
while vertices remain:
    u = unvisited vertex with min dist
    mark u visited
    for each neighbor v of u:
        if dist[u] + weight(u,v) < dist[v]:
            dist[v] = dist[u] + weight(u,v)
```

- **Optimal substructure**: shortest path is made of shortest sub-paths
- **Greedy**: always pick nearest unvisited vertex
- **Correctness**: fails on negative weights (no counterexample needed; algorithm is fundamentally greedy)
- **Efficiency**: Binary heap: O((V + E) log V); Fibonacci heap: O(E + V log V) theoretical but rarely used

Most implementations use a min-heap priority queue.

### Bellman-Ford — O(VE)

Relaxes all edges V-1 times. Works with **negative weights** and detects negative cycles.

```
dist[source] = 0
for i = 1 to |V|-1:
    for each edge (u,v,w):
        if dist[u] + w < dist[v]:
            dist[v] = dist[u] + w
check for negative cycle (one more pass)
```

- **Slower than Dijkstra** but handles negative weights
- **Detects negative cycles** (checked in a Vth iteration)
- **All-pairs variant**: Run from each source = O(V²E)

### Floyd-Warshall — O(V³)

Computes all-pairs shortest paths even with negative weights (but no negative cycles).

```
dist[i][j] = weight(i,j)
for k = 0 to V-1:
    for i = 0 to V-1:
        for j = 0 to V-1:
            dist[i][j] = min(dist[i][j], dist[i][k] + dist[k][j])
```

- **Simple dynamic programming**: "What if we're allowed to use vertices 0..k as intermediates?"
- **O(V³) time and O(V²) space** — only practical for small graphs (~500 vertices)
- **Handles negative edges** but not negative cycles

### A* Search — O(E) to O(V²) depending on heuristic

Informed best-first search using a heuristic estimate h(v) of distance to goal.

```
f(v) = g(v) + h(v)    // actual cost + estimated remaining
process vertex with min f(v)
```

- **Optimal if h is admissible** (never overestimates true distance)
- **Faster than Dijkstra** if h is informative
- **Applications**: pathfinding in games/robotics, GPS routing
- **Heuristic examples**: Euclidean distance (for geometric graphs), Manhattan distance (for grids)

## Connectivity & Components

### Strongly Connected Components (SCCs) — O(V + E)

In a directed graph, an SCC is a maximal set of vertices where every vertex reaches every other.

**Kosaraju's Algorithm**:
1. DFS on original graph, record finish times
2. DFS on transpose graph in decreasing finish time order
3. Each DFS tree in step 2 is one SCC

**Tarjan's Algorithm** (single pass):
- Uses a stack and low-link values (lowest finish time reachable)
- More cache-friendly than Kosaraju
- Same O(V + E) time

Applications: dependency resolution, circuit connectivity, web crawling (finding strongly connected components of web graph).

### Topological Sort — O(V + E)

Orders vertices of a directed acyclic graph (DAG) such that for every edge u→v, u comes before v.

- **Kahn's algorithm**: Remove vertices with in-degree 0 repeatedly
- **DFS-based**: Order by decreasing finish times
- **Applications**: task scheduling, build dependencies, course prerequisites

Fails on cyclic graphs (no valid topological order).

## Minimum Spanning Trees (MST)

An MST connects all V vertices with V-1 edges, minimizing total edge weight, forming a tree (no cycles).

### Kruskal's Algorithm — O(E log E)

1. Sort edges by weight
2. Use union-find to build tree: add edge if it doesn't create a cycle
3. Stop after V-1 edges

**Advantages**:
- Simple greedy: pick lightest edge that doesn't cycle
- Works well on sparse graphs
- Easily parallelizable (sort then greedy subset selection)

### Prim's Algorithm — O(E log V) with heap, O(V²) with array

1. Start from arbitrary vertex
2. Grow tree by adding minimum-weight edge to unvisited vertex
3. Repeat until all V vertices included

**Advantages**:
- Dense graph performance better than Kruskal when E ≈ V²
- Single priority queue instead of sorting all edges
- Produces spanning tree of specific connected component

## Maximum Flow

**Ford-Fulkerson method** (O(E × max_flow) or O(VE²) with BFS):

1. Find augmenting path from source to sink
2. Increase flow along that path by its bottleneck capacity
3. Repeat until no path exists

**Variants**:
- **Edmonds-Karp**: Use BFS for path selection → O(VE²)
- **Dinic's algorithm**: O(V²E) using level graphs
- **Push-relabel**: O(V²E) or O(V³) with heuristics

Applications: network routing, bipartite matching, airline scheduling.

## Graph Coloring

Assign colors (labels) to vertices such that no two adjacent vertices share a color. **NP-hard in general** but efficient heuristics exist.

- **Greedy coloring**: O(V + E); greedy but not optimal
- **Welsh-Powell**: Sort vertices by degree, then greedy → often better
- **Applications**: register allocation, frequency assignment, sudoku

## Practical Applications

| Problem | Algorithms |
|---------|-----------|
| Social network analysis | BFS, DFS, SCC detection |
| Road networks / GPS | Dijkstra, A*, bidirectional search |
| Task scheduling | Topological sort, DAG critical path |
| Network routing | Maximum flow, shortest path |
| Compiler optimization | Topological sort, dominance, CFG |
| Recommendation systems | Shortest paths, community detection |

---

**See also**: data-structures-algorithms, math-graph-theory, math-complexity-theory