# Knowledge Graphs — Representation, Reasoning, and Link Prediction

Knowledge graphs model world knowledge as entities, relations, and their attributes. They enable structured querying, semantic inference, and downstream AI tasks (recommendation, question answering, semantic search).

## Triple-Based Representation

### RDF (Resource Description Framework) Triples

RDF encodes all knowledge as `(subject, predicate, object)` tuples.

```
ex:Alice ex:knows ex:Bob.
ex:Alice ex:worksAt ex:Google.
ex:Google ex:location "Mountain View".
ex:Alice ex:age "30"^^xsd:integer.
```

**Components**:
- **Subject, Predicate, Object**: IRIs (internationalized resource identifiers), URIs, or literals
- **Literals**: Typed values (strings, integers, dates) with optional language tags and XSD datatypes
- **Blank nodes**: Anonymous entities (e.g., an unnamed author)
- **Named graphs**: Tag triples with a graph identifier (versioning, provenance)

**Advantages**: Global semantic interoperability (URIs are globally unique), formal semantics for reasoning, Linked Data foundation.

**Disadvantages**: Verbose (granular triples), high cardinality (many triples = many lookups), reasoning overhead.

### Property Graphs (Alternative Model)

Nodes and directed edges with properties.

```
Node: {id: 1, label: "Person", name: "Alice", age: 30}
Node: {id: 2, label: "Company", name: "Google"}
Edge: {from: 1, to: 2, label: "worksAt", since: 2020}
```

**Differences from RDF**:
- Implicit local meaning (labels don't need URIs)
- Efficient neighbor traversal (index-free adjacency)
- No built-in inference (custom application logic)

**Adoption**: Neo4j (property graphs), Wikidata (RDF), Amazon Neptune (both models).

## Schema & Semantic Constraints

### OWL (Web Ontology Language)

OWL layer on top of RDF enables logical inference and constraint checking.

**Key constructs**:
- **Class (Type)**: `ex:Person`, `ex:Company` (collections of entities)
- **Property (Relation)**: `ex:knows`, `ex:worksAt` (optional domain/range constraints)
- **Subclass**: `ex:Engineer rdfs:subClassOf ex:Person` (inheritance)
- **Cardinality**: `ex:hasBirthday owl:cardinality 1` (each person has exactly one birthday)
- **Disjointness**: `ex:Person owl:disjointWith ex:Company` (classes don't overlap)
- **Inverse Properties**: `ex:knows owl:inverseOf ex:knows` (symmetric relations)

**Inference Engines** (reasoning):
- **Forward chaining**: Apply rules to derive new facts incrementally
- **Backward chaining**: Query-driven, prove required facts on demand

Example inference:
```
Rule: ?person ex:worksAt ?company ∧ ?company ex:location ?place → ?person ex:locatedIn ?place

Given: Alice ex:worksAt Google. Google ex:location "Mountain View".
Inferred: Alice ex:locatedIn "Mountain View".
```

### SHACL (Shapes Constraint Language)

Validation constraints on RDF data.

```
PersonShape {
  targetClass: ex:Person,
  properties: [
    {property: ex:name, minCount: 1, maxCount: 1, datatype: xsd:string},
    {property: ex:age, minInclusive: 0, maxInclusive: 150}
  ]
}
```

## Querying: SPARQL

SPARQL is SQL for RDF: declarative pattern matching.

```sparql
SELECT ?person ?company WHERE {
  ?person rdf:type ex:Person.
  ?person ex:worksAt ?company.
  ?company ex:location ?city.
  FILTER (?city = "Mountain View")
}
```

**Execution**:
1. Parse graph pattern (triple patterns with variables)
2. Evaluate against RDF store:
   - For each triple pattern, look up matching triples
   - Join results on common variables
   - Apply filters (boolean conditions)
3. Project selected variables, apply aggregation (COUNT, MAX, etc.), ordering

**Extensions**:
- **OPTIONAL**: Left outer join (return results even if pattern doesn't match)
- **UNION**: Combine results from multiple patterns
- **GROUP BY, HAVING**: Aggregation
- **BIND**: Compute new variables via expressions

## Knowledge Extraction & Construction

Building knowledge graphs from unstructured text.

### Named Entity Recognition (NER)

Identify and classify entities (Person, Company, Location).

```
Text: "Steve Jobs founded Apple in 1976."

Entities:
  Steve Jobs: Person
  Apple: Company
  1976: Date
```

### Relation Extraction

Identify and classify relationships between entities.

```
Text: "Steve Jobs founded Apple in 1976."

Relation: (Steve Jobs, founded, Apple)
Relation: (Apple, foundationDate, 1976)
```

**Approaches**:
- **Feature-based**: Hand-crafted linguistic features (POS tags, parse trees), SVM/CRF classifiers
- **Sequence labeling**: BIO tagging (Begin, Inside, Outside), BiLSTM-CRF models
- **Neural (End-to-end)**: Transformer-based (e.g., BERT fine-tuned for NER/relation classification)

### Entity Linking

Map entity mentions to canonical entities in a knowledge base.

```
Text mentions: "Jobs" → Resolved to: dbr:Steve_Jobs (Wikidata/DBpedia)
```

**Challenge**: Disambiguation (many "Jobs" in the world; contextual resolution needed).

## Link Prediction & Embeddings

### Knowledge Graph Embeddings

Represent entities and relations as low-dimensional vectors to enable similarity-based search and link prediction.

#### TransE

Translates relation vectors as offsets in embedding space.

$$\text{subject} + \text{relation} \approx \text{object}$$

For triple (Alice, worksAt, Google):
$$e_{\text{Alice}} + e_{\text{worksAt}} \approx e_{\text{Google}}$$

**Loss**: Margin-based ranking loss penalizes true triples scoring lower than corrupted triples.

**Strengths**: Simple, scalable, interpretable.

**Weakness**: Assumes translations (works poorly for 1-N, N-1, N-N relations).

#### ComplEx

Embedding vectors are complex-valued (real + imaginary components). Captures symmetric/asymmetric and N-N relations better.

$$\text{score}(s, r, o) = \text{Re}(\langle e_s, e_r, \overline{e_o} \rangle)$$

$$= \text{Re}(e_s \cdot e_r \cdot \overline{e_o})$$

Where $\overline{e_o}$ is complex conjugate of object embedding.

#### DistMult, ConvE, Others

Trade-offs between expressiveness and efficiency. Semantic matching (dot product interpretable as similarity), convolutional encoders (higher capacity), rotation-based (RotatE).

### Link Prediction Task

Given (subject, predicate, ?), rank candidate objects.

```
Query: (Alice, worksAt, ?)
Candidates ranked by embedding similarity:
  1. Google (0.95)
  2. Microsoft (0.72)
  3. Apple (0.45)
```

Used for: Recommender systems (user→item predictions), knowledge base completion.

## Reasoning & Rule Mining

### Rule-Based Inference

Patterns extracted from data:

```
Rule 1: (X, worksAt, Y) ∧ (Y, locatedIn, Z) → (X, locatedIn, Z)
Rule 2: (X, knows, Y) ∧ (Y, knows, Z) → (X, mightKnow, Z)
```

**Mining**: Use rule learning algorithms (ILP, association rule mining) to discover frequent patterns automatically.

**Reasoning**: Apply rules forward to derive new facts, or backward to prove queries.

### Semantic Inference

OWL-based:
- **Transitivity**: If (X, parent, Y) ∧ (Y, parent, Z) → (X, ancestor, Z)
- **Subclass reasoning**: If Z is Person and (Q, Person, X) then Q applies to all X in subclasses
- **Property inference**: Inverse, symmetric properties inferred automatically

## Common Systems

### Triple Stores / RDF Stores
- **Apache Jena**: Open-source RDF framework with TDB backend
- **Virtuoso**: Multi-model database (RDF, SQL, SPARQL)
- **RDF4J**: Java RDF API and store
- **Amazon Neptune**: Managed graph database (RDF + property graphs)

### Knowledge Graphs / Semantic Platforms
- **Wikidata**: Community-built open knowledge base (~100M entities, SPARQL endpoint)
- **DBpedia**: Structured extraction from Wikipedia
- **Google Knowledge Graph**: Powers search entity information, not public API
- **Microsoft Academic Graph** (deprecated): Academic knowledge graph

### Property Graph Systems
- **Neo4j**: Market leader (Cypher query language)
- **JanusGraph**: Open-source multi-node GraphDB
- **ArangoDB**: Multi-model (docs + graphs)

## Scale & Performance

**Scalability challenges**:
- **Triple explosion**: Inference rules + ontologies multiply facts (100M base triples → 1B+ derived)
- **Query complexity**: Multi-hop joins are expensive; reasoning inference is NP-hard in general
- **Update latency**: Adding triples to reasoning engine requires re-inference (batch updates more efficient)

**Optimization strategies**:
- **Materialized views**: Pre-compute and store frequent inference results
- **Partitioning**: Hash triples by entity; scale across machines
- **Index structures**: Multiple sort orders (SPO, PSO, OPS) enable fast pattern matching
- **Approximate reasoning**: Trade completeness for speed (subset of rules, timeout-bounded)

## Cross-References

See also: [database-graph-database.md](database-graph-database.md) (property graph systems), [ml-nlp-fundamentals.md](ml-nlp-fundamentals.md) (NER and relation extraction), [genai-embeddings-vectors.md](genai-embeddings-vectors.md) (embedding fundamentals), [cs-natural-language-processing.md](cs-natural-language-processing.md) (NLP for KG construction)