# Security Testing — SAST, DAST, IAST, and SCA Tools

## Overview

Security testing automates detection of vulnerabilities in code, dependencies, and running systems. Tools scan for common attack vectors — SQL injection, cross-site scripting (XSS), weak cryptography, insecure dependencies — that humans miss and manual testing doesn't reveal.

See [security-devsecops.md](security-devsecops.md) for shift-left strategy and organizational integration. This note focuses on testing tools and CI/CD integration.

## SAST: Static Application Security Testing

SAST analyzes source code without running it. Scanners parse code, build abstract syntax trees, and detect patterns that indicate vulnerabilities.

### How SAST Works

1. **Parse code** into AST (abstract syntax tree)
2. **Find patterns** matching vulnerability signatures (e.g., `SQL = query("SELECT * FROM users WHERE id = " + user_input)`)
3. **Trace data flow** to confirm if untrusted input flows to dangerous sinks
4. **Report findings** with location, severity, remediation guidance

### SonarQube

SonarQube is a centralized code quality and security platform used by large organizations.

**Strengths**:
- Analyzes ~30 languages
- Tracks historical trends; shows code quality trajectory
- Community profiles for guidelines (OWASP Top 10, CWE, PCI DSS)
- Web dashboard for team visibility
- Integrates with CI/CD, IDEs, repositories

**Workflow**:
1. CI pipeline or IDE runs `sonar-scanner` on code
2. Scanner uploads findings to SonarQube server
3. Server analyzes and stores results
4. Web dashboard reports security issues, code smells, duplicates
5. PR integrations block merges if security gates fail

**Example SonarQube rule** (pseudo-SQL injection detection):
```
pattern: SQL = query("SELECT * FROM users WHERE id = " + untrustedVar)
severity: CRITICAL
remediation: Use parameterized queries / prepared statements
```

**Limitations**:
- Requires centralized server; not lightweight for small teams
- False positive rate can be high; requires tuning
- Triage overhead; security team must review and prioritize findings

### Semgrep

Semgrep is a rule-engine SAST tool designed for developer velocity. Write rules in a simple YAML syntax; run locally or in CI.

**Strengths**:
- Rules written in simple YAML; no deep language-theory needed
- Runs locally without server; instant feedback in IDE
- Lightweight; integrates into git hooks
- Open-source registry of ~1000 rules covering OWASP Top 10, CWE, language-specific issues

**Example Semgrep rule** (detecting hardcoded secrets):
```yaml
rules:
  - id: hardcoded-secret-api-key
    pattern: |
      api_key = "sk_live_..."
    message: API key hardcoded in source
    severity: CRITICAL
    languages: [python, javascript]
```

**Workflow**:
```bash
# Local scan
semgrep --config=p/owasp-top-ten .

# In CI
semgrep --config=p/security-audit . --json > results.json
```

**Tradeoff**: Semgrep is developer-friendly but less feature-rich than SonarQube (no long-term trend tracking, limited dashboard). Use Semgrep for fast iteration; SonarQube for organizational oversight.

### SAST Challenges

1. **False positives**: Signatures match benign patterns. "SQL = query('SELECT * FROM logs WHERE msg = $1', msg)" (parameterized) trips SQL-injection rule.
2. **False negatives**: Complex control flow, indirect sources, library calls aren't always tracked. Real vulnerabilities slip through.
3. **Noise**: High-volume findings overwhelm security teams. Requires triage, filtering, re-baselining.
4. **Architectural blindness**: SAST sees code, not deployment. Missing context about network isolation, WAF rules, authentication.

**Mitigation**:
- Tune rules; disable high-FP rules in your codebase
- Pair SAST with DAST; complement static analysis with runtime testing
- Establish triage SLAs; prioritize critical findings, defer low-severity items

## DAST: Dynamic Application Security Testing

DAST probes running applications for vulnerabilities. Tools send crafted requests, observe responses, and detect security flaws.

### How DAST Works

1. **Crawl application**: Follow links, find endpoints
2. **Inject payloads**: Send malicious inputs (e.g., `' OR '1'='1`) to find injection flaws
3. **Observe responses**: Look for error messages, XML/JSON parsing failures, unusual behavior
4. **Report findings**: Categorize by OWASP Top 10

DAST detects runtime vulnerabilities that SAST misses: authentication flaws, session management bugs, insecure deserialization of dynamically loaded data.

### OWASP ZAP (Zed Attack Proxy)

ZAP is an open-source, lightweight DAST tool. Designed for security developers and penetration testers.

**Strengths**:
- Open-source; no licensing cost
- Sits between client and server; intercepts traffic, modifies requests
- Passive scan (watch traffic) and active scan (send payloads)
- Supports authentication workflows (login, session handling)
- Extensible via plugins

**Workflow**:
```bash
# Passive scan (observe traffic, no injection)
zaproxy -cmd -option alert.overrides.skcanners='' -url https://app.test

# Active scan (with payloads)
zaproxy -cmd -config replacer.full_list\(0\).enabled=true \\
  -config replacer.full_list\(0\).matchType=REQ_HEADER \\
  -url https://app.test -quickurl https://app.test
```

**Limitations**:
- Requires running instance of application; testing window is limited
- Slow for large applications (many endpoints, deep crawl)
- High false positive rate without tuning

### Burp Suite

Burp Suite (Professional) is the industry standard for penetration testing and DAST. Used by security practitioners and enterprises.

**Strengths**:
- Most comprehensive DAST scanner
- Macro recording for complex workflows (multi-step login, CSRF tokens)
- Extensible via Burp extensions (Python, Java)
- Detailed reporting; compliance templates (PCI DSS, HIPAA)

**Workflow**:
1. Configure Burp as HTTP proxy
2. Browse application naturally; Burp passively scans traffic
3. Trigger active scan on endpoints of interest
4. Review findings, generate report

**Limitation**: Expensive (~$400/year); overkill for small teams.

**Burp Community** (free tier):
- Limited to passive scanning + manual testing
- Recommended for learning, not production use

### DAST Challenges

1. **Time cost**: Active scanning can take hours for large applications
2. **Requires running app**: Can't scan without infrastructure; limits CI/CD integration
3. **Coverage gaps**: Application crawlers miss dynamically loaded content, hidden endpoints
4. **False positives**: Injection payloads bypass WAF/input validation but don't harm system (false alarm)

**Mitigation**:
- Run DAST nightly or on-demand, not on every commit
- Disable aggressive payloads in CI; run full scan in staging
- Combine with SAST; use DAST to verify SAST findings

## IAST: Interactive Application Security Testing

IAST instruments running applications (via agents) to monitor data flow and detect vulnerabilities from within the application.

How it works:
1. **Agent installed** in application (JVM agent, Node.js module)
2. **Tracks tainted data**: Marks untrusted inputs (query params, POST body)
3. **Observes sinks**: When tainted data flows to dangerous operations (SQL query, OS command), agent detects
4. **Reports findings**: Name of sink, input source, data flow path

**Advantages over SAST/DAST**:
- Perfect visibility into data flow; no guessing whether input reaches sink
- Detects runtime context: checks if WAF or input validation stopped attack
- Lower false positive rate; confirms vulnerability path

**Challenges**:
- Requires agent installation; not all environments support it
- Performance overhead; agent slows application
- Licensing costs (Contrast, Synopsys, etc.)

**Tool example: Contrast Security**:
- Installed as Java agent, .NET agent, or Node.js package
- Monitors application; reports vulnerabilities to dashboard
- Integrates with SIEM, ticketing systems

IAST is best for high-security applications where false positives cause too much noise.

## SCA: Software Composition Analysis

SCA scanners identify vulnerable dependencies. Tools:
1. **Parse dependency manifests** (package.json, requirements.txt, pom.xml)
2. **Check against CVE databases** (NVD, GitHub Security Advisory, Snyk database)
3. **Report vulnerable versions** with fix guidance

### Snyk

Snyk is a developer-friendly SCA platform. Focuses on fixing over reporting.

**Strengths**:
- Integrated vulnerability database; daily updates
- Suggests fixes (minor version bumps, patches)
- Offers auto-remediation pull requests (GitHub, GitLab)
- CLI, web UI, CI/CD integrations, IDE plugins
- Free tier for open-source projects

**Workflow**:
```bash
# CLI scan
snyk test

# Auto-fix (generates PR)
snyk fix --pr

# CI integration
snyk test --fail-on=all  # Fail if any vulnerability
```

**Limitation**: Snyk's fix suggestions can introduce breaking changes; reviews needed.

### GitHub Dependabot

GitHub's native dependency scanning. Free for public and private repos.

**Strengths**:
- No external vendor required; runs on GitHub infrastructure
- Auto-creates pull requests with updated versions
- Grouped updates reduce PR volume
- Native to GitHub; no additional login

**Limitations**:
- Integrated scanning only (no CLI)
- Less frequent update cycle than Snyk
- Basic reporting; limited security insights

**Workflow**: Enable in repository settings; Dependabot automatically creates PRs.

### npm audit / cargo audit

Language-specific tools bundled with package managers.

```bash
npm audit              # Report vulnerabilities
npm audit fix          # Auto-fix
npm audit fix --force  # Force updates (may break compatibility)

cargo audit            # Rust dependencies
```

**Limitations**: Don't fix automatically; manual effort to update and test.

### SCA Challenges

1. **Transitive dependencies**: Vulnerable package three levels deep; hard to prioritize fixes
2. **False urgency**: High-severity CVEs often apply only to specific use-cases; not all exploitable
3. **Over-remediation**: Fixing every CVE can introduce breaking changes
4. **License compliance**: SCA tools also detect license violations (GPL, proprietary)

**Mitigation**:
- Audit CVE applicability before fixing (does the vulnerability apply to your usage?)
- Prioritize by CVSS score + exploitability
- Test fixes thoroughly before merging

## Container Scanning

SAST, DAST, SCA scan source code and dependencies. Container scanners inspect built container images for vulnerabilities.

### Trivy

Trivy scans container images for:
- **OS package vulnerabilities** (apt, yum packages in base image)
- **Application dependencies** (embedded Node modules, Python packages)
- **Configuration issues** (hardcoded secrets, misconfigurations)

**Workflow**:
```bash
# Scan local image
trivy image myapp:latest

# Scan in CI (fail on high severity)
trivy image --severity HIGH,CRITICAL myapp:latest --exit-code 1
```

**Output**: List of vulnerabilities, layers they were introduced in, fix guidance.

### Grype

Grype (from Anchore) is another container scanner. Similar to Trivy; supports multiple scanners via plugins.

```bash
grype myimage:latest
grype --fail-on high dir:.  # Also scan directories
```

## CI/CD Integration Pattern

Secure CI/CD pipelines chain multiple scanners:

```yaml
# .github/workflows/security.yml
security-tests:
  runs-on: ubuntu-latest
  steps:
    # 1. SAST
    - name: SAST Scan
      run: semgrep --config=p/security-audit . --json > sast.json
    
    # 2. SCA
    - name: SCA Scan
      run: snyk test --json > sca.json
    
    # 3. Build and scan image
    - name: Build Image
      run: docker build -t myapp:${{ github.sha }} .
    
    # 4. Container Scan
    - name: Container Scan
      run: trivy image myapp:${{ github.sha }} --exit-code 1
    
    # 5. Upload results to security dashboard
    - name: Upload Results
      run: curl -X POST https://security.internal/ingest -d @sast.json
```

## Triage and False Positives

Security findings require triage:

1. **Confirm findings**
   - Re-run tool with different settings
   - Verify finding is reproducible
   - Check if it's been previously dismissed

2. **Assess exploitability**
   - Does the vulnerability apply to your usage? (e.g., XSS in admin panel users only)
   - Are there compensating controls? (WAF, authentication, network isolation)
   - What's the realistic attack surface?

3. **Prioritize by risk**
   - CVSS score alone is insufficient; consider context
   - Critical RCE in internet-facing service = high priority
   - Critical SQLi in internal admin panel = lower priority
   - Track metrics: mean time to remediation (MTTR), open vulnerabilities trend

4. **Establish workflows**
   - Triage SLA: security team reviews findings within N days
   - False positive procedure: mark as "not applicable" with reason
   - Escalation: critical findings → immediate response; low → backlog

## Summary

Use SAST (SonarQube, Semgrep) to catch code-level vulnerabilities early. Use DAST (OWASP ZAP, Burp) in staging to verify runtime security. Use SCA (Snyk, Dependabot) to track vulnerable dependencies. Scan container images (Trivy, Grype) before deployment. Chain scanners in CI/CD to prevent vulnerable code from reaching production. Establish triage procedures to filter noise and prioritize real risks.