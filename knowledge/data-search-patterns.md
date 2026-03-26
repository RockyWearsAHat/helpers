# Search Implementation Patterns — Relevance, Analyzers, and Query Strategies

Search patterns describe how to structure queries, analyzers, and ranking functions to deliver relevant results. See also [Elasticsearch](database-elasticsearch.md) for system architecture; this note covers implementation patterns applicable to Elasticsearch, Solr, or similar systems.

## Analyzers and Tokenization

Text analysis transforms raw input into searchable tokens. Analyzer choice shapes relevance behavior and query precision.

### Tokenizers

**Standard Tokenizer** — Splits on whitespace and punctuation. Handles Unicode well; respects word boundaries.

```
Input: "Alice's email: alice@example.com"
Tokens: ["Alice", "s", "email", "alice", "example", "com"]
```

**Keyword Tokenizer** — Treats entire input as single token. No splitting.

```
Input: "alice@example.com"
Tokens: ["alice@example.com"]
```
Use for: SKUs, email addresses, postal codes (exact match semantics).

**Path Hierarchyizer** — Splits on `/`; useful for filesystem or URL hierarchies.

```
Input: "/users/alice/documents"
Tokens: ["/", "/users", "/users/alice", "/users/alice/documents"]
```
Use for: Hierarchical faceted search, breadcrumb navigation.

**Pattern Tokenizer** — Custom regex. Example: extract hashtags.

```
Pattern: #[a-z]+
Input: "Check #python and #golang"
Tokens: ["python", "golang"]
```

### Token Filters

Filters transform or discard tokens after tokenization.

**Lowercase Filter** — Normalize case. Essential for case-insensitive matching.

```
Input tokens: ["Alice", "SEARCH", "example"]
Output: ["alice", "search", "example"]
```

**Stop Word Filter** — Remove common words (the, a, and, etc.) that carry little semantic weight.

```
Language: English
Stop words: {the, a, an, and, or, is, ...}
Input: ["the", "quick", "brown", "fox"]
Output: ["quick", "brown", "fox"]
```
Trade-off: Reduces index size and noise; loses ability to search for common terms ("the who").

**Stemmer Filter** — Reduce words to root form. Language-specific (Porter stemmer for English, etc.).

```
Input: ["running", "runs", "ran", "runner"]
Output: ["run", "run", "ran", "runner"]  (imperfect; "ran" doesn't stem perfectly)
```
Trade-off: Improves recall (match "running" and "runs" together); risks precision loss (false positives).

**Lemmatizer Filter** — Morphological reduction via dictionary. More accurate than stemming but requires linguistic data.

```
Input: ["running", "runs", "ran"]
Output: ["run", "run", "go"]  (lemma form; note "ran" → "go")
```

**Synonym Filter** — Replace token with canonical form or add synonyms.

```
Config: {phone → [phone, telephone], car → [car, automobile]}
Input: ["phone"]
Output: ["phone", "telephone"]  (both index; query matches both)
Challenge: Synonym expansion increases index size.
```

**ASCII Folding Filter** — Convert accented characters to ASCII equivalent.

```
Input: ["café", "naïve", "Zürich"]
Output: ["cafe", "naive", "zurich"]
```

### Analyzer Composition

Analyzers combine tokenizer + filters. Language-specific analyzers bundle filters for that language.

```json
{
  "analyzer": "english_custom",
  "tokenizer": "standard",
  "filter": ["lowercase", "english_stop", "english_stemmer"]
}
```

**Text field** (analyzed): Breaks down and preprocesses. Use for free-text search.

**Keyword field** (not analyzed): Exact match. Use for filters, facets, sorting.

```json
{
  "name": {
    "type": "text",
    "analyzer": "english_custom",
    "fields": {
      "keyword": { "type": "keyword" }
    }
  }
}
```

Query on `name` runs analyzed search; query on `name.keyword` runs exact match. Both indexed; trade index size for flexibility.

---

## Relevance Tuning

Relevance scoring determines result order. BM25 is the default; customization via function score and query boosts.

### BM25 Scoring

**Okapi BM25** is a probabilistic model balancing term frequency, inverse document frequency, and document length.

$$\text{score}(q, d) = \sum_{i} \text{IDF}(q_i) \cdot \frac{f(q_i, d) \cdot (k_1 + 1)}{f(q_i, d) + k_1 \cdot (1 - b + b \cdot \frac{|d|}{\text{avgdl}})}$$

- $f(q_i, d)$: frequency of term $q_i$ in document $d$
- $|d|$: document length; $\text{avgdl}$: average document length
- $k_1$ (default 1.2): controls term frequency saturation (diminishing returns for repeated terms)
- $b$ (default 0.75): controls length normalization; 0 = no normalization, 1 = full normalization

**Tuning BM25:**
- Increase $k_1$ if tf saturation is too aggressive (single word hit should score higher than many hits)
- Decrease $b$ if short documents are unfairly penalized
- Decrease $b$ for title fields (length is less predictive of relevance)

### Function Score Query

Custom scoring function combining multiple criteria.

```json
{
  "query": {
    "function_score": {
      "query": { "match": { "title": "machine learning" } },
      "functions": [
        { "gauss": { "recency": { "origin": "2026-01-01", "scale": "30d", "decay": 0.5 } } },
        { "field_value_factor": { "field": "popularity", "factor": 1.2, "modifier": "log1p" } },
        { "script_score": { "script": { "source": "_score * doc['boost'].value" } } }
      ],
      "score_mode": "multiply",
      "boost_mode": "replace"
    }
  }
}
```

- **Decay functions** (gauss, linear, exponential): Prefer recent documents or geographically close results
- **Field value factor**: Boost by numeric field (popularity, rating, views)
- **Script score**: Custom JavaScript expression combining scores
- **Score mode**: `sum` (add), `multiply` (multiply), `min`/`max`
- **Boost mode**: `replace` (use function score), `multiply` (multiply BM25 × function), `sum` (add)

---

## Autocomplete Patterns

Fast, responsive suggestions as user types.

### Edge N-gram Tokenizer

Tokenize from the **start** of each word at each position.

```
Input: "hello"
Edge n-grams (min=1, max=5): ["h", "he", "hel", "hell", "hello"]

Index:
  "h"     → 1
  "he"    → 1
  "hel"   → 1
  "hell"  → 1
  "hello" → 1
```

Query: `prefix_query("hel")` → matches token "hel" → document found.

```json
{
  "analyzer": "autocomplete",
  "tokenizer": "edge_ngram",
  "filter": ["lowercase"]
}
```

**Trade-off:** Index size grows (each word expands to multiple tokens); query latency is minimal (prefix becomes a token, not a wildcard).

### Completion Suggester

Dedicated suggester type optimized for autocomplete. Uses trie structure for O(prefix_length) lookup.

```json
PUT /products
{
  "properties": {
    "product_name": {
      "type": "completion"
    }
  }
}

POST /products/_search
{
  "suggest": {
    "product_suggestions": {
      "prefix": "macb",
      "completion": {
        "field": "product_name",
        "fuzzy": { "fuzziness": "AUTO" },
        "size": 5
      }
    }
  }
}
```

**Advantages:** Faster than edge n-grams; supports fuzzy matching; built-in frequency weighting.

---

## Fuzzy Matching

Tolerating typos and misspellings.

### Edit Distance

**Levenshtein distance**: Minimum number of single-character edits (insert, delete, substitute).

```
"alice" → "alices": 1 insertion → distance = 1
"hello" → "hallo": 1 substitution → distance = 1
```

**Fuzziness in Elasticsearch**: Specified as number of edits or `AUTO` (scales with term length).

- `"fuzziness": 1` — Tolerate 1 edit
- `"fuzziness": "AUTO"` — Up to 2 edits for terms > 5 chars; 1 edit otherwise

### Fuzzy Query

```json
{
  "query": {
    "match": {
      "product_name": {
        "query": "macbok",
        "fuzziness": "AUTO",
        "prefix_length": 2
      }
    }
  }
}
```

- `prefix_length`: Don't allow edits in the first N characters ("mac" prefix must match exactly). Improves precision and performance.
- Trade-off: Increased computational cost (must evaluate all tokens within edit distance); slower queries.

### Phonetic Scoring

Match similar-sounding terms (Soundex, Metaphone).

```
Input: "smith"
Variants: ["smith", "smyth", "smythe", "smooth"]
```

Implement via custom tokenizer or synonym list. Useful for person names, domain-specific terms.

---

## Geo Search Patterns

Searching by geographic proximity.

### Geo-Distance Query

```json
{
  "query": {
    "geo_distance": {
      "distance": "10km",
      "location": {
        "lat": 40.7128,
        "lon": -74.0060
      }
    }
  }
}
```

Matches all documents with location within 10km of New York City. Stored as lat/lon pair; computed via haversine distance.

- Distance units: m, km, mi, in, yd
- Performance: Creates distance matrix for each document; slower on high-volume result sets

### Geo-Bounding Box

Cheaper than distance: rectangular filter.

```json
{
  "query": {
    "geo_bounding_box": {
      "location": {
        "top_left": { "lat": 40.9, "lon": -75.0 },
        "bottom_right": { "lat": 40.6, "lon": -73.5 }
      }
    }
  }
}
```

### Geo-Shape

Complex shapes (polygons, circles). Use for administrative boundaries, delivery zones.

```json
{
  "query": {
    "geo_shape": {
      "area": {
        "shape": {
          "type": "polygon",
          "coordinates": [[[0, 0], [10, 0], [10, 10], [0, 10], [0, 0]]]
        }
      }
    }
  }
}
```

---

## Search-as-You-Type

Real-time suggestions combining autocomplete + full-text search.

### Multi-Field Strategy

Index the same text under multiple fields with different analyzers:

```json
{
  "title": {
    "type": "text",
    "analyzer": "standard",
    "fields": {
      "autocomplete": { "type": "completion" },
      "prefix": { "analyzer": "edge_ngram", ... },
      "keyword": { "type": "keyword" }
    }
  }
}
```

Query routing:
- Typed characters 1-3: Query `title.autocomplete` (completion suggester)
- Typed characters 4+: Query `title.prefix` (edge n-grams) then rank by full `title` match

### Scoring Refinement

Prioritize by match type:

```json
{
  "query": {
    "bool": {
      "should": [
        { "match": { "title.keyword": "exact" } },
        { "prefix": { "title": "prefix" } },
        { "match": { "title": "contains" } }
      ]
    }
  }
}
```

Exact match scores highest; prefix second; substring third.

---

## Aggregations and Analytics

Faceted search and metrics.

### Terms Aggregation

Group documents by field value; compute count per group.

```json
{
  "aggs": {
    "by_category": {
      "terms": { "field": "category.keyword", "size": 10 }
    }
  }
}
```

Result:

```json
{
  "by_category": {
    "buckets": [
      { "key": "electronics", "doc_count": 150 },
      { "key": "clothing", "doc_count": 120 }
    ]
  }
}
```

### Date Histogram

Time-bucketed aggregation.

```json
{
  "aggs": {
    "sales_over_time": {
      "date_histogram": {
        "field": "date",
        "interval": "1d"
      }
    }
  }
}
```

### Nested Aggregations

Aggregate within aggregations (filter facets, multi-level hierarchies).

```json
{
  "aggs": {
    "by_category": {
      "terms": { "field": "category" },
      "aggs": {
        "avg_price": {
          "avg": { "field": "price" }
        }
      }
    }
  }
}
```

---

## See Also
- [Elasticsearch](database-elasticsearch.md) — Architecture and operations
- [Database Query Optimization](database-query-optimization.md) — Index design
- [Vector Databases](database-vector.md) — Semantic search via embeddings