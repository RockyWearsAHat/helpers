# Web Application Security Checklist

## Overview

Web application security requires defense-in-depth across HTTP headers, cookie configuration, input handling, authentication, and API design. This reference covers the practical configuration and implementation of browser-enforced and server-enforced security controls.

## CORS (Cross-Origin Resource Sharing)

### How CORS Works

```
Browser (origin: app.example.com) → API (api.example.com)

1. Simple request (GET/POST with simple headers):
   Browser sends request with Origin header
   Server responds with Access-Control-Allow-Origin

2. Preflight request (PUT/DELETE/custom headers):
   Browser sends OPTIONS with:
     Origin: https://app.example.com
     Access-Control-Request-Method: PUT
     Access-Control-Request-Headers: X-Custom-Header
   Server responds with:
     Access-Control-Allow-Origin: https://app.example.com
     Access-Control-Allow-Methods: GET, POST, PUT
     Access-Control-Allow-Headers: X-Custom-Header
     Access-Control-Max-Age: 86400
   Browser sends actual request
```

### Response Headers

| Header                             | Purpose                            | Example                       |
| ---------------------------------- | ---------------------------------- | ----------------------------- |
| `Access-Control-Allow-Origin`      | Allowed origin(s)                  | `https://app.example.com`     |
| `Access-Control-Allow-Methods`     | Allowed HTTP methods               | `GET, POST, PUT, DELETE`      |
| `Access-Control-Allow-Headers`     | Allowed request headers            | `Content-Type, Authorization` |
| `Access-Control-Expose-Headers`    | Headers readable by JS             | `X-Request-Id, X-Total-Count` |
| `Access-Control-Max-Age`           | Preflight cache duration (seconds) | `86400`                       |
| `Access-Control-Allow-Credentials` | Allow cookies/auth                 | `true`                        |

### CORS Security Rules

```
CRITICAL: Never use Access-Control-Allow-Origin: *
          with Access-Control-Allow-Credentials: true
          (browsers block this combination)

DANGEROUS: Reflecting the Origin header without validation
           → Allows any origin to access your API with credentials

SAFE: Explicit allowlist of origins
```

### Express.js CORS Configuration

```javascript
const cors = require("cors");

const allowedOrigins = [
  "https://app.example.com",
  "https://staging.example.com",
];

app.use(
  cors({
    origin: (origin, callback) => {
      // Allow requests with no origin (mobile apps, server-to-server)
      if (!origin) return callback(null, true);
      if (allowedOrigins.includes(origin)) {
        return callback(null, true);
      }
      return callback(new Error("Not allowed by CORS"));
    },
    credentials: true,
    methods: ["GET", "POST", "PUT", "DELETE"],
    allowedHeaders: ["Content-Type", "Authorization"],
    maxAge: 86400,
  }),
);
```

## CSRF Protection

### Attack Mechanism

```
1. User logs into bank.com (session cookie set)
2. User visits evil.com
3. evil.com contains: <form action="https://bank.com/transfer" method="POST">
4. Form auto-submits with user's bank.com cookies
5. Bank processes transfer (valid session cookie present)
```

### Synchronizer Token Pattern

```javascript
// Server: Generate CSRF token per session
const crypto = require("crypto");

function generateCsrfToken(session) {
  const token = crypto.randomBytes(32).toString("hex");
  session.csrfToken = token;
  return token;
}

// Middleware: Validate on state-changing requests
function csrfProtection(req, res, next) {
  if (["GET", "HEAD", "OPTIONS"].includes(req.method)) {
    return next();
  }
  const token = req.headers["x-csrf-token"] || req.body._csrf;
  if (!token || token !== req.session.csrfToken) {
    return res.status(403).json({ error: "Invalid CSRF token" });
  }
  next();
}
```

### Double Submit Cookie

```javascript
// Set CSRF token as cookie AND expect it in header
// Attacker can't read the cookie value cross-origin

// Server sets cookie:
res.cookie("csrf-token", token, {
  httpOnly: false, // JS must read it
  sameSite: "strict",
  secure: true,
});

// Client reads cookie and sends as header:
fetch("/api/transfer", {
  method: "POST",
  headers: {
    "X-CSRF-Token": getCookie("csrf-token"),
  },
  credentials: "include",
});
```

### SameSite Cookie Defense

```
Set-Cookie: session=abc; SameSite=Lax; Secure; HttpOnly

SameSite=Strict  → Cookie never sent cross-site (breaks OAuth redirects)
SameSite=Lax     → Cookie sent on top-level navigations (GET only)
SameSite=None    → Cookie always sent (MUST have Secure flag)
```

`SameSite=Lax` is the default in modern browsers and provides baseline CSRF protection for non-GET requests.

## Cookie Security

### Secure Cookie Attributes

```
Set-Cookie: session=abc123;
  Secure;           # HTTPS only
  HttpOnly;         # Not accessible via JavaScript
  SameSite=Lax;     # CSRF protection
  Path=/;           # Cookie scope
  Domain=.example.com;  # Include subdomains
  Max-Age=3600;     # Expiry in seconds
```

### Cookie Prefixes

| Prefix      | Requirements                                         | Purpose                            |
| ----------- | ---------------------------------------------------- | ---------------------------------- |
| `__Host-`   | Must have `Secure`, must NOT have `Domain`, `Path=/` | Strongest — locked to exact origin |
| `__Secure-` | Must have `Secure`                                   | Ensures HTTPS only                 |

```
# Recommended session cookie
Set-Cookie: __Host-session=abc123; Secure; HttpOnly; SameSite=Lax; Path=/

# Why __Host-: prevents subdomain attacks, cookie tossing,
# and ensures the cookie is only for the exact host
```

### Cookie Security Checklist

- [ ] `Secure` flag on ALL cookies (HTTPS only)
- [ ] `HttpOnly` on session/auth cookies (prevent XSS theft)
- [ ] `SameSite=Lax` minimum (CSRF baseline)
- [ ] `__Host-` prefix for session cookies
- [ ] Reasonable `Max-Age` (not years)
- [ ] No sensitive data in cookie values
- [ ] Cookie size minimized (ID only, not data)

## Clickjacking Protection

### Attack

Attacker embeds your site in a transparent iframe, overlaying it with deceptive UI so users click your site's buttons unknowingly.

### X-Frame-Options

```
X-Frame-Options: DENY             # Never frameable
X-Frame-Options: SAMEORIGIN       # Only same-origin framing
```

### Content-Security-Policy frame-ancestors (Preferred)

```
# No framing at all
Content-Security-Policy: frame-ancestors 'none';

# Same origin only
Content-Security-Policy: frame-ancestors 'self';

# Specific origins
Content-Security-Policy: frame-ancestors 'self' https://trusted.example.com;
```

`frame-ancestors` supersedes `X-Frame-Options` and supports multiple origins. Set both for backward compatibility.

### JavaScript Frame-Busting (Defense in Depth)

```html
<style>
  body {
    display: none;
  }
</style>
<script>
  if (self === top) {
    document.body.style.display = "block";
  } else {
    top.location = self.location;
  }
</script>
```

## Security Headers

### Complete Header Set

```
# Content type sniffing prevention
X-Content-Type-Options: nosniff

# Referrer policy (privacy + security)
Referrer-Policy: strict-origin-when-cross-origin

# Permissions policy (disable unused browser features)
Permissions-Policy: camera=(), microphone=(), geolocation=(), payment=()

# HSTS (force HTTPS, include subdomains, preload)
Strict-Transport-Security: max-age=63072000; includeSubDomains; preload

# CSP (XSS prevention — see dedicated section)
Content-Security-Policy: default-src 'self'; script-src 'self'; style-src 'self'

# Frame protection
Content-Security-Policy: frame-ancestors 'none'
X-Frame-Options: DENY

# Cross-origin isolation
Cross-Origin-Opener-Policy: same-origin
Cross-Origin-Embedder-Policy: require-corp
Cross-Origin-Resource-Policy: same-origin
```

### X-Content-Type-Options

```
X-Content-Type-Options: nosniff
```

Prevents browsers from MIME-sniffing responses away from the declared `Content-Type`. Without this, a file served as `text/plain` might be interpreted as JavaScript.

### Referrer-Policy

| Value                             | Behavior                                                |
| --------------------------------- | ------------------------------------------------------- |
| `no-referrer`                     | Never send Referer header                               |
| `same-origin`                     | Send only for same-origin requests                      |
| `strict-origin`                   | Send origin on HTTPS→HTTPS, nothing on HTTPS→HTTP       |
| `strict-origin-when-cross-origin` | Full URL same-origin, origin cross-origin (recommended) |
| `origin`                          | Always send origin only                                 |
| `no-referrer-when-downgrade`      | Full URL except HTTPS→HTTP                              |

### Permissions-Policy

```
# Disable dangerous browser features
Permissions-Policy:
  camera=(),
  microphone=(),
  geolocation=(),
  payment=(),
  usb=(),
  bluetooth=(),
  midi=(),
  magnetometer=(),
  gyroscope=(),
  accelerometer=(),
  autoplay=(self),
  fullscreen=(self)
```

### HSTS

```
Strict-Transport-Security: max-age=63072000; includeSubDomains; preload
```

- `max-age`: Duration browser remembers HTTPS-only (2 years = 63072000)
- `includeSubDomains`: Applies to all subdomains
- `preload`: Submit to browser preload list (hstspreload.org)

**Warning**: Setting `includeSubDomains` breaks any subdomain not serving HTTPS. Plan before enabling.

## Subresource Integrity (SRI)

### Purpose

Verify that CDN-hosted resources haven't been tampered with:

```html
<script
  src="https://cdn.example.com/lib.js"
  integrity="sha384-oqVuAfXRKap7fdgcCY5uykM6+R9GqQ8K/uxy9rx7HNQlGYl1kPzQho1wx4JwY8w"
  crossorigin="anonymous"
></script>

<link
  rel="stylesheet"
  href="https://cdn.example.com/style.css"
  integrity="sha384-abc123..."
  crossorigin="anonymous"
/>
```

### Generating SRI Hashes

```bash
# Generate hash
cat lib.js | openssl dgst -sha384 -binary | openssl base64 -A
# Or
shasum -b -a 384 lib.js | awk '{print $1}' | xxd -r -p | base64

# Multiple hashes for rotation
integrity="sha384-hash1 sha384-hash2"
```

### SRI with CSP

```
Content-Security-Policy: require-sri-for script style
```

Forces SRI for all scripts and stylesheets. Blocks any without integrity attributes.

## Rate Limiting

### Strategies

| Strategy           | Description                | Use Case                  |
| ------------------ | -------------------------- | ------------------------- |
| **Fixed window**   | N requests per time window | Simple API limiting       |
| **Sliding window** | Rolling window count       | Smoother rate enforcement |
| **Token bucket**   | Refilling bucket of tokens | Burst-tolerant APIs       |
| **Leaky bucket**   | Constant drain rate        | Smooth traffic shaping    |

### Implementation (Express.js)

```javascript
const rateLimit = require("express-rate-limit");

// Global rate limit
app.use(
  rateLimit({
    windowMs: 15 * 60 * 1000, // 15 minutes
    max: 100, // 100 requests per window
    standardHeaders: true, // RateLimit-* headers
    legacyHeaders: false,
    keyGenerator: (req) => req.ip,
    handler: (req, res) => {
      res.status(429).json({ error: "Too many requests" });
    },
  }),
);

// Stricter limit for auth endpoints
app.use(
  "/api/auth/",
  rateLimit({
    windowMs: 15 * 60 * 1000,
    max: 5,
    skipSuccessfulRequests: true, // only count failures
  }),
);
```

### Response Headers

```
RateLimit-Limit: 100
RateLimit-Remaining: 42
RateLimit-Reset: 1625000000
Retry-After: 60
```

## Input Validation

### Validation Layers

```
Client-side validation (UX only — never trust)
         │
         ▼
Server-side input validation (schema + type + range)
         │
         ▼
Business logic validation (semantic rules)
         │
         ▼
Data layer validation (constraints, types)
```

### Validation Principles

| Principle                   | Implementation                                       |
| --------------------------- | ---------------------------------------------------- |
| **Allowlist over denylist** | Define what IS allowed, reject everything else       |
| **Validate type**           | Number, string, boolean, enum — strict type checking |
| **Validate range**          | Min/max length, numeric bounds, date ranges          |
| **Validate format**         | Regex for structured data (email, phone, UUID)       |
| **Canonicalize first**      | Decode, normalize, then validate                     |
| **Reject on failure**       | Default deny — if validation fails, reject           |

### Schema Validation (Zod)

```typescript
import { z } from "zod";

const CreateUserSchema = z.object({
  name: z.string().min(1).max(100).trim(),
  email: z.string().email().max(254).toLowerCase(),
  age: z.number().int().min(13).max(150),
  role: z.enum(["user", "admin", "moderator"]),
  bio: z.string().max(1000).optional(),
  tags: z.array(z.string().max(50)).max(10).optional(),
});

// Express middleware
app.post("/users", (req, res) => {
  const result = CreateUserSchema.safeParse(req.body);
  if (!result.success) {
    return res.status(400).json({ errors: result.error.flatten() });
  }
  // result.data is typed and validated
  createUser(result.data);
});
```

## File Upload Security

### Checklist

| Control                   | Implementation                                     |
| ------------------------- | -------------------------------------------------- |
| **File type validation**  | Check magic bytes, not just extension              |
| **Size limit**            | Enforce max file size (server + reverse proxy)     |
| **Filename sanitization** | Strip path traversal, special chars, generate UUID |
| **Storage isolation**     | Store outside web root, use object storage         |
| **No execution**          | Disable script execution in upload directory       |
| **Virus scanning**        | ClamAV or cloud scanning API                       |
| **Content-Disposition**   | Force download for unknown types                   |
| **Separate domain**       | Serve uploads from different origin                |

### Secure Upload Implementation

```javascript
const multer = require("multer");
const crypto = require("crypto");
const path = require("path");

// Magic byte validation
const ALLOWED_TYPES = {
  "image/jpeg": [0xff, 0xd8, 0xff],
  "image/png": [0x89, 0x50, 0x4e, 0x47],
  "application/pdf": [0x25, 0x50, 0x44, 0x46],
};

function validateMagicBytes(buffer, mimetype) {
  const expected = ALLOWED_TYPES[mimetype];
  if (!expected) return false;
  return expected.every((byte, i) => buffer[i] === byte);
}

const upload = multer({
  storage: multer.diskStorage({
    destination: "/secure-uploads/", // outside web root
    filename: (req, file, cb) => {
      const ext = path.extname(file.originalname).toLowerCase();
      const safeName = crypto.randomUUID() + ext;
      cb(null, safeName);
    },
  }),
  limits: { fileSize: 10 * 1024 * 1024 }, // 10 MB
  fileFilter: (req, file, cb) => {
    const allowed = [".jpg", ".jpeg", ".png", ".pdf"];
    const ext = path.extname(file.originalname).toLowerCase();
    if (!allowed.includes(ext)) {
      return cb(new Error("File type not allowed"));
    }
    cb(null, true);
  },
});
```

## API Security

### Authentication

| Method          | Use Case                      | Security Level          |
| --------------- | ----------------------------- | ----------------------- |
| API key         | Server-to-server, public APIs | Low (shared secret)     |
| OAuth 2.0 + JWT | User-facing APIs              | High (scoped, expiring) |
| mTLS            | Service-to-service            | Highest (mutual cert)   |

### JWT Patterns

```javascript
// Token creation
const jwt = require("jsonwebtoken");

const token = jwt.sign(
  {
    sub: user.id,
    roles: user.roles,
    iss: "api.example.com",
    aud: "app.example.com",
  },
  process.env.JWT_SECRET,
  {
    algorithm: "HS256", // or RS256 for asymmetric
    expiresIn: "15m", // short-lived access tokens
    notBefore: 0,
  },
);

// Token verification
jwt.verify(token, process.env.JWT_SECRET, {
  algorithms: ["HS256"], // explicit algorithm allowlist
  issuer: "api.example.com",
  audience: "app.example.com",
  clockTolerance: 30, // 30-second clock skew
});
```

**Never**:

- Use `algorithm: "none"`
- Store sensitive data in JWT payload (it's base64, not encrypted)
- Use JWTs as long-lived sessions (use refresh tokens)
- Accept tokens without verifying signature, expiry, issuer, audience

### API Security Checklist

- [ ] Authentication on all non-public endpoints
- [ ] Authorization checked per-resource (not just per-endpoint)
- [ ] Rate limiting per API key / user / IP
- [ ] Request size limits
- [ ] Input validation with schema enforcement
- [ ] No sensitive data in URLs (query params logged by proxies)
- [ ] Pagination with maximum page size
- [ ] HTTPS only (HSTS enabled)
- [ ] API versioning strategy
- [ ] Error responses don't leak implementation details
- [ ] Audit logging for sensitive operations
- [ ] CORS restricted to known origins

## Comprehensive Security Headers (Nginx)

```nginx
server {
    # HTTPS only
    listen 443 ssl http2;

    # HSTS
    add_header Strict-Transport-Security "max-age=63072000; includeSubDomains; preload" always;

    # Anti-XSS / Anti-injection
    add_header Content-Security-Policy "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self'; connect-src 'self' https://api.example.com; frame-ancestors 'none'; base-uri 'self'; form-action 'self'" always;

    # Anti-clickjacking
    add_header X-Frame-Options "DENY" always;

    # Anti-MIME-sniffing
    add_header X-Content-Type-Options "nosniff" always;

    # Referrer
    add_header Referrer-Policy "strict-origin-when-cross-origin" always;

    # Permissions
    add_header Permissions-Policy "camera=(), microphone=(), geolocation=(), payment=()" always;

    # Cross-origin isolation
    add_header Cross-Origin-Opener-Policy "same-origin" always;
    add_header Cross-Origin-Resource-Policy "same-origin" always;
}
```

## Security Testing

| Tool                    | Tests                           | Integration        |
| ----------------------- | ------------------------------- | ------------------ |
| **OWASP ZAP**           | DAST — active/passive scanning  | CI/CD, API scan    |
| **Burp Suite**          | Pentest proxy, scanner          | Manual + automated |
| **nuclei**              | Template-based vuln scanning    | CI/CD              |
| **semgrep**             | SAST — pattern matching in code | Pre-commit, CI     |
| **Mozilla Observatory** | HTTP header analysis            | Manual check       |
| **securityheaders.com** | Header grade                    | Manual check       |
| **SSL Labs**            | TLS configuration               | Manual check       |
