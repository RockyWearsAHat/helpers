# DevSecOps — Shift-Left Security, Automated Testing & Threat Modeling in CI/CD

## Overview

DevSecOps integrates security into every phase of the software development lifecycle (SDLC) rather than bolting it on after deployment. "Shift-left security" means catching vulnerabilities early (design, development) rather than late (testing, production), when fixes are cheaper and easier.

## Shift-Left Security

Traditional security model: developers ship code → QA tests → security team scans pre-deployment → vulnerabilities found (expensive to fix, delays release).

Shift-left model: developers scan code locally → CI/CD pipeline runs automated security checks → manual security review → deployment.

**Benefits:**
- Vulnerabilities fixed in development (1–10x cheaper than post-deployment)
- Reduced time to remediation (minutes, not weeks)
- Developers own security quality; security team provides guidance
- Continuous feedback loop improves developer skills

**Challenges:**
- Developers need training on secure coding
- Must balance security checks with developer productivity (avoid alert fatigue)
- False positives slow down releases

## Security Testing in the Pyramid

Similar to testing pyramid (unit → integration → E2E), security testing spans layers:

```
      Manual Penetration Testing
      (Post-deployment, external attackers)
    ┌──────────────────────────────┐
    │  Dynamic Application Testing  │
    │  (DAST) — Runtime behavior    │
    ├──────────────────────────────┤
    │  Interactive Application      │
    │  Security Testing (IAST)      │
    ├──────────────────────────────┤
    │  Container & IaC Scanning     │
    │  Dependency Scanning (SCA)    │
    ├──────────────────────────────┤
    │  Static Analysis (SAST)       │
    │  Secrets scanning             │
    └──────────────────────────────┘
```

**Execution timing:**
- SAST, secrets, SCA: Developer commit / pull request (immediate feedback)
- DAST, IAST: CI/CD pipeline post-build (staging environment)
- Container scanning: Post-image build; pre-registry push
- Manual testing: Pre-production, external penetration testing

## SAST (Static Application Security Testing)

Analyzes source code without executing it, looking for:

- **Injection vulnerabilities:** SQL injection, OS command injection (code patterns that concatenate user input into queries)
- **Authentication & authorization flaws:** Hardcoded credentials, missing permission checks
- **Insecure cryptography:** Use of weak / broken ciphers, deprecated APIs
- **Data validation gaps:** Missing input sanitization
- **Information disclosure:** Logging passwords, PII, secrets

**Tools:** SonarQube, Checkmarx, Fortify, Semgrep, CodeQL (GitHub Advanced Security).

**Execution in CI/CD:**
```yaml
# GitHub Actions example
- name: Run SAST
  run: sonar-scanner -D:sonar.sources=./src
```

**Trade-offs:**

| Strength | Weakness |
|----------|----------|
| Catches bugs early (at development time) | High false-positive rate (requires tuning) |
| Works with source code access | Cannot detect runtime/business logic flaws |
| Scales easily (no infrastructure needed) | Language-specific; tools vary in quality |
| Integrates into IDE + pull request | Requires developer discipline to fix issues |

**Configuration:** Most SAST tools have tunable rules; balance security strictness (catch more vulns) vs. developer velocity (fewer false positives).

## DAST (Dynamic Application Security Testing)

Tests running applications by sending malicious inputs and observing responses. Black-box perspective (attacker viewpoint).

**Detects:**
- Server-side request forgery (SSRF)
- Cross-site scripting (XSS) — verifies escaping works
- Broken access control — attempts to access resources without authorization
- Insecure deserialization
- Path traversal
- Broken authentication

**Tools:** Burp Suite, Tenable Nessus, OWASP ZAP.

**Execution in CI/CD:**
```yaml
# Run DAST in staging environment after deployment
- name: Deploy to staging
  run: deploy.staging.sh

- name: Run DAST scan
  run: zap-cli scan https://staging.app.example.com
```

**Trade-offs:**

| Strength | Weakness |
|----------|----------|
| Tests real application; detects logic flaws SAST misses | Slow (minutes to hours per scan) |
| No source code needed | Shallow; may not discover all code paths |
| No false positives (actual findings are bugs) | Requires deployed instance; not in PR feedback |
| Mimics attacker behavior | Configuration complex (authentication, workflows) |

## SCA (Software Composition Analysis)

Scans dependencies (libraries, frameworks, open-source packages) for known vulnerabilities.

**Detects:**
- Published CVEs in dependencies
- License compliance issues (GPL, proprietary)
- Outdated packages
- Packages with high vulnerability density

**Tools:** Snyk, WhiteSource, Black Duck, OWASP Dependency-Check, GitHub Dependabot.

**Execution in CI/CD:**
```yaml
- name: Dependency check
  run: dependency-check --project myapp --scan .
```

**Architecture:**
- SCA tool maintains CVE database (National Vulnerability Database + vendors)
- Parses dependency manifests (package.json, pom.xml, Gemfile, requirements.txt, go.mod)
- Cross-references versions against known CVEs
- Alerts on matches

**Trade-offs:**

| Strength | Weakness |
|----------|----------|
| Catches known vulns in dependencies | Delays for new CVE disclosure |
| Automates patch recommendations | High false-positive rate (many CVEs missing functional exploit paths) |
| Early in supply chain | Does not catch zero-days |
| Quick execution | Requires network access (CVE database updates) |

## IAST (Interactive Application Security Testing)

Instruments application code at runtime to observe behavior and identify vulnerabilities.

**Approach:**
- Deploy agent inside application process
- Agent monitors data flow (taint tracking): where does user input go?
- Flags if untrusted data reaches sensitive sinks (SQL queries, file operations, command execution)
- Sends findings back to IAST platform

**Detects:**
- Injection vulnerabilities (with runtime context: "user param flows into SQL")
- Broken access control (observes authorization checks)
- Path traversal
- Information disclosure

**Tools:** Contrast Security, Checkmarx Interactive, Acunetix.

**Advantages over SAST/DAST:**
- Fewer false positives (runtime context)
- Detects vulnerabilities DAST misses (internal APIs, complex workflows)
- Automatic proof-of-concept generation

**Trade-off:** Requires application instrumentation; slight runtime overhead.

## Container Scanning

Analyzes container images for vulnerabilities before pushing to registry.

**Scans:**
- Base OS image (Alpine, Ubuntu vulnerabilities)
- Application dependencies (same as SCA)
- Misconfiguration (running as root, exposed ports, unnecessary packages)

**Tools:** Trivy, Clair, Grype, registry-native scanning (Docker Hub, ECR, GCR include scanning).

**Execution:**
```bash
trivy image myapp:v1.0
```

**Output:** List of CVEs by layer + severity.

**Best practice:** Scan on every build; reject images with Critical vulnerabilities; allow High with approval.

## IaC (Infrastructure as Code) Scanning

Validates infrastructure configuration (Terraform, CloudFormation, Kubernetes manifests) for security misconfigurations.

**Detects:**
- Overly permissive security groups / firewall rules
- Unencrypted data stores (RDS, S3)
- Missing authentication / TLS
- Hardcoded secrets / API keys
- Excessive IAM permissions

**Tools:** Checkov, Terraform Security Scanner, Kyverno (Kubernetes).

**Example:**
```yaml
# Checkov: Flag insecure S3 bucket
resource "aws_s3_bucket" "example" {
  bucket = "my-bucket"
  # ❌ Checkov error: Bucket versioning not enabled (CKV_AWS_21)
}
```

**Execution in CI/CD:**
```bash
checkov -d ./terraform/
```

**Benefit:** Prevents configuration drift; catches misconfigurations before infrastructure provisioned.

## Secrets Scanning

Prevents committing credentials (API keys, database passwords, private keys) to version control.

**Techniques:**
- Pattern matching: regex for AWS key format (`AKIA` + 16 alphanumeric)
- Entropy analysis: random-looking strings likely secrets
- Known secret formats: private key headers (`-----BEGIN RSA PRIVATE KEY-----`)

**Tools:** TruffleHog, git-secrets, Detective, Gitleaks.

**Execution:**
```bash
# Pre-commit hook: scan for secrets before commit allowed
gitleaks detect --source local
```

**Workflow:**
1. Developer attempts commit with secret
2. Pre-commit hook runs; detects secret
3. Commit rejected with message "Remove secret, then retry"
4. If secret accidentally committed, rotate credential immediately + remove from history

## Security Champions Program

Embeds security mindset into dev teams without creating security bottleneck.

**Model:**
- Designate 1–2 security champions per team (developers with security interest)
- Champions receive security training (OWASP, threat modeling, secure coding)
- Champions review code for security; gate pull requests; mentor teammates
- Champions escalate architectural security decisions to central security team

**Benefits:**
- Decentralizes security responsibility; faster decisions
- Improves team security maturity
- Reduces burden on central security team

**Challenge:** Requires time investment; champions need training.

## Threat Modeling in Agile

Traditional threat modeling (lengthy documentation) conflicts with agile (rapid iteration). Lightweight threat modeling fits:

**Lightweight approach (per sprint / feature):**
1. Draw data flow: Where does user data go? (frontend → API → database)
2. Identify trust boundaries (external networks, databases)
3. Brainstorm: How could attacker compromise this? (intercept data, inject SQL, brute-force auth)
4. Mitigate: What controls reduce risk? (encryption, parameterized queries, rate limiting)
5. Residual risk: What risk remains? Accept or mitigate further?

**Formats:**
- **STRIDE:** Spoofing, Tampering, Repudiation, Information Disclosure, Denial of Service, Elevation of Privilege
- **Kill chains:** Reconnaissance → Weaponization → Delivery → Exploitation → Installation → C&C → Actions on Objective (MITRE ATT&CK)

**Integration into dev cycle:**
- Threat modeling in sprint planning for new features
- Tickets created for mitigations (e.g., "add rate limiting to login endpoint")
- Threat model reviewed in security gate (pre-merge)

## Compliance as Code

Codifies compliance requirements (HIPAA, PCI-DSS, SOC 2) into infrastructure policy.

**Examples:**
```hcl
# Terraform Sentinel policy: Enforce encryption on RDS
rule "require_rds_encryption" {
  condition = resources.aws_db_instance.*.storage_encrypted contains true
  enforcement_level = "hard-mandatory"
}
```

**Benefits:**
- Prevents misconfiguration from drift
- Audit trail (policy + evidence)
- Reduces manual compliance checks

## DevSecOps Metrics

Track to measure program effectiveness:

- **Vulnerability discovery speed:** How fast do automated tests find vulns? (faster = better shift-left)
- **Mean time to remediate (MTTR):** How long between vulnerability discovery and fix deployed? Target: ≤ 7 days.
- **Coverage metrics:** % of code scanned by SAST, % of dependencies checked by SCA
- **False-positive rate:** What % of findings are not actual vulnerabilities? (tune rules to reduce)
- **Release velocity:** Are security checks slowing deployments? (should be < 5 min added per build)

## See Also

- API Security
- Static Analysis (SAST, tools & techniques)
- Security Incident Response
- CI/CD Patterns & Pipelines