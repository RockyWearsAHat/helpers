# Graph Theory — Structures, Algorithms & Applications in Software

Graphs provide one of the most versatile abstractions in computer science: a set of **vertices** (entities) connected by **edges** (relationships). From dependency resolution in build systems to social network analysis, routing algorithms, and compiler optimizations, graph structures model problems where the relationships between entities are as important as the entities themselves.

## Fundamental Definitions

A graph G = (V, E) consists of a vertex set V and an edge set E. The vocabulary of graph theory captures structural properties precisely:

| Term                 | Meaning                                                                             |
| -------------------- | ----------------------------------------------------------------------------------- | --- | -------------- | --- | --- |
| Degree               | Number of edges incident to a vertex (in-degree and out-degree for directed graphs) |
| Path                 | Sequence of vertices where consecutive pairs are connected by edges                 |
| Cycle                | Path that starts and ends at the same vertex                                        |
| Connected            | Every pair of vertices has a path between them (undirected)                         |
| Strongly connected   | Every pair of vertices has a directed path in both directions                       |
| Component            | A maximal connected subgraph                                                        |
| Subgraph             | A graph formed from a subset of vertices and edges of another graph                 |
| Complete graph (K_n) | Every pair of vertices is connected                                                 |
| Sparse graph         |                                                                                     | E   | much less than | V   | ²   |
| Dense graph          |                                                                                     | E   | approaches     | V   | ²   |

## Graph Variants

Different problem domains call for different graph types:

- **Directed vs undirected** — Directed graphs (digraphs) have edges with orientation: (u, v) differs from (v, u). Dependency graphs, state machines, and web link structures are inherently directed. Social "friendship" networks are often undirected; "follower" networks are directed.
- **Weighted vs unweighted** — Weighted graphs assign values to edges representing cost, distance, capacity, or probability. Unweighted graphs (or equivalently, uniformly weighted) simplify certain algorithms but lose the ability to model heterogeneous relationships.
- **Cyclic vs acyclic** — Cycles indicate circular dependencies, feedback loops, or the possibility of infinite traversal. Directed Acyclic Graphs (DAGs) are particularly important — they model partial orders, dependency chains, and dataflow.
- **Simple vs multigraph** — Simple graphs allow at most one edge between any pair of vertices and no self-loops. Multigraphs permit parallel edges and self-loops, modeling scenarios like multiple routes between cities or recursive state transitions.
- **Hypergraphs** — Edges connect arbitrary subsets of vertices rather than pairs. Database schemas and constraint systems sometimes benefit from hypergraph modeling.

## Representation: Adjacency Matrix vs Adjacency List

The choice of graph representation affects both space consumption and operation efficiency:

| Operation                  | Adjacency Matrix                        | Adjacency List                     |
| -------------------------- | --------------------------------------- | ---------------------------------- |
| Space                      | Θ(V²)                                   | Θ(V + E)                           |
| Check if edge (u,v) exists | Θ(1)                                    | O(degree(u))                       |
| Iterate neighbors of u     | Θ(V)                                    | Θ(degree(u))                       |
| Add edge                   | Θ(1)                                    | Θ(1)                               |
| Remove edge                | Θ(1)                                    | O(degree(u))                       |
| Dense graphs               | Space-efficient relative to info stored | Overhead per edge                  |
| Sparse graphs              | Wastes space on absent edges            | Space-proportional to actual edges |

For sparse graphs (the common case in software — dependency graphs, social networks, road networks), adjacency lists dominate. Adjacency matrices suit dense graphs or situations requiring constant-time edge queries, such as certain matrix-based algorithms.

Additional representations include:

- **Edge list** — Simple list of (u, v) pairs or (u, v, w) triples. Compact, easy to sort, good for algorithms that process edges sequentially (Kruskal's). Poor for neighbor queries.
- **Compressed Sparse Row (CSR)** — Cache-friendly packed arrays. Efficient for static graphs in high-performance or systems contexts.
- **Incidence matrix** — Rows are vertices, columns are edges. Useful in algebraic graph theory but rarely in practice.

## Traversal: BFS and DFS

The two fundamental traversal strategies explore graphs in fundamentally different orders, leading to different algorithmic applications:

### Breadth-First Search (BFS)

Explores vertices layer by layer, visiting all vertices at distance k before any at distance k+1. Uses a queue.

```
BFS(source):
  queue ← [source]
  visited ← {source}
  while queue is not empty:
    u ← dequeue
    for each neighbor v of u:
      if v not in visited:
        visited.add(v)
        enqueue(v)
```

Applications: shortest paths in unweighted graphs, level-order traversal, testing bipartiteness, finding connected components, web crawling strategies, peer-to-peer network broadcasting.

Time: O(V + E). Space: O(V) for the queue and visited set.

### Depth-First Search (DFS)

Explores as deep as possible along each branch before backtracking. Uses a stack (or recursion).

```
DFS(source):
  stack ← [source]
  visited ← {}
  while stack is not empty:
    u ← pop
    if u not in visited:
      visited.add(u)
      for each neighbor v of u:
        push(v)
```

Applications: cycle detection, topological sorting, finding strongly connected components, maze generation, path existence, articulation points and bridges.

Time: O(V + E). Space: O(V) for the stack, though recursion depth can be a concern for deep graphs.

### Choosing Between Them

| Need                       | Prefer                                                |
| -------------------------- | ----------------------------------------------------- |
| Shortest path (unweighted) | BFS                                                   |
| Cycle detection            | DFS                                                   |
| Topological order          | DFS                                                   |
| Level-by-level exploration | BFS                                                   |
| Path existence (any path)  | Either (DFS uses less memory in many implementations) |
| Connected components       | Either                                                |
| Exploring decision trees   | DFS (with pruning)                                    |

## Shortest Path Algorithms

Finding shortest (or cheapest) paths between vertices is among the most studied graph problems. Different algorithms suit different graph conditions:

### Dijkstra's Algorithm

Finds shortest paths from a single source to all vertices in graphs with **non-negative edge weights**. Greedily extends the shortest known path.

- Time: O((V + E) log V) with a binary heap; O(V² + E) with a simple array
- Limitation: breaks with negative weights (cannot "un-relax" already-finalized vertices)
- Use when: edge weights are non-negative, single-source shortest paths needed

### Bellman-Ford Algorithm

Handles **negative edge weights** by relaxing all edges V-1 times. Also detects negative-weight cycles.

- Time: O(V · E)
- Use when: negative weights exist, negative cycle detection is needed
- Limitation: slower than Dijkstra for non-negative graphs

### Floyd-Warshall Algorithm

Computes shortest paths between **all pairs** of vertices using dynamic programming.

- Time: O(V³), Space: O(V²)
- Use when: all-pairs shortest paths needed on moderate-sized graphs
- Handles negative weights (no negative cycles)

### Comparison

| Algorithm      | Source    | Negative Weights | Negative Cycles | Time           |
| -------------- | --------- | ---------------- | --------------- | -------------- |
| Dijkstra       | Single    | No               | No              | O((V+E) log V) |
| Bellman-Ford   | Single    | Yes              | Detects         | O(V·E)         |
| Floyd-Warshall | All pairs | Yes              | Detects         | O(V³)          |
| BFS            | Single    | N/A (unweighted) | N/A             | O(V+E)         |

For unweighted graphs, BFS already gives shortest paths — introducing Dijkstra or Bellman-Ford adds complexity without benefit.

## Minimum Spanning Trees

A **minimum spanning tree (MST)** of a connected, weighted, undirected graph is a spanning tree (subgraph connecting all vertices) with minimum total edge weight. MSTs model problems where connecting all nodes at minimum cost matters: network design, clustering, approximation algorithms for other problems.

Two classical approaches:

- **Kruskal's algorithm** — Sort edges by weight, add cheapest edge that doesn't create a cycle. Uses union-find for cycle detection. Time: O(E log E). Works well for sparse graphs.
- **Prim's algorithm** — Grow the tree from a starting vertex, always adding the cheapest edge connecting the tree to a non-tree vertex. Uses a priority queue. Time: O((V + E) log V). Works well for dense graphs.

Both produce the same MST weight (though the tree itself may differ when edge weights aren't unique). The choice between them is largely about implementation convenience and graph density.

## Topological Sorting

A **topological sort** of a DAG is a linear ordering of vertices such that for every directed edge (u, v), u appears before v. This captures dependency order — "do u before v."

Two approaches:

- **DFS-based** — Run DFS; output vertices in reverse finish order. Naturally produces a valid topological order.
- **Kahn's algorithm** — Repeatedly remove vertices with in-degree 0, adding them to the output. Detects cycles (if vertices remain but all have nonzero in-degree).

Applications pervade software engineering:

| Application                     | Vertices        | Edges                      |
| ------------------------------- | --------------- | -------------------------- |
| Build systems                   | Modules/targets | Dependencies               |
| Package managers                | Packages        | Version dependencies       |
| Task scheduling                 | Tasks           | Precedence constraints     |
| Spreadsheet evaluation          | Cells           | Formula references         |
| Course prerequisites            | Courses         | Prerequisite relationships |
| Database migration              | Migrations      | Ordering constraints       |
| Compiler instruction scheduling | Instructions    | Data dependencies          |

A DAG may have multiple valid topological orderings. When parallelism is available, vertices at the same "level" (no mutual dependencies) can be processed concurrently.

## Strongly Connected Components

In a directed graph, a **strongly connected component (SCC)** is a maximal set of vertices where every vertex is reachable from every other. The SCC decomposition collapses a directed graph into a DAG of components, revealing the high-level dependency structure.

Algorithms:

- **Tarjan's algorithm** — Single DFS pass using a stack and low-link values. Time: O(V + E).
- **Kosaraju's algorithm** — Two DFS passes (one on the original graph, one on the transpose). Conceptually simpler. Time: O(V + E).

Applications: analyzing call graphs (mutually recursive functions), detecting circular dependencies in module systems, simplifying constraint problems, identifying clusters in web graphs.

## Network Flow and Matching

**Maximum flow** asks: given a directed graph with edge capacities and source/sink vertices, what is the maximum amount of "flow" from source to sink? The **max-flow min-cut theorem** establishes that maximum flow equals the minimum capacity of any cut separating source from sink.

Algorithms include Ford-Fulkerson (augmenting paths), Edmonds-Karp (BFS-based augmenting paths, O(V·E²)), and push-relabel methods. The choice depends on graph density and capacity distributions.

**Matching** finds the largest set of edges with no shared vertices. In bipartite graphs, maximum matching can be found via max-flow or the Hopcroft-Karp algorithm (O(E·√V)).

Applications:

| Problem                       | Flow/Matching Model       |
| ----------------------------- | ------------------------- |
| Assignment (workers to tasks) | Bipartite matching        |
| Network bandwidth             | Maximum flow              |
| Project selection             | Min-cut                   |
| Image segmentation            | Min-cut                   |
| Airline crew scheduling       | Matching with constraints |
| Baseball elimination          | Max-flow reduction        |

## Bipartite Graphs

A graph is **bipartite** if its vertices can be divided into two disjoint sets such that every edge connects a vertex in one set to a vertex in the other. Equivalently, a graph is bipartite if and only if it contains no odd-length cycles.

Testing bipartiteness: Run BFS and attempt a 2-coloring. If successful, the graph is bipartite.

Bipartite structures model natural two-sided relationships: students and courses, documents and terms, buyers and sellers, applicants and positions. König's theorem links vertex cover size to maximum matching size in bipartite graphs — a relationship that doesn't hold in general graphs.

## Graph Coloring

A **proper k-coloring** assigns one of k colors to each vertex such that no two adjacent vertices share a color. The **chromatic number** χ(G) is the minimum k for which a proper coloring exists.

Determining whether a graph is k-colorable is NP-complete for k ≥ 3 (2-colorability is just bipartiteness testing). Despite this, graph coloring has significant practical applications:

- **Register allocation** — Variables are vertices, edges connect simultaneously live variables. Coloring assigns registers; the chromatic number indicates the minimum registers needed. When more variables are live than registers available, some must "spill" to memory.
- **Scheduling** — Exams/tasks are vertices, edges connect conflicting pairs. Coloring produces a conflict-free schedule; the chromatic number gives the minimum number of time slots.
- **Frequency assignment** — Transmitters are vertices, edges connect those that would interfere. Colors represent frequencies.
- **Map coloring** — The four-color theorem guarantees every planar graph is 4-colorable.

Heuristic approaches (greedy coloring, DSatur) provide practical solutions without optimality guarantees. The ordering in which vertices are colored significantly affects the number of colors used.

## Graph Databases

Graph databases store data as vertices and edges natively, optimizing for relationship-heavy queries. Where relational databases require expensive JOIN operations to traverse relationships, graph databases follow edges directly.

| Aspect                 | Relational                                | Graph                                                |
| ---------------------- | ----------------------------------------- | ---------------------------------------------------- |
| Relationship traversal | JOIN chains, cost grows with depth        | Direct pointer following, near-constant per hop      |
| Schema flexibility     | Fixed schema, ALTER TABLE for changes     | Vertices and edges can have heterogeneous properties |
| Multi-hop queries      | Increasingly expensive with depth         | Relatively stable performance                        |
| Aggregation/analytics  | Strong (SQL aggregates, window functions) | Varies; some graph databases less mature here        |
| Transactions           | Mature ACID support                       | Varies by implementation                             |

Graph databases suit domains with deeply connected data and traversal-heavy queries: social networks, recommendation engines, fraud detection, knowledge graphs, network/infrastructure modeling. They are less suited to bulk analytics, reporting, or problems without significant relationship structure.

## Social Network Analysis

Graphs model social structures with vertices as individuals and edges as relationships. Key concepts:

- **Centrality measures** — Quantify vertex importance. Degree centrality (most connections), betweenness centrality (most shortest paths through), closeness centrality (shortest average distance to others), eigenvector centrality (connected to important vertices).
- **Community detection** — Finding densely connected groups within sparse global structure. Approaches include modularity optimization, spectral methods, and label propagation.
- **Small-world property** — Most real social networks exhibit short average path lengths and high clustering coefficients.
- **Preferential attachment** — New vertices tend to connect to already well-connected vertices, producing power-law degree distributions.

These concepts extend beyond social networks to any system with emergent community structure: citation networks, protein interaction networks, co-purchasing patterns.

## PageRank and Link Analysis

**PageRank** models the web as a directed graph (pages are vertices, hyperlinks are edges) and computes the "importance" of each page based on the link structure. The core idea: a page is important if important pages link to it.

```
Simplified PageRank iteration:
  PR(v) = (1-d)/N + d · Σ(PR(u)/out_degree(u))  for each u linking to v

  d = damping factor (typically 0.85)
  N = total number of pages
```

The damping factor models a "random surfer" who follows links with probability d and jumps to a random page with probability 1-d. This ensures convergence by making the graph's transition matrix irreducible and aperiodic.

Beyond web search, variations of link analysis apply to: citation ranking, identifying influential research, social influence scoring, and any directed graph where "endorsement" flows along edges.

**HITS (Hyperlink-Induced Topic Search)** provides a complementary model with two scores per page: hub (links to good authorities) and authority (linked from good hubs). The mutual reinforcement produces useful rankings for topic-focused search.

## Trees as Graphs

A **tree** is a connected, acyclic graph. Equivalently: a connected graph with exactly V-1 edges. Trees are the most fundamental graph structures in computer science:

| Tree Type          | Structure                                    | Application                              |
| ------------------ | -------------------------------------------- | ---------------------------------------- |
| Binary search tree | Each node has ≤ 2 children, ordered          | Sorted data storage and lookup           |
| B-tree / B+ tree   | High branching factor, balanced              | Database indexes, file systems           |
| Trie (prefix tree) | Edges labeled with characters                | Autocomplete, spell checking, IP routing |
| Heap               | Parent ≤ children (min-heap) or ≥ (max-heap) | Priority queues, scheduling              |
| Spanning tree      | Subgraph connecting all vertices             | Network backbone, MST algorithms         |
| Parse tree / AST   | Grammar-driven structure                     | Compilers, interpreters, code analysis   |
| DOM tree           | Hierarchical document structure              | Web rendering, document manipulation     |
| Decision tree      | Branching on attribute values                | Machine learning classification          |
| Suffix tree        | All suffixes of a string                     | String matching, bioinformatics          |

Key tree properties: any two vertices are connected by exactly one path; removing any edge disconnects the tree; adding any edge creates exactly one cycle. These properties make trees structurally rigid and algorithmically tractable.

**Rooted vs unrooted**: Rooting a tree designates one vertex as the root, inducing parent-child relationships and enabling recursive algorithms. Many tree algorithms require a root; the choice of root can affect performance characteristics.

## Planarity and Graph Drawing

A graph is **planar** if it can be drawn in the plane without edge crossings. Planar graphs are structurally constrained: by Euler's formula (V - E + F = 2), a simple planar graph has at most 3V - 6 edges.

Testing planarity can be done in linear time. Kuratowski's theorem characterizes planar graphs: a graph is planar if and only if it contains no subdivision of K₅ (complete graph on 5 vertices) or K₃,₃ (complete bipartite graph on 3+3 vertices).

Graph drawing — producing readable visual layouts — is an active area combining graph theory with information visualization. Approaches include:

- **Force-directed layouts** — Model vertices as repelling particles and edges as springs. Iterative simulation produces aesthetically pleasing layouts for small-to-medium graphs.
- **Layered/hierarchical layouts** — Assign vertices to layers, minimize edge crossings. Well-suited for DAGs (call graphs, dependency diagrams).
- **Circular layouts** — Place vertices on a circle, useful for showing community structure.
- **Orthogonal layouts** — Edges follow horizontal/vertical paths. Used for circuit diagrams and UML.

The readability of a graph visualization depends on minimizing edge crossings, distributing vertices evenly, and reflecting the graph's structural properties. Crossing minimization is itself NP-hard, so practical layout algorithms use heuristics.

## Algorithmic Paradigms on Graphs

Graph problems frequently serve as the canonical examples for algorithmic paradigms:

- **Greedy** — Kruskal's MST, Dijkstra's shortest path, graph coloring heuristics. Locally optimal choices that sometimes achieve global optimality (provably so for MST and shortest paths with non-negative weights).
- **Dynamic programming** — Floyd-Warshall all-pairs shortest paths, longest path in DAGs, tree DP. Decomposing into overlapping subproblems along the graph structure.
- **Divide and conquer** — Graph partitioning, separator-based algorithms on planar graphs. Splitting the problem at structural cuts.
- **Randomization** — Karger's min-cut algorithm (random edge contraction), randomized matching, random walks for connectivity. Randomness can simplify algorithms and improve expected performance.

The richness of graph problems means that mastering graph algorithms provides a foundation applicable across domains — from compiler design to computational biology to network engineering.
