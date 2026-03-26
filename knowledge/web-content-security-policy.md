# Web Content Security Policy — Directives, Nonce/Hash Mitigations & Trusted Types

## Overview

Content Security Policy (CSP) is a browser security mechanism that restricts what scripts, styles, and resources can be loaded. It prevents XSS by disallowing inline scripts and by enforcing whitelist policies. CSP can be implemented via HTTP headers or meta tags and supports both report-only monitoring and enforcement modes.

---

## Core Concept: Preventing Inline Execution

### The Problem

Inline `<script>` tags and event handlers are easy targets for XSS:

```html
<!-- Vulnerable: inline script + user input -->
<div>Welcome, <span id="user"></span></div>
<script>
  document.getElementById('user').textContent = userInput; // If userInput = "<img src=x onerror=alert('xss')>"
</script>
```

Attacker injects: `<img src=x onerror=alert('xss')>` → script executes.

### CSP Solution

CSP uses `default-src` and specific directives to whitelist sources. By default, CSP blocks all inline scripts.

```http
Content-Security-Policy: default-src 'self'; script-src 'self' https://cdn.example.com
```

This policy:
- Allows resources (scripts, styles, images) only from same-origin (`'self'`) or https://cdn.example.com
- Blocks all inline `<script>` tags unless explicitly allowed via nonce or hash

---

## Directives

### default-src

Fallback for all resource types not explicitly set.

```http
default-src 'self'
```

If `script-src` not specified, `script-src` inherits `default-src`. This simplifies policy but is less precise.

### script-src & style-src

Controls script and stylesheet loading respectively.

```http
script-src 'self' https://trusted-cdn.com 'nonce-abc123'
style-src 'self' 'unsafe-inline'
```

**Values:**
- `'self'` — Same origin only
- `https://example.com` — Specific domain
- `https:` — Any HTTPS source
- `*` — Any source (defeats CSP; avoid)
- `'unsafe-inline'` — Allows inline `<script>` / `<style>` (defeats XSS protection; avoid in strict policies)
- `'nonce-X'` — Allows inline scripts with matching nonce attribute (see "Nonce-Based Policies")
- `'hash-sha256-X'` — Allows inline scripts matching hash (see "Hash-Based Policies")

### Other Key Directives

- **`img-src`** — Image sources
- **`font-src`** — Font sources
- **`connect-src`** — fetch, XHR, WebSocket targets
- **`frame-src`** — Embeddable frame sources (iframe src)
- **`object-src`** — Flash, applet objects (rarely needed; block with `'none'`)
- **`media-src`** — Audio/video sources
- **`manifest-src`** — Web app manifest sources
- **`form-action`** — Form submission targets (POST/GET redirects)
- **`frame-ancestors`** — Can this page be framed? (X-Frame-Options equivalent)
- **`base-uri`** — Can `<base>` tag change base URL? (Usually block with `'none'`)
- **`sandbox`** — Lightweight iframe sandboxing inline

---

## Nonce-Based Policies

### How It Works

Server generates random token (nonce); includes in CSP header and in `<script>` tag.

```http
Content-Security-Policy: script-src 'nonce-rnd123abc'
```

```html
<script nonce="rnd123abc">
  console.log('This script allowed because nonce matches');
</script>

<script>
  console.log('This script blocked; no nonce');
</script>
```

Browser allows only scripts with matching nonce.

### Server Implementation

```javascript
// Node.js pseudocode
const nonce = crypto.randomBytes(16).toString('base64');
res.setHeader('Content-Security-Policy', `script-src 'nonce-${nonce}'`);
res.send(`<html>
  <script nonce="${nonce}">
    // Safe inline script
  </script>
</html>`);
```

### Advantages

- Allows inline scripts without `'unsafe-inline'`
- Nonce must be cryptographically random per request (replay attacks prevented)
- Template engines can inject nonce automatically

### Disadvantages

- Requires server-side nonce generation per request (stateless systems need careful design)
- Nonce visible in HTML source (not secret; just proves server generated page)
- Every response regenerates nonce (script processing overhead minimal but present)

---

## Hash-Based Policies

### How It Works

Server specifies SHA-256 hash of exact script content. Browser allows only scripts matching hash.

```http
Content-Security-Policy: script-src 'sha256-abc123def456' 'nonce-xyz'
```

```html
<script>
  console.log('Static inline script');
</script>
```

Browser computes SHA-256 of script content; if matches policy, allows it.

### Advantages

- No nonce required; works with static assets
- Good for third-party widgets (include once, hash once)
- Can be precomputed (subresources pre-hashed)

### Disadvantages

- Any whitespace or character change breaks hash (fragile; comments, formatting changes break policy)
- Not suitable for dynamic content (script content changes = new hash)
- Attacker can still exploit logic bugs in static script; CSP doesn't validate script correctness, only origin/hash

---

## strict-dynamic

Powerful directive limiting script execution to dynamically-injected scripts (with nonce or hash) plus whitelisted scripts.

```http
Content-Security-Policy: script-src 'strict-dynamic' 'nonce-abc123' https://trusted-cdn.com
```

**Behavior:**
- Only scripts with valid nonce allowed
- Scripts loaded dynamically by those nonce'd scripts ALSO allowed (bypass restriction)
- Whitelisted URLs (`https://trusted-cdn.com`) ignored unless script was nonce'd

**Use case:** Loading libraries dynamically via trusted entry point.

```html
<script nonce="abc123">
  // This script is nonce'd, so allowed
  // If it injects new <script src="..."></script>, that new script also allowed (strict-dynamic effect)
  fetch('https://other-origin.com/lib.js').then(r => {
    const script = document.createElement('script');
    script.src = r.url;
    document.head.appendChild(script); // Allowed via strict-dynamic
  });
</script>
```

---

## Reporting

### report-uri (Deprecated)

Send CSP violation reports to endpoint.

```http
Content-Security-Policy: script-src 'self'; report-uri https://csp-collector.example.com
```

Browser sends JSON POST to endpoint on violation:

```json
{
  "document-uri": "https://example.com/page.html",
  "violated-directive": "script-src",
  "original-policy": "script-src 'self'",
  "blocked-uri": "https://evil.com/malware.js",
  "disposition": "enforce"
}
```

### report-to (Modern)

Newer directive using Reporting API.

```http
Content-Security-Policy: script-src 'self'; report-to csp-endpoint
```

Offers richer metadata and batching.

### Content-Security-Policy-Report-Only

Header that only reports violations without enforcing.

```http
Content-Security-Policy-Report-Only: script-src 'self' 'nonce-abc123'
```

Useful for testing policies in production without breaking functionality.

---

## Incremental Adoption & Phase-In

### Challenges

Strict CSP often breaks existing sites (inline scripts, inline styles, eval, etc.).

### Strategies

1. **Report-Only first.** Deploy `Content-Security-Policy-Report-Only` header; collect violations; fix issues.

2. **Relax specific directives.** Instead of `script-src 'self'`, allow:
   - Specific inline scripts via nonce
   - Specific third-party domains
   - Known event handlers via `'unsafe-inline'` (temporary)

3. **Use separate policies for different contexts.** Different CSP for admin vs. user-facing pages.

4. **Gradually restrict.** Start with broad allowlist; tighten over time as violations decrease.

---

## CSP Bypass Techniques & Limitations

### Not a Complete Defense

CSP blocks `<script>` injection but not all vectors:

- **DOM XSS:** If page constructs HTML via `innerHTML` from user input, CSP can't detect it. Must use innerText or sanitize.
  ```javascript
  // Even with CSP, this is vulnerable
  document.getElementById('target').innerHTML = userInput; // If userInput has event handlers
  ```
  Solution: Use `innerText` or DOM methods (textContent, createElement).

- **DOM clobbering:** Attacker defines global variable shadowing page script dependencies.
  ```html
  <img name="jQuery" src="x" /> <!-- Shadows jQuery lib -->
  ```
  Mitigation: Avoid relying on globals; use modules.

- **object/embed tags:** Can load Flash/plugins bypassing script-src. Use `object-src 'none'` to block.

- **Mutation-based XSS (mXSS):** HTML parser normalizes entities/attributes in unexpected ways, bypassing sanitizers.

### CSP + Trusted Types

Trusted Types framework (new) enforces DOM XSS prevention at runtime.

---

## Trusted Types

### Concept

Trusted Types requires data written to dangerous DOM APIs (innerHTML, insertAdjacentHTML) to be explicitly validated and marked as "trusted."

```javascript
// Without Trusted Types: any string works (dangerous if user-controlled)
element.innerHTML = userInput; // CSP can't prevent this

// With Trusted Types: must use TrustedHTML object
element.innerHTML = trustedHtml; // throws error if trustedHtml not from trusted policy
```

### Enable via CSP

```http
Content-Security-Policy: require-trusted-types-for 'script'
```

or

```http
Trusted-Types policy-name; default 'none'
```

### Define a Policy

```javascript
const policy = trustedTypes.createPolicy('myPolicy', {
  createHTML: (input) => DOMPurify.sanitize(input),
});

// Usage
element.innerHTML = policy.createHTML(userInput); // Safe: sanitized by policy
```

Browser enforces: if policy validation fails, throws error. No silent bypass.

---

## Best Practices

1. **Start strict; relax if needed.** `script-src 'self'` + `style-src 'self'` is a good minimal baseline.

2. **Use nonce for inline scripts in dynamic apps.** Regenerate per request.

3. **Prefer external scripts over inline.** External scripts easier to manage, cache, and CSP-compliant.

4. **Report violations to monitoring endpoint.** CSP report-uri helps detect attacks and policy misconfigurations.

5. **Use `Content-Security-Policy-Report-Only` before enabling enforcement.** Avoid breaking existing functionality.

6. **Block eval, setTimeout(string), etc.** `'unsafe-eval'` is dangerous; avoid.

7. **Frame-ancestors for clickjacking defense.** `frame-ancestors 'none'` or `frame-ancestors 'self'`.

8. **Combine with Trusted Types for DOM XSS prevention.**

---

## See Also
- [web-browser-security.md](web-browser-security.md) — Same-Origin Policy, XSS, CORS fundamentals
- [security-owasp-xss.md](security-owasp-xss.md) — XSS attack vectors and defenses
- [security-web-application.md](security-web-application.md) — Full web security checklist