# Web Subresource Integrity — Hashing, CORS, Supply Chain Defense & CDN Security

## Overview

Subresource Integrity (SRI) is a browser security mechanism that allows integrity verification of cross-origin resources (scripts, stylesheets, WebAssembly modules). By including a cryptographic hash in the `<script>` or `<link>` tag, the browser verifies the resource matches before execution. SRI protects against compromised CDNs, MITM attacks, and malicious third-party script injection.

---

## Core Concept: Hash-based Verification

### How It Works

Developer computes SHA-256 or SHA-384 hash of resource content; embeds hash in HTML; browser verifies before loading.

```html
<script 
  src="https://cdn.example.com/lib.js"
  integrity="sha256-abc123def456ghi789jkl"
></script>
```

**Browser process:**
1. Downloads https://cdn.example.com/lib.js
2. Computes SHA-256 of downloaded content
3. Compares computed hash to integrity attribute
4. If match: execute script
5. If mismatch: reject and fire SecurityError event

### Generate Hash

```bash
# macOS/Linux
curl https://cdn.example.com/lib.js | openssl dgst -sha256 -binary | base64

# Or with npm
npm install -g sri-hash
sri-hash https://cdn.example.com/lib.js
```

Example output:
```
sha256-nxr4/mHQCv42FHmMGWl9KP7IGrZLGJVXHWbmMDWBKxE=
```

---

## Integrity Attribute Syntax

### Basic Form

```html
<script integrity="sha256-HASH"></script>
<link rel="stylesheet" integrity="sha256-HASH" />
```

### Multiple Hashes (Fallback)

Support multiple algorithms; browser uses first one it supports.

```html
<script
  src="lib.js"
  integrity="sha384-abc123 sha256-def456"
></script>
```

Browser tries SHA-384 first; if not supported, falls back to SHA-256.

### Algorithms

- **sha256** (most common) — 256-bit hash
- **sha384** — 384-bit hash (stronger)
- **sha512** — 512-bit hash (strongest but rarely needed)

Modern browsers support sha256, sha384, sha512. Use sha384 for critical resources; sha256 for general use.

---

## When to Use SRI

### Cross-Origin Resources

SRI essential for scripts/stylesheets from third-party CDNs:

```html
<!-- FROM CDN: HIGH RISK; USE SRI -->
<script 
  src="https://cdn.jsdelivr.net/npm/react@18/dist/react.min.js"
  integrity="sha384-w1Uy2bBRJjPu5Y6j3lK0eXL"
></script>
```

CDN could be compromised, DNS poisoned, or MITM'd. SRI verifies content.

### Same-Origin Resources

SRI optional for same-origin but provides defense-in-depth:

```html
<!-- FROM SAME ORIGIN: OPTIONAL; ADDS DEPTH -->
<script src="/js/app.js" integrity="sha256-xyz"></script>
```

Helpful if server is compromised and serves modified scripts.

### Dynamic Resource Loading

SRI works with `fetch` + `eval` (not recommended) or module loaders:

```javascript
// NOT RECOMMENDED: fetch + eval
fetch('https://cdn.example.com/lib.js')
  .then(r => r.text())
  .then(code => {
    if (!verifySRI(code, 'sha256-abc123')) throw new Error('SRI mismatch');
    eval(code); // Bad practice; use module loaders instead
  });

// BETTER: Use <script> tag or module import with SRI on initial load
```

---

## CORS & crossorigin Attribute

### Why CORS + SRI Together

Cross-origin resources require `crossorigin` attribute for CORS headers use. Without it, some error details are hidden (script.onerror() event lacks information).

```html
<script 
  src="https://cdn.example.com/lib.js"
  integrity="sha256-abc123"
  crossorigin="anonymous"
></script>
```

**What `crossorigin="anonymous"` does:**
- Browser sends `Origin` header
- Server must respond with `Access-Control-Allow-Origin: *` or specific origin
- If no CORS header: script loads but error details hidden (security-by-obscurity)

### CORS Requirement for SRI Failure Reporting

Without `crossorigin`, browser suppresses error event details on SRI mismatch:

```javascript
// WITH crossorigin="anonymous"
script.addEventListener('error', (e) => {
  console.error('SRI mismatch or load failed');
  // e.message contains some details
});

// WITHOUT crossorigin
script.addEventListener('error', (e) => {
  console.error('Script error'); // Generic message; no details
});
```

**Best practice:** Always include `crossorigin="anonymous"` with `integrity` on cross-origin resources.

---

## Supply Chain Attack Scenarios

### Scenario 1: Compromised CDN

CDN server compromise → attacker modifies JavaScript before serving.

```
User → Request lib.js → CDN Compromised → Returns Malware
```

**SRI Protection:**
```html
<script
  src="https://compromised-cdn.com/lib.js"
  integrity="sha256-legitimate-hash"
></script>
```

Browser downloads malware but detects hash mismatch → rejects → page breaks (loud failure; prevents silent exploitation).

### Scenario 2: DNS Hijacking

Attacker rewrites DNS; user's request to cdn.example.com resolves to attacker server.

```
User → DNS Query → Hijacked → Attacker Server → Malware
```

**SRI Protection:** Attacker can't modify script to match original hash (can't invert hash); page breaks.

### Scenario 3: MITM (Network Eavesdropper)

Attacker intercepts HTTPS and replaces script (rare post-HTTPS ubiquity but theoretically possible via compromised CA, BGP hijacking, etc.).

**SRI Protection:** MITM'd script won't match hash; rejected.

### Scenario 4: Accidental Supply Chain Degradation

Legitimate library updates to next version with breaking changes or security patches. Developers using old pinned SRI hashes gets old version.

```javascript
// Developer pinned old version
<script
  src="https://cdn.example.com/lib.js"
  integrity="sha256-old-version-hash"
></script>
```

CDN updates to new version; SRI mismatch; page stuck on old code.

**Mitigation:** Monitor for SRI failures; update often; use version-pinned URLs (e.g., cdn.example.com/lib-18.2.3.js) + SRI.

---

## Import Maps & Module Integrity (Emerging)

### Import Maps

Allow JavaScript module remapping without changing source code:

```html
<script type="importmap">
{
  "imports": {
    "react": "https://cdn.jsdelivr.net/npm/react@18/dist/react.min.js"
  }
}
</script>

<script type="module" integrity="sha256-xyz">
  import React from 'react'; // Resolves via import map
</script>
```

**Integrity for modules:** Each imported module can have its own `integrity` (under development; not all browsers support yet).

### WebAssembly Integrity

WebAssembly modules can use integrity:

```html
<link rel="preload" as="fetch" href="module.wasm" integrity="sha256-abc123" />
```

---

## Performance Considerations

### Hash Computation Overhead

Browser computes hash after download. Overhead typically negligible (~1-5ms for large scripts) but measurable on high-latency networks.

**Trade-off:** Slightly slower load vs. higher security. Usually worth it for critical resources.

### Caching & Hash Changes

Content-delivery: if resource content never changes (immutable assets), hash is stable. If resource updates (new library version), hash changes.

**Strategy:** Version-pin CDN URLs to make hashes stable across redeployments.

```html
<!-- Pinned version: hash stable across releases -->
<script
  src="https://cdn.example.com/lib-18.2.3.js"
  integrity="sha256-stable-hash"
></script>

<!-- Non-pinned: hash changes on each update; less useful for SRI -->
<script
  src="https://cdn.example.com/lib.js"
  integrity="sha256-changes-with-each-update"
></script>
```

---

## SRI Limitations

### Not a Signature System

SRI is a hash; not cryptographically signed. An attacker with write-access to your HTML can change both the src and integrity attributes.

```html
<!-- Attacker modifies both; SRI bypassed -->
<script
  src="https://attacker.com/malware.js"
  integrity="sha256-attacker-computes-malware-hash"
></script>
```

**Not a defense against:** Compromised site/server, XSS in page construction, build system compromise.

**Is a defense against:** Compromised CDN (assumed integrity attribute in HTML is trusted), MITM, DNS hijacking.

### Only Static Resources

Dynamic JavaScript (inline scripts, JavaScript generated on-the-fly) can't use SRI. Only external resources with fixed content.

### No Granular Control

SRI all-or-nothing: hash matches or doesn't. No partial updates, selective loading, or fallback versions.

---

## Best Practices

1. **Use SRI for all third-party scripts and stylesheets.** GitHub-hosted, CDN, analytics—everything external.

2. **Pin library versions in CDN URLs.** Use immutable paths (e.g., cdn.example.com/lib-1.2.3.js) so hash is stable.

3. **Include `crossorigin="anonymous"`** with `integrity` for cross-origin resources.

4. **Use SHA-384 for critical resources.** SHA-256 fine for less critical; SHA-384 provides stronger collision resistance.

5. **Monitor for SRI violations.** Log mismatches; alerts indicate CDN compromise or MITM.

6. **Automate SRI hash updates.** Tools (webpack plugins, build tools) can compute hashes during deployment.

7. **Combine with CSP.** CSP restricts script sources; SRI verifies content. Layered defense.

8. **Test hash mismatches in development.** Intentionally trigger SRI error to verify fallback/error handling.

---

## See Also
- [web-browser-security.md](web-browser-security.md) — Same-Origin Policy, CORS, CSP context
- [security-cors.md](security-cors.md) — CORS headers and crossorigin attribute
- [web-cookie-security.md](web-cookie-security.md) — Secure resource delivery context
- [infrastructure-cdn.md](infrastructure-cdn.md) — CDN architecture and security