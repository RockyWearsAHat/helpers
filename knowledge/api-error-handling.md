# API Error Handling — HTTP Status Codes, Error Schemas & Problem Details

## Overview

Error handling is where API design most directly impacts client experience. Ambiguous errors force clients to guess intent; malformed error responses break parsing. RFC 9457 (Problem Details) codifies a standard, machine-readable error format. Effective error handling combines precise HTTP status codes, consistent response schemas, and actionable error details.

## HTTP Status Codes

Status codes convey both the nature of failure and whether clients should retry. The 1xx-5xx range distributes responsibility:

### Success Codes (2xx)

- **200 OK** — request succeeded; response body contains result
- **201 Created** — POST succeeded and created a resource; response includes the created resource and Location header pointing to it
- **202 Accepted** — request is valid but processing is asynchronous; client should poll or check webhook callback
- **204 No Content** — request succeeded; no body to return (typical for DELETE)
- **206 Partial Content** — response contains part of a range request (used in resumable downloads, pagination)

### Redirection Codes (3xx)

- **301 Moved Permanently** — resource permanently moved; client should update stored URL
- **302 Found** — temporary redirect; client should not cache
- **304 Not Modified** — cached response is still fresh (conditional GET); client uses cache
- **307 Temporary Redirect** — like 302 but client must preserve HTTP method (POST remains POST)

### Client Error Codes (4xx)

Indicate client mistake or invalid request; retrying identical request will fail.

- **400 Bad Request** — malformed request syntax; can't parse JSON or missing required fields
- **401 Unauthorized** — authentication missing or invalid; client should retry with credentials
- **403 Forbidden** — authentication succeeded but client lacks permission; retrying won't help
- **404 Not Found** — resource doesn't exist; retrying won't help
- **405 Method Not Allowed** — HTTP method not supported for this resource (POST to read-only endpoint)
- **409 Conflict** — state conflict; e.g., duplicate resource creation, optimistic lock failure, concurrent modification
- **410 Gone** — resource existed but is permanently removed (deprecated API endpoint)
- **415 Unsupported Media Type** — request body format not accepted; client used wrong Content-Type
- **422 Unprocessable Entity** — request syntax is valid but semantic validation failed (email format invalid, age negative, business rule violation)
- **429 Too Many Requests** — rate limit exceeded; client should back off; response should include Retry-After header

### Server Error Codes (5xx)

Indicate server failure; client *may* retry (idempotent operations should, mutations typically shouldn't without user consent).

- **500 Internal Server Error** — unexpected server error; generic fallback
- **501 Not Implemented** — feature not yet implemented; permanent, don't retry
- **502 Bad Gateway** — upstream service unavailable
- **503 Service Unavailable** — server temporarily unavailable (maintenance, overload); include Retry-After header
- **504 Gateway Timeout** — upstream service didn't respond in time

### Choosing the Right Code

| Scenario                            | Code |
| ----------------------------------- | ---- |
| JSON parse error                    | 400  |
| Missing required field              | 400  |
| Invalid field value (business rule) | 422  |
| User not authenticated              | 401  |
| User authenticated but not allowed  | 403  |
| Resource URL doesn't exist          | 404  |
| Duplicate email error               | 409  |
| Rate limit hit                      | 429  |
| Server crashed                      | 500  |
| External service down               | 502  |

## RFC 9457: Problem Details Format

Standard machine-readable error response format. Clients can parse errors programmatically without special per-API logic.

### Core Schema

```json
{
  "type": "https://api.example.com/problems/invalid-input",
  "title": "Invalid Input",
  "status": 422,
  "detail": "The field 'age' must be a positive integer",
  "instance": "/users"
}
```

| Field      | Description                                             | Required |
| ---------- | ------------------------------------------------------- | -------- |
| type       | URI identifying problem type (e.g., problem class docs) | No       |
| title      | Short human-readable summary                            | No       |
| status     | HTTP status code                                        | No       |
| detail     | Human-readable explanation specific to this instance   | No       |
| instance   | URI identifying the specific problem (request URI)      | No       |

### Extended with Custom Fields

Problem Details can include application-specific fields:

```json
{
  "type": "https://api.example.com/problems/validation-failed",
  "status": 422,
  "detail": "Validation failed",
  "errors": [
    {
      "field": "email",
      "message": "Invalid email format",
      "value": "not-an-email"
    },
    {
      "field": "age",
      "message": "Must be >= 18",
      "value": 15
    }
  ]
}
```

### Content-Type Header

RFC 9457 specifies content type:
```
Content-Type: application/problem+json
```

For XML: `application/problem+xml`.

## Error Response Patterns

### Validation Errors

Multiple field failures in single request:

```json
{
  "status": 422,
  "title": "Validation Failed",
  "errors": [
    {
      "field": "username",
      "code": "DUPLICATE",
      "message": "Username is already taken"
    },
    {
      "field": "password",
      "code": "TOO_SHORT",
      "message": "Password must be at least 12 characters"
    },
    {
      "field": "terms",
      "code": "REQUIRED",
      "message": "Must accept terms and conditions"
    }
  ]
}
```

### Authentication Errors

```json
{
  "status": 401,
  "type": "https://api.example.com/problems/invalid-token",
  "title": "Invalid or Expired Token",
  "detail": "Bearer token expired at 2026-03-25T10:00:00Z",
  "challengeScheme": "Bearer",
  "realm": "api.example.com"
}
```

### Rate Limit Errors

```json
{
  "status": 429,
  "type": "https://api.example.com/problems/rate-limit-exceeded",
  "title": "Rate Limit Exceeded",
  "detail": "Free tier allows 100 requests per hour; you've made 150",
  "retryAfter": 3600,
  "limit": 100,
  "remaining": 0,
  "reset": 1395896400
}
```

Also include headers:
```
Retry-After: 3600
RateLimit-Limit: 100
RateLimit-Remaining: 0
RateLimit-Reset: 1395896400
```

### Conflict/Idempotency Errors

```json
{
  "status": 409,
  "type": "https://api.example.com/problems/duplicate-request",
  "title": "Duplicate Request",
  "detail": "Request with idempotency key already processed",
  "idempotencyKey": "7e9f2c48-c4bc-48ce-8e69-b38e2a27ab42",
  "previousResponse": {
    "status": 201,
    "id": "res_12345"
  }
}
```

## Error Categorization

### By Retryability

**Safe to retry (idempotent operations only):**
- 408 Request Timeout
- 429 Too Many Requests
- 5xx Server Errors (except 501)

**Never retry:**
- 400, 401, 403, 404
- 405, 415, 422
- 501 Not Implemented

**Conditional:**
- 409 Conflict — depends on whether concurrent modification can be resolved

### By Root Cause

| Category              | Codes              | Cause                                    |
| -------------------- | ------------------ | ---------------------------------------- |
| Invalid input        | 400, 422           | Client request malformed or invalid      |
| Authentication       | 401                | Missing or invalid credentials           |
| Authorization        | 403                | Insufficient permissions                 |
| Not found            | 404                | Resource doesn't exist                   |
| State conflict        | 409                | Current state incompatible with request  |
| User error           | 4xx (client's job) | Client mistake, not repeated issue      |
| Rate limited         | 429                | Quota exhausted                          |
| Server error         | 5xx                | Transient or permanent server failure    |

## Documentation & Discovery

### Error Catalog

Document all errors by code and type:

```markdown
## Errors

### 422 Unprocessable Entity — Validation Failed

Client request is syntactically valid but fails business rules.

**Problem Type:** `https://api.example.com/problems/validation-failed`

**Common causes:**
- Email format invalid
- Age < 18
- Duplicate resource
- Insufficient balance

**Example:**
```json
{
  "status": 422,
  "errors": [{"field": "email", "code": "INVALID_FORMAT"}]
}
```

**Client action:** Validate input and retry.
```

### Type-Based Documentation

Map problem types to documentation:

```json
{
  "type": "https://api.example.com/docs/problems/duplicate-email",
  "status": 422,
  "detail": "Email 'alice@example.com' is already registered"
}
```

Clients can append type to docs base URL: `https://api.example.com/docs/problems/duplicate-email`.

## Idempotency & Error Recovery

### Idempotency Keys

Include idempotency key in request to retry safely:

```
POST /payments
Idempotency-Key: 7e9f2c48-c4bc-48ce-8e69-b38e2a27ab42
```

Server stores (request, response) pair; identical request returns cached response.

If request succeeds on retry, return same response:
```json
{
  "status": 201,
  "data": { "id": "charge_123" }
}
```

If original succeeded but request retries after server error, return **same status and cacheable response**, not 5xx.

## Related Concepts

See also: [api-design](api-design.md), [api-versioning](api-versioning.md), [api-authentication](api-authentication.md), [patterns-idempotency](patterns-idempotency.md), [patterns-retry-backoff](patterns-retry-backoff.md)