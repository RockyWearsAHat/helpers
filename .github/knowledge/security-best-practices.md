# Security Best Practices (OWASP & Industry Standards)

## OWASP Top 10 (2025)

### A01: Broken Access Control

Restrictions on what authenticated users can do are not properly enforced. Users can act outside their intended permissions.

- Enforce access control on the server (never trust the client).
- Deny by default — only allow what's explicitly permitted.
- Implement role-based access control (RBAC) or attribute-based access (ABAC).
- Rate-limit API and controller access to minimize automated attack damage.
- Invalidate sessions/tokens on logout.

### A02: Security Misconfiguration

Default configs, open cloud storage, verbose errors, unnecessary features enabled.

- Harden environments: remove defaults, disable unused features/ports/accounts.
- Automate configuration verification (IaC, security scanners).
- Different credentials for each environment (dev/staging/prod).
- Never expose stack traces or debug info in production.

### A03: Software Supply Chain Failures

Vulnerable dependencies, compromised CI/CD pipelines, unsigned packages.

- Pin dependency versions. Use lockfiles (`package-lock.json`, `Cargo.lock`, etc.).
- Audit dependencies regularly (`npm audit`, `pip-audit`, Dependabot, Snyk).
- Verify package integrity (checksums, signatures).
- Minimize dependency count — every dependency is an attack surface.

### A04: Cryptographic Failures

Sensitive data exposed through weak/missing encryption.

- Use TLS 1.2+ for all data in transit.
- Encrypt sensitive data at rest (AES-256 or equivalent).
- Never roll your own crypto. Use vetted libraries (libsodium, OpenSSL, Web Crypto API).
- Hash passwords with bcrypt, scrypt, or Argon2 — never MD5/SHA1.
- Rotate keys and secrets regularly.

### A05: Injection (SQL, XSS, Command, LDAP)

Untrusted data sent to an interpreter as part of a command or query.

- **SQL injection**: Use parameterized queries / prepared statements. Never string-concatenate user input into SQL.
- **XSS**: Escape output based on context (HTML, JS, URL, CSS). Use Content Security Policy (CSP) headers.
- **Command injection**: Avoid calling system shells. If unavoidable, use strict allowlists and never pass user input directly.
- **Template injection**: Use sandboxed template engines. Avoid `eval()`, `exec()`, `Function()` on user input.

## Secure Coding Checklist (OWASP Quick Reference)

### Input Validation

- Validate all input on the server side (type, length, range, format).
- Use allowlists over denylists where possible.
- Validate and sanitize file uploads (extensions, MIME types, size limits, scan for malware).

### Output Encoding

- Encode output contextually: HTML entity encode for HTML, JavaScript encode for JS, URL encode for URLs.
- Set `Content-Type` and `X-Content-Type-Options: nosniff` headers.

### Authentication & Session Management

- Enforce strong passwords (length > complexity rules).
- Implement multi-factor authentication (MFA) for sensitive operations.
- Use secure, HttpOnly, SameSite cookies for sessions.
- Invalidate sessions on password change and logout.
- Rate-limit and lock accounts after failed login attempts.

### Data Protection

- Classify data by sensitivity. Apply controls proportionally.
- Don't log sensitive data (passwords, tokens, PII, credit cards).
- Remove sensitive data from memory as soon as possible.
- Apply least-privilege principle to database accounts and APIs.

### Communication Security

- Enforce HTTPS everywhere (HSTS header).
- Validate TLS certificates.
- Don't transmit sensitive data in URL parameters (they end up in logs/referrers).

### Error Handling & Logging

- Don't leak internal details in error messages shown to users.
- Log security-relevant events (login attempts, access failures, input validation failures).
- Use structured logging with correlation IDs.
- Monitor and alert on anomalous patterns.

## Secrets Management

- **Never commit secrets to version control.** Use `.env` files (gitignored), environment variables, or secret managers (Vault, AWS Secrets Manager, 1Password CLI).
- Scan for leaked secrets: `git-secrets`, `trufflehog`, `gitleaks`.
- Rotate secrets on any suspected exposure.
- Use short-lived tokens over long-lived API keys when possible.

---

_Sources: OWASP Top 10 2025, OWASP Secure Coding Practices Quick Reference Guide, NIST SP 800-63 (Authentication), CWE/SANS Top 25_
