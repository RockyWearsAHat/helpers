# Web Browser Security — Same-Origin Policy, CORS, CSP, XSS Prevention & Sandboxing

## Overview

Browser security is enforced at multiple layers: the same-origin policy isolates sites from each other, Content Security Policy restricts what code can do, and sandboxing confines plugins and frames. Understanding these mechanisms is essential for building secure web applications and preventing attacks like XSS, CSRF, and data exfiltration.

---

## Same-Origin Policy (SOP): The Foundation

### Definition
**Same-origin** means protocol, domain, and port are identical. The browser blocks cross-origin requests by default.

```javascript
// https://example.com:443/page.html

// Same-origin (allowed)
fetch('https://example.com/api/users')          // same protocol, domain, port

// Different origins (blocked by SOP)
fetch('http://example.com/api/users')           // different protocol
fetch('https://example.com:8080/api/users')     // different port
fetch('https://api.example.com/api/users')      // different subdomain
fetch('https://different.com/api/users')        // different domain
```

### What SOP Protects

**Blocks cross-origin access to:**
- XMLHttpRequest/fetch responses (except CORS whitelisted)
- Cookies (except if same-domain)
- Local storage (per-origin sandbox)
- IndexedDB (per-origin sandbox)
- Form submission (allowed by default for historical reasons)

**Allows cross-origin:**
- `<script src="">` loading (JSONP workaround, now obsolete)
- `<img src="">` loading (image accessible, canvas tainted if cross-origin)
- `<style>` loading
- `<iframe>` embedding (but can't access content)
- Form submissions (to any origin)

### SOP Limitations

**Not a complete security boundary:**
- Plugins (Flash, Java) often ignore SOP
- Framing attacks possible (clickjacking via overlaid iframes)
- DNS rebinding can confuse origin (rare, but possible)

---

## CORS: Cross-Origin Resource Sharing

SOP blocks all cross-origin requests. CORS allows selective cross-origin access.

### How It Works

**Client makes request:**
```http
GET /api/data HTTP/1.1
Origin: https://example.com
```

**Server responds with CORS headers:**
```http
HTTP/1.1 200 OK
Access-Control-Allow-Origin: https://example.com
Access-Control-Allow-Methods: GET, POST
Access-Control-Allow-Headers: Content-Type, Authorization
Access-Control-Allow-Credentials: true
```

**Browser checks response headers:**
- If `Access-Control-Allow-Origin` includes requester origin → allow response to JavaScript
- Otherwise → block (still received, but not accessible)

### Preflight Requests (Automatic)

Some requests trigger a **preflight** (OPTIONS request) sent first:

```http
OPTIONS /api/data HTTP/1.1
Origin: https://example.com
Access-Control-Request-Method: POST
Access-Control-Request-Headers: Content-Type
```

Server responds with allowed methods and headers:
```http
Access-Control-Allow-Methods: GET, POST, PUT
Access-Control-Allow-Headers: Content-Type
Access-Control-Max-Age: 86400
```

If preflight approved, browser sends actual request.

**Triggers preflight:**
- Methods: PUT, DELETE, PATCH, or non-standard
- Headers: Authorization, X-Custom-Header, anything not simple
- Content-Type: application/json (non-form)

**Simple requests (no preflight):**
- Methods: GET, POST, HEAD
- Headers: Accept, Content-Type, Content-Language, etc.
- Content-Type: application/x-www-form-urlencoded, multipart/form-data, text/plain

### CORS Pitfalls

**`Access-Control-Allow-Origin: *` (wildcard)**
- Allows any origin; correct for public APIs
- But if `Access-Control-Allow-Credentials: true` also set → error (must specify exact origin)
- Doesn't leak credentials; credentials only sent if explicitly requested

**Credentials in CORS:**
```javascript
// Client must explicitly request credentials
fetch('https://api.example.com/data', {
  credentials: 'include'  // send cookies
})
```

```http
// Server allows credentials
Access-Control-Allow-Credentials: true
Access-Control-Allow-Origin: https://example.com  // exact origin, not *
```

**Private network access (new):**
- Websites accessing internal/private IPs (10.0.0.0/8, 192.168.0.0/16) now blocked
- Requires explicit preflight + new CORS header `Access-Control-Request-Private-Network: true`
- Prevents compromised website from scanning internal infrastructure

---

## Content Security Policy (CSP)

Set allowed sources for resources (scripts, styles, images, fonts) and disable unsafe inline code. Primary defense against XSS.

### Directives

**default-src**
```
Content-Security-Policy: default-src 'self'
```
- Fallback for all resource types not explicitly listed
- `'self'`: only same-origin
- `'none'`: block all
- `example.com`: specific domain

**script-src (Most Important)**
```
Content-Security-Policy: script-src 'self' https://trusted-cdn.com
```
- Controls where JavaScript can be loaded from
- Blocks inline `<script>` tags (prevent XSS if injection occurs)
- Blocks `eval()` and dynamic script generation

**style-src**
```
Content-Security-Policy: style-src 'self' https://fonts.googleapis.com
```
- CSS sources; blocks inline `<style>` by default

**img-src, font-src, media-src, frame-src**
- Control images, fonts, audio/video, nested frames

**connect-src**
```
Content-Security-Policy: connect-src 'self' https://api.example.com
```
- Allowed sources for fetch, WebSocket, beacon

**object-src**
```
Content-Security-Policy: object-src 'none'
```
- Disable `<object>`, `<embed>`, `<applet>` (plugins)

### Nonces: Inline Script Whitelisting

Allow specific inline scripts without using `unsafe-inline`:

```html
<script nonce="random-unique-value">
  console.log("trusted code");
</script>
```

```
Content-Security-Policy: script-src 'self' 'nonce-random-unique-value'
```

Server generates new nonce for every response; includes in CSP and script tag. Attacker can't guess nonce; injected scripts blocked.

**In practice (Express):**
```javascript
app.use((req, res, next) => {
  const nonce = crypto.randomBytes(16).toString('hex');
  res.locals.nonce = nonce;
  res.set('Content-Security-Policy', `script-src 'self' 'nonce-${nonce}'`);
  next();
});
```

### Hashes: Static Inline Script Approval

For static inline scripts, include SHA256 hash in CSP:

```html
<script>
  console.log("This is inline");
</script>
```

```
Content-Security-Policy: script-src 'self' 'sha256-hash-of-script-content'
```

Hash must match exact script content (whitespace sensitive). Changes = hash mismatch = blocked.

### Strict CSP (Best Practice)

Strict CSP restricts inline scripts heavily:

```
Content-Security-Policy: 
  default-src 'self'; 
  script-src 'self' 'nonce-random' https://trusted.com; 
  object-src 'none'; 
  base-uri 'self'; 
  frame-ancestors 'none'
```

- No nonce/hash inline scripts = blocked
- Prevents most XSS even if content injection occurs
- Requires dynamic nonce generation (better than `unsafe-inline`)

### CSP Reporting

Monitor CSP violations:

```
Content-Security-Policy-Report-Only: 
  default-src 'self'; 
  report-uri https://example.com/csp-report
```

Browser sends violations to report endpoint (not blocked, just reported). Useful for testing before enforcement.

---

## XSS Prevention via DOM APIs

Even with CSP, avoid creating XSS vulnerability in DOM manipulation.

### Safe vs. Unsafe APIs

**Unsafe (don't use):**
```javascript
// Creates new HTML; susceptible to injection
element.innerHTML = userInput;  // if userInput = "<img src=x onerror=alert(1)>", XSS

// Parses and evaluates as code
eval(userInput);
new Function(userInput)();
setTimeout(userInput, 1000);  // if userInput contains code
```

**Safe:**
```javascript
// Creates text node; no HTML parsing
element.textContent = userInput;

// Explicitly create elements; no parsing
const img = document.createElement('img');
img.src = userInput;  // URL, not code
element.appendChild(img);

// Use DOM API for attributes
element.setAttribute('data-value', userInput);

// template.innerHTML safe if content is static, then concatenate
const template = `<div>${css.escape(userInput)}</div>`;
```

### Context-Specific Escaping
Input must be escaped for context:

- **HTML context:** `<p>${html(input)}</p>` → escape `<`, `>`, `&`, `"`, `'`
- **JavaScript context:** inline script → use nonce instead
- **URL context:** `href="${url(input)}"` → validate URL scheme, encode special chars
- **CSS context:** `style="color: ${css(input)}"` → escape, validate

Example (underscore.js style):
```javascript
const escape = (s) => s
  .replace(/&/g, '&amp;')
  .replace(/</g, '&lt;')
  .replace(/>/g, '&gt;')
  .replace(/"/g, '&quot;')
  .replace(/'/g, '&#x27;');
```

---

## Trusted Types: Runtime XSS Prevention

API that enforces safe handling of DOM sinks (innerHTML, eval, etc.).

```javascript
// Create policy
const policy = trustedTypes.createPolicy('my-policy', {
  createHTML(input) {
    // Validate/sanitize
    if (!input.includes('script')) return input;
    throw new Error('Invalid');
  }
});

// Use policy
const safe = policy.createHTML(userInput);
element.innerHTML = safe;  // accepted (Trusted type)

// Direct assignment rejected
element.innerHTML = userInput;  // TypeError (not Trusted type)
```

**With CSP:**
```
Content-Security-Policy: require-trusted-types-for 'script'
```

Requires all DOM mutations use trusted types. Browser enforces at runtime; uncaught errors if trusted types not used.

---

## Subresource Integrity (SRI)

Verify that files loaded from CDN haven't been modified (hash verification).

```html
<script src="https://cdn.example.com/app.js" 
        integrity="sha384-abc123...">
</script>
```

If hash doesn't match, browser rejects the script. Protects against:
- Compromised CDN
- Man-in-the-middle modification
- Supply chain attacks

**In practice:**
```bash
# Generate SRI hash
$ openssl dgst -sha384 -binary app.js | base64
abc123...

# Use in HTML
<script src="..." integrity="sha384-abc123..."></script>
```

---

## iframe Sandboxing

Restrict capabilities of embedded frames:

```html
<iframe src="https://untrusted.com" 
        sandbox="allow-scripts allow-forms">
</iframe>
```

**Sandbox attributes:**
- No flags: frame has no capabilities (can't run JS, submit forms, etc.)
- `allow-scripts`: allow JavaScript
- `allow-same-origin`: allow same-origin requests (default: cross-origin)
- `allow-forms`: allow form submission
- `allow-popups`: allow popup windows
- `allow-pointer-lock`: mouse lock API
- `allow-top-navigation`: allow navigation to top window

**Use case:**
```html
<!-- Embed user-generated content safely -->
<iframe src="user-content.html" sandbox="allow-scripts"></iframe>
```
- Frame can run scripts
- Can't access parent window
- Can't make same-origin requests (cross-origin by default)
- Can't open popups or navigate parent

---

## Feature-Policy / Permissions-Policy

Control browser features at HTTP header level.

```
Permissions-Policy: camera=(), microphone=(self "https://example.com"), geolocation=(*)
```

- `camera=()`: disable camera (no page, no iframes can use)
- `microphone=(self)`: only this page (not embedded iframes)
- `geolocation=(*)`: allow all (default permissive)

**Use cases:**
- Disable mobile sensors if not needed
- Prevent third-party scripts from using microphone/camera
- Restrict payment API, VR APIs, etc.

---

## Cookie Security Attributes (Redux)

Covered in authentication patterns; key points for security:

```
Set-Cookie: sessionid=xyz; 
  Path=/; 
  Secure;           // HTTPS only
  HttpOnly;         // JS can't access
  SameSite=Strict;  // no cross-site
  Max-Age=3600
```

- **Secure:** Transmit only on HTTPS
- **HttpOnly:** Block JavaScript access (prevent XSS theft)
- **SameSite=Strict:** No cross-site requests (prevent CSRF)

---

## Storage Partitioning (Emerging)

Browsers now partition storage by top-level site (reducing tracking):

- `localStorage` now partitioned: `example.com` in iframe sees different storage than `example.com` directly
- Cross-site cookies require cookie attributes (SameSite)
- Similar to iframe sandbox, but for storage

Impact: third-party tracking diminished; same-origin storage unchanged.

---

## Clickjacking Prevention

Attacker overlays invisible frame on victim site; user clicks through to malicious action.

### X-Frame-Options Header
```
X-Frame-Options: DENY
```
- `DENY`: never allow framing
- `SAMEORIGIN`: allow framing by same origin
- `ALLOW-FROM uri`: allow framing by specific origin (deprecated; use CSP instead)

### CSP Frame Ancestors
```
Content-Security-Policy: frame-ancestors 'none'
```
- `'none'`: never allow framing
- `'self'`: allow same-origin framing
- Preferred over X-Frame-Options (more flexible)

---

## Mental Model: Defense Layers

```
┌─────────────────────────────────────────┐
│ Network (HTTPS, TLS)                    │  ← Transport security
├─────────────────────────────────────────┤
│ Origin & CORS                           │  ← Isolation between sites
├─────────────────────────────────────────┤
│ CSP (script-src, nonce)                 │  ← Restrict code sources
├─────────────────────────────────────────┤
│ DOM API Safe Usage + Escaping           │  ← Sanitize at encode
├─────────────────────────────────────────┤
│ Trusted Types (runtime validation)      │  ← Final enforcement
├─────────────────────────────────────────┤
│ Sandbox (iframe, plugin disable)        │  ← Containment
```

---

## Common Vulnerability Patterns

| Attack | SOP | CORS | CSP | Sandboxing | Mitigation |
|--------|-----|------|-----|-----------|-----------|
| **XSS (inject script)** | No | No | **Yes** (nonce) | No | CSP + escaping |
| **CSRF (forge request)** | Partial | **Yes** | No | No | SameSite cookies, CSRF tokens |
| **Clickjacking** | No | No | No | **Yes** (frame-ancestors) | CSP frame-ancestors, X-Frame-Options |
| **Data exfiltration** | **Yes** | **Yes** (whitelist) | **Yes** (connect-src) | No | Restrict connect sources |
| **Compromised CDN** | No | No | No | No | **SRI** hash verification |
| **Ambient authority (cookies)** | **Yes** (SOP) | **Yes** (CORS) | No | No | SameSite=Strict |

---

## See Also
- [security-owasp-xss.md](security-owasp-xss.md) — XSS attack types and defenses
- [security-web-application.md](security-web-application.md) — HTTP security headers, overall defense
- [web-authentication-patterns.md](web-authentication-patterns.md) — Cookie security in auth
- [api-design.md](api-design.md) — API security considerations
- [networking-http.md](networking-http.md) — HTTP protocol and headers