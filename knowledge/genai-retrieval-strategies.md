# RAG Retrieval Strategies — Techniques for Document Discovery and Ranking

## Core Retrieval Paradigms

Retrieval-augmented generation depends on effective document retrieval. Modern RAG systems employ multiple retrieval strategies, each with distinct trade-offs:

### Semantic Search (Dense Retrieval)

Semantic search uses neural embeddings to match queries to documents in a continuous vector space. The pipeline:

1. Embed the user query and all documents into a shared vector space (e.g., 384- or 1536-dimensional)
2. Compute similarity (cosine, L2, or learned kernels) between query and document embeddings
3. Retrieve top-k documents by similarity score

**Strengths**: Robust to paraphrasing and synonymy; captures semantic relationships invisible to keyword matching; works well with long documents.

**Weaknesses**: Embedding models have knowledge cutoffs; sensitive to distribution shift; expensive for real-time embedding; expensive to re-index large corpora.

**Model families**: BERT-like (distilbert-base-uncased, all-MiniLM), sentence-transformers (all-mpnet-base-v2), OpenAI embeddings (text-embedding-3-large), specialized medical/legal models.

### Keyword and Sparse Retrieval

BM25 (Best Matching 25) and TF-IDF assign scores based on term frequency and inverse document frequency. Modern systems use Elasticsearch or similar engines for fast indexing.

**Strengths**: Fast, interpretable, works with exact terminology; nearly zero latency at scale; precise for specific domain jargon; no embedding model needed.

**Weaknesses**: Fails on synonyms and paraphrasing; weak on semantic understanding; exact match bias; dominated by stop words if not filtered.

**When to use**: Domain-specific corpora with controlled vocabulary; medical records; legal documents; systems where explainability is critical.

### Hybrid Search (Sparse + Dense)

Combine BM25 and semantic search scores using a weighted ensemble or late fusion. Common approaches:

1. **Score normalization + weighted average**: $\text{score} = \alpha \cdot \text{norm\_bm25} + (1-\alpha) \cdot \text{norm\_dense}$
2. **Reciprocal Rank Fusion (RRF)**: Harmonic mean of rank positions; avoids score scale issues
3. **Learning-to-rank**: Train a neural ranker on combined features (sparse + dense scores, query length, document metadata)

**Strengths**: Balances recall (semantic) and precision (keyword); robust to both synonymy and exact matches; state-of-the-art in production systems. Meta, Weaviate, and Pinecone all recommend hybrid as default.

**Weaknesses**: More complex; hyperparameter tuning required; needs both retrieval engines; slightly higher latency.

**Alpha tuning**: Start at 0.5 (equal weight); tune on dev set or use BM25/semantic hitrates.

## Advanced Retrieval Techniques

### Reranking (Cross-Encoder)

After initial retrieval (e.g., top-100 from hybrid search), rerank using a cross-encoder—a model that scores each (query, document) pair jointly, not independently.

**Cross-encoder vs. bi-encoder**:
- **Bi-encoder** (semantic search): Encodes query and document separately. Fast ($O(d)$ where $d$ = corpus size) but lower relevance quality.
- **Cross-encoder**: Scores (query, doc) pairs jointly. Slower ($O(k)$ where $k$ = reranked set) but higher accuracy (0.05-0.15 recall@1 improvement typical).

**Models**: SENTENCE-TRANSFORMERS CROSS-ENCODERS (ce-v5-large, ce-ms-marco-MiniLM-L-6-v2), Cohere Rerank, HF cross-encoders.

**Pipeline**: Dense retrieval (fast) → top-100 → cross-encoder rerank → top-10 to LLM.

### Query Decomposition and Multi-Query Retrieval

A single user query is often ambiguous. Multi-query strategies:

1. **Multi-query expansion**: Generate 3-5 paraphrased or alternative queries; retrieve for each; merge results (RRF or union).
2. **Query decomposition**: Break complex queries into sub-queries. Example: "Compare PyTorch and TensorFlow for NLP" → {"What are PyTorch strengths?", "What are TensorFlow strengths?", "NLP use cases"}.
3. **Sub-question generation (Tree-of-Thoughts)**: Recursively decompose until retrieval succeeds.

**Tools**: LLM-based generation (prompt the LLM to generate paraphrases), HyDE-style (see below).

### Hypothetical Document Embeddings (HyDE)

Instead of embedding the query directly, ask an LLM to generate hypothetical documents that would answer the query, then embed those.

**Process**:
1. Prompt: "Generate a hypothetical document that would answer: [query]"
2. Generate 1-3 hypothetical documents
3. Embed hypothetical docs + original query (or just hypothetical)
4. Use those embeddings to retrieve

**Rationale**: Hypothetical documents are often more semantically aligned with real documents than short queries; reduces query-document vocabulary mismatch.

**Typical improvement**: 2-5% recall@5 depending on domain. Higher gains on verbose, complex queries.

**Caveat**: Adds LLM generation latency; sometimes hallucinates unrealistic document text; requires careful prompt engineering.

### Multi-Hop Retrieval

For questions requiring reasoning across multiple documents, retrieve iteratively:

1. Retrieve initial documents for the query
2. LLM extracts sub-questions or follow-up context
3. Retrieve again with new queries
4. Repeat until sufficient context is gathered

**Example**: "What did the CEO of company X founded in 1999 study?" → Retrieve for company X → Extract CEO name → Retrieve CEO bio → Extract degree.

**Complexity vs. benefit trade-off**: Each hop adds latency; but can handle reasoning-heavy questions. Typical: 2-3 hops max.

## Document Chunking Strategies

How documents are split into retrievable chunks significantly impacts retrieval quality:

### Fixed-Size Chunking

Split into non-overlapping chunks of $n$ tokens (e.g., 512). Add overlap (128 tokens) to preserve context at boundaries.

**Pros**: Fast, scalable; predictable memory usage.

**Cons**: Breaks meaning at arbitrary boundaries; overlap adds redundancy; not semantic.

**Best for**: Large homogeneous corpora (web pages, arxiv papers); when semantic structure is unknown.

### Semantic Chunking

Split documents at semantic boundaries detected via sentence embeddings, topic shifts, or formatting (paragraphs, sections).

**Method**: Compute embeddings for consecutive sentences; split where similarity drop exceeds threshold; preserve chapters/sections.

**Pros**: Respects document structure; fewer broken concepts; better retrieval quality.

**Cons**: Slower; variable chunk sizes; requires embedding model.

**Tools**: Langchain, Llama Index, custom algorithms (diff-based, topic modeling).

### Tree-Based / Hierarchical Chunking

Organize documents into nested levels (sections → subsections → paragraphs). Retrieve top-level sections first, then drill down.

**Benefit**: Enables progressive retrieval; can summarize section headings before fetching full content; supports structured documents.

**Trade-off**: Requires document structure; more complex.

### Metadata-Aware Chunking

Preserve metadata (author, date, document ID) with each chunk. Use metadata in filtering (date range, source) before semantic retrieval.

**Example**: Retrieve only 2024 docs, then within those, semantic search.

## Retrieval Evaluation Metrics

### Recall@K

Fraction of relevant documents retrieved in top-k results:

$$\text{Recall@}k = \frac{|\text{relevant docs in top-}k|}{|\text{all relevant docs}|}$$

**Typical targets**: Recall@10 > 0.7 (high), Recall@100 > 0.9 (very high).

### Mean Reciprocal Rank (MRR)

Average reciprocal rank of the first relevant result:

$$\text{MRR} = \frac{1}{N}\sum_{i=1}^N \frac{1}{\text{rank}_i}$$

**Interpretation**: MRR=1.0 means first result is always relevant; MRR=0.5 means first relevant is typically at position 2.

### Normalized Discounted Cumulative Gain (NDCG@K)

Ranks are weighted; higher-ranked relevant docs contribute more:

$$\text{NDCG@}k = \frac{\text{DCG@}k}{\text{IDCG@}k}, \quad \text{DCG} = \sum_{i=1}^k \frac{\text{relevance}_i}{\log_2(i+1)}$$

**Use when**: Relevance is graded (not just binary); position matters heavily.

### Mean Average Precision (MAP)

Average precision across all queries:

$$\text{MAP} = \frac{1}{N}\sum_q \text{Precision@Rank of each relevant doc}$$

### Practical Evaluation Setup

1. Build gold-standard evaluation set: query + labeled relevant documents (human or expert review)
2. Compute retrieval metrics (Recall@10, MRR, NDCG)
3. End-to-end metric: Does LLM generate correct answer given retrieved context? (faithfulness, groundedness)
4. A/B test in production: Track user clicks, feedback, or implicit signals

## Design Tensions and Trade-Offs

- **Latency vs. accuracy**: Single dense retrieval is fast; hybrid + reranking is slower but more accurate
- **Cost vs. quality**: Large embedding models (1536-dim) cost more; compression (quantization, distillation) reduces quality slightly
- **Recall vs. precision**: High recall (retrieve many) helps LLM find context; high precision (retrieve few) saves tokens and latency
- **Generalization**: Embedding models trained on English domain X may fail on domain Y; fine-tuning or domain-specific models needed

## See Also

- [Retrieval-Augmented Generation — Grounding LLMs in External Knowledge](genai-rag-patterns.md)
- [Embeddings & Vector Spaces — Semantic Representation in Software](genai-embeddings-vectors.md)
- [Vector Databases — Embeddings, Similarity Search, and ANN Algorithms](database-vector.md)
- [Information Retrieval — Indexing, Ranking, and Search Architectures](cs-information-retrieval.md)