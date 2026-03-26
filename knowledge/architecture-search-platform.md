# Search Platform Architecture — Indexing, Query Processing, Ranking & Experimentation

## Overview

**Search platform architecture** describes systems that enable users to find relevant information from large corpora efficiently. A search platform combines: document indexing (build-time), query processing (search-time), ranking (relevance scoring), and operational infrastructure (monitoring, A/B testing).

Understanding search architecture is relevant beyond search engines: any system with full-text search, filtering, autocomplete, or relevance ranking (document retrieval, e-commerce, recommendations) applies these principles.

See cs-information-retrieval.md for indexing data structures (inverted index, BM25, embeddings) and relevance models. This note focuses on architectural systems and patterns.

---

## Indexing Pipeline

### Index Build (Offline Batch)

Periodically rebuild index from source documents.

```
Documents → Parse → Tokenize → Index construction → Compression → Persist
            ↓                         ↓
         HTML → text              Inverted list
         JSON → fields            + postings
```

**Pipeline steps**:

1. **Extraction**: Fetch documents (database, file store, API)
2. **Parsing**: Convert to structured fields (title, body, metadata)
3. **Tokenization**: Split text into terms; handle special cases (hyphens, contractions, language-specific)
4. **Normalization**: Lowercase, stemming (running → run), stop word removal ("the", "a")
5. **Indexing**: Build inverted index (term → document list)
6. **Compression**: Encode posting lists to reduce disk size (delta varbyte)
7. **Persist**: Write to disk (B-tree, segment files)

**Frequency**: Daily (popular), hourly (news), or real-time (streaming → indexed within minutes)

### Real-Time Indexing (Streaming)

For fresher index, process new documents as they arrive.

```
New document → Parse → Add to index → Update online replica
               ↓
           Memory buffer (refreshed periodically)
```

**Trade-offs**:
- Lower latency (minutes vs. hours)
- Operational complexity: maintain in-memory index, durability, failover
- Throughput: streaming systems add latency if index operations are bottleneck

**Systems**: Kafka → Elasticsearch, Kafka → Solr, Kafka → custom index

### Index Structures

Modern search engines use layered indexing:

```
Hot index (memory, last hours)     — fastest, smallest
Warm index (SSD, last days)        — medium speed
Cold index (archived, years old)   — searched rarely

Query merges results from hot + warm, returns combined results
```

**Sharding**: Split documents across index shards (shard 0: A-M surnames, shard 1: N-Z).
Each shard maintains its own index replica for parallel search.

---

## Query Processing Pipeline

### Parsing & Rewriting

Convert user query to system query.

**Input**: `iphone case black under:$50`

**Parsing**:
```
Term: iphone
Term: case
Filter: black
Filter: price < 50
```

**Tokenization**: Split "iphone-case" → "iphone" + "case" (term splitting)

**Stemming**: Case normalization (iPhone → iphone, CASE → case)

**Query expansion**: Synonym expansion (iphone → iPhone, apple phone), fuzzy matching (mistypings corrected)

### Filtering

Apply structured predicates before ranking to reduce result set.

```
Query: "coffee" + filter:hot_drinks=true + filter:price<15
Matching: Coffee AND category=hot_drinks AND price<15
```

**Execution**: Index column stores enable fast filtering (Elasticsearch uses inverted indexes on fields)

### Query Execution (Retrieval)

Retrieve candidate documents matching query terms.

```
Query: ["coffee", "maker"]
Lookup: inverted_index["coffee"] → [doc1, doc3, doc15, ...]
Lookup: inverted_index["maker"]  → [doc2, doc3, doc5, ...]
Merge: AND → [doc3]
```

**Merge algorithms**:
- Conjunction (AND): Intersect posting lists (fast bitwise ops)
- Disjunction (OR): Union posting lists (more results, slower to rank)

**Boolean vs. ranked**: Boolean queries return yes/no; ranked queries score each result

---

## Ranking

Once candidates retrieved, rank by relevance. See cs-information-retrieval.md for BM25, TF-IDF details.

### Raw Scoring Functions

#### TF-IDF

$$\text{TF-IDF}(q, d) = \sum_{t \in q} \text{TF}(t, d) \times \text{IDF}(t)$$

Baseline; fast; works for keyword queries.

#### BM25

Industry standard probabilistic ranking:

$$\text{BM25}(q, d) = \sum_{t \in q} \text{IDF}(t) \times \frac{f(t, d) \times (k_1 + 1)}{f(t, d) + k_1 (1 - b + b \times L_d / L_{avg})}$$

Saturation effects: diminishing returns from repeated terms. Default parameters (k₁=1.2, b=0.75) tuned on TREC datasets.

#### Semantic Ranking

Vector embeddings + cosine similarity. Query and documents embedded in high-dimensional space.

$$\text{score}(q, d) = \cos(\text{embed}(q), \text{embed}(d))$$

**Fit**: Conceptual search ("show me romantic movies") vs. keyword search

**Trade-offs**: More expensive (embedding lookup ≈ 10-100x slower); requires embedding model

### Learning-to-Rank (LTR)

Train ML model to predict relevance scores from features.

**Input features**:
- BM25 score
- Document freshness (age)
- Document popularity (views, clicks)
- Position in inverted list (position bias)
- Query length, document length
- Click history (if user clicked this doc for similar queries before)

**Output**: Relevance score (0-1)

**Model**: Gradient Boosted Decision Trees (LightGBM, XGBoost) fastest in production.

```
Query + Document → Feature extraction → LTR model → Relevance score
                                                    (BM25=0.5, fresh=0.3, pop=0.2 → 0.8)
```

**Advantages**:
- Combines multiple signals beyond keyword match
- Learns user patterns (what documents users click)
- Domain-specific tuning possible

**Disadvantages**:
- Training data required (clicks, ratings)
- Model maintenance; retraining on stale data hurts quality
- Feature engineering effort

### Re-ranking

Two-stage retrieval: retrieve 1,000 candidates fast, re-rank top 100 with expensive model.

```
BM25: 1M docs → 1,000 candidates (fast, best recall)
LTR:  1,000 candidates → 100 results (slow, better precision)
User sees top 10
```

**Latency tradeoff**: Fast retrieval, expensive ranking, returns in <100ms

---

## Features Influencing Ranking

### Content Signals

**Text relevance**: BM25, embeddings, word overlap
**Recency**: Fresh documents ranked higher (time decay)
**Authority**: Popular documents (views, links, citations)
**Completeness**: Well-described documents (metadata, length) vs. sparse

### User Context

**Query history**: User previously searched X; results for query about X ranked higher
**Personalization**: User location, device, language preferences
**Implicit feedback**: User clicked result before; click history boosts ranking

### Engagement Signals

**Click data**: Which results users click ranks them higher (positive signal)
**Dwell time**: Long time on result = likely relevant (weak signal; noisy)
**Skip rate**: Users skip result = irrelevant (negative signal)

---

## Query Features & Suggestions

### Autocomplete

Suggest queries as user types.

```
User types: "cof"
Suggestions: "coffee maker", "coffee beans", "coffee shop near me"
```

**Implementation**:
1. Trie/prefix tree: keyed by character prefix
2. Sorted by frequency/popularity/personalization
3. Return top-k suggestions

**Data source**: Historical query logs (aggregate popular queries)

**Latency**: <50ms required for interactive UI

### Query Suggestions

Suggest related queries or refinements.

```
Query: "laptop"
Suggestions: "laptop for gaming", "cheap laptops", "laptop near me"
```

**Method**: N-gram model trained on query logs; or implicit collaborative filtering (if you searched X, users also searched Y)

---

## Faceted Search & Filtering

Enable structured filtering alongside ranking.

```
Query: "shoes"
Facets:
  Brand: Nike (500), Adidas (300), Puma (100)
  Size: 8 (200), 9 (250), 10 (180)
  Price: <$50 (400), $50-100 (300), >$100 (200)

User can narrow: Brand=Nike AND Size=9 AND Price<$100
```

**Implementation**: Filtered result set at retrieval time, then aggregate field values for facets

---

## A/B Testing Search Quality

Measuring success of ranking changes.

### Metrics

**Offline (no users)**:
- NDCG (Normalized Discounted Cumulative Gain): Predicted vs. ground truth ranking
- MAP (Mean Average Precision): Proportion of relevant docs in top-k
- MRR (Mean Reciprocal Rank): Position of first relevant result

**Online (A/B test)**:
- Click-through rate (CTR): % of users who click a result
- Query success rate: % of queries where user clicked something
- User satisfaction (survey): Explicit rating of results
- Internal metric (engagement): Dwell time, revisits

### Experiment Framework

```
Traffic split:
  50% → Control (old ranking)
  50% → Treatment (new ranking)

Record: Queries, results shown, clicks, engagement
Analyze: Did treatment improve CTR/satisfaction?
Statistical test: Is improvement significant (p < 0.05)?
```

**Challenges**:
- Position bias: Top result clicked more than bottom (regardless of quality)
- Selection bias: Different users in treatment vs. control
- Interleaving: Mix old and new ranking, average user preference
- Long-tail: Small sample size for rare queries; stats underpowered

---

## Systems Considerations

### Latency Requirements

p99 latency target: <100ms (typical requirement)

| Component | Latency |
|-----------|---------|
| Parse query | 1ms |
| Retrieve candidates (BM25) | 10-20ms |
| Rank (LTR) | 30-50ms |
| Format results | 5ms |
| Network | 10-15ms |
| **Total** | ~60-100ms |

Each 10ms increase in latency → measurable drop in user engagement.

### Scalability

**Index size**: GB to TB (Elasticsearch cluster: 100GB-1TB per node)

**Throughput**: 100K+ QPS (queries per second) requires clustering

**Replication**: Index replicated across nodes for availability and parallelism

---

## Emerging Directions

### Dense Retrieval (Embedding-Based)

Replace BM25 + sparse retrieval with dense embeddings + vector search. More robust to semantic variations, domain shifts.

**Trade-off**: Embedding lookup slower (requires vector DB), but captures meaning better.

### Hybrid Search

Combine BM25 (keyword precision) + embeddings (semantic recall).

```
Results = combine(
  bm25_results(query),
  embedding_results(query)
)
```

### Cross-Encoder Ranking

Instead of scoring document independently, score query + document pair jointly.

```
Input: (query, document) → Model → Relevance score
vs. old: compute embedding(query), embedding(doc), score cosine similarity
```

Slower but more accurate; used in re-ranking phase.

---

## See Also

- cs-information-retrieval.md — Indexing, BM25, TF-IDF, core data structures
- database-elasticsearch.md — Elasticsearch-specific architecture
- data-search-patterns.md — Search application patterns
- ml-operations.md — ML model serving (LTR models in production)
- genai-embeddings-vectors.md — Embedding-based search