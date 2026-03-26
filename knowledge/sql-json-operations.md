# SQL JSON Operations — Document Storage, Querying, and Validation

## Overview

Modern SQL databases treat JSON as a first-class type, enabling hybrid document-relational storage. PostgreSQL pioneered `JSONB`; MySQL, SQLite, and SQL Server have added `JSON` support. Each has unique operators and performance characteristics.

| Database   | Type           | Indexing | Pros                | Cons                      |
| ---------- | -------------- | -------- | ------------------- | ------------------------- |
| PostgreSQL | `JSONB`        | GIN      | Binary, indexable, rich API | Larger disk footprint |
| PostgreSQL | `JSON` (text)  | —        | Compact              | No indexing, validation required |
| MySQL      | `JSON`         | Limited  | Standardized funcs   | Index on computed columns only |
| SQLite     | `JSON`         | —        | Builtin, functions   | No indexing              |
| SQL Server | `NVARCHAR(MAX)` JSON functions | —        | Integrated           | No native type            |

**PostgreSQL is the de facto standard** for advanced JSON features.

## PostgreSQL JSONB: Operators and Functions

### Extraction Operators

```sql
-- JSONB getter operators
SELECT data->'name'                    -- returns JSONB (slower in comparisons)
       data->>'name'                  -- returns text (fast)
       data#>'{address,city}'         -- nested path, returns JSONB
       data#>>'{address,city}'        -- nested path, returns text
       data->'tags'->0                -- array index
FROM users;

-- Logical operators (only JSONB)
SELECT * FROM users
WHERE data @> '{"role":"admin"}'      -- containment: data has this subset
  AND NOT (data @> '{"status":"inactive"}');

-- Key existence
SELECT * FROM users
WHERE data ? 'email'                  -- key exists
  AND data ?| array['phone','slack']  -- any keys exist
  AND data ?& array['first_name','last_name'];  -- all keys exist

-- Array overlap
SELECT * FROM users
WHERE data->'tags' && '["go","rust"]'::jsonb;  -- any tag in array
```

### Modification Functions

```sql
-- jsonb_set: set value at path (creates intermediate keys)
UPDATE users
SET data = jsonb_set(data, '{address,zip}', '"90210"')
WHERE id = 1;

-- || operator: merge (right overwrites left)
UPDATE users
SET data = data || '{"verified":true, "updated_at":"2024-01-01"}'::jsonb;

-- - operator: remove key or array index
UPDATE users
SET data = data - 'temp_field'
  WHERE data ? 'temp_field';

DELETE FROM users
SET data = data - 0;  -- remove first array element

-- jsonb_set with nested delete
UPDATE users
SET data = data #- '{address,zip}';  -- remove nested path
```

### Aggregation and Expansion

```sql
-- Aggregate to JSONB object or array
SELECT jsonb_object_agg(name, salary) FROM employees;  -- {name: salary, ...}
SELECT jsonb_agg(to_jsonb(row)) FROM employees;         -- [{row}, {row}, ...]

-- Expand JSONB to rows (unnesting)
SELECT id, jsonb_each(data) AS (key, value) FROM users;
SELECT id, jsonb_each_text(data) AS (key, value) FROM users;  -- values as text

SELECT id, jsonb_array_elements(tags) FROM users WHERE jsonb_typeof(tags) = 'array';
```

### Querying with jsonb_path_query (PostgreSQL 13+)

```sql
-- JSONPath: SQL/JSON standard for predicates
SELECT jsonb_path_query(data, '$.tags[*]') FROM users;
SELECT jsonb_path_query_array(data, '$.settings.*?(@.enabled == true)') FROM users;

-- Predicates: filter within path expression
SELECT data
FROM users
WHERE jsonb_path_query(data, '$.orders[*]?(@.total > 100)') IS NOT NULL;
```

**jsonb_path_query** returns individual matches; **jsonb_path_query_array** returns all matches as an array.

### Indexing JSON (GIN Index)

```sql
-- GIN index on JSONB column enables fast key lookups and containment
CREATE INDEX idx_users_data ON users USING gin (data);

-- Querieswith GIN:
EXPLAIN SELECT * FROM users WHERE data ? 'email';
EXPLAIN SELECT * FROM users WHERE data @> '{"role":"admin"}';

-- Index on nested path (PostgreSQL 12+)
CREATE INDEX idx_data_email ON users USING gin ((data->'email'));
```

GIN indexes dramatically speed up `?`, `?|`, `?&`, `@>` operators. No index for `->` or `->>` extraction (these are full table scans).

## MySQL JSON Functions

```sql
-- Extraction (similar to PostgreSQL but different syntax)
SELECT JSON_EXTRACT(data, '$.name') FROM users;
SELECT JSON_EXTRACT(data, '$.address.city') FROM users;
SELECT JSON_UNQUOTE(JSON_EXTRACT(data, '$.name')) FROM users;  -- text

-- Modification
UPDATE users SET data = JSON_SET(data, '$.verified', true);
UPDATE users SET data = JSON_INSERT(data, '$.tags[0]', 'golang');

-- Validation
SELECT * FROM users WHERE JSON_VALID(data);

-- Aggregation
SELECT JSON_ARRAYAGG(name) FROM employees;
SELECT JSON_OBJECTAGG(name, salary) FROM employees;

-- Indexing: computed column + regular index (no native JSON index)
ALTER TABLE users ADD COLUMN email_extracted VARCHAR(255) GENERATED ALWAYS AS (JSON_UNQUOTE(JSON_EXTRACT(data, '$.email'))) STORED;
CREATE INDEX idx_email ON users(email_extracted);
```

## JSON Schema Validation

Enforce structure at insertion/update time.

```sql
-- PostgreSQL: validate against JSON Schema
CREATE TABLE documents (
  id uuid PRIMARY KEY,
  data jsonb NOT NULL CHECK (jsonb_matches(data, '{
    "type": "object",
    "properties": {"name": {"type": "string"}, "age": {"type": "number"}},
    "required": ["name"]
  }'::jsonb))
);

-- MySQL: no native schema validation; validate in application layer
-- SQL Server: similar (validation in app)
```

**In practice**: Most schemas are validated in the application layer, since DB schema enforcement is strict and difficult to evolve.

## JSON Aggregation Patterns

Common use case: collapse related rows into JSON arrays.

```sql
-- Collect all orders per customer as nested array
SELECT c.id, c.name,
  jsonb_agg(jsonb_build_object(
    'order_id', o.id,
    'date', o.date,
    'total', o.total
  ) ORDER BY o.date) AS orders
FROM customers c
LEFT JOIN orders o ON c.id = o.customer_id
GROUP BY c.id, c.name;

-- Nested aggregation (customers with their orders and line items)
SELECT c.id, c.name,
  jsonb_agg(jsonb_build_object(
    'order_id', o.id,
    'items', (
      SELECT jsonb_agg(jsonb_build_object('sku', sku, 'qty', qty))
      FROM order_items oi WHERE oi.order_id = o.id
    )
  )) AS orders
FROM customers c
JOIN orders o ON c.id = o.customer_id
GROUP BY c.id, c.name;
```

## Document-Relational Hybrid Patterns

Use JSON for semi-structured or nested data while keeping relational structure for querying and joins.

```sql
-- User table: relational core + flexible metadata
CREATE TABLE users (
  id uuid PRIMARY KEY,
  email text UNIQUE NOT NULL,
  created_at timestamptz NOT NULL,
  metadata jsonb  -- variable fields: phone, address, preferences, custom_fields
);

-- Query users by nested field efficiently
SELECT * FROM users WHERE metadata->'preferences'->>'theme' = 'dark';

-- But for common queries, use relational columns
SELECT * FROM users WHERE email = 'alice@example.com';

-- Aggregate metadata across users
SELECT COUNT(*), metadata->'preferences'->>'theme' AS theme
FROM users
GROUP BY metadata->'preferences'->>'theme';
```

**Trade-off**: Flexibility (JSON) vs. queryability (relational). Store static/queryable data as columns; use JSON for auxiliary/ad-hoc metadata.

## SQLite JSON Functions (Built-in, No Library)

```sql
-- Extraction
SELECT json_extract(data, '$.name') FROM users;
SELECT json('{"a": 1}');  -- validate

-- Modification
UPDATE users SET data = json_set(data, '$.verified', 'true');

-- Aggregation
SELECT json_group_array(json(data)) FROM users;  -- array of all rows as JSON

-- No indexing on JSON; use computed columns
ALTER TABLE users ADD COLUMN email_extracted TEXT GENERATED ALWAYS AS (json_extract(data, '$.email')) STORED;
CREATE INDEX idx_email ON users(email_extracted);
```

## Performance and Trade-Offs

- **JSONB indexing** (PostgreSQL GIN) is powerful for `@>` containment but adds write overhead.
- **Computed columns** (MySQL, SQLite) extract frequently-queried fields and index them, avoiding full table scans.
- **Stored vs. generated**: Stored columns use disk space; generated columns are recomputed on query (slow on large tables, fast on small updates).
- **Full-table scans**: Querying non-indexed JSON paths is expensive. Plan accordingly or maintain relational denormalization.

## See Also

- [PostgreSQL Deep Dive](database-postgresql.md)
- [Database Indexing Strategies](database-indexing-strategies.md)
- [SQL Conventions and Idioms](language-sql.md)