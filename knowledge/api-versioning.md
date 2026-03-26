# API Versioning — Strategies, Evolution & Deprecation Workflows

## Overview

API versioning manages breaking changes while maintaining backward compatibility with existing clients. Versioning decisions made early compound over years: poor choices create unmaintainable API sprawl; thoughtful design enables graceful evolution. The core tradeoff is between explicit versioning (clear boundaries, higher client complexity) and invisible evolution (transparent upgrades, stricter server constraints).

## Versioning Strategies

### URL Path Versioning (`/v1/`, `/v2/`)

The resource path contains the version as a segment: `/api/v1/users`, `/api/v2/users`.

**Advantages:**
- Immediately visible in logs, load balancer configs, and browser history
- Easy to route: reverse proxies route `/v1/*` and `/v2/*` to different handlers
- Familiar to developers; widely deployed in practice
- Enables running multiple versions simultaneously without coordination

**Disadvantages:**
- URLs proliferate; each version becomes a distinct resource
- Cache busting: `/v1/users` and `/v2/users` are different URIs even if content is identical
- HTTP caching becomes confusing (should `/users` redirect to `/v1/users`?)
- Couples API semantics to URL structure; harder to deliver different responses to same URL

**When to use:** For major breaking changes affecting many resources; when different versions must coexist in load-balanced deployment.

### HTTP Header Versioning

The version travels in an `Accept` or custom header:
```
Accept: application/vnd.company.v2+json
X-API-Version: 2
```

**Advantages:**
- Single URL for all versions: `/users` remains `/users` across upgrades
- Works with content negotiation; headers are the proper place for request metadata
- Cache-friendly: same URL, version varies by header
- RFC 7231 compliant (header-based media types)

**Disadvantages:**
- Invisible to naive clients; logs don't show version
- Harder to debug: must inspect headers, not URLs
- Load balancers and CDNs may ignore headers during routing
- Clients must remember to set header on every request

**When to use:** For minor versions or when a single URL must serve all versions transparently.

### Query Parameter Versioning (`?version=2`)

Version specified as query parameter: `/users?version=2`.

**Advantages:**
- Simple clients; can test in browser or curl
- Visible in URLs and logs

**Disadvantages:**
- Semantically incorrect per RFC 7231 (version is server preference, not client request parameter)
- Gets lost during form submissions and redirects
- Cache fragmentation: same resource with different cache keys
- Often conflates versioning with filtering

**When to use:** Avoid except for backward compatibility with legacy APIs; strongly disfavored.

### Content Negotiation (Accept Media Types)

Clients request a specific media type version:
```
Accept: application/vnd.github.v3+json
Accept: application/vnd.github.graphql-preview+json
```

GitHub uses this heavily. Version encoded in media type, not URL.

**Advantages:**
- Web standards compliant; leverages HTTP content negotiation design
- Single URL, multiple representations
- Explicit media type contracts

**Disadvantages:**
- Complex for clients unfamiliar with Accept headers
- Requires server to support multiple media type versions simultaneously
- Documentation must explain media type syntax

**When to use:** For APIs expecting sophisticated clients (libraries, microservices); common in enterprise/GitHub-style APIs.

## Breaking vs. Non-Breaking Changes

### Non-Breaking (Additive) Changes

Safe to deploy without versioning:

- **Add new endpoint** — existing clients unaffected
- **Add optional request parameter** — existing requests still valid
- **Add response field** — clients ignore unknown fields
- **Deprecate (mark unused) endpoint** — keep it running; send `Deprecation` header
- **Add new HTTP method** to same URI — GET clients still work
- **Extend enum values** — new options won't break clients that don't use them
- **Add new object property** — clients using JSON ignore extra fields

### Breaking Changes

Require versioning:

- **Remove endpoint or field** — breaks parsing
- **Change field type** — `"age": "30"` vs `"age": 30` breaks clients
- **Rename resource** — `/articles` becomes `/posts`
- **Change HTTP method** for same URI — breaking for existing clients
- **Modify required parameter** — requests that omit it now fail
- **Change authentication scheme** — all clients must update
- **Change status code meanings** — retry logic depends on codes

## Deprecation & Sunset Workflow

### Deprecation Headers

Signal upcoming breaking changes via HTTP headers (RFC 8594 — "Deprecation" and "Sunset"):

```
HTTP/1.1 200 OK
Deprecation: true
Sunset: Sun, 31 Dec 2025 23:59:59 GMT
Deprecation-Link: https://api.example.com/migration-guide
```

- **Deprecation: true** — resource is deprecated; new code shouldn't use it
- **Sunset: <date>** — exact date when resource stops being served
- **Deprecation-Link** — (custom) link to migration guide

### Typical Deprecation Timeline

1. **Month 0 (announcement):** Publish deprecation notice in release notes, changelog, documentation
2. **Month 1:** Add Deprecation and Sunset headers to endpoint; send email to API portal users
3. **Month 3-9:** Monitor telemetry; reach out to high-volume users
4. **Month 9:** Final warning; send notifications
5. **Month 12:** Remove endpoint; fail with 410 Gone status

Variance: internal APIs may compress timeline to 3 months; public APIs may stretch to 18+ months to accommodate large client bases.

### Graceful Degradation

Avoid breaking existing clients abruptly. Instead:

- **Wrapper endpoints:** `/legacy/users/42` redirects to `/v2/users/42`, translating response format
- **Coexistence:** v1 and v2 run in parallel; routing layer chooses based on header
- **Fallback transforms:** Server knows both old and new schemas; detects client version, returns appropriate format
- **Feature flags:** Conditional logic in single endpoint; gradually shift traffic

## GraphQL Versioning

GraphQL typically doesn't use explicit versioning because:

- **Field deprecation:** `@deprecated` directive marks unused fields; clients see deprecation in schema introspection
- **Additive design:** New fields don't break queries; queries ignore new fields
- **Schema evolution:** Clients request specific fields; adding new fields is always additive

```graphql
type User {
  id: ID!
  name: String!
  email: String! @deprecated(reason: "Use emailAddress instead")
  emailAddress: String
}
```

Clients continue using `email` until they update; remove field only after long deprecation period.

## Versioning Data Models

Store version with resources so clients can validate compatibility:

```json
{
  "apiVersion": "2.0",
  "resources": [{
    "kind": "User",
    "apiVersion": "1.2.1",
    "id": 42,
    "name": "Alice"
  }]
}
```

Enables:
- **Schema validation:** client rejects incompatible versions
- **Migration:** client detects and transforms data
- **API gateway routing:** proxy routes based on resource version

## Evolution Best Practices

1. **Version from the start** — even if only v1 exists initially; adds flexibility later
2. **Minimize version count** — maintain previous major version + current version; archive others
3. **Communicate early** — ship deprecation notices 6-12 months before sunset
4. **Monitor adoption** — track which versions are actually used; don't deprecate heavily used old versions
5. **Batch breaking changes** — don't deprecate one field per release; group into version upgrades
6. **Document migration paths** — make it easy for clients to understand how to upgrade
7. **Smooth upgrades** — provide adapters, transform proxies, or API wrappers for a transition period
8. **Test compatibility** — maintain test suites for v-1 and current version to ensure old clients work

## Related Concepts

See also: [api-design](api-design.md), [api-error-handling](api-error-handling.md), [web-api-patterns](web-api-patterns.md), [patterns-api-gateway](infrastructure-api-gateway-patterns.md)