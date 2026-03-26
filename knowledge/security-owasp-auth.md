# Authentication & Authorization Failures

## Authentication Attacks

### Credential Stuffing

Automated submission of stolen username/password pairs from data breaches. Users reuse passwords across sites, so breached credentials from one site work elsewhere.

**Scale**: billions of credentials available. Attack tools: Sentry MBA, OpenBullet, custom scripts.

**Prevention**:

- Rate limiting per IP and per account
- CAPTCHA after failed attempts
- Credential breach detection (HaveIBeenPwned API, password hash checking)
- MFA (stops 99.9% of automated attacks)
- Bot detection (browser fingerprinting, behavioral analysis)

### Brute Force

Systematic password guessing. Variants: dictionary attack, hybrid (dictionary + rules), mask attack, rainbow table lookup.

**Prevention**:

- Account lockout (temporary, escalating: 5 min → 15 min → 1 hour)
- Progressive delays (exponential backoff on failed attempts)
- Strong password requirements (minimum 12 chars, check against breach lists)
- Bcrypt/scrypt/Argon2 for hashing (high computational cost = slow brute force)

### Session Attacks

**Session Fixation**: Attacker sets victim's session ID before authentication. Victim logs in, attacker uses the known session ID. Prevention: regenerate session ID after authentication.

**Session Hijacking**: Stealing active session token via XSS, network sniffing, or malware. Prevention: HttpOnly + Secure + SameSite cookies, TLS everywhere, short session lifetime.

**Session Prediction**: Guessing valid session IDs from patterns. Prevention: cryptographically random session IDs (128+ bits of entropy).

## Authorization Failures

### IDOR (Insecure Direct Object References)

```
GET /api/invoices/12345   # User A's invoice
GET /api/invoices/12346   # User B's invoice — accessible if no authz check
```

**Prevention**:

- Check object ownership in every data access
- Use indirect references (UUIDs instead of sequential IDs)
- Implement object-level authorization (not just endpoint-level)
- Verify in the data layer, not just the API layer

### Broken Function-Level Authorization

Admin endpoints accessible to regular users:

```
GET /api/users          # Normal user endpoint
GET /api/admin/users    # Should require admin role — doesn't check
POST /api/users/promote # Privilege escalation via unprotected endpoint
```

**Prevention**: centralized authorization middleware, deny by default, role verification on every endpoint.

### Vertical Privilege Escalation

User gains higher-privilege access (regular user → admin). Caused by missing role checks, parameter tampering (`role=admin` in request), forced browsing to admin URLs.

### Horizontal Privilege Escalation

User accesses another user's data at the same privilege level. IDOR is the most common form.

## JWT Vulnerabilities

### None Algorithm Attack

```json
// Header changed to:
{ "alg": "none", "typ": "JWT" }
// Signature removed — server accepts unsigned token
```

Prevention: explicitly whitelist allowed algorithms. Never accept `none`.

### Algorithm Confusion (Key Confusion)

Server expects RSA (asymmetric) but attacker sends HMAC (symmetric) using the RSA public key as the HMAC secret. The server verifies the HMAC signature using its public key and accepts it.

Prevention: validate algorithm matches expected type per key. Use libraries that don't allow algorithm switching.

### Secret Brute Force

Weak JWT secrets can be brute-forced offline (no rate limiting since verification is local). Tools: hashcat, jwt-cracker.

Prevention: minimum 256-bit random secret for HMAC, or use asymmetric algorithms (RS256, ES256).

### Claim Manipulation

Changing `sub`, `role`, `exp` claims in the payload. Only works if signature verification is broken or skipped.

### JWK Injection

Attacker embeds their own public key in the JWT header (`jwk` or `jku` parameter) and signs with the corresponding private key.

Prevention: ignore `jwk`/`jku` headers; use a hardcoded JWKS endpoint.

## Authorization Models

### RBAC (Role-Based Access Control)

Users → Roles → Permissions. Simple and widely used.

```
Admin role:  [create, read, update, delete] on all resources
Editor role: [create, read, update] on content
Viewer role: [read] on content
```

Limitations: role explosion in complex systems, no contextual decisions.

### ABAC (Attribute-Based Access Control)

Policies evaluate attributes of user, resource, action, and environment:

```
ALLOW if user.department == resource.department
       AND user.clearance >= resource.classification
       AND time.current BETWEEN 09:00 AND 17:00
```

More flexible than RBAC but more complex to manage.

### ReBAC (Relationship-Based Access Control)

Authorization based on relationships between entities. Google Zanzibar model (used by SpiceDB, OpenFGA, Authzed, Ory Keto).

```
document:readme#viewer@user:alice     # Alice can view readme
folder:docs#parent@document:readme    # readme is in docs folder
folder:docs#editor@team:engineering   # engineering team can edit docs
```

Supports inheritance: if you can edit a folder, you can edit its documents.

## Prevention Checklist

| Control            | Implementation                                                     |
| ------------------ | ------------------------------------------------------------------ |
| Password hashing   | Argon2id (preferred), bcrypt (cost ≥ 12), scrypt                   |
| Password policy    | Min 12 chars, breach list check, no composition rules              |
| MFA                | TOTP, WebAuthn/passkeys (phishing-resistant), push notification    |
| Session management | Random 128-bit ID, regenerate on auth, HttpOnly+Secure+SameSite    |
| Account lockout    | Temporary (5-30 min), notify user, track by IP and account         |
| API authentication | OAuth 2.0 + PKCE for user flows, API keys + HMAC for service flows |
| Authorization      | Centralized middleware, deny by default, check at data layer       |
| Token validation   | Verify signature, algorithm, issuer, audience, expiration          |
| Audit logging      | Log all auth events (success, failure, lockout, MFA bypass)        |
| Rate limiting      | Per-IP, per-account, per-endpoint                                  |
