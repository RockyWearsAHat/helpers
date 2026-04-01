# Elasticsearch

## Core Concepts

### Inverted Index

Elasticsearch's fundamental data structure. Maps terms to the documents containing them.

```
Document 1: "The quick brown fox"
Document 2: "The quick brown dog"
Document 3: "The lazy brown fox"

Inverted Index:
  "the"   → [1, 2, 3]
  "quick" → [1, 2]
  "brown" → [1, 2, 3]
  "fox"   → [1, 3]
  "dog"   → [2]
  "lazy"  → [3]
```

Each indexed field gets its own inverted index. Numeric, date, and geo fields use BKD trees (for range queries), not inverted indexes.

### Index Architecture

- **Index**: logical namespace, like a database table
- **Shard**: Lucene index, the actual search unit (default: 1 primary shard since ES 7)
- **Replica**: copy of a primary shard on a different node
- **Segment**: immutable Lucene file within a shard, periodically merged

Shard sizing: target 10-50GB per shard, max ~200M documents. Too many small shards = overhead; too few large shards = slow queries and recovery.

## Mappings

Define how documents and their fields are stored and indexed. Set explicitly — don't rely on dynamic mapping in production.

```json
PUT /products
{
  "settings": {
    "number_of_shards": 3,
    "number_of_replicas": 1,
    "analysis": {
      "analyzer": {
        "custom_english": {
          "type": "custom",
          "tokenizer": "standard",
          "filter": ["lowercase", "english_stop", "english_stemmer"]
        }
      },
      "filter": {
        "english_stop": { "type": "stop", "stopwords": "_english_" },
        "english_stemmer": { "type": "stemmer", "language": "english" }
      }
    }
  },
  "mappings": {
    "properties": {
      "name":        { "type": "text", "analyzer": "custom_english", "fields": { "keyword": { "type": "keyword" } } },
      "description": { "type": "text", "analyzer": "custom_english" },
      "price":       { "type": "float" },
      "category":    { "type": "keyword" },
      "tags":        { "type": "keyword" },
      "created_at":  { "type": "date", "format": "strict_date_optional_time||epoch_millis" },
      "location":    { "type": "geo_point" },
      "metadata":    { "type": "object", "enabled": false },
      "reviews": {
        "type": "nested",
        "properties": {
          "rating": { "type": "integer" },
          "text":   { "type": "text" }
        }
      }
    }
  }
}
```

### Key Field Types

| Type                                 | Indexed For                              | Notes                                     |
| ------------------------------------ | ---------------------------------------- | ----------------------------------------- |
| `text`                               | Full-text search                         | Analyzed, tokenized. Not for aggregations |
| `keyword`                            | Exact match, aggregations, sorting       | Not analyzed. Max 32766 bytes             |
| `integer`, `long`, `float`, `double` | Range queries, aggregations              | Use smallest sufficient type              |
| `date`                               | Range queries                            | Stored as epoch millis internally         |
| `boolean`                            | Filtering                                |                                           |
| `geo_point`                          | Distance, bounding box queries           | `{ "lat": 40.73, "lon": -73.93 }`         |
| `nested`                             | Queries on object array preserving pairs | Each nested doc is a hidden Lucene doc    |
| `object`                             | Default for JSON objects                 | Flattened — field pairs not preserved     |
| `flattened`                          | Unknown/dynamic key-value structures     | All values as keywords                    |
| `dense_vector`                       | kNN / ANN similarity search              | For embeddings / semantic search          |

**text vs keyword**: Use multi-field mapping (`"fields": { "keyword": { "type": "keyword" } }`) when you need both full-text search and exact match/aggregations on the same field.

## Analyzers

An analyzer is a pipeline: **Character Filters → Tokenizer → Token Filters**.

### Built-in Analyzers

| Analyzer                    | Behavior                                     |
| --------------------------- | -------------------------------------------- |
| `standard`                  | Unicode tokenizer + lowercase (default)      |
| `simple`                    | Split on non-letters + lowercase             |
| `whitespace`                | Split on whitespace only                     |
| `keyword`                   | No tokenization (entire string as one token) |
| `english` / `french` / etc. | Language-specific stemming + stop words      |
| `pattern`                   | Regex-based tokenizer                        |

### Testing Analyzers

```json
POST /_analyze
{
  "analyzer": "english",
  "text": "The running foxes jumped quickly over the lazy dogs"
}
// Tokens: [run, fox, jump, quick, over, lazi, dog]
```

### Custom Analyzers

Common token filters: `lowercase`, `stop`, `stemmer`, `synonym`, `edge_ngram` (autocomplete), `phonetic`, `shingle` (word n-grams), `word_delimiter_graph`.

#### Autocomplete with Edge N-grams

```json
"analyzer": {
  "autocomplete": {
    "type": "custom",
    "tokenizer": "standard",
    "filter": ["lowercase", "autocomplete_filter"]
  }
},
"filter": {
  "autocomplete_filter": {
    "type": "edge_ngram",
    "min_gram": 2,
    "max_gram": 15
  }
}
// Use autocomplete analyzer at index time, standard at search time
```

## Query DSL

### Full-Text Queries

```json
// Match — analyzed, standard full-text search
{ "match": { "description": { "query": "quick brown fox", "operator": "and" } } }

// Match phrase — terms in order
{ "match_phrase": { "description": { "query": "quick brown fox", "slop": 1 } } }

// Multi-match — search across multiple fields
{ "multi_match": { "query": "search term", "fields": ["title^3", "body"], "type": "best_fields" } }
// Types: best_fields, most_fields, cross_fields, phrase, phrase_prefix

// Query string — Lucene syntax (power users)
{ "query_string": { "query": "title:(quick OR brown) AND NOT status:draft" } }
```

### Term-Level Queries (exact, not analyzed)

```json
{ "term": { "status": "published" } }
{ "terms": { "tags": ["go", "rust"] } }
{ "range": { "price": { "gte": 10, "lte": 100 } } }
{ "range": { "created_at": { "gte": "now-7d/d" } } }  // date math
{ "exists": { "field": "description" } }
{ "prefix": { "name.keyword": "Prod" } }
{ "wildcard": { "name.keyword": "Pro*ct" } }
{ "fuzzy": { "name": { "value": "produc", "fuzziness": "AUTO" } } }
```

### Compound Queries

```json
{
  "bool": {
    "must": [{ "match": { "title": "elasticsearch" } }],
    "filter": [
      { "term": { "status": "published" } },
      { "range": { "date": { "gte": "2024-01-01" } } }
    ],
    "should": [{ "term": { "featured": true } }],
    "must_not": [{ "term": { "category": "draft" } }],
    "minimum_should_match": 1
  }
}
```

- `must`: contributes to score, required
- `filter`: no scoring, required (cached, faster)
- `should`: contributes to score, optional (unless no `must`/`filter`)
- `must_not`: excludes, no scoring

### Nested Queries

```json
{
  "nested": {
    "path": "reviews",
    "query": {
      "bool": {
        "must": [
          { "range": { "reviews.rating": { "gte": 4 } } },
          { "match": { "reviews.text": "excellent" } }
        ]
      }
    }
  }
}
```

## Relevance Scoring (BM25)

Default since ES 5.0. BM25 considers:

- **TF (Term Frequency)**: how often the term appears in the document (saturating — diminishing returns)
- **IDF (Inverse Document Frequency)**: rarer terms score higher
- **Field length**: shorter fields score higher for matching terms

```
BM25(q, d) = Σ IDF(qi) · (f(qi, d) · (k1 + 1)) / (f(qi, d) + k1 · (1 - b + b · |d| / avgdl))
```

Parameters: `k1` (default 1.2, term frequency saturation), `b` (default 0.75, field length normalization).

### Boosting and Custom Scoring

```json
{
  "function_score": {
    "query": { "match": { "title": "elasticsearch" } },
    "functions": [
      {
        "field_value_factor": {
          "field": "popularity",
          "modifier": "log1p",
          "factor": 2
        }
      },
      {
        "gauss": { "date": { "origin": "now", "scale": "30d", "decay": 0.5 } }
      },
      {
        "script_score": {
          "script": { "source": "_score * doc['boost'].value" }
        }
      }
    ],
    "score_mode": "multiply",
    "boost_mode": "multiply"
  }
}
```

## Index Lifecycle Management (ILM)

Automate index management through phases: **hot → warm → cold → frozen → delete**.

```json
PUT _ilm/policy/logs_policy
{
  "policy": {
    "phases": {
      "hot": {
        "actions": {
          "rollover": { "max_primary_shard_size": "50gb", "max_age": "1d" },
          "set_priority": { "priority": 100 }
        }
      },
      "warm": {
        "min_age": "7d",
        "actions": {
          "shrink": { "number_of_shards": 1 },
          "forcemerge": { "max_num_segments": 1 },
          "set_priority": { "priority": 50 }
        }
      },
      "cold": {
        "min_age": "30d",
        "actions": {
          "searchable_snapshot": { "snapshot_repository": "my_repo" },
          "set_priority": { "priority": 0 }
        }
      },
      "delete": {
        "min_age": "90d",
        "actions": { "delete": {} }
      }
    }
  }
}
```

## Cluster Architecture

### Node Roles

| Role                                   | Purpose                                     |
| -------------------------------------- | ------------------------------------------- |
| `master`                               | Cluster state management, shard allocation  |
| `data`                                 | Store data, execute queries                 |
| `data_hot` / `data_warm` / `data_cold` | Tiered data nodes                           |
| `ingest`                               | Pre-process documents before indexing       |
| `coordinating`                         | Route requests, merge results (no role set) |
| `ml`                                   | Machine learning jobs                       |

Production minimum: 3 dedicated master-eligible nodes, N data nodes.

### Key Cluster Settings

```json
PUT _cluster/settings
{
  "persistent": {
    "cluster.routing.allocation.disk.watermark.low": "85%",
    "cluster.routing.allocation.disk.watermark.high": "90%",
    "cluster.routing.allocation.disk.watermark.flood_stage": "95%"
  }
}
```

## Ingest Pipelines

Pre-process documents before indexing.

```json
PUT _ingest/pipeline/web_logs
{
  "processors": [
    { "grok": { "field": "message", "patterns": ["%{COMBINEDAPACHELOG}"] } },
    { "date": { "field": "timestamp", "formats": ["dd/MMM/yyyy:HH:mm:ss Z"], "target_field": "@timestamp" } },
    { "geoip": { "field": "clientip", "target_field": "geo" } },
    { "user_agent": { "field": "agent", "target_field": "user_agent" } },
    { "remove": { "field": ["message", "agent", "timestamp"] } }
  ]
}

// Use on index
PUT /web_logs/_doc/1?pipeline=web_logs
{ "message": "93.180.71.3 - - [17/May/2015:08:05:32 +0000] ..." }
```

## Performance Tuning

### Indexing Performance

- Use `_bulk` API (500-5000 docs per request)
- Disable replicas during initial bulk load: `"number_of_replicas": 0`
- Increase `refresh_interval` to `30s` or `-1` during bulk
- Use `index.translog.durability: async` for speed (risk: lose up to 5s of data)
- Client-side: use concurrent bulk requests (3-5 threads)

### Search Performance

- Use `filter` context for non-scoring clauses (cached)
- Avoid `wildcard` or `regexp` queries on large fields
- Use `_source` filtering or `stored_fields` to return only needed fields
- Prefer `keyword` over `text` for exact matches
- Shard request cache (aggregation results): enabled by default on read-only indices
- `preference` parameter: route same queries to same shards for cache hits

### Monitoring

```
GET _cluster/health
GET _cat/indices?v&s=store.size:desc
GET _cat/shards?v
GET _nodes/stats
GET _cat/thread_pool/search?v
GET /my_index/_stats
```

## Comparison with Alternatives

| Feature        | Elasticsearch                  | OpenSearch      | Solr                 | Typesense        | Meilisearch          |
| -------------- | ------------------------------ | --------------- | -------------------- | ---------------- | -------------------- |
| Query language | Query DSL (JSON)               | Query DSL       | Lucene syntax / JSON | Simple JSON      | Simple JSON          |
| Scaling        | Horizontal (sharding)          | Horizontal      | SolrCloud (ZK)       | Horizontal       | Single-node focus    |
| Analytics      | Strong (aggregations)          | Strong          | Facets, pivot        | Basic            | Basic                |
| Ease of setup  | Medium                         | Medium          | Complex              | Very easy        | Very easy            |
| Typo tolerance | Fuzzy queries                  | Fuzzy queries   | Fuzzy, phonetic      | Built-in         | Built-in             |
| ML / Vectors   | kNN, learned sparse            | kNN             | Dense vectors        | Nearest neighbor | Basic vector         |
| License        | SSPL / Elastic License         | Apache 2.0      | Apache 2.0           | GPLv3            | MIT                  |
| Best for       | Large-scale search + analytics | Fork of ES, OSS | Legacy / Hadoop      | Simple search UX | Developer experience |

### When to Use Elasticsearch

- Full-text search across large document corpora
- Log analytics and observability (ELK stack)
- E-commerce product search with faceting
- Geo-spatial queries at scale
- Time-series metrics and SIEM

### When NOT to Use Elasticsearch

- Primary data store (no ACID transactions, eventual consistency)
- Simple key-value lookups (use Redis or a database)
- Small dataset with basic search (Typesense/Meilisearch are simpler)
- Budget-constrained (ES is memory-hungry — plan for ~64GB+ nodes in production)
