# REST API Maturity: Richardson Model, Resource Design & Versioning

## Overview

The Richardson Maturity Model provides a framework for evaluating REST API design quality across four levels, from basic HTTP-based services to full hypermedia-driven APIs. Most production APIs target Level 2 (HTTP verbs + resources); Level 3 (HATEOAS) remains theoretical in most organizations. This note covers maturity progression, resource design patterns, error handling standards, and versioning trade-offs.

## Richardson Maturity Model

The model, articulated by Leonard Richardson and popularized by Martin Fowler, measures REST conformance:

### Level 0: The Swamp of POX (Plain Old XML/JSON)

A single HTTP endpoint accepts POST requests with action names encoded in the payload.

```
POST /service
{ "action": "getUser", "userId": 42 }
POST /service
{ "action": "createUser", "name": "Alice" }
```

- One URI, tunnel all semantics through the request body
- No HTTP method distinction; no standard response codes
- Common in early SOAP and JSON-RPC services
- Easy to implement; difficult to cache, scale, or reuse

### Level 1: Resources

Introduces multiple URIs (resource-oriented design), but HTTP methods remain secondary.

```
POST /users/42
POST /users
```

- Each resource has its own URI
- Still uses POST for all operations (create, read, update, delete)
- HTTP method semantics not yet respected
- Enables better URL routing and URI naming conventions

### Level 2: HTTP Verbs

Fully embraces HTTP method semantics: GET (safe, idempotent read), POST (create), PUT/PATCH (update), DELETE (remove). Adds meaningful status codes (200, 201, 400, 404, 500, etc.).

```
GET /users/42              → 200 OK
POST /users                → 201 Created
PATCH /users/42            → 200 OK
PUT /users/42              → 200 OK (full replacement)
DELETE /users/42           → 204 No Content
GET /invalid               → 404 Not Found
```

- **Most production APIs stop here.**
- Enables HTTP caching (GET is cacheable; POST/PUT/PATCH/DELETE typically aren't)
- Standard HTTP tooling (curl, browsers, proxies) works naturally
- Semantically cleaner than Level 0–1

### Level 3: Hypermedia Controls (HATEOAS)

Responses include links to related resources and transitions, allowing clients to discover API capabilities dynamically.

```json
{
  "id": 42,
  "name": "Alice",
  "email": "alice@example.com",
  "_links": {
    "self": { "href": "/users/42" },
    "all_users": { "href": "/users" },
    "update": { "href": "/users/42", "method": "PATCH" },
    "orders": { "href": "/users/42/orders" }
  }
}
```

- Client traverses the API through link relationships, not hard-coded URLs
- Enables graceful API evolution (new links can be added without client changes)
- Rarely fully implemented; complexity vs. benefit trade-off unfavorable for most teams
- Standards: HAL (Hypertext Application Language), JSON:API

**Reality check:** Level 3 is rarely justified because:
- Clients typically hard-code API URLs anyway (defeating the purpose)
- Documentation is still required (discoverability is theoretical)
- Complexity increases significantly
- Caching becomes harder with link-heavy payloads

## Resource Naming & Design

### Nouns, Not Verbs

Resources are **nouns** representing entities. HTTP methods are **verbs** representing actions.

- Good: `GET /orders`, `POST /orders`, `PATCH /orders/42`, `DELETE /orders/42`
- Bad: `GET /getOrders`, `POST /createOrder`, `DELETE /deleteOrder`

### Collections vs. Singular

- Plural for collections: `GET /users` → array of users
- Single trailing ID for a specific resource: `GET /users/42` → one user

Avoid mixing: `/users` vs. `/user` in the same API or `/orders` vs. `/order/42`.

### Hierarchical Resources

Represent relationships through path composition:

```
GET /users/42/orders           → orders by user 42
GET /users/42/orders/7         → specific order 7 by user 42
POST /users/42/orders          → create order for user 42
```

Limit nesting to 2–3 levels; deeper hierarchies become hard to route and reason about. Consider query parameters for complex filters:

```
GET /orders?userId=42&status=pending
```

## HTTP Status Codes & RFC 7807

### Core Codes

| Code | Meaning | When to Use |
|------|---------|------------|
| 200 OK | Successful GET, PUT, PATCH | Default success |
| 201 Created | Resource successfully created | POST that created a resource |
| 204 No Content | Success, no response body | DELETE or PATCH with no data to return |
| 400 Bad Request | Malformed or invalid input | Validation failure, missing fields |
| 401 Unauthorized | Authentication missing/invalid | No/expired credentials |
| 403 Forbidden | Authenticated but not authorized | Valid credentials, insufficient permissions |
| 404 Not Found | Resource does not exist | Invalid ID or path |
| 409 Conflict | State conflict | Duplicate email, version conflict |
| 422 Unprocessable Entity | Valid syntax, semantic failure | Good JSON but illogical state (e.g., birth date in future) |
| 429 Too Many Requests | Rate limit exceeded | Client quota hit |
| 500 Internal Server Error | Unexpected server failure | Bugs, unexpected exceptions |

### RFC 7807: Problem Details for HTTP APIs

A standardized error format improves debugging and integration. The spec defines a JSON object:

```json
{
  "type": "https://example.com/errors/validation",
  "title": "Validation Failed",
  "status": 400,
  "detail": "The 'email' field must be a valid email address.",
  "instance": "/orders/42",
  "errors": [
    { "field": "email", "message": "Invalid format" },
    { "field": "quantity", "message": "Must be > 0" }
  ]
}
```

- **type**: Machine-readable error category (URI, not necessarily resolvable)
- **title**: Short, human-readable summary
- **status**: HTTP status code (redundant but explicit)
- **detail**: Extended explanation
- **instance**: Which resource/request triggered the error
- **errors**: (custom field) Array of specific field errors for validation failures

Benefits:
- Clients can parse standardized error shapes
- Monitoring tools can aggregate by `type`
- Documentation is clearer (all errors follow one shape)

## Content Negotiation & Versioning

### Content Negotiation

Clients indicate desired representation via the `Accept` header:

```
GET /users/42
Accept: application/json

GET /users/42
Accept: application/xml

GET /users/42
Accept: application/vnd.myapi.v2+json
```

- Enables serving multiple formats (JSON, XML, Protobuf) from one URI
- Version negotiation via `Accept` header is RESTful but hard to test in browsers
- Most APIs use version in the URL path instead

### Versioning Strategies

#### URL Path (Most Common)
```
GET /api/v1/users
GET /api/v2/users
```

- Explicit and visible in logs, bookmarks
- Easy to test (just change URL)
- Requires routing on version (more complex at scale)
- Old versions remain available indefinitely (maintenance burden)

#### Header-Based Versioning
```
GET /users
Accept: application/vnd.myapi.v2+json
```

- Cleaner URLs; one endpoint per resource
- RESTful (uses standard HTTP mechanism)
- Harder to test and debug (often missed in logs)
- Clients must remember correct header

#### Query Parameter
```
GET /users?version=2
```

- Easy but non-standard
- Often overlooked by developers
- Inconsistent with REST philosophy

### Versioning Best Practices

1. **Version early**, even if not needed; adding it later is disruptive
2. **Keep old versions for N years** (often 2–3 for public APIs)
3. **Sunset policy**: Announce deprecation dates at least 6–12 months in advance
4. **Deprecation headers**: Include `Sunset`, `Deprecation`, and `Link: rel=deprecation` headers
5. **Semantic versioning**: Use `v1`, `v2` (not floating versions like `v1.1`)

## Pagination

Never return unbounded lists.

```json
{
  "data": [ { "id": 1, ... }, { "id": 2, ... } ],
  "pagination": {
    "limit": 10,
    "offset": 0,
    "total": 150,
    "hasMore": true
  }
}
```

**Offset pagination** (simple) vs. **cursor pagination** (scalable):
- Offset: Skip first N items; breaks under concurrent modifications
- Cursor: Opaque token pointing to last seen item; robust at scale

## HATEOAS in Practice

Most APIs omit HATEOAS for simplicity. Where it's valuable:

- **Public APIs** that need graceful evolution
- **CLI tools** or **mobile clients** that benefit from link-based navigation
- **API gateways** that proxy to heterogeneous backends

Minimal HATEOAS: Include `_links` for related resources and state transitions.

## Cross-References

See also: [api-design.md](api-design.md), [web-api-patterns.md](web-api-patterns.md), [api-graphql-depth.md](api-graphql-depth.md)