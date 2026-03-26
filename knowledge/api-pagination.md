# API Pagination — Cursor-Based, Offset-Based & Consistency Trade-Offs

## Overview

Pagination moves ordered data from server to client in chunks, solving two problems: preventing client buffer exhaustion and avoiding expensive full-table scans. The choice of algorithm affects query latency, cursor complexity, and correctness under concurrent mutations. From an API design perspective, pagination is a contract between server and client about how to traverse large result sets.

## Pagination Scope & Ordering

Pagination requires a total order. "Give me the next 25 users" is ambiguous without specifying: "ordered by what?" Users ordered by creation date yield different results than ordered by username alphabetically, and results differ again if you include soft-deleted users.

**API design principle:** Always specify default sort order explicitly:
```
GET /users
→ Returns users ordered by creation_date DESC (default)

GET /users?sort=username ASC
→ Order by username (ascending)
```

**Semantic rule:** If sort order is unspecified, state what the default is. Clients that don't read docs will assume insertion order, which may surprise them if backend re-indexes or reorders results.

## Offset-Based Pagination

Skip N rows, return the next M.

```
GET /users?offset=100&limit=25
→ Rows 100–124 (if 0-indexed) or 101–125 (if 1-indexed, varies by convention)
```

### How Backend Query Works

```sql
SELECT * FROM users ORDER BY created_at DESC LIMIT 25 OFFSET 100;
```

The database skips the first 100 rows, then returns 25. Cost grows linearly with offset:
- offset=10: Quick (index scan skips 10 rows)
- offset=100k: Slow (skip 100k rows, even with index)
- offset=1M: Very slow (scan and discard 1M rows)

Real-world: At offset=100k on a busy table, latency jumps from 10ms to 500ms+. Users notice.

### Correctness Issue: Mutations During Pagination

If rows are inserted or deleted between requests, client's view becomes inconsistent:

**Example: Interleaved insert**
```
Request 1: GET /users?offset=0&limit=10
  → Users [A, B, C, D, E, F, G, H, I, J]
  
  (New user X inserted, sorts before E)

Request 2: GET /users?offset=10&limit=10
  → Users [F, G, H, I, J, K, L, M, N, O]  
  
  (Should be [E, F, ...] but E is now before offset 10 due to insertion)
```

Client sees user F twice (once in page 1 via previous query, again here) or misses user E entirely.

**Why this happens:** Offset is positional, not value-based. If the data under those positions changes, client gets stale results.

### When to Use Offset

- Small offsets (< 10k rows): Performance is acceptable
- Internal APIs where consistency matters less than simplicity (e.g., admin dashboards)
- Search results where rough consistency is tolerable (users accept that results shift as items are added)
- Stateless requirements: Some clients need ability to jump to page 50 without requesting pages 1–49 first

### API Design: Offset Guidance

```
GET /products?offset=0&limit=25
Response:
{
  "items": [...25 products...],
  "offset": 0,
  "limit": 25,
  "total": 5000,  # Total count (optional, expensive at scale)
  "has_more": true
}
```

**Total count caveat:** Including `"total": 5000` requires a full count scan (`SELECT COUNT(*) FROM products`), which is slow on large tables. Options:
1. Always include it (simplest, slowest)
2. Include it only for small result sets (e.g., < 100k rows)
3. Exclude it and use `has_more` flag instead (faster, less predictable)
4. Cache count and serve stale data (risky but fast)

**Recommendation:** Omit total count. Use `has_more` flag to tell clients whether to fetch next page.

## Cursor-Based Pagination (Recommended for Large Datasets)

A cursor is an opaque token representing a position in the result set. The server encodes the position (typically: the ID of the last seen item, plus optional metadata) and returns it to client. Client passes cursor back; server decodes it and continues from that point.

### How It Works

```
Request 1: GET /products?limit=25
ResponseReturns:
{
  "items": [...25 products...],
  "next_cursor": "eyJpZCI6IDI1MCwgImNyZWF0ZWRfYXQiOiAxNjMwNzAzNDQ1fQ=="  
}

Request 2: GET /products?after=eyJpZCI6IDI1MCwgImNyZWF0ZWRfYXQiOiAxNjMwNzAzNDQ1fQ==&limit=25
→ Next 25 products, starting after product 250
```

The cursor encodes:
```python
cursor = base64_encode(json.dumps({
  "id": last_seen_id,
  "created_at": last_seen_timestamp
}))
```

### Query Construction

```sql
-- Decode cursor to get last_id = 250, last_created = 2021-09-01T12:30:45Z
SELECT * FROM products 
WHERE 
  created_at < '2021-09-01T12:30:45Z'
  OR (created_at = '2021-09-01T12:30:45Z' AND id > 250)
ORDER BY created_at DESC, id DESC
LIMIT 25;
```

This ensures stable ordering even if multiple rows have the same `created_at`.

### Strengths

**Consistency under mutations:** Cursor encodes actual values (id, timestamp), not position. If new rows are inserted before the cursor, they don't affect the next fetch.

**O(1) lookup at scale:** No skip-scan overhead. Query starts directly at cursor position using index.

**Fairness:** All clients experience the same speed, regardless of which page they're on.

### Weaknesses

**Opaque to client:** Client can't jump to page 50; must navigate sequentially (page 1 → 2 → 3 → ... → 50). Stateless jumping is impossible.

**Cursor complexity:** Client must treat cursor as blackbox. If server changes encoding, old cursors break. Versioning required.

**Bidirectional navigation:** Supporting both `after` and `before` cursors complicates query logic. Most APIs only support forward (unidirectional).

### API Design: Cursor Guidance

```
GET /products?limit=25
Response:
{
  "edges": {
    "items": [...25 products...],
    "cursor": "eyJpZCI6IDI1MCwgImNyZWF0ZWRfYXQiOiAxNjMwNzAzNDQ1fQ=="
  },
  "page_info": {
    "has_next_page": true,
    "has_previous_page": false,
    "end_cursor": "eyJpZCI6IDI1MCwgImNyZWF0ZWRfYXQiOiAxNjMwNzAzNDQ1fQ==",
    "start_cursor": "eyJpZCI6IDIyNSwgImNyZWF0ZWRfYXQiOiAxNjMwNzAzNDQ1fQ=="
  }
}
```

This structure (from Relay specification) allows GraphQL clients to know whether to fetch next/previous pages and how.

## Relay Connection Specification

Relay (Facebook's GraphQL client framework) standardized a cursor-based pagination format widely adopted even by REST APIs:

```graphql
connection {
  edges {
    node { ... }
    cursor
  }
  pageInfo {
    hasNextPage
    hasPreviousPage
    startCursor
    endCursor
  }
}
```

**Advantages:**
- Standardized vocabulary (queries across different APIs look similar)
- Supports bidirectional navigation
- Clear semantics (edges = items, pageInfo = navigation metadata)

**Disadvantages:**
- Verbosity (more nesting than simple offset pagination)
- GraphQL-centric (REST APIs retrofitting this feel awkward)

## Combining Approaches: Practical Guidance

### Simple APIs (< 10k total resources)
Use offset-based pagination. Simplicity trumps performance. Include total count for UX (knowing there are 500 users feels good).

### Medium APIs (10k–1M resources)
Use cursor-based pagination for public APIs. Offset still works internally but keep cursor as public interface to avoid scaling surprises.

### Large APIs (> 1M resources, financial, search)
Cursor-based only. Total count is expensive; omit it. Clients must navigate sequentially.

### Search APIs (Elasticsearch, Solr)
Blend approaches: Provide offset-based for relevance results (typically small, < 100k total matches) and cursor-based for exact/deep scans.

## Consistency & Isolation

### Snapshot Isolation
To ensure consistent pagination results, execute all pages within a database transaction or snapshot:

```sql
BEGIN TRANSACTION ISOLATION LEVEL SNAPSHOT;
SELECT * FROM products ... LIMIT 25;  -- page 1
SELECT * FROM products ... LIMIT 25;  -- page 2 (sees same data as page 1)
COMMIT;
```

**Trade-off:** Transactions lock rows. Long pagination sessions hold locks, blocking writes. Not always feasible for large result sets.

### Weakly Consistent (Default)
Each page request is independent. Results may shift between pages due to concurrent inserts/deletes. This is usually acceptable for public APIs (search results, feeds) but not for financial systems.

## Cursor Encoding Best Practices

**Keep cursors opaque:** Don't document the encoding. If you change encoding, old cursors remain valid by maintaining backward compatibility in your decoder.

```python
def decode_cursor(cursor_string):
  try:
    data = json.loads(base64.b64decode(cursor_string))
    # Support old cursor format (v1) and new format (v2)
    if "version" in data and data["version"] == 2:
      return data["id"], data["timestamp"]
    else:  # v1 legacy format
      return data["id"], None
  except Exception:
    raise InvalidCursorError("Cursor invalid or expired")
```

**Expiry:** Optionally include a timestamp in cursor and reject stale cursors (> 24 hours old). Prevents API client confusion: "I saved this cursor from yesterday; why doesn't it work?"

## See Also

- `patterns-pagination.md` — Algorithm deep dive and performance analysis
- `api-design.md` — REST principles and endpoint design
- `database-query-optimization.md` — Indexing strategies for pagination speed