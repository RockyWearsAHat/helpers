# Graph Databases — Models, Query Languages, and Scaling Challenges

Graph databases store and query data where relationships are first-class. Unlike normalized SQL schemas (where relationships are reconstructed via joins), graph databases keep edges materialized, enabling efficient traversal and pattern matching.

## Data Models: Property Graphs vs. RDF

### Property Graph Model

A **property graph** consists of:
- **Nodes** — Entities with optional labels (tags) and key-value properties
- **Relationships** — Directed edges with a type and optional properties
- **Labels and Types** — Categorical tags for filtering and schema hints

Example: A movie database:

```
(Person {name: "Meryl Streep", born: 1949})
  -[:ACTED_IN {character: "Donna"}]->
(Movie {title: "Mamma Mia!", released: 2008})
```

**Strengths**: Intuitive, flexible schema (add properties ad hoc), efficient for hierarchies and networks.

**Limits**: No standardized semantics for types; inference and reasoning require custom logic.

**Systems**: Neo4j, ArangoDB, JanusGraph, Memgraph.

### RDF (Resource Description Framework)

RDF represents everything as **triples**: `(subject, predicate, object)`. 

```
ex:MerylStreep ex:actedIn ex:MammaMia
ex:MammaMia ex:released "2008"^^xsd:int
```

URIs identify all entities globally; literals (strings, numbers) are data values. **Blank nodes** represent anonymous entities.

**SPARQL** is the query language (declarative, similar to SQL):

```sparql
SELECT ?person ?movie WHERE {
  ?person rdf:type ex:Person.
  ?person ex:actedIn ?movie.
  ?movie ex:released "2008"^^xsd:int.
}
```

**Strengths**: Semantic standard, global identifiers, built-in reasoning frameworks (OWL, RDF-S), knowledge graphs.

**Limits**: Verbose (triples are granular), performance degrades with high cardinality, reasoning inference is computationally expensive.

**Systems**: Triple stores (Apache Jena, OpenLink Virtuoso), RDF databases (Amazon Neptune, Wikidata), knowledge graphs.

### Choosing Between Models

- **Property graphs**: When modeling application domain (social networks, fraud networks, organizational structures)
- **RDF**: When integrating heterogeneous data sources, need formal semantics, or publishing as linked data

Many systems now support both (Neptune, ArangoDB with GraphQL).

## Index-Free Adjacency

The most significant performance advantage of native graph databases is **index-free adjacency**. 

Each node stores direct pointers (physical memory addresses or disk offsets) to its adjacent nodes. Traversing from node A to a neighbor requires no index lookups — dereference the pointer and read the neighbor's data. Cost: O(1) per hop, regardless of graph size.

Contrast: Relational database traversal (with indexes):
```sql
SELECT * FROM Movie WHERE id IN (
  SELECT movie_id FROM ActedIn WHERE person_id = ?
)
```
Lookups require B-tree scans (O(log N)) or hash table probes.

**Consequence**: Path queries like "6-hop neighborhoods" remain efficient in graph databases, but degrade quadratically in relational systems.

## Query Patterns: Cypher and SPARQL

### Cypher (Neo4j Standard)

Cypher is an ASCII-art DSL for graph patterns:

```cypher
-- Find all movies an actor appeared in, filtering by release year
MATCH (p:Person)-[:ACTED_IN]->(m:Movie)
WHERE m.released > 2010 AND p.name = 'Tom Hanks'
RETURN m.title, m.released ORDER BY m.released DESC;

-- Variable-length paths: "people with recommendations in 1-3 hops"
MATCH (u:User)-[:RECOMMENDS*1..3]->(movie:Movie)
RETURN movie.title;

-- Shortest path
MATCH p = shortestPath((a:Person)-[*]-(b:Person))
WHERE a.name = 'Alice' AND b.name = 'Bob'
RETURN p;

-- Pattern with aggregation
MATCH (actor:Person)-[:ACTED_IN]->(movie:Movie)<-[:WATCHED]-(viewer:Person)
RETURN actor.name, COUNT(viewer) AS viewers
GROUP BY actor.name
ORDER BY viewers DESC;
```

**Strength**: Intuitive for relationship patterns.

### SPARQL (RDF Standard)

```sparql
PREFIX ex: <http://example.org/>

SELECT ?movieTitle WHERE {
  ?actor rdf:type ex:Person.
  ?actor ex:name "Tom Hanks".
  ?actor ex:actedIn ?movie.
  ?movie ex:title ?movieTitle.
  ?movie ex:released ?year.
  FILTER (?year > 2010)
}
ORDER BY ?year DESC
```

**Strength**: SQL-like familiarity, inference through ontology reasoning.

## Use Cases and Patterns

### Fraud Detection

Graph traversal detects circular money flows, unusual transaction patterns:
- Bank account A transfers to B, B to C, C back to A (ring detection)
- Compromised accounts have abnormal degree distribution
- Community detection identifies fraud rings

### Social Networks

- Friend recommendations: mutual friends, second-degree connections
- Community detection: find clusters of tightly connected users
- Influence ranking: PageRank variants

### Knowledge Graphs

Semantic search across integrated data:
- Wikidata, DBpedia integrate Wikipedia as a graph
- Named entity linking: "Steve Jobs" → unambiguous URI
- Relationship inference: if X worked at Y and Y is in Z, infer X's location

### Recommendation Engines

Collaborative filtering as graph traversal:
```cypher
MATCH (user:User)-[:LIKES]->(product),
      (similar:User)-[:LIKES]->(product),
      (similar)-[:LIKES]->(recommendation)
WHERE user.id = ? AND NOT (user)-[:LIKES]->(recommendation)
RETURN recommendation ORDER BY COUNT(*) DESC LIMIT 10
```

## Scaling Challenges

### Replication vs. Sharding

**Replication** (leader-follower): Full graph on each node, writes go to leader, read-heavy workloads scale.

**Sharding**: Partition nodes by a shard key (e.g., user ID hash). Problems:
- **Cross-shard traversals**: Queries touching multiple shards require coordination (expensive)
- **Imbalanced graphs**: Some nodes (celebrities, hubs) have high degree, concentrating load on one shard
- **Joins across communities**: Social networks have clusters; sharding typically separates them

Most graph databases shard reluctantly or not at all. JanusGraph (distributed, raft-based) relies on multi-node transactions to handle cross-shard queries, incurring latency.

### Cardinality Explosion

Relationships can multiply: O(N²) edges in a fully connected graph. Memory models that pre-load the entire neighborhood (common for fast traversal) face memory exhaustion. Selective loading and index filtering become critical.

### Reasoning Complexity

RDF-based reasoning (inferring triples from rules and existing triples) is NP-hard in the general case. Systems use tractable fragments:
- **OWL DLP** (Description Logic Program): Decidable reasoning
- **RDFS**: Simpler, polynomial-time reasoning

Materialization (pre-computing all inferred triples) or query-time inference both have costs.

## System Comparisons

### Neo4j

- **Model**: Property graph, native storage with index-free adjacency
- **Query**: Cypher, GQL (Graph Query Language, emerging standard)
- **Scaling**: Single node (community edition), Causal Cluster replication available
- **Strength**: Most mature ecosystem, Cypher tooling, visualization integrations
- **Trade-off**: Single-writer bottleneck in RW workloads without Enterprise license

### Amazon Neptune

- **Modes**: Both property graph (Apache Gremlin) and RDF (SPARQL)
- **Scaling**: Managed, replication built-in, partition tolerate design
- **Strength**: Serverless pay-as-you-go, automatic failover
- **Trade-off**: Vendor lock-in, query limits (timeouts), pricing opacity

### ArangoDB

- **Model**: Multi-model (documents, graphs, search)
- **Query**: AQL (similar to SQL with graph extensions)
- **Scaling**: Replication and sharding built-in
- **Strength**: Flexible querying, multi-model integration, manageable scaling
- **Trade-off**: Less mature Cypher support than Neo4j

### JanusGraph (Distributed)

- **Model**: Property graph on distributed storage (Cassandra, HBase, Bigtable)
- **Query**: Gremlin (Tinkerpop DSL, functional style)
- **Scaling**: Horizontal sharding from the start
- **Strength**: Handles massive graphs (terabyte-scale), distributed from design
- **Trade-off**: Complex operational setup, query optimization less sophisticated than Neo4j

## Algorithms in Graph Databases

Most graph databases expose or optimize for:

- **Shortest path**: Dijkstra, A* (BFS optimizations)
- **All-pairs shortest paths**: Floyd-Warshall (pruned)
- **Centrality**: Betweenness, closeness, PageRank
- **Community detection**: Louvain, label propagation
- **Triangle counting**: For clustering coefficient

Some provide them as stored procedures or query operations; others require client-side implementation.

## One More Thing: Interoperability

The GQL (Graph Query Language) standard, adopted by ISO, aims to unify Cypher and SPARQL-like constructs. Adoption is gradual; most systems still use proprietary languages. Wikidata and DBpedia provide SPARQL endpoints; expect a slow shift toward standardization over the next 5 years.