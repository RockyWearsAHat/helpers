# Graph Data Patterns — Social Networks, Fraud Detection, Recommendations

Graph patterns describe recurring structural and algorithmic approaches for extracting insights from connected data. This complements stored graph structure by focusing on the **analysis and traversal patterns** that unlock value.

## Social Network Analysis

Social networks model people, organizations, or entities as nodes and relationships (follows, collaborates, communicates) as edges. Analysis patterns identify influential actors, community structure, and information flow.

### Centrality Measures

**Centrality** quantifies how "important" a node is in the network. Different definitions suit different questions.

**Degree Centrality** — Count of direct connections. Nodes with high degree centrality are local hubs.
- Quick to compute; ignores the broader network structure
- Use when local popularity matters (highly connected users, key infrastructure nodes)

**Closeness Centrality** — Average distance to all other nodes. Nodes that can reach others quickly have high closeness.
- $C_i = \frac{1}{\sum_j d(i,j)}$ where $d(i,j)$ is shortest path distance
- Nodes with high closeness are good relay points; information from them spreads efficiently
- Expensive: requires all-pairs shortest paths (Floyd-Warshall, $O(n^3)$, or BFS from each node)

**Betweenness Centrality** — Fraction of shortest paths between all node pairs that pass through a given node.
- $C_B(i) = \sum_{s \ne t} \frac{\text{paths}_{s,t} \text{ through } i}{\text{all paths}_{s,t}}$
- High betweenness nodes are bottlenecks; removing them fragments the network
- Expensive: $O(n^3)$ worst case, but approximations exist
- Use for infrastructure design, supply chain vulnerability, communication hubs

**Eigenvector Centrality** — Importance is determined by connections to other important nodes. Solved via eigenvalues of the adjacency matrix.
- Iterative: power iteration converges to principal eigenvector
- High eigenvector centrality = connected to influential neighbors (e.g., academic citations)

**PageRank** — Graph-wide importance score based on link structure. Originally used by Google for web ranking.
- Nodes with many incoming links are important; links from important nodes count more
- $PR(i) = \frac{1-d}{n} + d \sum_{j \text{ links to } i} \frac{PR(j)}{|outgoing(j)|}$
- Damping factor $d$ (typically 0.85) models random jum chance token walks
- Scales well; can be computed in MapReduce or graph engines (Neo4j, Spark GraphX)

### Community Detection

Communities are dense subgraphs where nodes are tightly connected internally but sparsely connected to the rest.

**Modularity Optimization** — Measures clustering quality as $Q = \frac{1}{2m}\sum_{ij}\left(A_{ij} - \frac{k_i k_j}{2m}\right)\delta(c_i, c_j)$
- $A_{ij}$ = adjacency, $k_i$ = degree, $m$ = total edges, $\delta$ = indicator (same community)
- Higher modularity (max 1.0) = stronger community structure
- Greedy and simulated annealing approaches exist; exact optimization is NP-hard

**Louvain Method** — Fast two-phase heuristic: local optimization then consolidation.
- Phase 1: Move each node to the community that maximizes modularity gain
- Phase 2: Collapse communities into nodes and repeat
- Scales to millions of nodes; finds hierarchical structure
- Trade-off: faster than exact methods, but may converge to local optima

**Girvan-Newman** — Iteratively remove highest-betweenness edges, splitting communities.
- Most accurate but expensive ($O(n^3)$); useful for small networks or reference
- Produces dendrogram (hierarchical communities)

---

## Fraud Detection in Networks

Fraudsters exhibit distinct patterns: abnormal velocity, circular transactions, anomalous network clusters, and mimicry of legitimate behaviors.

### Pattern Categories

**Velocity Anomalies** — Sudden spikes in activity frequency or transaction volume.
- Detection: Compare recent count (e.g., last hour) to rolling average (e.g., last 30 days)
- Alert threshold: $\frac{\text{recent}}{\text{baseline}} > k$ (typically $k = 2-5$)
- Challenge: Legitimate bursts (sales, promotions) can trigger false positives
- Mitigation: Machine learning classifier combining velocity with other features

**Cyclic Patterns (Triangles)** — A → B → C → A. Fraudsters may create fake transaction cycles to move money or inflate metrics.
- Detection: Count triangles in a sliding window; flag networks with elevated triangle count at low volumes
- Efficient: Use local clustering coefficient or dense subgraph discovery (Clique Packing Algorithm)
- Insight: Legitimate networks form triangles due to mutual friends; fraud triangles cluster in time and volume

**Graph Morphology** — Fraudsters form disjoint "islands" or star topologies (one operator controls many accounts).
- Detection: Connected component analysis; flag sudden new components or star-shaped subgraphs
- Weakly connected components in directed graphs connect all nodes ignoring edge direction; identify isolated fraud rings
- Strongly connected components reveal explicit feedback loops

**Deviation from Baseline** — Legitimate nodes have stable graphs (consistent neighbors, degree distribution). Anomalies: new high-degree node, sudden location jump, new device, unusual spending pattern.
- Approach: Implicit graph (transactions, logins, devices) + anomaly detection (Local Outlier Factor, Isolation Forest)
- Flag: Nodes whose neighborhoods change sharply or edge attributes diverge (amount, time, location)

**Sybil Attacks** — Attacker controls many fake identities linked to a small set of real identities.
- Detection: Small cut → large external degree. Verify using Sybil Limit (exploit random walk properties)
- Structural insight: Attackers must connect fake to real; connection bottleneck reveals attack structure

---

## Recommendation Graphs

Recommendation graphs model user-item interactions or item-item similarity as edges, enabling personalized suggestions via graph traversal.

### Collaborative Filtering on Graphs

Represent users and items as nodes; edges encode interactions (viewed, purchased, rated, clicked).

**User-Based** — "Find similar users, recommend items they liked."
- Similarity: Cosine similarity of user vectors (which items they interacted with)
- Recommendation: Items liked by similar users but not by the target user
- Challenge: Sparsity (new users, items have few interactions); cold start problem

**Item-Based** — "Find similar items, recommend based on items the user already liked."
- Similarity: Item vectors (which users interacted with them) or content features
- More stable than user-based because item interactions are more consistent than user interests
- Precompute item similarity; serve recommendations via lookup

**Graph-Based Propagation** — Random walks and matrix factorization.
- Personalized PageRank: Start random walk from target user; stationary distribution gives recommendation scores
- Matrix factorization: Decompose user-item matrix into latent factors; dot product of factors predicts interaction probability
- Hybrid: Combine graph structure with content features (embeddings)

### Knowledge Graphs for Recommendations

Augment user-item bipartite graphs with rich entity relationships (movie → director, actor, genre, plot keywords).

- **Multi-hop reasoning**: User likes movie A, movie A stars actor B, actor B stars movie C → recommend movie C
- **Explicit paths**: Find paths of length 3-5 from user to candidate items through semantic relations
- **Embedding-based**: Translate graph structure into vector space (TransE, DistMult); score latent semantic matches
- Challenge: Scalability (billion-node graphs); trade-off between precision (few, high-quality recs) and coverage (diverse recs)

---

## Shortest Path Analysis

Shortest path algorithms underpin navigation, network resilience, and delivery optimization.

**Dijkstra's Algorithm** — Single-source shortest paths to all nodes (non-negative weights).
- $O((V + E)\log V)$ with binary heap; $O(V^2)$ with naive priority queue
- Greedy: always expand the nearest unvisited node
- Use: Route planning, social distance (degrees of separation)

**Bellman-Ford** — Handles negative weights; detects negative cycles.
- $O(VE)$; impractical for large graphs if negative edges are common
- Use: Currency arbitrage, constraint satisfaction

**A*** — Guided search using heuristic (estimated cost to goal).
- $O(E)$ if heuristic is admissible (never overestimates); degenerates to Dijkstra without heuristic
- Use: Pathfinding in games, GPS navigation

**All-Pairs Shortest Paths** — Floyd-Warshall ($O(V^3)$), Johnson's algorithm ($O(VE + V^2 \log V)$).
- Precompute once; answer distance queries in $O(1)$
- Use: Social distance matrices, network diameter, resilience analysis (node/edge removal impact)

---

## Graph Partitioning

Partitioning distributes a large graph across multiple machines for parallel computation.

**Edge-Cut** — Each node resides on one machine; edges may cross partitions.
- Low communication if internal edges >> cross-partition edges
- Challenge: Skewed degree distribution (power-law graphs have numerous hub nodes that create many cross-partition edges by themselves)

**Vertex-Cut** — Each edge resides on one machine; vertices may be replicated.
- Assigns edges (not nodes) to partitions; replicates high-degree nodes across multiple machines
- Reduces communication for power-law graphs where removing hubs significantly reduces cross-partition edges
- Trade-off: Increases memory due to vertex replication

**Balanced Partitioning** — Minimize cross-partition edges while balancing partition sizes.
- NP-hard; heuristics: greedy, METIS (multilevel recursive bisection), streaming algorithms
- Multilevel approach: Coarsen (combine nodes/edges), partition coarse graph, uncoarsen (refine)
- Streaming: Process edges in order; assign to partitions online (no global knowledge); useful for massive graphs

---

## See Also
- [Graph Theory](math-graph-theory.md) — Fundamental definitions and algorithms
- [Graph Databases](database-graph-database.md) — Storage and query systems
- [Algorithms: Graphs](algorithms-graph.md) — Implementation details
- [Event-Driven Patterns](patterns-event-driven.md) — Event streams as graphs