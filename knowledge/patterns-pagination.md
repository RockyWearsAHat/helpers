# Pagination Patterns

Pagination retrieved ordered data in chunks, solving two problems: (1) not overwhelming the client with millions of rows, and (2) not forcing the server to buffer entire result sets. The choice of pagination algorithm affects query performance, consistency, and API usability.

## Offset/Limit (Keyset Scan with Row Number)

The simplest, most intuitive approach: skip N rows, return the next M.

```
GET /products?offset=100&limit=25
```

Returns rows 100–124 (if 0-indexed).

### How It Works

```sql
SELECT * FROM products ORDER BY id LIMIT 25 OFFSET 100;
```

### Strengths

- Simple to understand and implement
- Stateless: client can jump to any page without server tracking
- Works with any underlying data structure

### Weaknesses

**At scale, offset becomes expensive.**

Database query planner must skip N rows from the beginning before returning the limit. Skipping 1M rows means scanning and discarding 1M rows.

```sql
-- This scans 1 million rows, discards 999,975, returns 25 (SLOW)
SELECT * FROM products ORDER BY id LIMIT 25 OFFSET 1000000;
```

**Real-world impact:** Query time grows linearly with offset. At offset=100k, typical latency jumps from 10ms to 500ms+.

**Correctness issue: Interleaved inserts**

If rows are inserted during pagination, result set is not stable.

```
Request 1: GET /products?offset=0&limit=10
  → Returns product IDs [1, 2, 3, ..., 10]
  
(New products 5a, 5b, 5c inserted)

Request 2: GET /products?offset=10&limit=10
  → Returns product IDs [11, 12, 13, ..., 20]  (should be [6a, 6b, 6c, 11, 12, ...])
```

The client sees a consistent stream within a single page request, but may see duplicates or gaps across pages.

### When to Use

- Small offsets (< 1000), small tables (< 10M rows)
- Internal APIs where predictability matters more than efficiency
- Simple CRUD operations where performance is not critical

## Cursor-Based Pagination (Opaque Cursor)

A cursor is an opaque token representing a position in the result set. The server encodes the position (e.g., a bookmark) and returns it to the client. The client passes it back unchanged.

### How It Works

```
GET /products?limit=25
  → 200 OK
  {
    "items": [...25 products...],
    "next_cursor": "eyJpZCI6IDI1MCwgImV4cCI6IDE2MDQ0NDAwMDB9"  (base64 encoded)
  }

GET /products?after=eyJpZCI6IDI1MCwgImV4cCI6IDE2MDQ0NDAwMDB9&limit=25
  → Next 25 products
```

The cursor encodes the last seen `id` (and optionally timestamp, version):

```python
def encode_cursor(last_id, timestamp=None):
  data = {"id": last_id, "timestamp": timestamp}
  return base64.b64encode(json.dumps(data).encode()).decode()

def decode_cursor(cursor_string):
  data = json.loads(base64.b64decode(cursor_string.encode()).decode())
  return data["id"], data.get("timestamp")
```

### Query with Cursor

```sql
-- Decode cursor to get last_id = 250
SELECT * FROM products 
WHERE id > 250 
ORDER BY id 
LIMIT 25;
```

### Strengths

- **Consistent:** Inserts/deletes between requests don't affect the stream (assuming unique ordering key)
- **Efficient:** Uses index on the ordering key; query time independent of position in dataset
- **Prevents deep pagination:** Clients can't arbitrarily jump to page 100k

### Weaknesses

- **Opaque to client:** Cannot directly jump to page N; must navigate sequentially
- **Complex implementation:** Must handle backwards navigation, cursor validity over time
- **Requires unique, ordered column:** Cursor assumes strict ordering (typically an ID or timestamp)

### Cursor Validation

Cursors can expire or become invalid:

```python
def get_paginated_results(after_cursor=None, limit=25):
  if after_cursor:
    try:
      last_id, timestamp = decode_cursor(after_cursor)
      # Verify timestamp is recent (within 1 hour)
      if time_since(timestamp) > 3600:
        raise CursorExpiredError()
    except (JSONDecodeError, KeyError):
      raise InvalidCursorError()
  
  if after_cursor:
    results = db.query(f"SELECT * FROM products WHERE id > {last_id} ORDER BY id LIMIT {limit}")
  else:
    results = db.query(f"SELECT * FROM products ORDER BY id LIMIT {limit}")
  
  if results:
    next_cursor = encode_cursor(results[-1].id)
  else:
    next_cursor = None
  
  return {"items": results, "next_cursor": next_cursor}
```

### Bidirectional Cursors

Some APIs support backward navigation:

```
GET /products?before=cursor&limit=25
  → Previous page
```

Implementation:

```sql
-- Fetch one extra row to detect if there are more results
SELECT * FROM products 
WHERE id < last_id 
ORDER BY id DESC 
LIMIT 26;

-- If 26 rows returned, has_previous = True; return only 25
```

## Keyset Pagination (Composite Key Cursor)

When ordering by multiple columns, cursor must track all ordering columns, not just the primary key.

```
GET /reviews?order_by=rating,date&limit=25
  → Order by rating DESC, then date DESC
```

Cursor encodes both:

```python
cursor = {
  "rating": 3.5,
  "date": "2024-01-15",
  "id": 12345  # tiebreaker for identical rating+date
}
next_cursor = encode_cursor(cursor)
```

Query:

```sql
SELECT * FROM reviews 
WHERE (rating, date, id) < (3.5, '2024-01-15', 12345)
ORDER BY rating DESC, date DESC, id DESC 
LIMIT 25;
```

**Limitation:** Works only if ordering columns have a stable total order. When new data arrives that sorts before the cursor, the page is always different (unlike simple cursor which includes new data in subsequent pages).

## Relay Cursor Pattern (GraphQL Standard)

GraphQL APIs (Relay spec) standardize cursor pagination with metadata.

```graphql
query {
  products(first: 25) {
    edges {
      node { id, name, price }
      cursor
    }
    pageInfo {
      hasNextPage
      hasPreviousPage
      startCursor
      endCursor
    }
  }
}
```

Response:

```json
{
  "data": {
    "products": {
      "edges": [
        { "node": { "id": "1", "name": "Product A" }, "cursor": "eyJpZCI6MX0=" },
        { "node": { "id": "2", "name": "Product B" }, "cursor": "eyJpZCI6Mn0=" }
      ],
      "pageInfo": {
        "hasNextPage": true,
        "endCursor": "eyJpZCI6Mn0="
      }
    }
  }
}
```

**Query with cursor:**

```graphql
query {
  products(first: 25, after: "eyJpZCI6Mn0=") { ... }
}
```

Advantage: Rich metadata (`hasNextPage`, bidirectional cursors) support complex navigation patterns.

## Page Tokens (Sequential)

Instead of encoding position, issue stateful tokens. Server maintains a map `token → query state`.

```
GET /search?order_by=relevance&limit=25
  → 200 OK
  {
    "results": [...25 items...],
    "next_page_token": "token_abc123"
  }

GET /search?page_token=token_abc123
  → Next 25 results
```

Server-side:

```python
page_tokens = {}  # In-memory or Redis

def handle_search(order_by, limit, page_token=None):
  if page_token:
    query_state = page_tokens.get(page_token)
    if not query_state:
      raise InvalidPageTokenError()
    offset = query_state["offset"]
  else:
    offset = 0
  
  results = db.query(f"SELECT * FROM products ORDER BY {order_by} LIMIT {limit} OFFSET {offset}")
  
  new_token = generate_token()
  page_tokens[new_token] = {"offset": offset + limit, "order_by": order_by}
  
  return {"items": results, "next_page_token": new_token}
```

**Strengths:**
- Stateful; can encode complex query state
- Server controls result consistency

**Weaknesses:**
- Token storage grows with active sessions
- Tokens expire; stale tokens become invalid
- Not suitable for caching (unique token per request)

## Deep Pagination Problems

### Problem 1: Offset Scans

As shown earlier, large offsets require scanning many rows.

**Solution:** Enforce max offset limit or use cursor pagination.

```python
if offset > 10000:
  raise ValueError("offset too large; use cursor pagination")
```

### Problem 2: Consistency During Pagination

Result set changes (new inserts, deletes) between page requests create inconsistency.

**Solutions:**
- **Snapshot consistency:** Take a database snapshot (READ COMMITTED, REPEATABLE READ) and paginate within it. [PostgreSQL `REPEATABLE READ` isolation level]
- **Cursor pagination:** Inherently stable if ordering key is stable
- **Timestamp-based:** Include `as_of_timestamp` parameter; fetch data consistent to that timestamp (useful for time-series, event stores)

### Problem 3: Slow Count Queries

Clients often request `total_count` (e.g., "1–25 of 50,000 results"). Counting all rows can be expensive.

**Solutions:**
- **Approximate counts:** Use database statistics (PostgreSQL `SELECT count_estimate(...)`; MySQL `row_count()`)
- **Omit count:** Many APIs (Stripe, Google Search) don't provide total count; just indicate `has_more: true`
- **Lazy count:** Count only the first few pages; after page 5, stop counting

**Example (omit total count):**

```python
def paginate(limit=25, after_cursor=None):
  # Fetch limit + 1 to detect if there are more results
  results = db.query(f"... WHERE id > {last_id} ORDER BY id LIMIT {limit + 1}")
  
  has_more = len(results) > limit
  results = results[:limit]
  
  next_cursor = encode_cursor(results[-1].id) if results else None
  
  return {
    "items": results,
    "next_cursor": next_cursor if has_more else None
    # No total_count
  }
```

### Problem 4: Real-Time Pagination (Search Results Shifting)

In relevance-based search, ranking may change mid-pagination (new documents indexed, ranking algorithm updated).

**Solution:** Freeze ranking for the session using a snapshot ID.

```
GET /search?q=python&snapshot_id=snap_12345
  → Results ranked consistent to snapshot_id
```

## Sorting and Filtering

**Sorting:** Impacts performance and consistency.

- **Primary key sort (id ASC):** Fastest; index typically exists
- **Timestamp sort (created_at DESC):** Common; requires index
- **Relevance sort (no stable order):** Harder for pagination; use keyset pagination with relevance + id

```sql
-- Good: pagination with id tiebreaker
SELECT * FROM products 
WHERE relevance > threshold 
ORDER BY relevance DESC, id ASC 
LIMIT 25;
```

**Filtering:** Apply filters before pagination.

```sql
SELECT * FROM products 
WHERE status = 'active' AND price < 1000 
ORDER BY id 
LIMIT 25 OFFSET 0;
```

Filters reduce the result set size, improving performance.

## Comparison Table

| Pattern | Stateless | Efficient | Consistent | Bidirectional | Best For |
| --- | --- | --- | --- | --- | --- |
| Offset/Limit | Yes | ✗ (slow at depth) | ✗ | Yes | Small tables, simple UI |
| Cursor | Yes | ✓ | ✓ | ✗ (extra logic) | Large tables, stable orderings |
| Keyset | Yes | ✓ | ✓ | ✗ | Multi-column sort |
| Page Token | ✗ | ✓ | ✓ | ✓ | Complex queries, stateful sessions |
| Relay (GraphQL) | Yes | ✓ | ✓ | ✓ | GraphQL APIs |

## See Also

- `patterns-rate-limiting.md` — Control pagination request rates
- `database-query-optimization.md` — Index strategies for pagination
- `api-design.md` — General API design principles