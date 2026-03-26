# Neo4j

## Property Graph Model

Neo4j stores data as a **property graph**: nodes connected by relationships, both of which can carry key-value properties.

| Element      | Description                                                           |
| ------------ | --------------------------------------------------------------------- |
| Node         | Entity with zero or more labels (e.g., `:Person`, `:Movie`)           |
| Relationship | Directed, typed connection between two nodes (e.g., `-[:ACTED_IN]->`) |
| Property     | Key-value pair on a node or relationship                              |
| Label        | Named tag for categorizing nodes (a node can have multiple labels)    |

### Storage Architecture

- **Native graph storage**: Nodes and relationships are stored as fixed-size records with direct pointers to adjacent elements. Traversals follow pointers — no index lookups needed.
- **Index-free adjacency**: Each node stores direct physical references to its neighbors, making traversal O(1) per hop regardless of total graph size.
- This gives Neo4j constant-time traversal performance — the cost of following a relationship doesn't increase as the graph grows.

## Cypher Query Language

### Reading Data

```cypher
-- Basic pattern matching
MATCH (p:Person)-[:ACTED_IN]->(m:Movie)
WHERE m.released > 2010
RETURN p.name, m.title, m.released
ORDER BY m.released DESC
LIMIT 25;

-- Variable-length paths (1 to 5 hops)
MATCH path = (a:Person)-[:KNOWS*1..5]->(b:Person)
WHERE a.name = 'Alice' AND b.name = 'Bob'
RETURN path, length(path) AS hops
ORDER BY hops;

-- Shortest path
MATCH p = shortestPath((a:Person {name: 'Alice'})-[:KNOWS*]-(b:Person {name: 'Bob'}))
RETURN p;

-- All shortest paths
MATCH p = allShortestPaths((a:Person {name: 'Alice'})-[*]-(b:Person {name: 'Bob'}))
RETURN p;

-- Optional match (left outer join equivalent)
MATCH (p:Person)
OPTIONAL MATCH (p)-[:REVIEWED]->(m:Movie)
RETURN p.name, collect(m.title) AS reviews;

-- Aggregation
MATCH (p:Person)-[:ACTED_IN]->(m:Movie)
RETURN p.name, count(m) AS movieCount, collect(m.title) AS movies
ORDER BY movieCount DESC;
```

### Writing Data

```cypher
-- Create node
CREATE (p:Person {name: 'Alice', born: 1985})
RETURN p;

-- Create relationship
MATCH (a:Person {name: 'Alice'}), (m:Movie {title: 'The Matrix'})
CREATE (a)-[:ACTED_IN {roles: ['Trinity']}]->(m);

-- MERGE: create if not exists, match if exists (idempotent)
MERGE (p:Person {name: 'Alice'})
ON CREATE SET p.created = datetime()
ON MATCH SET p.lastSeen = datetime()
RETURN p;

-- MERGE relationship
MATCH (a:Person {name: 'Alice'}), (b:Person {name: 'Bob'})
MERGE (a)-[:KNOWS]->(b);

-- Update properties
MATCH (p:Person {name: 'Alice'})
SET p.email = 'alice@example.com', p.age = 40;

-- Remove property
MATCH (p:Person {name: 'Alice'})
REMOVE p.email;

-- Delete node and all its relationships
MATCH (p:Person {name: 'Alice'})
DETACH DELETE p;

-- Delete specific relationship
MATCH (a:Person {name: 'Alice'})-[r:KNOWS]->(b:Person {name: 'Bob'})
DELETE r;
```

### UNWIND

Converts a list into rows — essential for bulk operations.

```cypher
-- Bulk create from list
UNWIND [{name: 'Alice', age: 30}, {name: 'Bob', age: 25}] AS person
CREATE (p:Person) SET p = person;

-- Expand list property into rows
MATCH (m:Movie)
UNWIND m.genres AS genre
RETURN genre, count(*) AS movieCount
ORDER BY movieCount DESC;

-- Create relationships from parameter list
UNWIND $friendships AS f
MATCH (a:Person {id: f.from}), (b:Person {id: f.to})
MERGE (a)-[:KNOWS]->(b);
```

### Subqueries and Advanced Patterns

```cypher
-- CALL subquery (correlated)
MATCH (p:Person)
CALL (p) {
    MATCH (p)-[:ACTED_IN]->(m:Movie)
    RETURN count(m) AS movieCount
}
RETURN p.name, movieCount;

-- UNION
MATCH (p:Person)-[:ACTED_IN]->(m:Movie) RETURN p.name AS name, m.title AS work
UNION
MATCH (p:Person)-[:DIRECTED]->(m:Movie) RETURN p.name AS name, m.title AS work;

-- WITH for query pipelining (chaining match stages)
MATCH (p:Person)-[:ACTED_IN]->(m:Movie)
WITH p, count(m) AS movies
WHERE movies > 5
MATCH (p)-[:DIRECTED]->(d:Movie)
RETURN p.name, movies, collect(d.title) AS directed;

-- CASE expressions
MATCH (p:Person)
RETURN p.name,
    CASE
        WHEN p.born < 1970 THEN 'Boomer'
        WHEN p.born < 1985 THEN 'Gen X'
        WHEN p.born < 2000 THEN 'Millennial'
        ELSE 'Gen Z'
    END AS generation;

-- EXISTS subquery in WHERE
MATCH (p:Person)
WHERE EXISTS {
    MATCH (p)-[:ACTED_IN]->(m:Movie)
    WHERE m.released > 2020
}
RETURN p.name;
```

## Indexes and Constraints

### Index Types

```cypher
-- Range index (B-tree, for equality and range lookups)
CREATE INDEX person_name FOR (p:Person) ON (p.name);

-- Composite index
CREATE INDEX movie_year_title FOR (m:Movie) ON (m.released, m.title);

-- Text index (for CONTAINS and string matching)
CREATE TEXT INDEX person_bio FOR (p:Person) ON (p.bio);

-- Full-text index (Lucene-backed, for search)
CREATE FULLTEXT INDEX movieSearch FOR (m:Movie) ON EACH [m.title, m.tagline];
-- Query:
CALL db.index.fulltext.queryNodes('movieSearch', 'matrix~') YIELD node, score
RETURN node.title, score;

-- Point index (spatial)
CREATE POINT INDEX location_idx FOR (p:Place) ON (p.location);

-- Relationship index
CREATE INDEX acted_in_role FOR ()-[r:ACTED_IN]-() ON (r.role);

-- Show all indexes
SHOW INDEXES;
```

### Constraints

```cypher
-- Unique constraint (also creates an index)
CREATE CONSTRAINT person_unique_email FOR (p:Person) REQUIRE p.email IS UNIQUE;

-- Node existence constraint (Enterprise)
CREATE CONSTRAINT person_name_exists FOR (p:Person) REQUIRE p.name IS NOT NULL;

-- Node key constraint (unique + exists, Enterprise)
CREATE CONSTRAINT person_key FOR (p:Person) REQUIRE (p.firstName, p.lastName) IS NODE KEY;

-- Relationship property existence (Enterprise)
CREATE CONSTRAINT acted_in_roles FOR ()-[r:ACTED_IN]-() REQUIRE r.roles IS NOT NULL;

-- Type constraint (5.0+)
CREATE CONSTRAINT person_born_type FOR (p:Person) REQUIRE p.born IS :: INTEGER;

SHOW CONSTRAINTS;
```

## APOC (Awesome Procedures on Cypher)

APOC is a standard library of 450+ procedures and functions.

```cypher
-- Batch processing (avoid OOM on large operations)
CALL apoc.periodic.iterate(
    'MATCH (p:Person) WHERE p.age IS NULL RETURN p',
    'SET p.age = 0',
    {batchSize: 10000, parallel: true}
);

-- Load JSON
CALL apoc.load.json('https://api.example.com/users') YIELD value
UNWIND value.users AS user
MERGE (p:Person {id: user.id}) SET p.name = user.name;

-- Load CSV with custom settings
LOAD CSV WITH HEADERS FROM 'file:///import/movies.csv' AS row
MERGE (m:Movie {id: toInteger(row.id)})
SET m.title = row.title, m.released = toInteger(row.year);

-- Export to JSON
CALL apoc.export.json.all('export.json', {});

-- Path expansion (more control than variable-length)
MATCH (start:Person {name: 'Alice'})
CALL apoc.path.expandConfig(start, {
    relationshipFilter: 'KNOWS>|WORKS_WITH>',
    minLevel: 1,
    maxLevel: 3,
    uniqueness: 'NODE_GLOBAL'
}) YIELD path
RETURN path;

-- Schema introspection
CALL apoc.meta.graph();    -- visual schema of the database
CALL apoc.meta.stats();    -- node/relationship counts by label/type

-- Refactoring: merge duplicate nodes
MATCH (p:Person)
WITH p.email AS email, collect(p) AS nodes
WHERE size(nodes) > 1
CALL apoc.refactor.mergeNodes(nodes, {properties: 'combine'}) YIELD node
RETURN node;
```

## Graph Data Science (GDS) Library

GDS provides graph algorithms for analytics: centrality, community detection, similarity, pathfinding, ML embeddings.

```cypher
-- Project a named graph into memory (required before running algorithms)
CALL gds.graph.project(
    'social',                          -- graph name
    'Person',                          -- node labels
    'KNOWS',                           -- relationship types
    { nodeProperties: ['age'] }
);

-- PageRank (centrality)
CALL gds.pageRank.stream('social')
YIELD nodeId, score
RETURN gds.util.asNode(nodeId).name AS name, score
ORDER BY score DESC LIMIT 10;

-- Louvain community detection
CALL gds.louvain.write('social', {writeProperty: 'communityId'})
YIELD communityCount, modularity;

-- Node similarity (Jaccard)
CALL gds.nodeSimilarity.stream('social')
YIELD node1, node2, similarity
RETURN gds.util.asNode(node1).name AS p1,
       gds.util.asNode(node2).name AS p2,
       similarity
ORDER BY similarity DESC LIMIT 20;

-- Shortest path (Dijkstra)
MATCH (source:Person {name: 'Alice'}), (target:Person {name: 'Bob'})
CALL gds.shortestPath.dijkstra.stream('social', {
    sourceNode: source,
    targetNode: target,
    relationshipWeightProperty: 'weight'
}) YIELD path, totalCost
RETURN path, totalCost;

-- Node embeddings (FastRP — for ML features)
CALL gds.fastRP.stream('social', {embeddingDimension: 128, iterationWeights: [0.0, 1.0, 1.0]})
YIELD nodeId, embedding
RETURN gds.util.asNode(nodeId).name, embedding;

-- Drop projected graph when done
CALL gds.graph.drop('social');
```

### Algorithm Categories

| Category        | Algorithms                                      | Use Case                   |
| --------------- | ----------------------------------------------- | -------------------------- |
| Centrality      | PageRank, Betweenness, Closeness, Degree        | Find influential nodes     |
| Community       | Louvain, Label Propagation, WCC, Triangle Count | Detect clusters/groups     |
| Similarity      | Node Similarity, KNN                            | Find similar entities      |
| Pathfinding     | Dijkstra, A\*, Yen's K-Shortest                 | Route optimization         |
| Embeddings      | FastRP, Node2Vec, GraphSAGE                     | Feature generation for ML  |
| Link Prediction | Adamic Adar, Common Neighbors                   | Predict future connections |

## Clustering (Neo4j 5+)

### Cluster Architecture

| Role           | Purpose                                                         |
| -------------- | --------------------------------------------------------------- |
| Primary        | Accepts writes, participates in Raft consensus                  |
| Secondary      | Read replicas, async replication from primaries                 |
| Raft consensus | Leader election, write commit agreement (majority of primaries) |

```cypher
-- Check cluster topology
SHOW SERVERS;

-- Databases and their allocation
SHOW DATABASES;

-- Create database with specific topology
CREATE DATABASE mydb TOPOLOGY 3 PRIMARIES 2 SECONDARIES;

-- Route reads to secondaries
:use neo4j  -- in cypher-shell
-- Driver: session with READ access mode routes to secondaries
```

### Driver Routing

```python
from neo4j import GraphDatabase

driver = GraphDatabase.driver(
    "neo4j://cluster-address:7687",    # neo4j:// enables routing
    auth=("neo4j", "password")
)

# Writes go to leader
with driver.session(database="neo4j") as session:
    session.execute_write(lambda tx: tx.run("CREATE (p:Person {name: $name})", name="Alice"))

# Reads can go to followers/secondaries
with driver.session(database="neo4j", default_access_mode="READ") as session:
    result = session.execute_read(lambda tx: tx.run("MATCH (p:Person) RETURN p.name").data())
```

## Common Graph Patterns

### Social Network

```cypher
-- Friends of friends (exclude direct friends)
MATCH (me:Person {name: 'Alice'})-[:KNOWS]->()-[:KNOWS]->(fof:Person)
WHERE NOT (me)-[:KNOWS]->(fof) AND me <> fof
RETURN DISTINCT fof.name;

-- Mutual friends count
MATCH (a:Person {name: 'Alice'})-[:KNOWS]->(mutual)<-[:KNOWS]-(b:Person {name: 'Bob'})
RETURN count(mutual) AS mutualFriends;

-- Influence propagation (who reaches the most people in 3 hops)
MATCH (p:Person)-[:KNOWS*1..3]->(reached:Person)
RETURN p.name, count(DISTINCT reached) AS reach
ORDER BY reach DESC LIMIT 10;
```

### Recommendation Engine

```cypher
-- Collaborative filtering: "users who bought X also bought Y"
MATCH (u:User {id: $userId})-[:PURCHASED]->(p:Product)<-[:PURCHASED]-(other:User)
MATCH (other)-[:PURCHASED]->(rec:Product)
WHERE NOT (u)-[:PURCHASED]->(rec)
RETURN rec.name, count(other) AS score
ORDER BY score DESC LIMIT 10;

-- Content-based: similar items by shared categories
MATCH (p:Product {id: $productId})-[:IN_CATEGORY]->(c:Category)<-[:IN_CATEGORY]-(similar:Product)
WHERE p <> similar
RETURN similar.name, count(c) AS sharedCategories
ORDER BY sharedCategories DESC LIMIT 10;
```

### Knowledge Graph

```cypher
-- Entity resolution with relationships
MERGE (e:Entity {name: 'Python'})
MERGE (c:Concept {name: 'Programming Language'})
MERGE (e)-[:IS_A]->(c);

-- Inference: find all entities related through type hierarchy
MATCH (e:Entity)-[:IS_A*]->(ancestor:Concept {name: 'Software'})
RETURN e.name, labels(e);

-- Full-text search + graph context
CALL db.index.fulltext.queryNodes('entitySearch', 'machine learning')
YIELD node, score
MATCH (node)-[r]-(related)
RETURN node.name, score, type(r), related.name
ORDER BY score DESC;
```

## Performance Tips

| Area                 | Guideline                                                                                                       |
| -------------------- | --------------------------------------------------------------------------------------------------------------- |
| Indexes              | Index properties used in MATCH/WHERE for better query performance. Profile queries with `PROFILE` or `EXPLAIN`. |
| Eager operations     | Avoid patterns that force Cypher to materialize all rows (e.g., `MERGE` after `MATCH` without `WITH`).          |
| Parameterize queries | Using `$param` avoids plan cache misses.                                                                        |
| Batch writes         | Use `UNWIND` + parameters or `apoc.periodic.iterate` for bulk operations.                                       |
| Memory               | Configure `dbms.memory.heap.max_size` and `server.memory.pagecache.size` (page cache should fit the graph).     |
| Traversal depth      | Limit variable-length paths (`*1..5` not `*`) — unbounded paths explode exponentially.                          |
| LIMIT early          | Push `LIMIT` as close to `MATCH` as possible to reduce work.                                                    |

```cypher
-- PROFILE reveals execution plan with actual row counts
PROFILE
MATCH (p:Person)-[:ACTED_IN]->(m:Movie)
WHERE m.released > 2010
RETURN p.name, m.title;
```
