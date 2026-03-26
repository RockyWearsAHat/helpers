# OAuth 2.0 Advanced Flows — DPoP, PAR, RAR, Token Introspection & Revocation

## Overview

OAuth 2.0 core flows (authorization code, client credentials, device flow) provide baseline authorization. Modern deployments require additional protections and capabilities:

- **DPoP** (Proof of Possession) — Binds access tokens to client device; prevents token theft and replay
- **PAR** (Pushed Authorization Requests) — Moves authorization request off the front-channel; reduces phishing and simplifies mobile redirects
- **RAR** (Rich Authorization Requests) — Expresses fine-grained authorization scopes beyond simple strings
- **Token Introspection** — Query token validity and metadata (expiry, scope), enabling offline authorization decisions
- **Token Revocation** — Explicitly invalidate tokens, securely logout, and rotate keys

These extensions address real-world vulnerabilities and operational needs. Understanding them is essential for secure API design and compliance (OpenBanking, FAPI standards).

## Proof of Possession (DPoP, RFC 9449)

### Problem: Token Theft & Replay

Access tokens are bearer tokens—possession implies authorization. If a token is stolen (network sniffing, compromised client, leaked logs), attacker can use it anywhere. DPoP binds tokens to a specific client device or public key, making token theft valueless without the client's proof device.

### DPoP Protocol

Client generates a proof JWT for each API request:

```
DPoP header:
{
  "typ": "dpop+jwt",
  "alg": "ES256",
  "jwk": { "kty": "EC", "crv": "P-256", "x": "...", "y": "..." }
}

DPoP payload:
{
  "jti": "unique-id",
  "htm": "POST",
  "htu": "https://api.example.com/resource",
  "iat": 1516239022,
  "exp": 1516239082
}
```

**Signature**: Signed with client's private key (ES256, RS256, etc.). Each DPoP proof is unique per request (different `jti`, timestamp).

### Token Endpoint Interaction

During token exchange or refresh:

```
POST /token
Content-Type: application/x-www-form-urlencoded
DPoP: eyJhbGc...  (JWT signed by client's private key)

grant_type=authorization_code
&code=SplxlOBeZQQYbYS6WxSbIA
&redirect_uri=https%3A%2F%2Fclient.example.com%2Fcb
```

Authorization server validates the DPoP proof:
1. Decode JWT (must be a JWT)
2. Verify signature (public key from `jwk` header)
3. Verify `jti` is not replayed (idempotency guard)
4. Verify timestamp `iat` is recent (within ~5 seconds)
5. Verify `htm` and `htu` match the HTTP method and request URI

If valid, server returns an access token marked with `dpop` binding:

```json
{
  "access_token": "eyJhbGc...",
  "token_type": "DPoP",
  "expires_in": 3600,
  "scope": "email profile"
}
```

**Token type**: `DPoP` signals the token is bound to the client's key (not `Bearer`).

### API Request with DPoP

Client includes DPoP proof with each API call:

```
GET /resource HTTP/1.1
Host: api.example.com
Authorization: DPoP eyJhbGc...  (access token)
DPoP: eyJhbGc...  (new proof for this request)
```

API verifies:
1. Access token is valid and not expired
2. Token type is `DPoP`
3. DPoP proof is valid (signature, timestamp, method/URI match)
4. Token's bound public key (from issuance) matches the DPoP proof's key

Only if all checks pass, the request is authorized.

### DPoP Advantages & Limitations

**Advantages**:
- Stolen token is useless without client's private key
- Replay attacks blocked (nonce validation `jti`)
- No server-side state required (stateless validation)

**Limitations**:
- Requires client to hold and manage a keypair (operational complexity for web apps)
- Proof per request adds latency and computation
- Incompatible with existing bearer token flows (requires opt-in at token endpoint)
- Not yet widely adopted (early in deployment)

## Pushed Authorization Requests (PAR, RFC 9126)

### Problem: Front-Channel Authorization Request Exposure

Standard authorization code flow:

```
User clicks "Login"
→ Browser redirected to https://auth.example.com/authorize?client_id=...&scope=...&redirect_uri=...&state=...
→ Auth server shows consent screen
→ Browser redirected back to https://client.example.com/callback?code=...&state=...
```

Vulnerabilities:
- **Authorization URL exposure**: Long URL may be visible in browser history, server logs, referer headers, CDN logs
- **Phishing**: Client ID, scope, and callback in URL; attacker can craft fake authorization URL and phish users
- **Mobile complexity**: URL schemes, app links, deep links lengthen authorization URLs and may exceed length limits

### PAR Protocol

Client pushes the authorization request to the auth server *before* redirecting the user:

```
POST /pushed_authorization_requests
Content-Type: application/x-www-form-urlencoded

client_id=s6BhdRkqt3
&client_secret=gY1xQcESL5x
&response_type=code
&client_id=s6BhdRkqt3
&redirect_uri=https%3A%2F%2Fclient.example.com%2Fcb
&scope=openid+email
&state=af0ifjsldkj
```

Server validates the request parameters and returns a **request URI**:

```json
{
  "request_uri": "urn:example:bwc4JK87-PC5oa",
  "expires_in": 90
}
```

Client then redirects the user with just the request URI:

```
https://auth.example.com/authorize?client_id=s6BhdRkqt3&request_uri=urn:example:bwc4JK87-PC5oa
```

Auth server reconstructs the full request from the URI. User sees minimal information in the URL (only client ID and URI handle). Request URI is **opaque** and short; useless to phishers.

### PAR Advantages

- **Short, opaque URIs**: Front-channel is limited to client ID and request URI; not human-readable
- **Server-side validation**: Auth server validates all parameters before user sees consent screen
- **Mobile-friendly**: Deep links and URL schemes are shorter, less likely to exceed limits
- **Phishing-resistant**: Attacker can't inject parameters into the user-facing URL

## Rich Authorization Requests (RAR, RFC 9396)

### Problem: Scope Strings Are Coarse

OAuth scope is traditionally space-separated strings:

```
scope=read:emails write:calendar delete:files
```

Issues:
- **Unclear semantics**: `calendar:write` could mean write events, write calendar config, write sharing settings (ambiguous)
- **No structured expressions**: Can't express conditional access ("write events in the future, not the past") or resource-specific constraints
- **Consent UI is binary**: All-or-nothing; user can't request "read emails from this folder only"

### RAR Protocol

Client expresses authorization requirements as structured JSON objects:

```
POST /token
...
grant_type=authorization_code
&authorization_details=[
  {
    "type": "payment_initiation",
    "currency": "EUR",
    "amount": 123.50
  },
  {
    "type": "account_information",
    "operations": ["read"],
    "accounts": [
      {
        "iban": "DE89370400440532013000",
        "currency": "EUR"
      }
    ]
  }
]
```

Auth server processes structured details:
1. Validates each detail's schema (type, required fields)
2. Parses constraints (amount, operations, resource IDs)
3. Shows granular consent UI ("Transfer €123.50 to recipient X?" vs. "Grant access to payment?")
4. Issues token with authorization details encoded in the access token (JWT payload or introspection endpoint)

API validates details:
- Token carries details (e.g., transaction amount, account restrictions)
- API enforces constraints at request time (e.g., reject transfer if amount exceeds authorized limit)

### RAR Use Cases

**OpenBanking/Payment Initiation**:
```json
{
  "type": "payment_initiation",
  "instructedAmount": {
    "currency": "EUR",
    "amount": "123.50"
  },
  "creditorName": "Merchant XYZ",
  "creditorAccount": {
    "iban": "DE89370400440532013000"
  },
  "remittanceInformationUnstructured": "Invoice #123"
}
```

**Federated Access (subject to resource owner approval)**:
```json
{
  "type": "federated_resource_access",
  "roles": ["READER"],
  "resources": ["resource:123", "resource:456"],
  "validUntil": "2026-04-25T12:00:00Z"
}
```

## Token Introspection (RFC 7662)

### Purpose

API or authorization server queries a token's validity and metadata *offline* (without contacting the authorization server on every request). Enables distributed authorization decisions.

### Protocol

Resource server (API) calls:

```
POST /introspect
Content-Type: application/x-www-form-urlencoded
Authorization: Basic czZCaGRSa3F0MzpnWTF4UWN
  (Resource server authenticates to auth server via client credentials)

token=2YotnFZFEjr1zCsicMWpAA
&token_type_hint=access_token
```

Authorization server validates the token and returns metadata:

```json
{
  "active": true,
  "scope": "read:emails write:calendar",
  "client_id": "s6BhdRkqt3",
  "username": "jdoe",
  "token_type": "Bearer",
  "exp": 1516239022,
  "iat": 1516239200,
  "nbf": 1516239200,
  "sub": "jdoe",
  "aud": "https://api.example.com",
  "iss": "https://auth.example.com",
  "jti": "abc123"
}
```

If token is invalid or expired:

```json
{
  "active": false
}
```

### Benefits

- **Offline authorization**: API can cache or validate tokens without real-time auth server dependency
- **Centralized revocation**: Revoking a token is immediate; introspection reflects the change
- **Auditing**: `jti` (JWT ID) enables token tracking and audit trails
- **Debugging**: `exp`, `iat`, `nbf` clarify token lifecycle

### Limitations

- **Latency**: Introspection adds a network round-trip (unless cached)
- **Cache consistency**: Cached introspection results may be stale; immediate revocation is not guaranteed (depends on API cache TTL)
- **Server dependence**: API must trust the auth server's response; no cryptographic proof without JWT

## Token Revocation (RFC 7009)

### Purpose

Client explicitly invalidates a token (logout, app uninstall, user revoke access). Accelerates security event response and key rotation.

### Protocol

Client calls:

```
POST /revoke
Content-Type: application/x-www-form-urlencoded
Authorization: Basic czZCaGRSa3F0MzpnWTF4UWN

token=2YotnFZFEjr1zCsicMWpAA
&token_type_hint=access_token
```

Authorization server invalidates the token immediately. Future introspection queries or API requests with the token are rejected.

Response is always 200 OK (per RFC), regardless of success (prevents token invalidation detection attacks):

```
HTTP/1.1 200 OK
Content-Type: application/json
```

### Revocation vs. Refresh Token Rotation

**Revocation**: Client explicitly requests token invalidation. Immediate, but depends on client cooperation. If client is compromised, attacker can revoke legitimate tokens.

**Refresh token rotation**: On each refresh, auth server issues a new refresh token and invalidates the old one. Detects compromised tokens: if a stolen old refresh token is replayed, the auth server issues a new token and invalidates all issued tokens (breach detection).

```
POST /token

grant_type=refresh_token
&refresh_token=n4E2YotnFZFEjr1zCsicMWpAA

Response:
{
  "access_token": "new_access_token",
  "refresh_token": "new_refresh_token",  // Old one invalidated
  "token_type": "Bearer",
  "expires_in": 3600
}

// If old refresh token replayed:
Error response:
{
  "error": "invalid_grant",
  "error_description": "Refresh token has been revoked (possible token compromise detected)"
}
```

Auth server can take additional action (invalidate all user tokens, force re-authentication) on rotation failure.

## Refresh Token Rotation

### Threat Model

Refresh tokens are long-lived (weeks/months). If compromised:
1. Attacker can issue new access tokens indefinitely
2. User doesn't know until they manually examine token usage

Rotation detects this: if attacker uses a **old** refresh token, the auth server detects the anomaly and revokes all tokens.

### Rotation Flow

**First refresh (legitimate client)**:
```
Client sends: refresh_token_v1
Server responds: access_token_v1, refresh_token_v2
Server invalidates: refresh_token_v1
```

**Second refresh with old token (attacker)**:
```
Client sends: refresh_token_v1  (old, already used)
Server detects duplicate, responds: error (invalid_grant)
Server action: revoke refresh_token_v2 and all associated tokens
```

**Legitimate client's next refresh (after attacker's attempt)**:
```
Client sends: refresh_token_v2
Server responds: error (invalid_grant, token was revoked in breach response)
Server prompts: "Suspicious activity detected; please re-authenticate"
```

### Implementation Considerations

- **Grace period**: Some servers allow a brief window for retries (network flakiness). If client retries with the same refresh token within ~30s, don't treat as breach.
- **Offline clients**: Apps that don't refresh for weeks are vulnerable (attacker has old token, no rotation to detect). Combine with TTL-based re-authentication (access token expires, forces return to online auth).
- **Token family**: Some servers issue refresh tokens in "families"; if one token in the family is replayed, all tokens in the family are revoked (prevents attacker from pivoting).

## Practical Deployment Patterns

### Securing High-Risk Operations (DPoP + RAR)

```
POST /initiate-payment

// Client proves possession of device key
DPoP-Proof: urn:example:...

// Rich authorization details
authorization_details=[
  {
    "type": "payment_initiation",
    "amount": 100.00,
    "currency": "EUR",
    "recipient": "ACME Inc"
  }
]

// Server validates:
// 1. DPoP proof is valid (not stolen; tied to device)
// 2. Authorization details are approved by user (granular consent)
// 3. Signature prevents tampering
```

### Phishing-Resistant Flows (PAR + DPoP)

```
1. User clicks "Login with OAuth"
2. Client pushes authorization request to auth server (PAR)
3. Auth server returns opaque request URI
4. Client redirects: https://auth.example.com/authorize?client_id=...&request_uri=...
5. User logs in and consents (short URL; no parameter tampering)
6. Auth server redirects with authorization code
7. Client exchanges code for tokens, signing with DPoP
8. Token is bound to client's device; bearer tokens useless without proof
```

### Token Lifecycle Management (Introspection + Rotation)

```
1. Client caches token metadata from introspection (TTL 5 minutes)
2. API calls introspection on cache miss (lazy validation)
3. Every refresh, server rotates refresh token (breach detection)
4. On revocation (logout), client sends revoke request (immediate invalidation)
5. Optional: API can periodically re-introspect high-risk operations
```

## Adoption & Standards

**FAPI (Financial API)** mandates or recommends:
- DPoP for high-risk paymentAPIs
- PAR for authorization requests (phishing resistance)
- Refresh token rotation
- Token introspection for offline validation

**OpenID Connect**: Incorporates DPoP as extension; many providers support it.

**Microsoft Entra, Google Workspace, AWS**: Gradual rollout of DPoP and PAR support.

## Related Notes

See [OAuth 2.0 & OpenID Connect](security-oauth2-oidc.md) for core flows (authorization code, client credentials, device code). See [API Authentication](api-authentication.md) for JWT validation and token-based authorization at the API layer. See [Security Identity Management](security-identity.md) for broader identity infrastructure and federation patterns.