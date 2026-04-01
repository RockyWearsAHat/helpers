# API Security — Authentication, Authorization, Rate Limiting & Protection

## Overview

API security spans multiple layers: verifying caller identity (authentication), determining what they can access (authorization), and protecting against resource exhaustion and abuse. APIs are often the most exposed surface of an application and frequently targeted for data theft, account takeover, and denial-of-service attacks.

## Authentication Methods

### API Keys

Simple bearer tokens sent in headers or query parameters. Minimal overhead, widely supported, but stateless — once issued, a key grants access until revoked (no expiration mechanism built-in).

**Trade-offs:**
- Simple for internal services and machine-to-machine (M2M) communication
- No built-in revocation, key rotation, or expiration
- Lost keys grant perpetual access unless discovered and revoked
- Not suitable for user-facing APIs; OAuth is preferred

**Best practices:**
- Store keys server-side; never commit to version control
- Rotate keys periodically (monthly or less); track creation date
- Implement rate limiting per key
- Revoke keys immediately upon exposure or when no longer needed

### OAuth 2.0 / OIDC

OAuth 2.0 delegates authentication to a trusted identity provider, issuing short-lived access tokens and refresh tokens. OpenID Connect (OIDC) adds an identity layer (ID tokens containing user claims).

**Architecture:**
- User requests access to a resource protected by an API
- API redirects to identity provider (IdP) for authentication
- IdP returns authorization code back to client
- Client exchanges code for access token (and optionally refresh token)
- Client sends access token in API requests; API validates it

**Token types:**
- **Access token:** Used to authorize API requests; short-lived (5–60 minutes typical)
- **Refresh token:** Used to obtain new access tokens without re-authenticating; longer-lived (days/weeks)
- **ID token:** OIDC-specific; contains claims about the authenticated user; not intended for API authorization

**PKCE (Proof Key for Code Exchange):** Mitigates authorization code interception on mobile/single-page apps by requiring a dynamically generated code verifier.

### JWT (JSON Web Tokens)

Self-contained tokens carrying claims (user ID, issued-at time, scopes) signed by an issuer. Stateless — the API validates the signature without querying a server.

**Structure:** Header (alg) . Payload (claims) . Signature (HMAC or RSA)

**Strengths:**
- Reduces database queries on every API request
- Works well for distributed systems
- Can encode scopes and permissions directly

**Weaknesses:**
- Cannot revoke tokens mid-lifetime (once issued, valid until expiration)
- If a secret is leaked, all JWTs signed with it become compromised
- Requires secure key management

**Best practices:**
- Use RS256 (RSA) over HS256 (HMAC); HMAC requires sharing the secret across all services
- Keep expiration times short (5–15 minutes); use refresh tokens for longer-lived sessions
- Include issued-at (`iat`), expiration (`exp`), and issuer (`iss`) claims
- Never embed sensitive data in JWTs (refresh tokens, passwords)

### mTLS (Mutual TLS)

Both client and server present X.509 certificates to each other. Common in service-to-service communication.

**Use case:** Backend services communicating over insecure networks where certificate revocation is managed centrally.

## Authorization Patterns

### RBAC (Role-Based Access Control)

Users are assigned roles; roles are assigned permissions on resources.

**Example:** `User has Role: Admin`, `Admin has Permission: Delete*`

**Simplicity:** O(1) permission lookup; easy to understand and audit.

**Limitations:** Cannot express context-dependent permissions (e.g., "delete only your own resources," "edit only during business hours").

### ABAC (Attribute-Based Access Control)

Policies evaluate attributes of the user, resource, environment, and action.

**Example:** `Allow if (user.department == "finance" AND resource.type == "ledger" AND time.hour >= 9 AND time.hour <= 17)`

**Strengths:** Expresses complex, context-aware policies without creating new roles.

**Trade-off:** Policy evaluation can be expensive; requires centralized authorization engine (e.g., OPA, Authzed).

### ReBAC (Relationship-Based Access Control)

Models authorization as a graph of relationships between entities.

**Example:** `User A is owner of Document X` → `User A can delete Document X`. `User B is member of Team Y` → `User B can edit Team Y's documents`.

**Use case:** Collaborative tools, file sharing systems, multi-tenant platforms.

## Rate Limiting & Throttling

Rate limiting prevents resource exhaustion by restricting request volume per client/user over a time window.

### Token Bucket Algorithm

Conceptual: A bucket holds tokens (up to capacity). Tokens are added at a fixed refill rate. Each request consumes one token; if no tokens remain, the request is rejected.

**Behavior:**
- Allows bursts (bucket capacity) followed by sustained load (refill rate)
- Simple to implement via `current_tokens = min(capacity, current_tokens + refill_rate * (now - last_refill_time))`
- Common in API gateways

**Parameters:**
- `capacity`: Maximum tokens (burst allowance), e.g., 100 requests at once
- `refill_rate`: Tokens per second, e.g., 10 req/s sustained

### Sliding Window

Tracks requests within a rolling time window (not fixed intervals).

**Example:** "No more than 1000 requests per 60 seconds." For each new request, check how many occurred in the last 60 seconds; reject if ≥ 1000.

**Strength:** More accurate than fixed windows; no edge-case burst at window boundaries.

**Trade-off:** Requires storing request timestamps; more memory overhead than token bucket.

### Leaky Bucket

Conceptually opposite of token bucket: requests are added to a queue; a leak pours requests out at a fixed rate.

**Behavior:** Smooths bursty traffic into steady output. Requests queued; processed in FIFO order.

**Use case:** Rate limiting at the gateway level before forwarding to backend.

### Fixed Window (Naive)

Count requests in fixed time intervals (e.g., "0–60s = 1000 requests").

**Problem:** Distributed spike at window boundaries. If limit is 1000/min, an attacker can send 1000 req at 59s and 1000 at 1m, resulting in 2000 in 2 seconds.

Not recommended for public APIs.

## OWASP API Top 10 Security Risks

### API1:2023 – Broken Object Level Access Control (BOLA)

Users can access resources belonging to other users by manipulating object identifiers.

**Example:** `GET /users/123/orders` accessible as `GET /users/456/orders` by changing the ID.

**Mitigation:**
- Validate that the authenticated user owns/has permission on the requested resource
- Avoid using sequential/predictable IDs; use UUIDs or obfuscated values
- Log access to sensitive resources

### API2:2023 – Broken Authentication

Weak authentication implementation: missing token validation, expired tokens accepted, no expiration, weak credentials.

**Mitigation:**
- Validate and refresh tokens on every request
- Use short token lifetimes
- Implement rate limiting on authentication endpoints
- Require strong credentials; enforce MFA

### API3:2023 – Broken Object Property Level Authorization

APIs expose object properties users shouldn't access.

**Example:** User's email returned in a field intended for internal use only.

**Mitigation:**
- Explicit allowlist of properties per user role
- Filter response based on authorization level

### API4:2023 – Unrestricted Resource Consumption

APIs lacking limits on request size, query complexity, or concurrent operations; enable DoS.

**Mitigation:**
- Rate limiting
- Input validation (max query depth, max fields returned)
- Pagination limits

### API5:2023 – Broken Function Level Authorization

Users bypass authorization on administrative endpoints.

**Example:** `POST /admin/delete-user` doesn't verify the caller is an admin.

**Mitigation:**
- Check authorization before executing any function
- Audit all endpoints for authorization gaps

### API6:2023 – Unrestricted Access to Sensitive Business Flows

Workflows can be manipulated (e.g., test payment without verification, skip approval steps).

**Mitigation:**
- Validate workflow state transitions
- Require approval from authorized users
- Log state changes

### API7:2023 – Server-Side Request Forgery (SSRF)

API accepts URLs and fetches them internally, allowing attackers to access internal resources.

**Example:** `POST /api/fetch?url=http://internal-admin-panel:8080`

**Mitigation:**
- Allowlist internal hosts
- Validate URLs against a safe schema/domain
- Use network segmentation; restrict outbound egress

### API8:2023 – Security Misconfiguration

Unpatched systems, default credentials, unnecessary features enabled, overly verbose error messages.

**Mitigation:**
- Keep dependencies patched
- Use security headers (HSTS, CSP, X-Frame-Options)
- Disable debug endpoints in production
- Remove unnecessary HTTP methods (e.g., TRACE)

### API9:2023 – Improper Inventory Management

Undocumented, deprecated, or shadow endpoints remain exposed.

**Example:** Old `/v1/api` endpoint remains accessible alongside new `/v2/api`.

**Mitigation:**
- Maintain an inventory of all endpoints
- Deprecate old endpoints explicitly with timeline
- Monitor for unauthorized/unexpected endpoints

### API10:2023 – Unsafe Consumption of APIs

Applications blindly trust third-party APIs and don't validate responses.

**Example:** A webhook endpoint changes data without validating origin.

**Mitigation:**
- Validate responses from external APIs
- Verify digital signatures on webhooks
- Use timeouts and circuit breakers

## API Gateway Security

API gateways sit between clients and backend services, providing centralized:

- **Authentication & token validation**
- **Rate limiting & throttling**
- **Input validation & request filtering**
- **Response transformation** (e.g., stripping sensitive headers)
- **SSL/TLS termination**
- **Logging & monitoring** of all traffic

**Trade-off:** Gateways can become bottlenecks; ensure high availability and horizontal scalability.

## Input Validation

**Principles:**
- **Never trust client input.** Validate on the server; client-side validation is UX only.
- **Whitelist, not blacklist.** Define what's "good" rather than blocking known bad patterns.
- **Validate type, length, format, and range.** SQL injection happens when attackers submit SQL in string fields; parameterized queries + input validation combo is essential.
- **Encode output.** When returning data to clients, encode appropriately (e.g., HTML entity encoding for web responses).

## Bot Detection

Protects APIs from automated abuse (scraping, credential stuffing, DDoS botnets).

**Techniques:**
- **Challenge-response (CAPTCHA):** Block if bot-like behavior detected; present puzzle
- **Behavioral analysis:** Rate limits, geographic anomalies, suspicious user-agent patterns
- **API fingerprinting:** Detect headless browser usage or automated tools
- **IP reputation:** Block known bot networks via threat intelligence
- **Rate limiting per IP/user:** Threshold triggers review or blocking

## See Also

- Security principles (OWASP & Industry Standards)
- Web authentication patterns (Sessions, tokens, passkeys)
- OAuth 2.0 & OpenID Connect
- Zero Trust Architecture
- API Design Principles