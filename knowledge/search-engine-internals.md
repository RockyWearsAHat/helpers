# Search Engine Internals — Indexing, Ranking, and Query Processing

## Inverted Index: The Core Data Structure

A **forward index** maps documents to terms: `doc_1 → ["cat", "sat", "mat"]`. A **inverted index** reverses this: `"cat" → [doc_1, doc_5, doc_12]`. Inverted indexes enable fast full-text search: given a query term, retrieve matching documents instantly via hash lookup rather than scanning every document and every word.

A **record-level inverted index** stores document IDs for each term. A **word-level inverted index** (full inverted index) also stores positions within each document: `"cat" → [(doc_1, 2), (doc_5, 1), (doc_12, 3)]` — term "cat" appears at position 2 in doc_1, position 1 in doc_5, etc. Word positions enable phrase queries ("cat sat on mat" as exact sequence).

Example: Indexing three documents:
```
doc_1: "the quick brown fox"
doc_2: "a quick fox"
doc_3: "the fat cat"

Inverted index:
"the" → [doc_1, doc_3]
"quick" → [doc_1, doc_2]
"brown" → [doc_1]
"fox" → [doc_1, doc_2]
"a" → [doc_2]
"fat" → [doc_3]
"cat" → [doc_3]
```

Query "quick fox" retrieves: `[doc_1, doc_2] ∩ [doc_1, doc_2] = [doc_1, doc_2]`.

Inverted indexes trade incremental indexing cost (added documents require updating multiple postings lists) for fast query retrieval. They are the foundational data structure for search engines.

## Text Analysis: Tokenization, Stemming, Lemmatization

**Tokenization** breaks text into terms (words). Simple version: split on whitespace and punctuation. Sophisticated version: handle contractions ("don't" → "do", "n't"), possessives, dashes, URLs. Different languages need different rules (Chinese has no whitespace; Arabic has diacritical marks).

**Stemming** reduces words to their root form by removing suffixes. Porter stemmer ("running", "runs", "ran" → "run") is rule-based and fast. Drawbacks: over-stemming ("university", "universe" → "univers", which is not a real root), under-stemming ("organize", "organ" don't merge to a common root).

**Lemmatization** applies linguistic knowledge (grammar, dictionaries) to find true root forms. "Was", "is", "am" → "be". More accurate than stemming but slower and language-dependent (requires training data or lexicons).

Use case: "running shoes" should match query "run" — apply stemming/lemmatization at index time and query time to normalize. Tradeoff: stemming hurts precision if overshoots (e.g., "universal" and "universe" both stem to "univers" despite different meanings).

## Relevance Ranking: TF-IDF and BM25

**TF-IDF** (Term Frequency - Inverse Document Frequency) scores document relevance:

$$\text{TF-IDF}(term, doc) = \text{TF}(term, doc) \times \text{IDF}(term)$$

$$\text{TF}(term, doc) = \frac{\text{count of term in doc}}{\text{total words in doc}}$$

$$\text{IDF}(term) = \log \frac{\text{total docs}}{\text{docs containing term}}$$

Intuition: frequent terms in a document are relevant (TF), but common terms across all documents are less discriminative (IDF). Term "the" has low IDF (appears everywhere); term "vectorization" has high IDF (appears in few docs). Query "the vectorization" gives high score to docs with "vectorization" but downweights "the".

TF-IDF is simple and interpretable but has drawbacks: raw term frequency is vulnerable to term repetition spam, rare terms get outsized weight if they appear once in a long document.

**BM25** (Best Matching 25) improves on TF-IDF with a non-linear term-frequency saturation curve and document length normalization:

$$\text{BM25}(term, doc, query) = \frac{\text{IDF}(term) \times \text{TF}(term, doc) \times (k_1 + 1)}{\text{TF}(term, doc) + k_1 \times (1 - b + b \times \frac{|doc|}{avgdoclen})}$$

Parameters $k_1$ and $b$ tune the curve shape. BM25 is the mainstream ranking function in Elasticsearch, Solr, and most modern search engines. Tradeoff: more complex tuning vs. better empirical results than TF-IDF.

## Faceted Search and Filters

Queries often include filters: "search for 'laptop' AND price < $1000 AND brand = Dell". **Faceted search** breaks results into filterable categories. Each facet shows options and hit counts: "Brand: Dell (532), Apple (401), Lenovo (289)...". Users narrow by selecting facets.

Implementation: Pre-compute or dynamically compute facet histograms during query. Adding facets slows queries (need to accumulate counts per facet value). To scale, facet values are typically limited to high-cardinality fields (brand, category — hundreds of values). Low-cardinality fields (binary flags) are fast; high-cardinality fields (user IDs — millions of values) are avoided as facets.

## Autocomplete: Prefix Trees and N-grams

**Autocomplete** ("Did you mean...") and **typeahead** (suggestions as you type) require fast prefix matching. "In" should match documents starting with "In..." (India, International, Insurance).

**Trie** (prefix tree) nodes represent single characters. Each path from root to node represents a prefix. Trie lookup is O(prefix length), not O(vocabulary size). Tries are memory-intensive but efficient for prefix queries.

**N-gram** indexing creates overlapping substrings: "fox" → ["f", "fo", "fox", "o", "ox", "x"] (character n-grams). N-grams enable approximate matching ("fax" approximates "fox") and typo tolerance. Tradeoff: n-grams multiply index size but enable flexible matching.

Implementation patterns:
- Edge n-grams: only prefixes ("f", "fo", "fox") — efficient for typeahead.
- Token n-grams: overlapping substrings from tokenized text — useful for fuzzy search.

## Lucene Segments and Index Structure

**Apache Lucene** (underlying Elasticsearch, Solr) organizes indexes into immutable **segments**. When documents are added, they're buffered in memory; when the buffer fills or a commit occurs, it's flushed as a new read-only segment on disk. Queries search all segments and merge results.

Segments are indexed independently (each has its own inverted index, term dictionary, postings lists). Over time, many small segments accumulate. **Segment merging** (background process) combines segments into fewer, larger segments, improving query performance and reducing file descriptor count.

Each segment contains:
- **Inverted index**: term → postings list (doc IDs + positions)
- **Stored fields**: document source data (original JSON, text)
- **Term vectors**: per-document term frequencies (for relevance feedback)
- **Norms**: document length factors for BM25 tuning

Queries route to all segments, aggregate results, and return top-K. Index size grows monotonically as documents are added; deletion marks documents as deleted (purged during merges). Concurrency: writes append to current segment buffer; reads hit committed segments only.

## Relevance Tuning and Learning-to-Rank

Relevance is not one-dimensional. Simple keyword matches may miss semantic similarity; BM25 alone misses user intent. **Learning-to-Rank** (LTR) systems combine dozens of signals: TF-IDF score, PageRank, click-through-rate, freshness, word embeddings. A trained ML model (gradient boosting, neural net) predicts relevance.

LTR requires labeled training data: (query, document) pairs with explicit relevance scores. Lambdamart, LambdaRank, and neural ranking models optimize ranking loss functions. Most large-scale search engines (Google, Bing, Amazon) use LTR extensively.

Tradeoff: LTR is powerful but complex (requires ML infrastructure, labeled data) and slow (inference per result at ranking time). For simple use cases, BM25 suffices; at scale (recommending millions of results), LTR is standard.

## Related Concepts

See also: [cs-information-retrieval.md](cs-information-retrieval.md) for IR theory and models, [database-elasticsearch.md](database-elasticsearch.md) for Elasticsearch architecture, [architecture-search-platform.md](architecture-search-platform.md) for end-to-end search platform design, [algorithms-compression.md](algorithms-compression.md) for postings list compression.