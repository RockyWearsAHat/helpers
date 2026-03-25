# API Design Best Practices

## REST API Principles

### Resource-Based URLs

URLs represent resources (nouns), not actions (verbs).

- Good: `GET /users/42`, `POST /orders`, `DELETE /products/7`
- Bad: `GET /getUser?id=42`, `POST /createOrder`, `POST /deleteProduct`

Use plural nouns for collections: `/users`, `/orders`, `/products`.

### HTTP Method Semantics

| Method | Purpose                   | Idempotent | Safe |
| ------ | ------------------------- | ---------- | ---- |
| GET    | Read resource             | Yes        | Yes  |
| POST   | Create resource           | No         | No   |
| PUT    | Replace resource entirely | Yes        | No   |
| PATCH  | Partial update            | No\*       | No   |
| DELETE | Remove resource           | Yes        | No   |

\*PATCH can be made idempotent with proper design.

### Status Codes

Use meaningful HTTP status codes:

- **200** OK — successful GET/PUT/PATCH
- **201** Created — successful POST that created a resource
- **204** No Content — successful DELETE
- **400** Bad Request — malformed input, validation failure
- **401** Unauthorized — missing or invalid authentication
- **403** Forbidden — authenticated but not authorized
- **404** Not Found — resource doesn't exist
- **409** Conflict — state conflict (e.g., duplicate email)
- **422** Unprocessable Entity — valid syntax but semantic errors
- **429** Too Many Requests — rate limit exceeded
- **500** Internal Server Error — unexpected server failure

### Versioning

- **URL path**: `/api/v1/users` — simple, explicit, most common.
- **Header**: `Accept: application/vnd.myapi.v2+json` — cleaner URLs but harder to test.
- **Query param**: `/users?version=2` — easy but can be missed.

Always version from the start. Breaking changes require a new version.

### Pagination

Never return unbounded lists. Always paginate.

- **Offset-based**: `?page=3&per_page=25` — simple, but degrades at large offsets.
- **Cursor-based**: `?after=eyJpZCI6MTAwfQ&limit=25` — consistent performance, better for real-time data.
- Include pagination metadata in response: `total`, `next_cursor`, `has_more`.

### Filtering, Sorting, Searching

- Filter: `GET /users?status=active&role=admin`
- Sort: `GET /users?sort=-created_at` (prefix `-` for descending)
- Search: `GET /users?q=john`

### Error Responses

Return consistent, structured error responses:

```json
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Email format is invalid",
    "details": [
      { "field": "email", "message": "Must be a valid email address" }
    ]
  }
}
```

### Idempotency

For non-idempotent operations (POST), support idempotency keys:

- Client sends `Idempotency-Key: <uuid>` header.
- Server deduplicates — same key returns the same response.
- Critical for payment APIs and any operation that must not double-execute.

## GraphQL Principles

- Single endpoint: `POST /graphql`.
- Client specifies exactly which fields to return — no over/under-fetching.
- Strong schema with types as documentation.
- Use for: complex data graphs with many relationships. Avoid for: simple CRUD.
- Implement query depth/complexity limits to prevent abuse.
- Use DataLoader pattern to batch and deduplicate database queries (N+1 prevention).

## General API Design Principles

1. **Consistency**: Same naming conventions, error formats, and patterns across all endpoints.
2. **Least surprise**: APIs should behave as consumers expect from the naming.
3. **Forward compatibility**: Add fields/endpoints without breaking existing clients. Never remove or rename fields in existing versions.
4. **Documentation**: Auto-generate from schema (OpenAPI/Swagger, GraphQL introspection). Docs rot fast if separated from code.
5. **Rate limiting**: Protect against abuse. Return `429` with `Retry-After` header.
6. **HATEOAS**: Include links to related resources/actions in responses. Helps clients discover API capabilities without hardcoding URLs.
7. **Authentication**: Use standard mechanisms (OAuth2, JWT, API keys). Pass tokens in `Authorization` header, never in URLs.
8. **Request validation**: Validate early, fail fast, return specific error messages.
9. **Compression**: Support `Accept-Encoding: gzip` for large responses.
10. **CORS**: Configure explicitly for browser clients. Never use `Access-Control-Allow-Origin: *` with credentials.

---

_Sources: RESTful Web APIs (Richardson & Ruby), Microsoft REST API Guidelines, Google API Design Guide, Stripe API (gold standard reference), GitHub API documentation_
