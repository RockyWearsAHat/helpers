# Security — JWT Tokens: Structure, Algorithms, Vulnerabilities & Token Lifecycle

## Overview

JWT (JSON Web Tokens) are compact, signed tokens used for stateless authentication and authorization. Despite widespread adoption, JWTs have multiple high-impact vulnerability classes: algorithm confusion attacks, weak secrets, none-algorithm bypass, key confusion between HMAC and RSA, and poor refresh token management. This note covers JWT structure, security properties, common vulnerabilities, and defensive patterns for token lifecycle management.

---

## JWT Structure and Components

A JWT is three base64url-encoded parts separated by dots: `header.payload.signature`.

### Header
```json
{
  "alg": "HS256",
  "typ": "JWT"
}
```

- `alg`: Signing algorithm (HS256, RS256, ES256, etc.)
- `typ`: Token type (usually "JWT")

### Payload
```json
{
  "sub": "user123",
  "email": "user@example.com",
  "iat": 1234567890,
  "exp": 1234571490,
  "iss": "https://auth.example.com",
  "aud": "my-app"
}
```

- `sub`: Subject (typically user ID)
- `iat`: Issued at (Unix timestamp)
- `exp`: Expiration time (Unix timestamp)
- `iss`: Issuer (identity provider)
- `aud`: Audience (intended recipient)
- Custom claims: Any additional data (roles, permissions)

### Signature
The server signs the first two parts (header + payload) with a key and appends the signature. The client verifies the signature to ensure the token hasn't been tampered with.

```
HMACSHA256(base64url(header) + "." + base64url(payload), secret)
```

---

## Signing Algorithms: Trade-offs and Pitfalls

### HMAC-Based (HS256, HS384, HS512)

**Symmetric signing**: The same secret signs and verifies the token.

```javascript
const secret = 'my-super-secret-key';
const token = jwt.sign(payload, secret, { algorithm: 'HS256' });
const decoded = jwt.verify(token, secret);  // Must use same secret
```

**Pros:**
- Simple, fast, single shared key
- Suitable for internal services and M2M APIs

**Cons:**
- The key must be shared with every entity that verifies tokens
- Horizontal scaling requires distributing the key
- If any verifier is compromised, attackers can forge tokens
- Key rotation is difficult (multiple secrets must be supported simultaneously)

**Vulnerability: Weak Secrets**
If the shared secret is weak or default (e.g., "secret" or "password"), attackers can brute-force or dictionary-attack the signature:

```javascript
// Attacker guesses the secret and forges a token
const secret = 'secret';
const fakePayload = { sub: 'admin' };
const fakeToken = jwt.sign(fakePayload, secret, { algorithm: 'HS256' });
```

---

### RSA-Based (RS256, RS384, RS512)

**Asymmetric signing**: A private key signs; a public key verifies. Only the issuer (identity provider) holds the private key.

```javascript
const privateKey = fs.readFileSync('private.pem');
const publicKey = fs.readFileSync('public.pem');

const token = jwt.sign(payload, privateKey, { algorithm: 'RS256' });
const decoded = jwt.verify(token, publicKey);  // Public key only verifies
```

**Pros:**
- Scales horizontally: public key can be widely distributed
- Private key never leaves the issuer
- Public key can be published (e.g., at `/.well-known/jwks.json`)
- Key rotation is cleaner: issue new JWTs with a new `kid` (key ID)

**Cons:**
- Slightly slower than HMAC (asymmetric crypto is expensive)
- Requires infrastructure to manage key pairs
- Public key mismanagement still exposes the system

**Vulnerability: Key Confusion Attack**
Attackers switch the algorithm from RS256 to HS256, then use the public key as the HMAC secret. If the server doesn't validate the `alg` header, it may accept the forged token:

```javascript
// Attacker obtains the public key (it's public)
const publicKey = fs.readFileSync('public.pem');

// Attacker forges a token signed with HMAC using the public key as secret
const fakePayload = { sub: 'admin' };
const fakeToken = jwt.sign(fakePayload, publicKey, { algorithm: 'HS256' });

// Vulnerable server
const decoded = jwt.verify(fakeToken, publicKey);  // VULNERABLE!
// Server doesn't enforce RS256, accepts HS256 with public key
```

**Defense**: Always explicitly validate the algorithm:

```javascript
const payload = jwt.verify(token, publicKey, { algorithms: ['RS256'] });
```

---

### ECDSA-Based (ES256, ES384, ES512)

**Asymmetric signing** using elliptic curve cryptography, similar to RSA but more efficient.

**Pros:**
- Faster than RSA for same security level
- Smaller key sizes and signatures
- Modern alternative to RSA

**Cons:**
- Less widely deployed than RS256
- Requires ECDSA support in libraries and infrastructure

---

## Critical JWT Vulnerabilities

### Algorithm: None

Some JWT libraries allow an `alg: 'none'` header, which skips signature verification entirely. Attackers can craft an unsigned token that the server accepts:

```json
{"alg": "none", "typ": "JWT"}
{"sub": "admin"}
[empty signature]
```

Modern libraries reject this by default, but older versions and misconfigured parsers may accept it.

**Defense**: Explicitly require a signing algorithm in any library:

```javascript
jwt.verify(token, secret, { algorithms: ['HS256'] });
```

Never call `jwt.decode()` without verification in authentication code. Decoding without verification trusts the client's payload.

### Weak or Default Secrets

HMAC-based JWTs are only as strong as the shared secret. If the secret is weak, attackers can forge tokens in seconds:

```javascript
// Bad: default or weak secret
jwt.sign(payload, 'password', { algorithm: 'HS256' });

// Good: cryptographically random, 256+ bits
const secret = crypto.randomBytes(32).toString('hex');
jwt.sign(payload, secret, { algorithm: 'HS256' });
```

### Expired Tokens

JWTs include an `exp` (expiration) claim. If the server doesn't validate expiration, an attacker can replay old, captured tokens indefinitely:

```javascript
const payload = jwt.verify(token, secret);
// If exp claim is missing or ignored, token never expires
```

**Defense**: Most JWT libraries validate `exp` by default, but verify this behavior:

```javascript
const payload = jwt.verify(token, secret, { ignoreExpiration: false });
```

### Algorithm Downgrade

An attacker intercepts an RS256 token and downgrades it to HS256, signing it with the public key. If the server accepts any algorithm, it's vulnerable:

```javascript
// Vulnerable
jwt.verify(token, publicKey);  // Server accepts any algorithm

// Secure
jwt.verify(token, publicKey, { algorithms: ['RS256'] });
```

### Clock Skew & Token Leeway

`iat` (issued at) and `exp` (expiration) rely on server clocks. If clocks are out of sync, tokens may be rejected or accepted past expiration. Some libraries allow leeway (grace period):

```javascript
jwt.verify(token, secret, { clockTolerance: 30 });  // 30 seconds grace
```

Excessive leeway (e.g., > 60 seconds) weakens expiration guarantees and increases the window for replayed tokens.

### Claim Manipulation (Not a JWT Problem, But Common)

Developers sometimes trust JWT claims without server-side validation:

```javascript
// Vulnerable: Client-controlled permissions
const role = jwt.decode(token).role;
if (role === 'admin') {
  // Give admin access
}

// Secure: Server re-validates after token is verified
const user = database.getUserById(token.sub);
if (user.role === 'admin') {
  // Give admin access
}
```

JWTs prove **identity** (the issuer verified this user), not **authorization**. Always re-check permissions server-side.

---

## Token Lifecycle: Refresh Tokens and Rotation

### Access Tokens vs. Refresh Tokens

- **Access Token**: Short-lived (5-15 minutes), used to authenticate API requests
- **Refresh Token**: Long-lived (days to months), used to obtain a new access token without re-authenticating

```javascript
// Initial login
POST /auth/login
Response: {
  "access_token": "eyJ...",  // expires in 15 minutes
  "refresh_token": "eyJ...", // expires in 30 days
  "token_type": "Bearer"
}

// After access token expires
POST /auth/refresh
Body: { "refresh_token": "eyJ..." }
Response: {
  "access_token": "eyJ...",  // new access token
  "refresh_token": "eyJ..."  // optionally, new refresh token
}
```

**Why separate tokens?**
- Limits the blast radius of a compromised access token (short-lived)
- Refresh tokens can be rotated independently
- Prevents storing long-lived keys on the client

### Refresh Token Rotation

**Rotate refresh tokens on every refresh** to detect and invalidate compromised tokens:

```javascript
// Server receives refresh request
const oldToken = request.body.refresh_token;
const decoded = jwt.verify(oldToken, refreshTokenSecret);

// Check if token was already rotated (prevent replay)
if (database.isTokenRotated(oldToken)) {
  throw new Error('Refresh token already used—suspected token theft');
}

// Issue new tokens
const newAccessToken = issueAccessToken(decoded.sub);
const newRefreshToken = issueRefreshToken(decoded.sub);

// Mark old token as consumed
database.markTokenRotated(oldToken);

return { access_token: newAccessToken, refresh_token: newRefreshToken };
```

If an attacker steals a refresh token and tries to use it, the server detects the rotation and rejects all subsequent refresh attempts, alerting the user to revoke the session.

---

## Token Revocation Strategies

JWTs are stateless (no server-side storage), but revocation is sometimes necessary (logout, password change, permission revocation). Three approaches:

### 1. Token Expiration (Implicit Revocation)
Accept the short-lived token as implicitly revoked after expiration. Works well with short access tokens (5-15 minutes).

**Pros:** Simple, scale-free
**Cons:** Attacker can use the token until expiration; logout not immediate

### 2. Blacklist (Explicit, Stateful)
Store revoked token IDs in a cache or database.

```javascript
// Logout
const tokenJTI = jwt.decode(token).jti;  // JWT ID claim
database.addToBlacklist(tokenJTI);

// Authenticate
const tokenJTI = jwt.decode(token).jti;
if (database.isBlacklisted(tokenJTI)) {
  throw new Error('Token revoked');
}
```

**Pros:** Immediate revocation
**Cons:** Requires server-side state; doesn't scale as well as stateless JWTs; requires cleanup (expired tokens from blacklist)

### 3. Invalidation on Re-login (Implicit)
Increment a user's token generation counter; invalidate all tokens issued before the counter increment.

```javascript
// Login
const tokenGeneration = user.tokenGeneration;
const token = jwt.sign({ sub: user.id, gen: tokenGeneration }, secret);

// Logout (password change, etc.)
user.tokenGeneration += 1;
database.save(user);

// Authenticate
const decoded = jwt.verify(token, secret);
const user = database.getUserById(decoded.sub);
if (decoded.gen !== user.tokenGeneration) {
  throw new Error('Token invalidated');
}
```

**Pros:** Immediate revocation; revokes all tokens for a user; avoids blacklist overhead
**Cons:** Requires user record lookup on every request; slower than stateless verification

---

## Best Practices

### For Issuers (Identity Providers)

1. Use RS256 or ES256 for distributed systems; HMAC only for internal services with a single issuer
2. Generate strong, random secrets (256+ bits) for HMAC
3. Always include `exp`, `iat`, and `jti` (JWT ID) claims
4. Rotate keys regularly; include `kid` (key ID) in header for key management
5. Use short expiration times for access tokens (5-15 minutes)
6. Implement refresh token rotation to detect token theft

### For Consumers (Resource Servers)

1. Always validate the signature with the issuer's public key
2. Explicitly specify allowed algorithms; never accept `alg: 'none'`
3. Verify expiration (`exp`), issued at (`iat`), issuer (`iss`), and audience (`aud`)
4. Never trust JWT claims for authorization; re-validate permissions server-side
5. Use HTTPS only; never transmit JWTs over plain HTTP
6. Consider SameSite cookies if storing tokens in cookies; prefer Authorization headers for APIs

### For Clients

1. Store access tokens in memory or temporary storage (not localStorage, which is vulnerable to XSS)
2. Refresh access tokens before expiration using refresh token
3. Clear tokens on logout; implement token rotation
4. Use HTTPS for all communication
5. Validate certificate pinning in mobile apps if available

---

## See Also

- security-authentication-attacks (session hijacking, credential theft)
- security-oauth2-oidc.md (OAuth 2.0 and OpenID Connect protocols)
- security-web-application (HTTP headers, CSRF protection)
- security-identity.md (identity and access management architecture)