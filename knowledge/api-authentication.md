# API Authentication — Keys, OAuth, JWT, HMAC, mTLS & Zero-Trust

## Overview

API authentication verifies the *caller's identity*. Unlike web authentication (user-centric: "who is this person?"), API authentication is application-centric: "what trusted service/client made this request?" The choice of authentication method patterns directly into deployment: API keys fit internal microservices; OAuth 2.0 fits delegated user-facing APIs; mutual TLS fits zero-trust infrastructure. No single method dominates; production systems often employ multiple, layered schemes.

## API Keys

Simple bearer tokens issued by server, presented by client in subsequent requests. Stateless on server (no lookup required); trade-off is revocation latency and lack of expiration.

### Characteristics

- **Format:** opaque string (32-64 characters, base64 or hex)
- **Transmission:** HTTP header (`Authorization: Bearer sk_live_abc123...`) or query param (deprecated)
- **Server-side storage:** Compare against database; no cryptographic verification needed
- **Lifetime:** indefinite (until revoked or rotated)
- **Issuance:** API admin portal or programmatic API request

### Key Design

```
API Key = [prefix] + [random bytes]
Example: sk_live_abcdef1234567890abcdef1234567890
```

Prefix identifies key type (example: `sk_live_` = secret key, production; `pk_live_` = public key). Enables:
- Distinguishing key types at a glance
- Revoking entire key class
- Identifying environment (live vs. test)

### Rotation & Revocation

**Rotation:** periodically issue new key; keep old key valid for 30-90 days; clients migrate during window.

```
Timeline:
Day 0: Issue new key; notify clients
Day 7: Log warning: "old key used; please update"
Day 60: Disable old key; active sessions fail
Day 90: Delete old key from database
```

**Revocation:** immediate (on compromise) or scheduled (key retired).

**Scoping:** Limit key permissions to minimize blast radius on compromise:

```json
{
  "key": "sk_live_abc123",
  "scopes": ["read:users", "write:orders"],
  "rateLimit": "1000/hour",
  "allowedIPs": ["203.0.113.0/24"],
  "environment": "production",
  "expiresAt": "2026-12-31"
}
```

### Strengths & Weaknesses

**Strengths:**
- Simple for clients and servers
- No round-trip authentication needed (fast)
- Good for machine-to-machine (service-to-service)
- Familiar to developers

**Weaknesses:**
- No built-in expiration; keys grant perpetual access if not rotated
- Revocation requires database lookup (can't be instant)
- Lost key = full compromise until discovery
- Not suitable for user-facing APIs (users shouldn't hold privileged credentials)
- No refresh mechanism
- Audit trail depends on server logging

**Best for:** Internal service calls, simple APIs, development/testing

## OAuth 2.0 & OpenID Connect

OAuth 2.0 is an *authorization delegation* framework; OpenID Connect (OIDC) adds an identity layer.

### Flow: Authorization Code (User-Facing API)

```
1. User visits web app
2. Web app redirects to identity provider (IdP): GET https://idp.example.com/authorize?client_id=...&redirect_uri=...&scope=openid email
3. User logs in at IdP, grants permission
4. IdP redirects back to web app with authorization code
5. Web app backend exchanges code for access token (POST, private channel, includes client_secret)
6. Web app receives access token (short-lived, e.g., 1 hour) + refresh token
7. Web app stores tokens securely (httpOnly cookie or server session)
8. On subsequent requests, web app sends Bearer token in Authorization header
```

### Flow: Client Credentials (Service-to-Service)

```
1. Service A needs to call Service B
2. Service A requests token from IdP: POST /token
   - client_id: A's identifier
   - client_secret: A's secret (like symmetric key)
   - grant_type: client_credentials
3. IdP issues short-lived access token
4. Service A calls Service B with Bearer token
5. Service B validates token with IdP (or cached key set)
6. Service B processes request
```

### Token Properties

**Access Token** (short-lived, ~1 hour)
- Bearer token sent with every API request
- Grants access to protected resources
- Revocable by server or IdP
- Can include claims (user ID, scopes, audience)

**Refresh Token** (long-lived, ~30 days to lifetime)
- Kept secure on client/backend
- Used to retrieve new access token without user re-authentication
- Can be revoked
- Rotated on use (new refresh token issued with each access token request)

**ID Token** (OIDC only, assertions about user identity)
- Contains claims: user ID, name, email, issued-at, expiration
- JWT format (signed by IdP)
- Used by apps to learn about user; not for API authorization

### Token Formats

**Opaque tokens:** Server-issued, stateless lookup in token store.
```
Authorization: Bearer ey9f2c48c4bc48ce8e69b38e2a27ab42
```

**JWT (JSON Web Token):** Self-contained claims, cryptographic signature. Server validates signature offline (no lookup).

```
eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c
```

Decodes to:

```json
{
  "alg": "HS256",
  "typ": "JWT"
}
.
{
  "sub": "user_123",
  "email": "alice@example.com",
  "iat": 1516239022,
  "exp": 1516325422,
  "iss": "https://idp.example.com",
  "aud": "api.example.com"
}
.
[signature]
```

## JWT Bearer Tokens

JWTs are self-describing, allowing API servers to validate without querying identity provider. Single point of trust: the public key used to verify signature.

### Typical API Usage

```
1. Client authenticates (login, OAuth flow)
2. IdP issues JWT: Authorization: Bearer eyJhbGc...
3. Client sends JWT with every request
4. API server:
   a. Extracts JWT from Authorization header
   b. Validates signature using public key (from IdP's JWKS endpoint, cached)
   c. Checks exp (expiration), aud (audience), iss (issuer)
   d. Extracts claims (user ID, scopes, roles)
   e. Makes authorization decision based on claims
   f. Processes request
```

### Signature Verification

Server holds IdP's public key(s) (JWKS — JSON Web Key Set). On every request:

```json
GET https://idp.example.com/.well-known/jwks.json
{
  "keys": [
    {
      "kty": "RSA",
      "use": "sig",
      "kid": "2024-key",
      "n": "0vx7agoebGcQSuuPi...",
      "e": "AQAB"
    }
  ]
}
```

Server caches keys (short TTL, e.g., 5 minutes). Validates JWT signature against cached public key. Invalid signature → reject (401).

### Revocation Problem

JWTs are stateless; if token leaked, server can't instantly revoke it. Mitigations:

- **Short expiration:** 15-60 minutes; leaked token has limited window
- **Revocation list:** Server maintains blacklist; checks on request (defeats stateless benefit)
- **Logout:** Clear token from client (doesn't invalidate server-side if stateless)
- **Token binding:** Tie token to specific IP or TLS certificate; change = invalid

## HMAC Signatures

Client and server share a symmetric secret. Client computes HMAC-SHA256 over request body/method/path; sends signature in Authorization header.

```
GET /api/users/123
Date: Wed, 26 Mar 2026 10:00:00 GMT
Authorization: HMAC-SHA256 Credential=access_key_id/20260326/api_request, SignedHeaders=host;x-amz-date, Signature=abc123...
```

Server recomputes HMAC using shared secret; if signatures match, request is authentic and unmodified.

### Strengths

- No certificate infrastructure needed
- Tamper-proof: any modification invalidates signature
- Works without third-party IdP
- High-performance offline verification

### Weaknesses

- Shared secret must be protected (if compromised, attacker can forge requests)
- No expiration; rotating key requires coordination
- Vulnerable to timing attacks if not constant-time comparison
- Client must include request details in signature (complex)

**Best for:** Webhook verification, microservice communication, when PKI is overkill

## Mutual TLS (mTLS)

Client and server both authenticate via X.509 certificates. TLS handshake verifies both parties.

### Requirements

1. Server has certificate (public key) signed by CA
2. Client has certificate (public key) signed by same CA
3. Server is configured to require client certificate
4. TLS handshake exchanges certificates; both verify against CA

### Flow

```
1. Client initiates TLS connection
2. Client sends its certificate
3. Server verifies client certificate against trusted CA
4. Server accepts connection only if cert is valid and trusted
5. Encrypted channel established
6. HTTP request sent over encrypted channel
7. Server extracts client identity from certificate CN (Common Name) or SAN (Subject Alt Name)
```

### Strengths

- Machine-level authentication (certificate = identity)
- Zero-knowledge: no secrets transmitted (certificates could be public)
- Encrypted + authenticated in one layer
- Works for any protocol (not just HTTP)
- Ideal for zero-trust deployments

### Weaknesses

- Requires certificate infrastructure (CA, PKI)
- Certificate rotation complexity
- Developer experience poor (certificate chains, paths confusing)
- Debugging harder (certificates opaque to curl/browser)

**Best for:** Internal service mesh (Istio, Linkerd), zero-trust infrastructure, high-security deployments

## Zero-Trust API Authentication

Zero-trust assumes no request is inherently trustworthy. Every API call must prove identity and intent, regardless of origin.

### Principles

1. **Verify every request** — No trust based on network location or previous successful calls
2. **Use multiple signals** — IP, certificate, token, request signature, device posture
3. **Grant least privilege** — Token scopes, role-based access, time-limited access
4. **Audit everything** — Log all authentication decisions and requests
5. **Revoke fast** — Ability to instantly deny compromised credentials
6. **Encrypt in transit** — TLS/mTLS always; no plain HTTP

### Implementation

**Layer 1: mTLS**
```
Client certificate validates client identity to server
```

**Layer 2: Bearer token**
```
Authorization: Bearer <short-lived JWT>
```

**Layer 3: Request signature (optional)**
```
X-Signature: HMAC-SHA256(secret, method+path+body)
```

**Layer 4: Rate limiting & quota per identity**
```
Limit requests per token/certificate
```

**Layer 5: Audit log**
```
Every request logged with identity, intent, decision
```

## Webhook Verification

Webhooks (outbound calls from server to client) require authentication too.

### Common Pattern: HMAC Signature

Server sends webhook with signature:

```http
POST https://client.example.com/webhooks/order.shipped
Authorization: Bearer shared_secret
X-Signature: SHA256=abc123def456
X-Webhook-ID: webhook_123
X-Webhook-Timestamp: 1645564800

{
  "id": "order_456",
  "status": "shipped"
}
```

Client verifies:

```python
import hmac
import hashlib

signature = request.headers['X-Signature'].split('=')[1]
body = request.raw_body  # Raw bytes, not parsed JSON
expected_signature = hmac.new(
    secret.encode(),
    body,
    hashlib.sha256
).hexdigest()

if not hmac.compare_digest(signature, expected_signature):
    return 401  # Signature mismatch
```

Also validate timestamp (prevent replay):

```python
now = time.time()
webhook_timestamp = int(request.headers['X-Webhook-Timestamp'])
if abs(now - webhook_timestamp) > 300:  # 5-minute window
    return 401  # Stale webhook
```

## Rate Limiting by Auth Tier

Different authentication tiers get different rate limits:

| Auth Method          | Requests/hour | Burst | Use Case              |
| -------------------- | ------------- | ----- | --------------------- |
| Anonymous (no auth)  | 100           | 10    | Public API, no limits |
| API key (free tier)  | 1,000         | 100   | Development          |
| API key (paid tier)  | 100,000       | 1,000 | Production           |
| OAuth (user)         | 10,000        | 500   | Mobile apps          |
| OAuth (service)      | 1,000,000     | 10K   | Trusted partners     |
| mTLS                 | Unlimited     | —     | Internal only        |

Server tracks usage per principal (token, API key, certificate CN) and enforces limits.

## Related Concepts

See also: [security-api](security-api.md), [security-oauth2-oidc](security-oauth2-oidc.md), [web-authentication-patterns](web-authentication-patterns.md), [patterns-rate-limiting](patterns-rate-limiting.md), [security-identity](security-identity.md)