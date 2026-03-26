# OAuth 2.0 & OpenID Connect

## OAuth 2.0 Core Concepts

OAuth 2.0 is an **authorization** framework — it grants third-party applications limited access to resources on behalf of a user. It does NOT authenticate the user (that's OIDC's job).

### Roles

| Role                 | Description                                    |
| -------------------- | ---------------------------------------------- |
| Resource Owner       | The user who owns the data                     |
| Client               | The application requesting access              |
| Authorization Server | Issues tokens (Keycloak, Auth0, Okta, Cognito) |
| Resource Server      | API that accepts access tokens                 |

### Tokens

- **Access Token**: short-lived (5-60 min), sent with API requests. Can be opaque string or JWT. Carries scopes.
- **Refresh Token**: long-lived, used to get new access tokens without user interaction. Store securely server-side.
- **ID Token** (OIDC only): JWT containing user identity claims. For the CLIENT, not the API.

## Grant Types

### Authorization Code + PKCE (Recommended for All Clients)

The standard flow for web apps, SPAs, and native apps.

```
1. Client generates code_verifier (random 43-128 chars) and code_challenge (SHA-256 hash)
2. Client redirects user to authorization endpoint with code_challenge
3. User authenticates and consents
4. Authorization server redirects back with authorization code
5. Client exchanges code + code_verifier for tokens
6. Server verifies SHA-256(code_verifier) == code_challenge
```

PKCE (Proof Key for Code Exchange) prevents authorization code interception. **Required for SPAs and mobile apps, recommended for all clients.**

### Client Credentials

Service-to-service authentication (no user involved):

```
POST /token
Content-Type: application/x-www-form-urlencoded

grant_type=client_credentials
&client_id=my-service
&client_secret=secret123
&scope=api:read
```

Returns access token only. No refresh token, no user context.

### Device Code (Device Authorization)

For devices with limited input (TVs, CLI tools, IoT):

```
1. Device requests device_code and user_code from authorization server
2. Device displays user_code and verification URL to user
3. User visits URL on phone/computer, enters code, authenticates
4. Device polls token endpoint until user completes flow
```

### Refresh Token

```
POST /token
Content-Type: application/x-www-form-urlencoded

grant_type=refresh_token
&refresh_token=xRxGGEs...
&client_id=my-app
```

**Rotation**: authorization server issues new refresh token with each use, invalidating the old one. Detects token replay.

### Implicit (DEPRECATED)

Access token returned directly in URL fragment. Vulnerable to token leakage via browser history, referrer headers, and open redirectors. **Never use for new applications.**

## Scopes

Define the level of access requested:

```
scope=openid profile email read:repos write:repos
```

- `openid` — required for OIDC, triggers ID token issuance
- `profile` — name, picture, locale
- `email` — email address, email_verified
- Custom scopes: `read:repos`, `admin:org`, `api:full`

**Principle of least privilege**: request only the scopes you need.

## OpenID Connect (OIDC)

OIDC is an identity layer ON TOP of OAuth 2.0. Adds: ID tokens, UserInfo endpoint, standard claims, discovery.

### ID Token (JWT)

```json
{
  "iss": "https://auth.example.com",
  "sub": "user-12345",
  "aud": "my-client-id",
  "exp": 1711382400,
  "iat": 1711378800,
  "nonce": "random-nonce-value",
  "name": "Alice Smith",
  "email": "alice@example.com",
  "email_verified": true
}
```

### Standard Claims

| Claim                               | Description                         |
| ----------------------------------- | ----------------------------------- |
| sub                                 | Subject identifier (unique, stable) |
| name                                | Full name                           |
| given_name, family_name             | First/last name                     |
| email, email_verified               | Email and verification status       |
| phone_number, phone_number_verified | Phone                               |
| address                             | Structured address object           |
| locale, zoneinfo                    | Localization                        |
| updated_at                          | Last profile update timestamp       |

### Discovery Document

```
GET https://auth.example.com/.well-known/openid-configuration
```

Returns JSON with: `authorization_endpoint`, `token_endpoint`, `userinfo_endpoint`, `jwks_uri`, `scopes_supported`, `response_types_supported`, `claims_supported`.

## JWT Structure

```
eyJhbGciOiJSUzI1NiJ9.eyJzdWIiOiIxMjM0NSJ9.signature
|---- Header ----||----- Payload -----||- Signature -|
```

### Header

```json
{ "alg": "RS256", "typ": "JWT", "kid": "key-id-123" }
```

### Common Algorithms

| Algorithm | Type                       | Key                           |
| --------- | -------------------------- | ----------------------------- |
| HS256     | Symmetric (HMAC)           | Shared secret (≥256 bits)     |
| RS256     | Asymmetric (RSA + SHA-256) | RSA key pair (≥2048 bits)     |
| ES256     | Asymmetric (ECDSA + P-256) | EC key pair (faster, smaller) |
| EdDSA     | Asymmetric (Ed25519)       | Edwards curve (fastest)       |

**Recommendation**: ES256 or EdDSA for new systems. RS256 for maximum compatibility.

### JWKS (JSON Web Key Set)

```
GET https://auth.example.com/.well-known/jwks.json
```

```json
{
  "keys": [
    {
      "kty": "RSA",
      "kid": "key-id-123",
      "use": "sig",
      "n": "base64url-modulus...",
      "e": "AQAB"
    }
  ]
}
```

Cache JWKS with appropriate TTL. Refetch when `kid` in JWT header doesn't match cached keys.

### Token Validation Checklist

1. Verify signature using public key from JWKS
2. Validate `alg` matches expected algorithm
3. Check `iss` matches expected issuer
4. Check `aud` contains your client ID
5. Verify `exp` > current time (with clock skew tolerance)
6. Verify `iat` is reasonable
7. Check `nonce` matches if using OIDC (prevents replay)
8. Validate `nbf` (not before) if present

## Common Vulnerabilities

### Redirect URI Manipulation

Attacker changes `redirect_uri` to receive the authorization code:

```
/authorize?redirect_uri=https://evil.com/callback
```

Prevention: exact string matching of registered redirect URIs. No wildcards, no pattern matching.

### CSRF in OAuth

Attacker initiates OAuth flow, replaces authorization code with their own. Victim links attacker's account.

Prevention: `state` parameter (random, verified on callback) or PKCE.

### Token Leakage

Access tokens in URLs (query params, fragments) leak via browser history, referrer headers, server logs.

Prevention: use authorization code flow (not implicit), POST for token exchange, short token lifetime.

### Open Redirector

If `redirect_uri` validation allows open redirectors on the same domain, attacker chains: legitimate redirect → open redirector → evil site.

## Authorization Servers

| Server               | Type            | Key Features                                                                    |
| -------------------- | --------------- | ------------------------------------------------------------------------------- |
| Keycloak             | Open source     | Full-featured, SAML + OIDC, LDAP federation, themes, admin console              |
| Auth0                | SaaS            | Extensive SDKs, Actions (serverless hooks), machine-to-machine, universal login |
| Okta                 | SaaS/Enterprise | Workforce + customer identity, SCIM, lifecycle management                       |
| AWS Cognito          | AWS-managed     | User pools (auth) + identity pools (AWS creds), hosted UI, Lambda triggers      |
| Azure AD/Entra ID    | Azure-managed   | Enterprise SSO, B2C for customer identity, conditional access policies          |
| Ory (Hydra + Kratos) | Open source     | OAuth2 server (Hydra) + identity management (Kratos), headless, API-first       |
| Supabase Auth        | Open source     | GoTrue-based, email/phone/social login, row-level security integration          |

## Implementation Considerations

- Always use PKCE, even for confidential clients
- Store refresh tokens server-side; for SPAs use secure httponly cookie or BFF pattern
- Validate tokens on every request (don't trust client-side token storage alone)
- Use short access token lifetimes (5-15 min) with refresh token rotation
- Implement token revocation for logout
- Use `state` parameter even with PKCE (defense in depth)
- Prefer asymmetric algorithms (RS256/ES256) over symmetric (HS256)
- Never expose client secrets in frontend code
