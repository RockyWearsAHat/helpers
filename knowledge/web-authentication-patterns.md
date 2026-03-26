# Web Authentication Patterns — Sessions, Tokens, OAuth, Passkeys & MFA

## Overview

Web authentication verifies user identity. Two primary approaches dominate:

- **Session-based:** Server stores state; client sends opaque session ID cookie
- **Token-based:** Client stores token; server validates cryptographic signature

Each pattern has security, scalability, and deployment trade-offs. Modern systems combine multiple techniques: tokens for APIs, sessions for web, OAuth for delegation, passkeys for passwordless, MFA for defense-in-depth.

---

## Session-Based Authentication

### How It Works

1. User submits login credentials (username + password)
2. Server validates credentials; creates session record in database/memory
3. Server sends session ID to client (via cookie)
4. Client sends cookie on subsequent requests; server looks up session
5. Server verifies session exists and is fresh; processes request
6. On logout, server deletes session; client cookie becomes invalid

### Session State
- Stored server-side: database, Redis, memory
- Contains: user ID, creation time, last-activity time, permissions, user data
- Opaque to client: client cannot decode or forge it

### Cookie Transport
```
Set-Cookie: sessionid=abc123def456; Path=/; HttpOnly; Secure; SameSite=Strict
```

- **HttpOnly:** Cookie not accessible via JavaScript (prevents XSS theft)
- **Secure:** Transmitted only over HTTPS (prevents man-in-the-middle interception)
- **SameSite=Strict:** Never sent cross-site (prevents CSRF attacks)
  - `Strict`: only same-site requests
  - `Lax`: cross-site navigation (click, redirect) but not form POST, fetch
  - `None`: sent in all contexts; requires Secure flag

### Advantages
- **Simplicity:** Built into browsers; no JavaScript required
- **Revocation:** Immediate — delete session, cookie becomes invalid
- **CSRF protection:** SameSite cookies prevent forged requests
- **Server control:** Server can revoke, modify, or terminate any session

### Limitations
- **Scalability:** Requires server-side state; hard to scale across stateless servers
  - Solution: sticky sessions (client routed to same server) or shared session store (Redis)
- **Multiple domains:** Cookies bound to single domain; cross-domain not practical
  - Workaround: session proxy, OAuth, or token-based auth
- **Mobile/API:** Native apps, IoT, and some APIs don't handle cookies well

### Best Practices
- Session ID length ≥ 128 bits (high entropy to prevent brute force)
- Store only non-sensitive data in session (user ID, permissions)
- Enforce HTTPS; reject non-HTTPS requests
- Set HttpOnly, Secure, SameSite=Strict flags
- Implement session timeout (e.g., 30 min inactivity, 8h absolute max)
- Clear all sessions on suspicious activity (multiple logins, impossible geo)
- Regenerate session ID after login (prevent fixation attacks)

---

## Token-Based Authentication (JWT)

### How It Works

1. User submits credentials; server validates
2. Server creates token (JSON Web Token): `header.payload.signature`
3. Token sent to client (stored in memory, localStorage, or cookie)
4. Client sends token in `Authorization: Bearer <token>` header
5. Server decodes token, verifies signature, extracts claims
6. No database lookup needed (signature verification is enough)
7. Token stores expiration time; expired tokens rejected

### JWT Structure
```
eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.
eyJzdWIiOiIxMjMiLCJuYW1lIjoiQWxpY2UiLCJpYXQiOjE2NDQ5NTczMzUsImV4cCI6MTY0NDk2MDkzNX0.
xxxx...signature
```

- **Header:** Algorithm (HS256, RS256), token type
- **Payload (Claims):** User ID, name, role, permissions, issued-at (`iat`), expiration (`exp`)
- **Signature:** HMAC or RSA signature of header + payload (prevents tampering)

Only server knows signing key; signature can't be forged without key.

### Stateless Advantage
- No database lookup required; cryptographic validation only
- Scales horizontally; any server can validate any token
- No session sync needed

### Advantages
- **API-friendly:** Works with mobile, SPA, and native apps
- **Cross-domain:** Token sent in header; works across domains/subdomains
- **Scalability:** Stateless; no session database overhead
- **Distributed:** Works across multiple servers/data centers without shared state

### Limitations
- **Revocation is hard:** Token valid until expiration; can't immediately revoke inactive users
  - Workaround: maintain revocation list (small database), or use short-lived tokens with refresh
- **Size:** JWT is larger than session ID (contains all claims); increases payload size
- **Token theft:** If client storage compromised, token stolen
  - Mitigation: short expiration, use secure storage (memory, not localStorage)

### Vulnerabilities & Mitigations
- **No algorithm (alg: none):** Attacker creates unsigned token; server accepts it
  - Mitigation: always validate algorithm matches expected (HS256, not none)
- **Key confusion:** Sign with secret key, verify with public key (or vice versa)
  - Mitigation: clearly separate asymmetric vs. symmetric keys
- **Weak secrets:** Short or predictable signing key can be brute-forced
  - Mitigation: use strong, randomly generated key (≥256 bits)

### Best Practices
- **Access token expiration:** 5–15 minutes (short-lived; limit damage if stolen)
- **Signing algorithm:** HMAC-SHA256 (symmetric) or RS256/ES256 (asymmetric)
- **Claims validation:** Always check `exp`, `iss` (issuer), `aud` (audience), `nbf` (not before)
- **Storage:** Memory (volatile; lost on refresh) or httpOnly cookie (not accessible by JS)
- **Never localStorage for sensitive tokens:** localStorage is accessible to any JavaScript (XSS vulnerability)
- **Validate signature on every request:** Don't trust token structure alone

---

## Refresh Tokens & Token Rotation

The goal: keep access tokens short-lived, but avoid requiring login every 5 minutes.

### Pattern

1. User logs in; server issues two tokens:
   - **Access token:** Short-lived (5–15 min), used for API requests
   - **Refresh token:** Long-lived (days/weeks), used only to get new access tokens

2. Client uses access token for requests
3. Access token expires; client uses refresh token to get new access token
4. Refresh token can be revoked server-side (e.g., on logout or suspicious activity)

### Refresh Token Rotation (Best Practice)
- Each refresh yields new access token **and** new refresh token
- Issue new refresh token with shorter TTL than previous
- If old refresh token reused (token replayed), detect anomaly and revoke all tokens
- Mitigates impact of stolen refresh token

```
// Login
POST /auth/login → { accessToken, refreshToken }

// Use accessToken for API calls
GET /api/user Authorization: Bearer <accessToken>

// AccessToken expires; refresh
POST /auth/refresh { refreshToken } → { newAccessToken, newRefreshToken }

// If old refreshToken replayed → revoke all tokens for user
```

### Storage
- **Access token:** Memory (lost on page reload; forces re-login) or httpOnly cookie
- **Refresh token:** httpOnly cookie (harder to steal) or secure storage
- Never put sensitive tokens in localStorage/sessionStorage

---

## OAuth 2.0 Authorization Flow

OAuth 2.0 is **authorization** (delegated access to resources), not authentication. It enables apps to access user data on another service without asking for password.

### Delegated Access Use Case
- User clicks "Sign in with Google"
- App redirects to Google login
- User approves permissions ("this app can read your email")
- Google redirects back to app with authorization code
- App exchanges code for access token
- App uses token to fetch user profile, email, etc.

### Authorization Code Flow (Most Common, Secure)

```
┌─────────┐         ┌─────────────┐         ┌────────────┐
│  App    │         │   Browser   │         │   Google   │
└────┬────┘         └──────┬──────┘         └─────┬──────┘
     │                     │                      │
     │ Redirect to Google  │                      │
     │─────────────────────┼────────────────────>│
     │                     │                      │
     │                     │ User approves        │
     │                     │                      │
     │ Redirect back code  │                      │
     │<─────────────────────────────────────────┤
     │                     │                      │
     │ Exchange code       │                      │
     │ (backend to backend)│                      │
     ├────────────────────────────────────────>│
     │                     │                      │
     │ Return access token │                      │
     │<─────────────────────────────────────────┤
     │                     │                      │
```

### Key Features
- **Client ID + Secret:** App credentials; "secret" must never be exposed in browser
- **Scopes:** Permissions requested; "email", "profile", "offline_access"
- **Consent screen:** User approves which data app can access
- **Redirect URI:** App endpoint where authorization code is sent; must be registered with provider
- **State parameter:** CSRF token; prevents malicious redirects

### Implicit Flow (Legacy, Deprecated)
- Older approach; access token returned directly in URL
- **Insecure:** Token exposed in browser history, referrer headers, logs
- Replaced by Auth Code Flow with PKCE

### PKCE (Proof Key for Code Exchange)
- Enhances Auth Code Flow for public/mobile clients (no secrets)
- Client generates random `code_verifier`, computes `code_challenge = hash(verifier)`
- Sends `code_challenge` in auth request; authorization server stores it
- Client sends `code_verifier` when exchanging code; server verifies hash matches
- Prevents authorization code interception/replay from malicious app

```
code_verifier = random string (128 chars)
code_challenge = base64url(sha256(code_verifier))

// Step 1: Send challenge
GET /authorize?...&code_challenge=...

// Step 2: Exchange code + verifier
POST /token client_id=...&code=...&code_verifier=...
```

---

## Cookie Security Attributes

When using cookies (session-based or to store tokens), secure these attributes:

### HttpOnly
- Cookie not accessible via `document.cookie` or JavaScript
- Prevents XSS attacks from stealing authentication cookies
- Always set for sensitive cookies

### Secure
- Cookie transmitted only over HTTPS
- Prevents man-in-the-middle interception over HTTP
- Always set for authentication cookies

### SameSite
- **Strict:** Sent only in same-site requests; prevents all CSRF
- **Lax:** Sent on cross-site navigation (link click, redirect) but not cross-site POST, fetch, frames
- **None:** Sent in all contexts (requires Secure flag); rare, for embedded content
- Default (no SameSite) behavior varies by browser; historically Lax, now moving to Strict

### Path & Domain
- **Path=/:** Cookie sent for all paths on domain
- **Domain:** Scope to domain and subdomains; empty means current domain only
- Restrict to necessary scope to limit exposure

```
Set-Cookie: sessionid=xyz; Path=/; Domain=example.com; HttpOnly; Secure; SameSite=Strict
```

---

## Passkeys / WebAuthn / FIDO2

Passwordless authentication using cryptographic keys stored on user's device.

### How It Works
1. User registers: browser/OS creates public/private key pair
2. Public key sent to server; private key stays on device
3. On login: server sends challenge; device signs it with private key
4. Server verifies signature using public key; login succeeds
5. No password stored or transmitted; no risk of password reuse

### Advantages
- **No passwords:** Eliminates password breach risk, phishing, weak passwords
- **Faster:** No password entry; tap fingerprint or security key
- **Recovery:** Multiple devices or backup keys prevent lockout
- **Phishing-resistant:** Server origin verified; can't be tricked into logging into fake site

### Types
- **Platform authenticators:** Built into device (Face ID, Windows Hello, fingerprint)
- **Cross-platform authenticators:** Security keys (YubiKey, etc.); work across devices
- **Backup/recovery:** QR codes, one-time recovery codes

### Adoption Note
- Emerging standard (FIDO2, WebAuthn); still evolving
- Requires browser support (modern browsers: Chrome, Firefox, Safari, Edge support)
- Best used in combination with passwords (not replacement yet for broad compatibility)

---

## Multi-Factor Authentication (MFA)

Requires multiple independent factors to verify identity. Factors:

- **Something you know:** Password
- **Something you have:** Phone (SMS, authenticator app), security key
- **Something you are:** Biometric (fingerprint, face)

### MFA Patterns

**SMS OTP (One-Time Password)**
- Server sends 6-digit code via SMS after password login
- User enters code; server verifies
- Cons: SMS is insecure; can be intercepted or SIM-swapped

**Authenticator App (TOTP)**
- User installs app (Google Authenticator, Authy, Microsoft Authenticator)
- Server provides secret; app generates time-based 6-digit code
- User enters code; server validates
- More secure than SMS; not vulnerable to interception

**Security Keys (FIDO2/U2F)**
- User has physical key (YubiKey, etc.)
- Server sends challenge; user presses button on key
- Key signs challenge; server verifies
- Extremely secure; phishing-resistant

**Backup Codes**
- Generate one-time use codes during setup
- User stores securely; used if primary MFA unavailable
- Essential for account recovery

### MFA Deployment
- Prompt for MFA after password validation
- If MFA fails, don't complete login; force retry
- Store MFA verification timestamp (e.g., remember device for 30 days)
- Require re-MFA on sensitive operations (password change, payment)

---

## Stateful Session Management

Best practices for long-running user sessions:

### Session Lifecycle
1. **Creation:** After successful logins or OAuth callback
2. **Validation:** On each request; check freshness, revocation status
3. **Extension:** Update last-activity timestamp on use; extend timeout
4. **Termination:** On logout, account deletion, or inactivity timeout

### Timeout Strategies
- **Inactivity timeout:** 30 minutes of no requests → logout
- **Absolute timeout:** 8 hours max, regardless of activity → force re-login
- **Sliding window:** Each request resets timer
- **Remember-me:** Extend timeout if user checks "remember me" during login

### Concurrent Session Limits
- Limit users to N concurrent sessions (e.g., 5 devices)
- New login from device N+1 invalidates oldest session
- Prevents credential sharing; improves security

### Suspicious Activity Detection
- Multiple logins from different geographies within short time → fraud
- Login from new device/location → challenge or MFA
- Password changed from new location → re-login all sessions
- Implementation: store (user_id, IP, User-Agent, location); check against history

---

## Mental Model: Authentication vs. Authorization

| Aspect | Authentication | Authorization |
|--------|----------------|---------─────|
| **Definition** | Verify identity: "Who are you?" | Grant access: "What can you do?" |
| **Tools** | Session, JWT, OAuth, passkeys | Roles, permissions, scopes |
| **Example** | Login verified user | User has "admin" role |
| **Revocation** | Logout destroys session | Role removed; scope unchanged |

---

## Comparison: When to Use Which

| Pattern | Use When | Pros | Cons |
|---------|----------|------|------|
| **Sessions** | Single-domain web apps | Simple, revocable, CSRF-resistant | Doesn't scale, not mobile-friendly |
| **JWT** | APIs, SPAs, multi-domain | Stateless, scalable, mobile-friendly | Can't revoke immediately |
| **OAuth 2.0** | Delegated access, social login | No password shared, user-controlled | Complex flow, third-party dependency |
| **Passkeys** | Future-proof, passwordless | Phishing-resistant, no password risk | New, browser support varies |
| **MFA** | High-security systems | Multiple factors prevent compromise | User friction, recovery complexity |

---

## Common Vulnerabilities

| Vulnerability | Cause | Mitigation |
|----------------|-------|-----------|
| **Credential stuffing** | Reused passwords | Require strong passwords, MFA, breach monitoring |
| **CSRF** | Forged cross-site request | SameSite cookies, CSRF tokens, origin validation |
| **XSS → token theft** | JavaScript accesses token | HttpOnly cookies, CSP, input validation |
| **Weak JWT algorithm** | Allows unsigned tokens | Always validate algorithm, no "alg: none" |
| **Session fixation** | Attacker sets user's session ID | Regenerate session ID on login |
| **Unencrypted channels** | HTTP instead of HTTPS | Enforce HTTPS, Secure flag on cookies |
| **Long token expiry** | Tokens valid for months | Use short expiry (5-15 min), refresh tokens |

---

## See Also
- [security-oauth2-oidc.md](security-oauth2-oidc.md) — Detailed OAuth 2.0 and OpenID Connect
- [security-owasp-auth.md](security-owasp-auth.md) — Authentication failure modes
- [security-web-application.md](security-web-application.md) — HTTP header security, CSRF, XSS
- [networking-http.md](networking-http.md) — HTTP mechanics, cookies, headers