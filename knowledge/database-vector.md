# Vector Databases — Embeddings, Similarity Search, and ANN Algorithms

Vector databases store and retrieve high-dimensional embeddings efficiently, enabling semantic search and retrieval-augmented generation (RAG). The core operation is **similarity search**: given a query vector, find the K most similar vectors in the dataset.

## Vector Fundamentals

### Embeddings

An **embedding** is a learned mapping from discrete entities (text, images, users) to dense vectors in high-dimensional space (typically 384–1536 dimensions for text models, 4096+ for multimodal).

Models like BERT, GPT, or CLIP output embeddings where **similar entities cluster together** — cosine distance between embeddings reflects semantic similarity.

Example: Sentence embeddings from a model like MiniLM:
```
"The cat sat on the mat" → [0.012, -0.089, 0.234, ..., 0.105] (384-dim)
"A feline rested on a rug" → [0.018, -0.091, 0.231, ..., 0.103] (384-dim)
    (very close in space)
```

### Similarity Metrics

Distance measures between vectors q and p:

| Metric | Formula | Range | Use Case |
|--------|---------|-------|----------|
| Cosine Similarity | (q · p) / (||q|| · ||p||) | [-1, 1] | Cosine distance = 1 - similarity; normalized vectors; text embeddings |
| Euclidean (L2) | √Σ(q_i - p_i)² | [0, ∞) | Untransformed embeddings; stable for outliers |
| Manhattan (L1) | Σ\|q_i - p_i\| | [0, ∞) | Similar to L2, computationally simpler, rarely used |
| Dot Product | q · p | (-∞, ∞) | Fast; preference signals in recommendation systems |
| Hamming | # differing bits | [0, d] | Binary vectors; very fast on CPU |

Cosine distance is standard for NLP embeddings (they're unit-normalized). Euclidean/L2 is common for vision embeddings and general-purpose learned representations.

## Approximate Nearest Neighbor (ANN) Algorithms

Exact k-NN search in high dimensions is computationally expensive. **Approximate Nearest Neighbor** algorithms trade perfect accuracy for speed, finding "good-enough" nearest neighbors in sub-linear time.

### HNSW (Hierarchical Navigable Small World)

**Core idea**: Build a hierarchical structure (similar to skip lists) where each layer is a sparse proximity graph.

**Construction**:
1. Start with a single node; successively insert new nodes
2. For each new node, find nearby nodes in the upper layer, then descend through layers, greedily moving toward the query vector
3. When inserting, connect the new node to its M nearest neighbors at each layer

**Search**:
1. Start at the top layer with an entry point
2. Search greedily (enter/exit neighbors) at current layer; when no closer neighbor exists, descend
3. At the bottom layer, collect K neighbors

**Properties**:
- **Time complexity**: O(log N) search, O(log N) insertion (probabilistic)
- **Memory**: O(N) with small constants (M ≈ 10-20)
- **Strengths**: Fast, low memory, excellent for incremental indexing
- **Weaknesses**: Hyperparameter tuning (M, ef), can get stuck in local minima (especially for high-dimensional spaces)

Used in: Pinecone, Weaviate, Milvus, pgvector, LanceDB.

### IVF (Inverted File / Inverted Vector)

**Core idea**: Cluster vectors into K partitions, store partition assignments, then search relevant partitions.

**Construction**:
1. Apply k-means clustering (or another method) to partition N vectors into K clusters
2. For each cluster, compute centroid; store cluster assignments and pointer lists

**Search**:
1. Compute nearest clusters to query (usually probe top-M clusters)
2. Search only vectors in those clusters (still exhaustive within partition, but much fewer vectors)

**Properties**:
- **Time complexity**: O(probes × partition_size) — typically much faster than linear for well-separated clusters
- **Memory**: O(N) with minimal overhead
- **Strengths**: Extremelya fast for highly clustered data, easy to parallelize
- **Weaknesses**: Sensitive to cluster quality and dimensionality; performance degrades if clusters overlap; retraining required when data grows

Used in: Milvus, Chroma, Weaviate (optional), Vespa.

### Product Quantization (PQ)

**Core idea**: Reduce vector dimensionality lossily by quantizing subspaces.

**Construction**:
1. Divide d-dimensional vectors into m segments (e.g., 96-dim vectors → 8 segments of 12-dim)
2. For each segment, apply k-means (k=256) to create 256 cluster centroids
3. Encode each segment as a 1-byte ID (0-255 of the centroid)

**Search**:
1. Quantize the query vector the same way
2. Look up distances between query subspaces and stored centroids (256 × m lookups, precomputed)
3. Sum asymmetric distances; retrieve vectors with smallest distances

**Properties**:
- **Memory**: Extreme compression (32x or more); 384-dim float32 → 48 bytes → 12 bytes (with 4-bit quantization)
- **Time complexity**: O(N × m × lookups) — still linear but with small constants
- **Strengths**: Fits massive datasets in RAM; cache-efficient
- **Weaknesses**: Information loss; quality depends on PQ segment tuning; rarely used alone (often combined with IVF or HNSW)

Used in: Pinecone (hybrid), Milvus (optional).

### Comparison

| Algorithm | Search Latency | Memory | Insertion | Best For |
|-----------|----------------|--------|-----------|----------|
| HNSW | Fast | Low | Fast, dynamic | Real-time, streaming |
| IVF | Very fast (clustered data) | Low | Slow (retraining) | Large, static datasets |
| PQ | Fast | Minimal | N/A (offline) | Memory-constrained |
| Brute Force | Slow (O(N)) | Min | Instant | Baseline, <1M vectors |

Most systems combine: IVF + PQ (coarse + fine quantization) or HNSW with post-filtering.

## Index-Level Features

### Metadata Filtering

Queries often include structured predicates: "Find similar documents where author='Knuth' and year > 2000."

**Approaches**:
1. **Post-filtering**: Run ANN search, filter results in-memory (simple, but returns fewer than K results)
2. **Filtering before search**: Pre-filter candidates, run ANN only on matching subset (loses diversity)
3. **Hybrid filtering**: Index metadata alongside vectors, prune search space using predicates during traversal (more complex, better results)

Most mature systems (Milvus, Pinecone, Weaviate) support hybrid filtering, trading query latency for accuracy.

### Sparse and Dense Vectors

**Sparse vectors** (mostly zeros, like TF-IDF or BM25 scores) enable keyword-based retrieval.

**Dense vectors** (embeddings) enable semantic retrieval.

**Hybrid search** combines both:

```python
# Pseudocode: hybrid retrieval for RAG
sparse_results = bm25_search(query, k=10)  # Keyword match
dense_results = vector_search(embedding(query), k=10)  # Semantic match
combined = rerank(sparse_results + dense_results, query, k=5)
```

Systems like Weaviate and Milvus support both in a single index.

## Storage and Indexing Strategies

### In-Memory vs. On-Disk Storage

- **In-memory** (Pinecone, Weaviate): Full vectors + metadata in RAM. Fast but expensive at scale.
- **On-disk** (Milvus with RocksDB, pgvector with B-trees): Compressed or quantized vectors on disk. Slower, cheaper.
- **Hybrid** (Pinecone Plus, Milvus with tiering): Hot data in RAM, cold data on disk.

### Index Construction and Rebuild

Rebuilding indexes (especially IVF, PQ) requires offline work. Systems vary:
- **Real-time rebuild**: ANN algorithms that support incremental insertion (HNSW) avoid rebuild overhead
- **Scheduled rebuild**: Accept stale indexes for speed (Milvus allows async rebuilds)
- **Streaming updates**: Buffer new vectors, periodically merge into main index (common in databases)

## RAG Integration and Query Patterns

### Retrieval-Augmented Generation

RAG pipeline: Query → retrieve context from vector DB → pass to LLM → generate answer.

```python
# Pseudocode
query = "What did Einstein mean by E=mc²?"
context_vectors = vector_db.search(embedding(query), k=3, filter={author: "Einstein"})
context_text = [doc for doc, score in context_vectors]
prompt = f"Using this context: {context_text}, answer: {query}"
answer = llm.generate(prompt)
```

### Real-World Challenges

1. **Embedding drift**: Model versions change; recompute embeddings for consistency
2. **Staleness**: Vector DB and source documents diverge; require sync mechanisms
3. **Cardinality and recall**: High recall (R@10) vs. latency trade-off; often need 100+ top-K candidates for quality reranking
4. **Chunk boundaries**: Splitting documents into chunks, then embedding each chunk—chunk size affects retrieval quality

## Vector Database Comparison

### Pinecone

- **Model**: Managed, cloud-only, proprietary ANN (HNSW-inspired)
- **Strengths**: Easiest to start (no ops), hybrid search, metadata filtering, namespaces for multi-tenancy
- **Trade-offs**: Vendor lock-in, pricing sensitive to dimension count and storage

### Weaviate

- **Model**: Open-source, self-hosted or managed, HNSW with dynamic indexing
- **Strengths**: Built-in GraphQL, multi-vector support, modular architecture
- **Trade-offs**: Operational overhead, less mature cluster scaling than commercial alternatives

### Milvus

- **Model**: Open-source, distributed, supports HNSW, IVF, IVF-PQ combinations
- **Strengths**: Highly configurable, good for research, strong cluster scaling, no vendor lock-in
- **Trade-offs**: Steeper operational learning curve, requires tune-ups for production

### pgvector (PostgreSQL Extension)

- **Model**: In-database vectors, local (single-node) or via streaming replication
- **Strengths**: Familiar SQL, ACID guarantees, existing backup/replication infrastructure
- **Trade-offs**: Performance lags specialized systems, limited to ~100k vectors per node practically

## Practical Considerations

### Dimensionality and Compression

Higher dimensions (1536-dim models) make faster retrieval harder. Consider:
- Dimensionality reduction (PCA, learned compression) for initial filtering
- Quantization (8-bit, 4-bit) for aggressive memory reduction
- Distillation of larger models into smaller embeddings

### Recall vs. Latency

ANN algorithms don't guarantee top-K correctness. Examine recall metrics:
- **Recall@10**: "Of true top-10 results, how many did ANN return?"
- **Recall@100**: More forgiving for retrieval then reranking

Most RAG systems target 85-95% recall; beyond that, latency gains plateau.

### Cost and Scale

Vector DBs charge on stored dimensions × vectors. A 1M-vector, 1536-dim dataset:
- Pinecone: ~$500/month for standard index
- Self-hosted (Milvus, Weaviate): Pay for compute + bandwidth, minimal storage cost

Evaluate your scale and query SLAs before committing.