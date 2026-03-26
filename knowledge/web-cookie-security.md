# Web Cookie Security — Attributes, Third-Party Cookies, Privacy & CHIPS

## Overview

HTTP cookies are fundamental to web applications for authentication, personalization, and analytics, but they present significant security and privacy risks. Understanding cookie attributes, cross-site request forgery (CSRF) mitigations, privacy regulations, and emerging partitioning technologies is essential for building secure applications that respect user privacy.

---

## Cookie Attributes & Security

### Secure

Restricts cookie transmission to HTTPS connections only. Prevents interception on unencrypted channels.

```http
Set-Cookie: session_id=abc123; Secure
```

**Requirement in prod:** Every HTTP-only authentication cookie should have `Secure` set. Cookies transmitted over HTTP are visible to network eavesdroppers (WiFi sniffing, proxy inspection).

**Enforcement:** Browsers ignore `Secure` on http:// origins when the flag is present, but some browsers now enforce stricter rules (e.g., Chrome requires `Secure` with `SameSite` on certain cookies).

### HttpOnly

Prevents JavaScript access via `document.cookie`. Only sent with HTTP requests.

```http
Set-Cookie: session_id=abc123; HttpOnly
```

**Defense against:**
- XSS attacks stealing session tokens via JavaScript
- Malicious scripts leaking sensitive data

**Limitation:** Doesn't prevent the token from being sent in HTTP requests (vulnerable to network sniffing if not paired with `Secure`).

**Best practice:** Use `HttpOnly` for all authentication tokens. This removes the XSS vector requiring only the token value; if attacker wants the token, they must intercept network traffic (harder post-HTTPS).

### SameSite

Restricts when cookies are sent in cross-site requests. Prevents cross-site request forgery (CSRF) and many cross-site tracking scenarios.

**Three values:**

- **`SameSite=Strict`** — Sent only in same-site requests. No cookies when following external links to your site.
- **`SameSite=Lax`** (default in modern browsers) — Sent only in same-site requests and top-level navigations (links, form GET submissions). NOT sent in iframes, img tags, or form POST from external sites.
- **`SameSite=None`** — Sent in all contexts; requires `Secure` flag.

```http
Set-Cookie: session_id=abc123; SameSite=Strict; Secure
Set-Cookie: tracking_id=xyz789; SameSite=None; Secure
```

**CSRF Prevention:** `Lax` is sufficient for most use cases. Attacker cannot forge POST requests from external sites because `SameSite=Lax` blocks cross-site form submissions.

**Trade-offs:** `Strict` breaks legitimate workflows (e.g., clicking a link to your site from an external referrer doesn't include your cookie session). `Lax` balances security and usability.

### Domain & Path

Restricts cookie scope to specific subdomains and paths.

```http
Set-Cookie: user=alice; Domain=.example.com; Path=/api
```

- **`Domain=.example.com`** — Cookie sent to example.com, sub.example.com, and any subdomain. Omitting `Domain` restricts to exact host.
- **`Path=/api`** — Cookie sent only for requests to /api/ and deeper. Default is `/`.

**Security implication:** Overly broad `Domain` allows all subdomains to access the cookie (e.g., if one subdomain is compromised or serves untrusted content, it can exfiltrate cookies meant for another).

**Best practice:** Use minimal `Domain` scope. For modern applications, omit `Domain` (cookie scoped to current host only).

### Max-Age & Expires

Controls cookie lifetime.

```http
Set-Cookie: session_id=abc123; Max-Age=3600
Set-Cookie: remember_me=bob; Expires=Wed, 09 Jun 2026 10:18:14 GMT
```

- **`Max-Age=N`** — Expires in N seconds. Takes precedence over `Expires`.
- **`Expires`** — Absolute expiry date. If omitted and Max-Age not set, cookie is a **session cookie** (deleted when browser closes).

**Session vs. Persistent:** Session cookies are cleared when browser closes; persistent cookies survive tab/browser restarts.

---

## Third-Party Cookies & Tracking

### What They Are

Cookies set by a domain different from the site user is visiting. Example: Analytics service (analytics.com) sets cookies on pages across many sites.

Possible via:
- Embedded iframes: `<iframe src="https://tracker.com/beacon"></iframe>`
- Image pixels: `<img src="https://tracker.com/pixel.gif" />`
- Script tags: `<script src="https://ads.com/tracker.js"></script>`

### Privacy & Regulation

**GDPR (EU):** Third-party cookies for tracking require explicit prior consent. Website must ask user before loading tracker scripts.

**CCPA (California):** Users have right to opt-out of data sale; sites must disclose third-party data sharing.

**ePrivacy Directive:** Electronic Privacy Directive (EU) treats cookies as PII-adjacent; "cookies and similar tracking technologies" require consent before deployment.

**Practical implication:** Most sites now deploy cookie consent banners (CMP: Consent Management Platform) that gate third-party trackers until user opts in.

### Browser Deprecation

Major browsers are phasing out third-party cookies:

- **Chrome:** Third-party cookie deprecation started January 2024; full phase-out planned for end of 2024 (delays ongoing).
- **Safari/Firefox:** Already block third-party cookies by default (ITP: Intelligent Tracking Prevention).

**Migration strategies:**
- First-party data collection (zero-party data: user provides directly)
- Server-side tracking (send data to your server; your server forwards to analytics)
- First-Party Sets (FPS): Related domains declare trust; cookies can be shared (limited rollout)

---

## CHIPS: Partitioned Cookies

### What It Is

CHIPS (Cookies Having Independent Partitioned State) allows third-party cookies scoped to top-level domain, not globally.

```http
Set-Cookie: tracker_id=xyz; Secure; SameSite=None; Partitioned
```

**How it differs:** Instead of tracker.com setting a global cookie visible across all sites, each top-level site gets its own partitioned cookie.

**Example:**
- User visits site-a.com → tracker.com sets cookie for site-a.com partition
- User visits site-b.com → tracker.com sets NEW cookie for site-b.com partition
- Cookies are not shared; cross-site tracking prevented

### Use Cases

- Embedded widgets that need state (e.g., shopping cart, chat widget) without full third-party cookie access
- Subresources that need persistent IDs per context

### Adoption

CHIPS is supported in Chrome (enabled by default as of 2024) and increasingly adopted. Not a full replacement for third-party cookies but reduces tracking capability while preserving some embed functionality.

---

## Cookie Prefixes

Explicit prefixes in cookie names signal intended scope and restrictions.

```http
Set-Cookie: __Secure-session_id=abc123; Secure
Set-Cookie: __Host-token=xyz; Secure; Path=/; SameSite=Strict
```

### __Secure-

Cookie MUST have `Secure` flag. Browser rejects if `Secure` missing.

### __Host-

Strictest. Cookie MUST have:
- `Secure` flag
- `SameSite` flag (any value)
- `Path=/` (no path restriction)
- No `Domain` (scoped to current host only)

**Prevents:** Cookies from being overwritten by sibling subdomains or via path tricks.

**Use case:** High-security tokens (OAuth state, CSRF tokens). The `__Host-` prefix ensures a subdomain cannot override the cookie or extend its scope.

---

## CSRF Protection Layers

### SameSite=Lax (Primary Modern Defense)

Blocks cross-site form submissions by default. Attacker site cannot forge requests with your cookies.

```http
Set-Cookie: session_id=abc123; SameSite=Lax; Secure; HttpOnly
```

### CSRF Tokens (Legacy & Reinforcement)

Server issues opaque token; forms/requests must include token.

```html
<form action="/transfer" method="POST">
  <input type="hidden" name="csrf_token" value="abc123defg456" />
  <input type="submit" />
</form>
```

Server validates: request from attacker site includes session cookie (due to `SameSite=None` or old SameSite policy), but attacker cannot guess CSRF token.

**Modern stance:** `SameSite=Lax` sufficient for new applications; CSRF tokens are redundant unless app must support older browsers or uncommon scenarios.

### Double-Submit Cookie Pattern

Client mirrors token in both cookie and request body/header. Server validates match.

Less common now (SameSite superior), but used in constrained environments (e.g., no server state).

---

## Best Practices

1. **Always use `Secure` + `HttpOnly` for auth cookies** in production. Binds tokens to HTTPS; prevents JavaScript access.

2. **Set `SameSite=Lax` or `SameSite=Strict`** by default. CSRF protection out-of-the-box.

3. **Minimize `Domain` scope.** Omit `Domain` for host-specific cookies; use explicit subdomains if sharing across specific hosts only.

4. **Use session cookies for auth** (Max-Age/Expires omitted). Persistent auth cookies increase risk window if device compromised.

5. **Implement cookie consent for tracking.** Explicitly ask before loading third-party trackers; document data flows.

6. **Monitor for unused cookies.** Audit periodically; remove old tracking domains no longer needed (reduces data surface).

7. **Use `__Host-` for critical tokens** (API keys, state tokens). Prevents subdomain confusion.

---

## See Also
- [security-cors.md](security-cors.md) — Cross-Origin Resource Sharing headers & misconfigurations
- [web-browser-security.md](web-browser-security.md) — Same-Origin Policy, XSS, sandboxing
- [security-web-application.md](security-web-application.md) — Full web app security checklist