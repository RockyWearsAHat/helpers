# Web Application Firewall (WAF) — Rules, Scoring, OWASP CRS, Evasion & Cloud Deployment

## Overview

A Web Application Firewall sits between clients and web applications, inspecting HTTP/HTTPS traffic and enforcing security policies. Unlike network firewalls (which operate on IP/port), WAFs understand HTTP semantics: request methods, headers, body content, URLs, cookies. They detect and block high-level attacks (SQL injection, XSS, path traversal, DDoS) that bypass network-layer defenses. Modern WAFs combine signature-based rules with behavioral detection and machine learning.

---

## Rule Types: Signatures vs. Anomaly Scoring

### Signature-Based Detection

Signature rules identify known attack patterns using rule definitions or regular expressions. Examples:

- **SQL injection:** Detect SQL keywords (UNION, SELECT, DROP) in request parameters, especially when combined with SQL special characters (quotes, semicolons).
- **XSS (Cross-Site Scripting):** Detect script tags, JavaScript event handlers (onclick, onerror), or HTML entity encodings in user input.
- **Path traversal:** Detect ../ sequences or URL-encoded equivalents (..%2F) attempting to access parent directories.
- **Command injection:** Detect shell metacharacters (| & ; $) or command substitution syntax (` or $()).

Example rule (pseudo-code):

```
rule "SQL Injection in POST parameter"
  if request.method == POST and
     request.parameter["username"] matches /('|union|select|drop)/i
  then
    alert "SQL injection attempt"
    block
```

**Advantages:** Fast, deterministic, human-auditable.

**Disadvantages:** Requires tuning; legitimate requests can trigger false positives (e.g., a product called "SELECT" in a URL). Attackers can evade via encoding, case variation, or syntax variations (UNION vs. UNION ALL).

### Anomaly Scoring

Anomaly scoring assigns points to requests based on deviations from baseline behavior. Requests scoring above a threshold are blocked.

**Example:**
- Unusual parameter count: +2 points
- SQL-like keywords in parameter: +3 points
- Executable attachment: +5 points
- Non-ASCII characters in User-Agent: +1 point

If total score ≥ 5, block the request.

**Advantages:** Detects novel attacks and bypasses. Reduces noise (legitimate requests rarely score high).

**Disadvantages:** Requires baseline training; scoring threshold tuning is environment-specific. False negatives (attacks that score below threshold) are harder to debug than false positives.

### Hybrid Approach

Contemporary WAFs combine signatures (for known threats) with anomaly scoring (for novel threats). Signatures detect low-hanging fruit; anomaly scoring catches creative bypasses.

---

## OWASP Core Rule Set (CRS)

OWASP CRS is the most widely deployed open-source WAF ruleset, providing protection against OWASP Top 10 vulnerabilities. Implemented in ModSecurity (Apache/nginx) and commercial WAFs.

### Rule Organization

CRS is divided into rule files organized by threat category:

- **REQUEST-900-EXCLUSION-RULES-BEFORE-CRS.conf** — Exceptions (whitelist specific requests to avoid false positives).
- **REQUEST-901-INITIALIZATION.conf** — CRS initialization (variables, defaults).
- **REQUEST-920-PROTOCOL-ENFORCEMENT.conf** — HTTP protocol compliance (invalid methods, malformed headers, excessive request sizes).
- **REQUEST-921-PROTOCOL-ATTACK.conf** — HTTP protocol attacks (HTTP request smuggling, HTTP response splitting).
- **REQUEST-930-APPLICATION-ATTACK-LFI.conf** — Local File Inclusion (attempts to read /etc/passwd, ../ sequences).
- **REQUEST-931-APPLICATION-ATTACK-RFI.conf** — Remote File Inclusion (http:// or ftp:// in parameters).
- **REQUEST-932-APPLICATION-ATTACK-RCE.conf** — Remote Code Execution (shell metacharacters, script injection).
- **REQUEST-933-APPLICATION-ATTACK-PHP.conf** — PHP-specific attacks (PHP code injection, PHP wrappers).
- **REQUEST-941-APPLICATION-ATTACK-XSS.conf** — Cross-Site Scripting (script tags, event handlers, DOM APIs).
- **REQUEST-942-APPLICATION-ATTACK-SQLI.conf** — SQL Injection (SQL keywords, comment syntax, conditional logic).
- **REQUEST-943-APPLICATION-ATTACK-SESSION-FIXATION.conf** — Session fixation (JWT tampering, cookie manipulation).
- **REQUEST-949-BLOCKING-EVALUATION.conf** — Final scoring and decision logic.

### Paranoia Levels

CRS supports **paranoia levels** (1-4), increasing strictness:

- **Level 1:** Default; minimal false positives. Catches most attacks.
- **Level 2:** Stricter rules; detects more edge cases. Some false positives on legitimate requests.
- **Level 3:** Very aggressive; significant false positive risk. Used for high-security applications.
- **Level 4:** Extreme; blocks many legitimate patterns. Often requires extensive whitelisting.

Organizations typically run Level 1 or 2 in production. Higher levels are used in testing or very sensitive environments (financial, healthcare, government).

### Execution Phases & Rule Chaining

Rules run in distinct phases within a request lifecycle:

1. **Phase 1: REQUEST-HEADERS** — Evaluate incoming HTTP headers.
2. **Phase 2: REQUEST-BODY** — Evaluate request body (POST data, multipart uploads).
3. **Phase 3: RESPONSE-HEADERS** — Evaluate response headers from the server.
4. **Phase 4: RESPONSE-BODY** — Evaluate response body (for data leakage).
5. **Phase 5: LOGGING** — Final logging and decision.

Rules can set variables in one phase and use them in another, enabling stateful rule chaining.

---

## Evasion Techniques & Bypass Strategies

### Encoding & Obfuscation

Attackers evade signature rules via encoding:

- **URL encoding:** %2B for +, %27 for single quote. WAF must decode to detect.
- **HTML entities:** &#60; for <, &lt; for <script> tags.
- **Hex & octal encoding:** \x27 for single quote, \047 for octal.
- **Double-encoding:** %252F for /, confusing decoders.
- **Unicode normalization:** Different byte sequences representing the same character.

**Mitigation:** WAF must normalize input (URL decode, HTML decode, convert to canonical form) before rule matching.

### Protocol Ambiguity

- **HTTP request smuggling:** Craft requests with ambiguous Content-Length / Transfer-Encoding headers, exploiting differences in parser interpretation between WAF and backend server. WAF sees a benign request; backend sees an injected one.
- **Character set confusion:** Set charset to an unusual encoding (shift_jis, euc-jp); multi-byte sequences bypass ASCII-based filters.
- **Whitespace tricks:** Insert tabs, newlines, or unusual whitespace; some parsers skip whitespace, others don't.

**Mitigation:** Normalize HTTP parsing alignment between WAF and backend; standardize character sets.

### Polyglot & Context-Specific Evasion

- **SQL injection variants:** Use SQL comments (--), stacked queries, conditional logic (IF, CASE) to hide intent from signatures.
- **XSS variants:** Use SVG, data: URIs, event handlers on non-script elements (img onerror=, input onfocus=).
- **Command injection:** Use command substitution $(cmd), backticks, or process redirection /dev/tcp/

**Mitigation:** Deeper parsing (understanding SQL syntax, HTML parsing, shell syntax) improves detection. Behavioral analysis (detecting anomalies, not just patterns) catches variants.

### Zero-Day Exploitation

Attackers sometimes exploit WAF rule gaps—behaviors the rules don't cover. Example: a WAF blocks SQL keywords but doesn't block parameterized SQL if the parameter contains encoded keywords. Mitigation: regular rule updates, threat intelligence integration, behavioral anomaly detection.

---

## WAF Tuning & False Positive Management

### Root Cause Analysis (RCA)

When legitimate traffic is blocked (false positive), investigate:

1. **Which rule triggered?** Check WAF logs for rule ID and matched pattern.
2. **Why is the pattern in the legitimate request?** Analyze the application. (A product database might contain "UNION" as a field name.)
3. **Adjust or whitelist?** Either relax the rule (reduce sensitivity) or whitelist the specific request (IP, path, parameter).

### Whitelisting Strategies

- **Rule exclusions:** Disable specific rules for specific paths or parameters. Example: disable SQL injection rule for /admin/search?q= parameter (known to accept legitimate SQL-like input).
- **IP whitelisting:** Trust traffic from specific networks (internal IPs, partner APIs).
- **User-agent exceptions:** Trust specific user agents (internal automation, known testing tools).

**Pitfall:** Over-whitelisting defeats the WAF. Define exceptions narrowly (specific path + rule + condition) to maintain security.

### Threshold Tuning

If using anomaly scoring, adjust the threshold:
- **Lower threshold:** More blocks (higher security, more false positives).
- **Higher threshold:** Fewer blocks (lower security, fewer false positives).

Gradual tuning: start with a high threshold, slowly reduce as the system stabilizes and you understand legitimate traffic patterns.

---

## Cloud WAF Platforms

### AWS WAF

AWS WAF is a managed service that protects CloudFront, Application Load Balancer (ALB), and API Gateway.

**Features:**
- **IP-based rules:** Whitelist/blacklist by IP or IP range.
- **Rate-limit rules:** Block IPs exceeding request threshold.
- **String matching:** Detect strings in headers, body, URL.
- **Geographic restrictions:** Block traffic from specific countries.
- **Managed rules:** Pre-built rulesets (OWASP Top 10, known CVEs, bot detection) maintained by AWS.
- **Custom rules:** Express rules in AWS WAF Query Language.

**Considerations:**
- **No request body inspection by default:** Body inspection adds latency; often disabled on high-volume endpoints.
- **Integration with Shield Standard (included); Shield Advanced (paid) provides DDoS protection.

### Cloudflare WAF

Cloudflare WAF inspects traffic to any domain on Cloudflare's network. Available on Business and Enterprise plans.

**Features:**
- **Custom rules:** Use Cloudflare Rules Language (similar to AWS WAF Query Language).
- **Managed rules:** Pre-configured rulesets for common attacks; updated frequently.
- **Rate limiting:** Limit requests per user, IP, or custom criteria.
- **Bot management:** Detect and challenge bots (separate paid feature).
- **Attack score:** ML-based score indicating likelihood of malicious activity.

**Advantages:**
- **No backend changes:** WAF is transparent; works with any origin.
- **Global edge:** Rules execute at Cloudflare's edge, blocking traffic early.

### Azure Web Application Firewall (WAF)

Integrated with Azure Application Gateway and Front Door. Similar features to AWS WAF.

### Managed vs. Custom Rules

**Managed rules (Moderately Secure):**
- Maintained by vendor; updated regularly for new threats.
- Fewer false positives than aggressive custom rules.
- Limited customization.

**Custom rules (Maximum Control):**
- Tailored to application; higher sensitivity.
- Requires tuning and maintenance.
- Often used alongside managed rules.

---

## Bot Detection & DDoS Mitigation

### Bot Classification

WAFs classify traffic into:
- **Good bots:** Search engines (Googlebot), monitoring services, authorized APIs.
- **Bad bots:** Scrapers, credential stuffers, DDoS botnets.
- **Unknown bots:** Ambiguous user agents or behavioral patterns.

**Detection methods:**
- **User-agent inspection:** Block known bot signatures.
- **Behavioral analysis:** Detect high request rates, rapid scraping, no interaction (clicks, form fills).
- **Challenge tokens:** Issue CAPTCHA or JavaScript challenges to verify human interaction.
- **Reputation data:** Query threat intelligence feeds on IPs.

### DDoS Mitigation

- **Rate limiting:** Drop requests from IPs exceeding rate threshold.
- **Connection limits:** Limit concurrent connections per IP.
- **Geographic filtering:** Block traffic from unexpected regions (crude but effective for many attacks).
- **Upstream DDoS services:** Cloudflare, Akamai, AWS Shield offer volumetric DDoS protection (filtering at scale before reaching origin).

---

## Operational Considerations

### Bypass Risk

No WAF blocks 100% of attacks. Sophisticated attackers will eventually bypass rules. WAF is one layer; defense in depth (input validation, parameterized queries, secure coding) is essential. Applications must never trust WAF protection—validate inputs server-side.

### Performance Impact

WAF inspection adds latency (typically 10-50ms for small requests). High-velocity APIs or websockets may require rule tuning or selective enforcement (inspect only POST/PUT, not GET).

### Logging & Retention

WAF logs can be voluminous. Log rotation and indexing are essential. Store logs for incident investigation (30-90 days typical). Correlation with application logs improves debugging.

### Testing & Rules Evolution

Test rules in "log-only" mode before enforcement. Monitor false positive rates when deploying new rules. Update managed rules regularly (weekly to monthly).

---

## Related Topics

See also: [security-web-application](security-web-application.md), [security-network](security-network.md), [api-error-handling](api-error-handling.md), [security-rate-limiting-defense](security-rate-limiting-defense.md).