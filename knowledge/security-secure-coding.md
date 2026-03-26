# Security — Secure Coding: Input Validation, Encoding, Parameterization

## Overview

Secure coding operates on a core principle: **untrusted input is the attack surface**. Data from users, network requests, files, databases, and system calls must be validated, transformed, and handled defensively. This guide covers input validation strategies, context-aware output encoding, parameterized queries, file upload security, deserialization safety, and defense-in-depth architecture.

## Input Validation

### Allowlist vs. Blacklist

**Blacklist (denylist):** specify what's forbidden.
```
if (input.contains("<script>") || input.contains("onclick")) reject();
```
Fails against new attack vectors: `<iFrame>`, `svg onload`, HTML entities, Unicode escapes.

**Allowlist (whitelist):** specify what's permitted.
```
if (!/^[a-zA-Z0-9@._-]+$/.test(email)) reject();  // Only alphanumerics + specific chars
```
More secure. Explicit about valid input shape. Harder to bypass.

**Guidance**: Use allowlists for input validation wherever feasible. Blacklists work only as a secondary defense layer.

### Validation Layers

1. **Type**: Is the data the expected type? (string, number, array, JSON object)
2. **Length**: Within acceptable bounds? (Max length prevents buffer overflow, DoS)
3. **Range/Format**: Does value fit constraints? ("email" regex, "age" > 0 and < 150)
4. **Encoding**: Is the data in expected encoding? (UTF-8, hex-encoded, base64)
5. **Schema**: Does complex structure match expected format? (JSON schema validation, protobuf verification)

**Server-side validation is non-negotiable.** Client-side validation improves UX but provides zero security—it's trivially bypassed by sending raw HTTP requests.

### Type Coercion Attacks

Languages with loose type systems can be exploited via unexpected coercion.

```javascript
// JavaScript
0 == "0"        // true — string coerced to number
null == undefined // true — unexpected coercion
"0" ? true = false // "0" is truthy in JS, but might be falsy in other contexts
```

**Defense**: Use strict equality (`===` in JS, type-specific checks). Validate before coercion. Use static analysis tools to catch type confusion.

## Output Encoding and Escaping

Encoding depends on **context**: the encoding that's safe for HTML is wrong for JavaScript, URLs, or CSS.

| Context       | Unsafe Input | Safe Encoding | Example |
|---------------|--------------|---------------|---------|
| HTML body     | `<script>alert(1)</script>` | HTML entity encode | `&lt;script&gt;alert(1)&lt;/script&gt;` |
| HTML attribute | `" onmouseover="alert(1)` | Quote + entity encode | `&quot; onmouseover=&quot;alert(1)` |
| JavaScript string | `'; alert(1); //` | JS escape (backslash) | `\'; alert(1); //` |
| URL query     | `foo=<script>` | URL encode | `foo=%3Cscript%3E` |
| CSS value     | `red; }</style><script>` | CSS escape | `red\3b \7d \3c script\3e` |

**Principle**: Encode output at the point of generation, in the target language/context. Use well-tested libraries (e.g., OWASP ESAPI, templating engines with auto-escaping).

## Parameterized Queries (Prepared Statements)

**Vulnerable (string concatenation):**
```sql
query = "SELECT * FROM users WHERE email = '" + userinput + "'";
// User enters: admin' OR '1'='1
// Query becomes: SELECT * FROM users WHERE email = 'admin' OR '1'='1'
```

**Safe (parameterized query):**
```sql
query = "SELECT * FROM users WHERE email = ?";
executeQuery(query, [userinput]);
```

The SQL parser interprets `?` as a literal placeholder. User input is never treated as SQL syntax.

**All database libraries support parameterized queries.** String concatenation is never necessary. Default to parameterized queries in every ORM, driver, and raw SQL context.

## File Upload Security

File uploads are a rich attack surface: malicious code execution, denial of service, directory traversal.

### Validation: Type, Encoding, Size

1. **File type validation**: Check MIME type (`application/pdf`, `image/jpeg`). But MIME type can be spoofed—always validate magic bytes (file signature).
   ```
   JPEG: FF D8 FF
   PNG:  89 50 4E 47
   PDF:  25 50 44 46
   ```
2. **Size limits**: Prevent DoS via large files. Limit both individual file size and aggregate upload size per user-session.
3. **Filename sanitization**: Reject paths containing `../`, `..\\`. Store as UUID, not user-provided name. Prevent TOCTOU (time-of-check-time-of-use) attacks via atomic operations.

### Sandboxing and Execution Prevention

1. **Store outside web root**: Files should not be directly executable via HTTP. Serve from a separate CDN or private storage.
2. **Disable script execution**: Set web server headers to prevent `.php`, `.asp`, `.jsp` execution.
   ```
   AddType text/plain .php .asp .jsp
   ```
3. **Content-Disposition header**: Force download rather than inline rendering.
   ```
   Content-Disposition: attachment; filename="file.pdf"
   ```
4. **Scan for malware**: Use ClamAV, Metadefender, or cloud antivirus services.

## Deserialization Safety

Deserialization of untrusted data can deserialize malicious objects that execute code during construction.

**Unsafe (Java):**
```java
ObjectInputStream ois = new ObjectInputStream(untrustedData);
MyObject obj = (MyObject) ois.readObject();  // Can invoke arbitrary constructors
```

Gadget chains (chains of existing classes whose constructors/methods execute code) can be weaponized.

**Safe approaches:**
- Use JSON instead of binary serialization (JavaScript objects, Python dicts).
- Validate JSON structure before deserialization (JSON schema).
- Use allowlists for class types: `ObjectInputFilter` in Java 9+.
- Whitelist serializable classes explicitly; reject unknown types.

## Error Handling Without Information Leakage

Verbose errors aid attackers: stack traces reveal framework versions, database structure, internal code paths.

```
# Unsafe:
Exception: Database connection failed: user=admin@db.internal:3306
File: /var/app/src/models/User.php:124
```

**Principle**: Log detailed errors server-side. Return generic errors to clients.

```
# Safe:
User sees: "An error occurred. Please try again later. Reference ID: abc123def456"
Server logs: Full stack trace, query, database connection details, timestamp
```

Reference IDs let support teams correlate user reports to server logs without exposing internals.

## Defense in Depth

No single control is perfect. Assume each layer can fail:

1. **Input validation** catches most attacks
2. **Parameterization** prevents injection if validation fails
3. **Output encoding** stops XSS if data somehow reaches output
4. **Security headers** (CSP, HSTS, X-Frame-Options) restrict attack scope
5. **Runtime protections** (WAF, IDS, sandboxing) catch what everything else missed
6. **Logging and alerting** detect and respond to attack attempts

See also: security-best-practices.md, security-owasp-injection.md, security-web-application.md, web-browser-security.md