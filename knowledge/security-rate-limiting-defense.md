# Security — Rate Limiting for Defense: Brute Force, Credential Stuffing, DDoS, and WAF

## Overview

Rate limiting is a defensive layer that restricts request volume from a client, IP address, or API key to prevent resource exhaustion and credential attacks. It operates at multiple network layers (L3/L4/L7) with different purposes: API rate limiting prevents abuse, DDoS mitigation absorbs volumetric attacks, and authentication rate limiting stops brute force and credential stuffing. This note covers rate limiting strategies, implementation patterns, and limitations.

---

## Rate Limiting Layers and Purposes

### Layer 7 (Application): API & Authentication Rate Limiting

Enforced by the application server on HTTP requests. Typically per-user, per-IP, or per-API-key.

**Purpose:**
- Prevent brute force attacks (auth endpoints)
- Stop credential stuffing (login endpoints)
- Throttle high-volume API clients
- Protect against application-layer abuse (search spam, data scraping)

**Advantage:** Context-aware (knows user, endpoint, API key)
**Disadvantage:** Requires application logic; first to be overwhelmed in DDoS

### Layer 4 (Transport): Connection Rate Limiting

TCP-level throttling; resets connections or drops packets from clients sending too many requests in short time.

**Purpose:**
- Slow down TCP connection floods
- Mitigate some forms of DDoS
- Protect TCP stack from exhaustion

**Advantage:** Works before application logic; lightweight
**Disadvantage:** Less context; can't distinguish legitimate vs. malicious traffic patterns

### Layer 3 (Network): IP-Level Throttling and IGP Flow Specification

BGP flow spec and firewalls drop packets from sources exceeding bandwidth or packet rates.

**Purpose:**
- Absorb volumetric DDoS attacks
- Prevent IP spoofing in amplification attacks
- Network-wide traffic shaping

**Advantage:** Protects the entire infrastructure; deployed at ISP level
**Disadvantage:** Coarse-grained; can't distinguish APIs or users

---

## Brute Force and Credential Stuffing: L7 Defense

### Brute Force Attacks

Attackers systematically try many passwords against a known username or email:

```
POST /auth/login
Body: { "email": "user@example.com", "password": "password1" }

POST /auth/login
Body: { "email": "user@example.com", "password": "password2" }

POST /auth/login
Body: { "email": "user@example.com", "password": "password3" }
... (1000x attempts per second)
```

With no rate limiting, attackers can try thousands of passwords in seconds.

### Credential Stuffing

Attackers use leaked credentials (from previous breaches) to attempt account takeover across many sites:

```
POST /auth/login with [email from LinkedIn breach, password1]
POST /auth/login with [email from LinkedIn breach, password2]
... (different people, same email/password pairs across services)
```

### L7 Rate Limiting Defense

**Per-email rate limit:**
Limit login attempts to 5 failures per email address per 15 minutes.

```javascript
const MAX_ATTEMPTS = 5;
const LOCKOUT_WINDOW = 15 * 60 * 1000;  // 15 minutes

async function handleLogin(email, password) {
  const attempts = await getFailedAttempts(email);
  
  if (attempts.count >= MAX_ATTEMPTS) {
    if (Date.now() - attempts.firstAttempt < LOCKOUT_WINDOW) {
      return { error: 'Too many failed attempts. Try again later.' };
    } else {
      await clearFailedAttempts(email);
    }
  }
  
  const user = await authenticateUser(email, password);
  if (!user) {
    await recordFailedAttempt(email);
    return { error: 'Invalid email or password' };
  }
  
  await clearFailedAttempts(email);
  return { token: issueToken(user) };
}
```

**Per-IP rate limit:**
Limit login attempts to 10 per IP per minute (catches bulk attacks).

**Per-user device fingerprint:**
Track based on combination of IP, user-agent, device ID to detect credential stuffing on previously seen accounts.

### Adaptive Rate Limiting

Tighten limits based on risk signals:

```javascript
// Risk scoring
let riskScore = 0;
if (newIP && user.neverLoggedIn) riskScore += 3;
if (request.headers['user-agent'] !== user.lastUserAgent) riskScore += 2;
if (isLeakedPassword(password)) riskScore += 5;

// Adjust limits
if (riskScore > 7) {
  MAX_ATTEMPTS = 3;  // Stricter
  LOCKOUT_WINDOW = 60 * 60 * 1000;  // 1 hour
}
```

---

## DDoS Protection Layers: L3, L4, and L7

### Volumetric Attacks (L3/L4)

Attackers flood the target with massive traffic volume, exhausting bandwidth and network equipment.

**Common vectors:**
- **UDP floods**: Millions of UDP packets to random ports
- **DNS amplification**: Spoofed requests to open DNS resolvers, responses flood the target
- **NTP reflection**: Abuse NTP servers to amplify attack traffic
- **ICMP floods**: Ping floods (less common; ICMP is often rate-limited)

### L3/L4 Mitigation: ISP & CDN DDoS Services

**AWS Shield Standard / Cloudflare Free Tier:**
- Automatic protection against common DDoS attacks (UDP floods, DNS amplification)
- Deployed at edge; absorbs traffic before it reaches origin

**AWS Shield Advanced / Cloudflare Pro:**
- Paid DDoS protection; AI-driven detection
- Mitigation SLAs and incident response

**Implementation:**
Route traffic through a DDoS protection service (Cloudflare, AWS Shield, Akamai):

```
             Normal Request
    Client ----------> CDN Edge ----------> Origin
                     (Filter)

             DDoS Attack (millions of packets)
    Attacker ------> CDN Edge (Absorb) -----X
                          |
                    (Drop/Rate-limit)
```

### Protocol Attacks (L4)

**SYN floods**: Attacker sends millions of TCP SYN packets without completing handshake.

```
SYN ---> Server (half-open connection)
         Server allocates memory, waits for ACK
SYN ---> Server (half-open connection)
... (server runs out of connection slots)
```

**Mitigation:**
- TCP SYN cookies: Encode connection state in the SYN-ACK sequence number; don't store half-open connection state
- Connection rate limiting: Drop SYN packets from sources exceeding threshold
- Load balancer SYN proxy: Terminate connections on behalf of the origin

### Application-Layer Attacks (L7)

Attackers send legitimate-looking HTTP requests that exploit application logic.

**HTTP floods:**
```
GET /api/expensive-query
GET /api/expensive-query
GET /api/expensive-query
... (millions per second)
```

Each request is syntactically valid but computationally expensive (database query, machine learning inference, etc.).

**Slowloris attacks:**
```
GET /api HTTP/1.1
Host: example.com
Connection: Keep-Alive

[Send one header every 30 seconds, never complete request]
```

Keeps connections open, exhausting the server's connection pool.

### L7 Mitigation: WAF and Behavioral Detection

**Web Application Firewall (WAF):**
- Sits between attacker and origin; analyzes HTTP traffic
- Blocking rules: Match signatures (SQL injection, path traversal, etc.)
- Rate limiting rules: Block IPs/users after threshold
- Behavioral rules: Detect unusual patterns (many 404s, slow requests, bot user-agents)

**WAF Rules:**

```
Rule 1: Block if >50 requests/min from single IP
Rule 2: Block if >20 failed login attempts from single IP/hour
Rule 3: Block if request contains SQL injection patterns (or'; DROP TABLE)
Rule 4: Block if user-agent is known bot, except Googlebot
Rule 5: Block if >10 requests to same endpoint with different payloads (scanning)
```

**Behavioral detection:**
- Anomaly-based: Flag unusual traffic patterns compared to baseline
- Machine learning: Train models on known DDoS traffic; classify new traffic
- Fingerprinting: Detect and block known attack IPs

**Challenge phases:**
Some WAFs challenge suspicious requests before allowing:
- CAPTCHA (proves human)
- JavaScript check (proves browser with JS enabled)
- Cell phone verification (proves account holder)

---

## Bot Detection and Mitigation

### Bot Traffic Identification

**Signature-based:**
- User-agent matches known bots (check against list of legitimate bot user-agents)
- Rapid requests; no browser behavior (no GPU rendering time, no CSS parsing time)
- Requests missing browser headers (`Sec-Fetch-*`, `Accept-Language`)

**Behavioral-based:**
- No mouse movements; instant form submissions
- Requests to non-existent resources (404s) in sequence (scanning)
- No cookies; no cache hits (fresh requests every time)

**Device fingerprinting:**
- Consistent device ID across multiple requests
- Bot cannot duplicate authentic browser fingerprint (hardware ID, device model, screen resolution, GPU capabilities)

### Bot Mitigation Strategies

**1. CAPTCHA**
Challenges that humans pass but bots fail. Traditional image-based CAPTCHAs are fragile (broken by OCR and AI); modern CAPTCHAs (reCAPTCHA v3) use behavioral scoring.

```javascript
// User attempts sensitive action (login, password change)
if (riskScore > THRESHOLD) {
  // Challenge user with CAPTCHA
  return { captcha_required: true, captcha_token: generateChallenge() };
}
```

**2. JavaScript Execution**
Require JavaScript execution; bots running in headless mode may fail. Lightweight mitigation but easily bypassed.

**3. Rate Limiting**
Enforce strict rate limits on sensitive endpoints:

```javascript
// Per-IP rate limit
const KEY = `rate_limit:${request.ip}`;
const count = await redis.incr(KEY);
await redis.expire(KEY, 60);  // 1 minute window

if (count > 60) {  // 60 requests per minute
  return { error: 'Rate limited', status: 429 };
}
```

**4. Device Fingerprinting (Cloudflare Bot Management, AWS WAF)**
Generate a fingerprint of the client device and verify it's consistent across requests. Real browsers have consistent fingerprints; bots often fail.

**5. Behavioral Analysis**
Track and score suspicious behaviors; combine signals for risk scoring:

```javascript
const riskScore = 
  (isFirstTimeIP ? 2 : 0) +
  (noBrowserHeaders ? 3 : 0) +
  (rapidRequests ? 2 : 0) +
  (404_scanning ? 5 : 0) +
  (leakedCredential ? 3 : 0);

if (riskScore > 8) {
  // Challenge or block
}
```

---

## Rate Limiting Implementation Patterns

### Token Bucket Algorithm

The classic algorithm. Allows burst traffic while maintaining average rate.

- **Capacity**: Bucket holds N tokens (burst size)
- **Refill rate**: Add R tokens per second
- **Per request**: Remove 1 token; reject if bucket empty

```javascript
async function tokenBucket(userId) {
  const CAPACITY = 100;      // Burst capacity
  const REFILL_RATE = 10;    // Tokens/second
  
  const key = `bucket:${userId}`;
  const bucket = await redis.get(key) || CAPACITY;
  
  if (bucket <= 0) {
    return { allowed: false };
  }
  
  await redis.set(key, bucket - 1);
  await redis.expire(key, Math.ceil(1 / REFILL_RATE));
  return { allowed: true };
}
```

**Advantage:** Allows burst; smooth over time
**Disadvantage:** Slightly complex; requires state

### Sliding Window Log

Track request timestamps in a window; reject if count exceeds limit.

```javascript
async function slidingWindow(userId) {
  const WINDOW = 60;        // 60 seconds
  const LIMIT = 100;        // 100 requests
  
  const key = `window:${userId}`;
  const now = Date.now();
  const windowStart = now - WINDOW * 1000;
  
  // Remove old requests outside window
  await redis.zremrangebyscore(key, '-inf', windowStart);
  
  // Count requests in window
  const count = await redis.zcard(key);
  if (count >= LIMIT) {
    return { allowed: false };
  }
  
  // Add current request
  await redis.zadd(key, now, `${now}:${Math.random()}`);
  await redis.expire(key, WINDOW);
  return { allowed: true };
}
```

**Advantage:** Precise; no burst allowance; clean semantics
**Disadvantage:** Memory-intensive (stores all timestamps)

### Fixed Window

Simple: counter resets every period. Allows abuse at window boundaries.

```javascript
async function fixedWindow(userId) {
  const WINDOW = 60;  // seconds
  const LIMIT = 100;
  
  const key = `window:${userId}:${Math.floor(Date.now() / 1000 / WINDOW)}`;
  const count = await redis.incr(key);
  await redis.expire(key, WINDOW);
  
  return { allowed: count <= LIMIT };
}
```

**Advantage:** Very simple; minimal memory
**Disadvantage:** Burst at boundary; not suitable for strict SLAs

---

## Rate Limiting Limitations

### Distributed Attacks (Botnet)

If attackers use a botnet (thousands of compromised machines), each IP stays under the rate limit:

```
IP_1: 5 requests/min (under limit)
IP_2: 5 requests/min (under limit)
IP_3000: 5 requests/min (under limit)
Total: 15,000 requests/min (overwhelms service)
```

**Mitigation:** Use behavioral detection and fingerprinting; block scrapers and bots at L7; use DDoS services at L3/L4.

### Legitimate User Overlap

Users on the same corporate network (NAT) share an IP. Per-IP rate limits may block legitimate users:

```
Company A (1000 employees, shared NAT IP)
Employee 1: GET /api
Employee 2: GET /api
... (Each is legitimate, but combined they hit rate limit)
```

**Mitigation:** Combine IP + authentication; adjust limits; allow higher burst for authenticated users.

### Cache Bypass

Attackers craft unique requests to bypass cache, forcing origin processing:

```
GET /api?v=1
GET /api?v=2
GET /api?v=3
... (Different query params, same underlying data)
```

**Mitigation:** Normalize requests before rate limit check; cache based on semantic equivalence.

---

## Defense-in-Depth Example

Combine multiple layers for robust protection:

```
Layer 3: ISP/CDN DDoS protection (Cloudflare, Shield)
         ↓
Layer 4: WAF rate limiting (100 req/min per IP)
         ↓
Layer 7: 
  - Per-user API rate limit (1000 req/min)
  - Per-email login rate limit (5 failures/15 min)
  - Bot detection (fingerprint + behavioral scoring)
  - Adaptive rate limiting (tighten on high risk)
```

Every layer catches a different attack vector.

---

## See Also

- security-api.md (API authentication and protection)
- patterns-rate-limiting.md (rate limiting algorithms and implementation)
- security-network.md (firewalls, IDS/IPS, network defense)
- infrastructure-api-gateway-patterns.md (API gateway rate limiting)