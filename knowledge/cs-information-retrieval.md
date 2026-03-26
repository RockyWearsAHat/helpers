# Information Retrieval — Indexing, Ranking, and Search Architectures

Information retrieval (IR) is the science of finding relevant information in large text corpora efficiently. Unlike databases (which find exact records), IR systems rank results by relevance probability, enabling full-text search over unstructured documents.

## Core Data Structures

### Inverted Index

The fundamental IR data structure. Maps terms to the documents containing them.

```
Documents:
1: "machine learning algorithms"
2: "deep learning neural networks"
3: "machine learning models"

Inverted Index:
  "machine" → [1, 3]
  "learning" → [1, 2, 3]
  "algorithms" → [1]
  "deep" → [2]
  "neural" → [2]
  "networks" → [2]
  "models" → [3]
```

**Posting lists** store document IDs (or (docID, position) pairs for phrase queries). Compressed with delta encoding and variable-byte encoding to reduce memory.

**Index construction** (offline):
1. Parse documents into tokens (tokenization)
2. Build term→posting list map
3. Sort postings by docID (enables binary search, compression)
4. Compress and persist to disk

**Query evaluation** (online):
1. Parse query, tokenize
2. Look up each query term's posting list
3. Merge postings (AND for conjunction, OR for disjunction)
4. Return matching docIDs

### BKD Trees (Block K-D Trees)

For numeric, date, and geospatial fields: hierarchical range indexes enabling efficient range queries (e.g., `price < 100`, `location within 5km`).

## Relevance Ranking

### TF-IDF (Term Frequency–Inverse Document Frequency)

A probabilistic scoring model. Assumes: relevant documents contain query terms frequently, and rare terms are more discriminative.

$$\text{TF-IDF}(q, d) = \sum_{t \in q} \text{TF}(t, d) \times \text{IDF}(t)$$

Where:
- $\text{TF}(t, d)$ = term frequency in document (raw count, log-scaled, or boolean)
- $\text{IDF}(t) = \log \frac{N}{n_t}$ where $N$ = total docs, $n_t$ = docs containing $t$

**Interpretation**: Terms appearing in many documents get low IDF (less discriminative); terms appearing in few documents get high IDF.

**Strengths**: Fast, interpretable, works well for keyword queries.

**Limits**: No term dependencies (treats query terms independently), no semantic understanding.

### BM25 (Best Matching 25)

Probabilistic ranking function, derived from the Binary Independence Model (BIM). Refinement of TF-IDF with saturation effects.

$$\text{BM25}(q, d) = \sum_{t \in q} \text{IDF}(t) \times \frac{f(t, d) \times (k_1 + 1)}{f(t, d) + k_1 \times \left(1 - b + b \times \frac{|d|}{L}\right)}$$

Where:
- $f(t, d)$ = raw term frequency in document
- $|d|$ = document length, $L$ = average document length
- $k_1$ (usually 1.2) = term saturation parameter
- $b$ (usually 0.75) = length normalization parameter

**Key improvement over TF-IDF**:
- Term frequency saturates: doubling term count doesn't double score (avoids spam)
- Length normalization: penalizes long documents (otherwise they win on raw count alone)
- Tunable parameters for domain-specific data

**Industry adoption**: Standard in Elasticsearch, Lucene, Solr.

### Vector Search (Dense Ranking)

Represent queries and documents as dense embedding vectors (using neural models like BERT), then rank by cosine similarity.

$$\text{relevance}(q, d) = \cos(e_q, e_d) = \frac{e_q \cdot e_d}{||e_q|| \cdot ||e_d||}$$

**Strengths**: Semantic understanding (synonymy, paraphrase), multilingual transfer, out-of-vocabulary handling.

**Limits**: Computationally expensive (embedding inference), requires pre-trained models, less interpretable than BM25.

**Efficiency**: Approximate nearest neighbor (ANN) search — HNSW, LSH, IVF — enables sub-linear search time.

## Hybrid Search

Combine sparse (BM25) and dense (vector) ranking:

1. **Fetch**: Run BM25 query and vector query in parallel, each returning top-K results
2. **Merge**: Combine and re-rank by normalized weighted sum: $\alpha \times \text{BM25} + (1-\alpha) \times \text{cosine}$

**Rationale**: BM25 excels at exact keyword matching; vectors excel at semantic match. Neither alone optimizes all use cases.

## Search Engine Architecture

### Indexing Pipeline

```
Raw Documents
    ↓ [Parser]
Structured Records
    ↓ [Analyzer: Tokenize, Lowercase, Stemming, Stop-word Removal]
Tokens
    ↓ [Indexer: Build Inverted Index]
Indexed Shards (on Disk)
```

**Design decisions**:
- **Tokenization**: How to split text (whitespace, punctuation-aware, morphology-aware)
- **Stemming/Lemmatization**: Reduce inflections (running→run, better→good). Trade-off: recall gain vs. precision loss
- **Stop words**: Remove common words (the, a, is) to reduce index size (often optional in modern IR)
- **Analyzers**: Language-specific (English stemming differs from German)

### Query Processing

```
Query String ("machine learning models")
    ↓ [Parse → AND Operator]
    ↓ [Tokenize, Analyzer]
Tokens: ["machine", "learning", "models"]
    ↓ [Lookup Posting Lists]
Postings: machine=[1,3], learning=[1,2,3], models=[3]
    ↓ [Merge (AND)]
Result Docs: [3] (only doc 3 contains all terms)
    ↓ [Rank (BM25/Vector)]
Ranked Results: [(3, 0.95)]
```

**Query complexity**: 
- Proximity queries (phrases): "deep learning" requires consecutive postings
- Boolean: AND, OR, NOT
- Wildcard: prefix*, *suffix (demands special index structures)
- Range: date > 2020 (uses BKD tree)

### Distributed Indexing (Multi-shard Architecture)

For large corpora (billions of documents):

- **Primary shards**: Index partitions across nodes (e.g., hash-based partitioning by docID)
- **Replicas**: Read replicas on different nodes (query load balancing, fault tolerance)
- **Merge policy**: Periodically merge small segment files (reduces query overhead)
- **Refresh**: Flush in-memory indices to disk (configurable delay: 1s default in Elasticsearch, trades latency for throughput)

**Query execution**:
1. Client sends query to coordinator
2. Coordinator broadcasts to all shards (or subset via routing)
3. Each shard evaluates locally, returns top-K results + scores
4. Coordinator merges and re-ranks top-K from all shards
5. Fetch full documents from primary/replica

## Faceted Search & Aggregations

**Faceting**: Count results by category dimension (refine search iteratively).

```
Query: "laptop"
Results: 50,000 documents

Facets:
  Brand: Apple(5000), Dell(4500), HP(3200), ...
  Price Range: $0-500(3000), $500-1000(2500), ...
  Condition: New(30000), Used(20000)
```

Implemented with:
- **Filter-then-aggregate**: Execute main query, then run sub-queries per facet
- **Shared term aggregations**: Use posting lists (term→docID) and bitmaps for efficient counting

## Relevance Tuning

**A/B testing**: Compare ranking algorithms on live queries, measure click-through rate, dwell time, conversion.

**Metrics**:
- Precision@K: Among top K results, how many are relevant?
- Recall: What fraction of relevant documents were returned?
- NDCG (Normalized Discounted Cumulative Gain): Penalizes irrelevant results appearing early
- MRR (Mean Reciprocal Rank): Where is the first relevant result?

**Tuning levers**:
- BM25 parameters ($k_1$, $b$): Domain-specific (scientific documents vs. tweets differ)
- Embedding model: General (BERT) vs. domain-specific (SciBERT for papers, BioBERT for biomedical)
- Weighting in hybrid search
- Position bias in learning-to-rank models

## Common Systems

- **Lucene**: Embeddable Java IR library; powers Elasticsearch, Solr
- **Elasticsearch**: Distributed search engine, REST API, near-real-time indexing
- **Solr**: Enterprise search, faceted search, complex queries
- **Weaviate, Pinecone, Milvus**: Vector databases with vector search
- **Meilisearch**: Lightweight fulltext search, WASM-based, instant results

## Cross-References

See also: [database-elasticsearch.md](database-elasticsearch.md) (implementation details), [database-vector.md](database-vector.md) (vector indexing), [genai-rag-patterns.md](genai-rag-patterns.md) (IR in RAG), [genai-embeddings-vectors.md](genai-embeddings-vectors.md) (embedding models)