# RAG Architecture — Document Preparation, Vector Stores, Retrieval

## The RAG Stack

Retrieval-Augmented Generation requires four infrastructure layers:

1. **Ingestion**: Document preprocessing, chunking, embedding
2. **Storage**: Vector database, metadata store, indexing
3. **Retrieval**: Similarity search, filtering, reranking
4. **Generation**: Context injection, answer synthesis

Each layer has design choices and trade-offs; poor choices upstream cascade into retrieval failures.

## Document Preprocessing and Chunking

### The Chunking Problem

Documents (PDFs, web pages, books) must be split into passages for embedding and efficient retrieval. Naive chunking (fixed-size windows) breaks coherence and loses context.

**Naive approach (problematic):**
```
Chunk size: 256 tokens, 50% overlap
Document: "The company was founded in 2010. 
          It pioneered cloud storage. 
          By 2020, it had 10,000 employees."

Naive chunks:
[Chunk 1] "The company was founded in 2010. It pioneered..."
[Chunk 2] "...cloud storage. By 2020, it had 10,000..."

Problem: Chunk 2 loses context about what "it" refers to.
```

### Chunking Strategies

#### 1. Semantic Chunking
Split on logical boundaries (sentences, paragraphs, headers) rather than token counts.

```
Document structure:
## Section A: History
  Paragraph 1 (2-3 sentences)
  Paragraph 2
## Section B: Impact
  Paragraph 1
  
Semantic chunks:
- [Section A, Paragraph 1] ~200 tokens
- [Section A, Paragraph 2] ~180 tokens
- [Section B, Paragraph 1] ~220 tokens
```

**Pros**: Preserves semantic coherence, metadata (section, type) attachable.

**Cons**: Requires document structure (not all documents are well-formed); variable chunk size complicates batching.

#### 2. Overlapping Windows
Fixed-size windows with overlap to preserve local context.

```
Chunk size: 512 tokens, 25% overlap
Chunks slide across document:
- [Token 0-512]
- [Token 384-896]
- [Token 768-1280]
```

**Pros**: Simple, deterministic, uniform size.

**Cons**: Creates redundancy (25-50% token overlap); risk of near-duplicate embeddings; context may not align to semantic boundaries.

#### 3. Hierarchical Chunking
Multi-level extraction: summaries at layer 1, detailed passages at layer 2.

```
Document
  ├─ Layer 1 (abstract): 50-100 tokens
  ├─ Layer 2 (section): 200-400 tokens
  └─ Layer 3 (detail): 100-200 tokens per sentence group
```

At retrieval time, query matches are ranked by layer (summary layer for broad queries, detail layer for specifics).

**Pros**: Supports multi-scale queries; high recall via layer hierarchy.

**Cons**: Complex; requires multiple embedding passes; moderate increase in storage.

#### 4. Recursive Splitting
Split on hierarchy of delimiters: paragraphs → sentences → tokens.

```
Attempt split on paragraphs (most coarse)
  If chunk > threshold:
    Attempt split on sentences
      If chunk > threshold:
        Fall back to token-level split
```

Python pseudocode:
```python
def recursive_split(doc, chunk_size=512, overlap=128):
  for delimiter in ['\n\n', '\n', '. ', ' ']:
    chunks = split_on(doc, delimiter)
    if max(len(c) for c in chunks) <= chunk_size:
      return merge_with_overlap(chunks, overlap)
  return token_level_split(doc, chunk_size, overlap)
```

**Pros**: Preserves structure when possible; falls back gracefully.

**Cons**: Implementation complexity moderate.

### Preprocessing Best Practices

- **Clean text**: Remove boilerplate (headers, footers, navigation), normalize whitespace
- **Preserve metadata**: Attach document ID, source URL, publication date, author to each chunk
- **Handle images/tables**: Extract alt-text, captions, or OCR text; don't discard
- **Deduplication**: Remove near-duplicate passages (high Jaccard similarity) before embedding to save storage and reduce noise
- **Language detection**: Separate documents by language; embeddings are language-specific

## Embedding Models

### Model Selection Criteria

| Criteria             | Impact                                                          | Trade-off                                  |
|----------------------|-----------------------------------------------------------------|--------------------------------------------|
| Dimension            | Higher dim → better expressiveness, worse recall speed          | 1536 (OpenAI) vs 384 (smaller models)       |
| Semantic vs. Lexical | Semantic embeddings capture meaning; lexical catch exact terms  | Semantic better for "car" ≈ "automobile"   |
| Domain specificity   | Domain embeddings outperform general on specialized corpora     | Training cost; requires labeled data        |
| Multilingual support | Model handles multiple languages; cross-language search tricky  | Performance degrades on non-training langs  |
| Latency              | Batch embedding offline; per-query embed adds latency           | Large models 50-200ms; small models 5-10ms  |

### Embedding Model Families

**Closed-source (hosted):**
- OpenAI text-embedding-3-large (1536d): High quality, high cost, fully managed
- Cohere embed-english-v3.0: Strong semantic + lexical fusion
- Google Vertex embeddings: Competitive quality, good for GCP ecosystems

**Open-source (self-hosted):**
- BAAI/bge-large-en-1.5 (1024d): Strong on MTEB benchmark, MIT license
- jinaai/jina-embeddings-v2-base-en (768d): Good latency/quality balance
- sentence-transformers/all-MiniLM-L6-v2 (384d): Lightweight, fast
- gte-base (384d): Lightweight semantic embeddings

**Specialized:**
- Domain-adapted models (e.g., BioBERT for biomedical): 10-20% better precision on domain tasks
- Multilingual models (e.g., xlm-r-distilroberta-base): Support 100+ languages but trade per-language quality

### Dimension and Performance

Smaller embedding dimensions (256-384) are faster to search but lose nuance. Larger dimensions (1024-1536) capture finer semantic distinctions:

```
Task: Questions about Python programming
Model 1: 384-dim → Recall@10: 78%
Model 2: 1536-dim → Recall@10: 89%
Cost per query: Model 2 ~2x slower similarity search
```

## Vector Databases

### Storage and Search Trade-offs

| Database            | Index Type | Latency (1M vectors) | Scalability | Use Case                                  |
|---------------------|------------|----------------------|-------------|-------------------------------------------|
| Pinecone            | HNSW       | <50ms@1B             | Fully managed, millions of collections | Serverless, multi-tenancy, search-as-service |
| Weaviate            | HNSW       | <100ms@1M            | Kubernetes-native, self-hosted or managed | On-prem privacy, hybrid search              |
| Qdrant              | HNSW       | <10ms@1M             | Snapshot/backup, high-performance         | High-volume, latency-sensitive workloads    |
| Chroma              | Annoy/HNSW | <100ms@100K          | Lightweight, in-process or serverless    | Prototyping, single-machine                 |
| pgvector (Postgres) | IVFFlat    | <500ms@1M            | Built on PostgreSQL, ACID transactional | SQL + vector, ACID guarantees needed        |
| Milvus              | HNSW/IVF   | <50ms@1B             | Cloud-native, distributed                | Large-scale deployments, on-prem clusters  |

### What These Databases Provide

**Core vector search:**
All support approximate nearest neighbor search over high-dimensional embeddings. HNSW (Hierarchical Navigable Small World) is most common and offers ~100ms queries at 1B scale.

**Metadata filtering:**
Filter results by schema fields (date range, category, source) BEFORE or AFTER similarity search.

```
Query: "machine learning models"
  → Find top 100 by similarity
  → Filter: (date > "2023-01-01") AND (source = "arxiv")
  → Return top 10 remaining
```

Essential for limiting scope (don't search an entire corpus if a category filter narrows it).

**Hybrid search:**
Combine dense embeddings (semantic) with sparse lexical search (BM25/TF-IDF). Weaviate and others support this.

```
Query: "python async await"
Semantic embedding score: 0.85 (captures "Python concurrency")
Lexical BM25 score: 0.92 (exact terms match)
Hybrid score: 0.9 (weighted average)
```

Hybrid search recovers exact-term matches that semantic search misses; effective for technical/scientific queries.

**Persistence and replication:**
Production databases replicate across nodes, persist to disk, support backups and failover. In-memory databases (Chroma by default) are fast but data-loss risky.

### Vector Database Anti-Patterns

- **Storing raw documents in vectors**: Store document ID; metadata separately. Vectors should be immutable IDs to embeddings.
- **No metadata indexing**: Without indexed metadata fields, filtering (date, category, author) is slow.
- **Homogeneous dimension**: Mixing 384-dim and 1536-dim embeddings from different models breaks similarity. Normalize to single dimension.
- **Stale indexes**: If vectors grow but index isn't re-optimized, search degradation (latency, recall) follows.

## Retrieval Strategies

### Similarity Search (Dense Retrieval)

Standard approach: embed query, find K nearest neighbors in vector space.

```
Query: "How do transformers work?"
Query embedding: [0.2, -0.5, 0.8, ...]
Database search: Find top-10 vectors closest to query embedding
Results: Passages from papers on attention, BERT, transformer architecture
```

**Pros**: Fast, semantic understanding, language-agnostic.

**Pros**: Misses exact terms ("transformer architecture" might rank lower than semantically similar "neural attention mechanism").

### Hybrid Search (Dense + Sparse)

Combine semantic with keyword matching via BM25 or TF-IDF.

```
1. Dense search: Top 100 by embedding similarity
2. Sparse search: Top 100 by BM25 keyword overlap
3. Score fusion: Combined score = 0.7 * dense_score + 0.3 * sparse_score
4. Return: Top-20 by fused score
```

**Pros**: Captures both semantic and exact-match results; strong of precise technical queries.

**Cons**: Latency ~2x vs dense alone; tuning fusion weights is empirical.

### Reranking

After retrieval, rerank results with a more expensive model.

```
1. Fast retrieval: Retrieve 100 candidates (fast embedding model)
2. Rerank: Score top 100 with expensive cross-encoder
3. Return: Top-10 after reranking
```

Cross-encoders (e.g., ms-marco-MiniLM) compute relevance scores directly without embedding:
```
Score = cross_encoder(query, candidate_text)
```

More accurate than embedding similarity but slower; used as second stage.

**Pros**: Precision improved 5-15%; late-stage filtering cost amortized over retrieval.

**Cons**: Adds latency (~50-100ms for reranking 100 candidates).

### Query Expansion and Decomposition

**Query expansion**: Generate multiple query variants, retrieve for each, merge results.

```
Original query: "climate change effects on agriculture"
Expanded queries:
- "global warming agricultural impact"
- "crop yield climate"
- "drought farming"
```

Union results, deduplicate by document ID.

**Query decomposition**: Break complex query into sub-queries.

```
Query: "How did World War 2 end in Europe and the Pacific?"
Decompose:
- "World War 2 Europe end 1945"
- "World War 2 Pacific end 1945"
Retrieve separately, interleave results.
```

Both increase recall but add latency and require aggregation logic.

## Generation with Retrieved Context

### Context Injection

Once passages are retrieved, inject into the LLM prompt:

```
System: You are a Q&A assistant. Answer based solely on provided context.

Context:
[Document 1]: "Transformers were introduced by Vaswani et al. in 2017..."
[Document 2]: "Attention mechanisms compute weighted sums of values..."
[Document 3]: "Positional encodings inject word order into embeddings..."

Question: How do transformers use positional encodings?

Answer: [LLM generates here]
```

Order of context matters; models weight early context more heavily. Retrieve most relevant first.

### Context Window Management

LLMs have finite context windows (2K-200K tokens). For large retrieval results:

1. **Truncation**: Keep top-N passages (typically 5-10), discard rest
2. **Compression**: Summarize passages before injection
3. **Sliding window**: If document is long, retrieve chunks separately
4. **Lost-in-the-middle effect**: Place most critical context at beginning or end; middle context less attended

Modern long-context models (Claude 3.5, GPT-4 Turbo 128K window) mitigate this; use all retrieved passages.

### Hallucination Mitigation

Retrieved context reduces but doesn't eliminate hallucination:

- **Attributed generation**: Force model to cite source documents: "According to [Document 3], ..."
- **Faithfulness scoring**: Verify generated answer against retrieved context; flag if no support found
- **Few-shot grounding**: Provide examples of well-attributed answers in prompt

```
Example:
Q: What year was Python released?
A (hallucinated): Python was released in 1989.
A (grounded): According to the Python.org docs, Python 1.0 was released 
             in January 1994. (Source: python-history.md)
```

## See Also

- **genai-rag-patterns.md**: Core RAG pipeline and design patterns
- **genai-embeddings-vectors.md**: Embedding model architectures and training
- **database-vector.md**: Technical details of vector database indexes (HNSW, IVF, SCANN)
- **architecture-search-platform.md**: Large-scale search infrastructure
- **cs-information-retrieval.md**: Ranked retrieval, BM25, and evaluation metrics