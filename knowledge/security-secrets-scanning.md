# Security — Secrets Scanning: Detection, Tools, and Remediation Workflow

## Overview

Secrets scanning detects and prevents accidental commits of sensitive data (API keys, tokens, credentials, private keys, database URLs). Modern scanning operates at multiple stages: pre-commit hooks (prevent local leaks), repository scanning (catch what slipped through), and CI integration (enforce policy). The challenge is balancing detection sensitivity (catch real secrets) with false positive rates (entropy-based patterns produce noise) and operational friction (alert fatigue reduces human response).

---

## Detection Methods

### Entropy-Based Detection

Entropy detectors flag high-randomness strings as potential secrets. The premise: most credentials are cryptographically generated and thus have high Shannon entropy.

**Shannon Entropy Calculation:**
```
H(X) = -Σ p(x) * log₂(p(x))
```

A uniformly random 32-byte base64-encoded string: ~192 bits entropy. Real passwords and tokens often 32+ bits.

**Thresholds:**
- Low: 3.0 bits/char → catches more false positives (UUIDs, hashes)
- Moderate: 3.5-4.0 bits/char → balance
- High: 4.5+ bits/char → strict, misses some real secrets (e.g., simple tokens)

**Limitations:**
- High entropy ≠ secret. UUIDs, git hashes, base64-encoded data trigger false alerts
- Some legitimate secrets (passwords with repeating chars) have low entropy
- Entropy varies by character set (base64 vs hex vs alphanumeric)

### Pattern-Based Detection

Regex patterns match known secret formats:

```regex
# AWS Access Key
AKIA[0-9A-Z]{16}

# RSA Private Key Header
-----BEGIN RSA PRIVATE KEY-----

# GitHub Personal Access Token (classic)
ghp_[A-Za-z0-9_]{36,255}

# Stripe API Key
sk_live_[0-9a-zA-Z]{20,}

# Slack Token
xox[abps]-[0-9a-zA-Z]{10,48}

# Database URL
postgres://user:password@host:5432/db
```

**Sensitivity:**
- Specific patterns (e.g., `sk_live_` prefix for Stripe) produce few false positives
- Generic patterns (e.g., any base64 blob > 20 chars) produce many false positives
- Patterns drift as vendors change token formats

### Machine Learning & Entropy Context

Advanced tools combine entropy baselines with linguistic context:
- Keywords near the suspected secret (`api_key=`, `secret:`, `password:`)
- File type and path (`.env` files more likely to contain secrets than JSON schemas)
- Commit message context ("added credentials" vs "updated docs")

---

## Scanning Tools

### Gitleaks

**Focus:** High-signal pattern matching with vendor-specific rules.

**Method:** Regex rules organized by issuer (AWS, GitHub, Slack, etc.). Default rule set covers 100+ vendors.

**Configuration:**
```toml
[rules.s3_bucket]
regex = '''(?i)s3://[a-z0-9.\-]+'''
entropy = 3.5
```

**Invocation:**
```bash
gitleaks detect --source github --config rules.toml
gitleaks protect --staged  # Pre-commit hook

# Find secrets in history
gitleaks detect --source filesystem --verbose
```

**Strengths:**
- Minimal false positives (vendor-specific rules)
- Fast scanning
- Pre-commit hook support
- GitHub Actions integration

**Weaknesses:**
- Requires curated rule sets; can miss custom token formats
- No entropy-based catch-all (intentional design choice)

### TruffleHog

**Focus:** Context-aware entropy detection with verified findings.

**Method:** Scans for high-entropy strings, cross-references with issuer APIs to verify if credential is live (reduces noise).

**Configuration:**
```bash
trufflehog filesystem /path/to/repo \
  --json \
  --json-legacy \
  --max-depth 3

# Custom entropy threshold
trufflehog filesystem . --entropy 4.0
```

**Verification:**
- Stripe: Tests live key against Stripe API
- GitHub: Queries `api.github.com/user` with token
- AWS: Attempts to fetch caller identity

**Strengths:**
- Verifies discoveries (known-live secrets, not false alarms)
- Catches unexpected token formats
- Built-in verification reduces analyst workload

**Weaknesses:**
- API verification can be slow (one HTTP call per finding)
- Verification only works for public APIs; custom internal tokens unverified
- Higher entropy threshold (default ~4.0) misses some secrets

### Detect-Secrets (Yelp)

**Focus:** Plugin architecture combining entropy and keyword-based detection.

**Method:** Configurable detectors: entropy, keyword lists, provider-specific patterns (similar to gitleaks).

**Configuration:**
```yaml
detectors:
  - type: Basic
    entropy: 4.0
  - type: Keyword
    keywords:
      - "password"
      - "secret_key"
  - type: AWS
```

**Baseline Mode:**
Detects only *new* secrets (compares scan against stored baseline), reduces noise in existing codebases.

```bash
# Scan with baseline; only report NEW findings
detect-secrets scan --baseline .secrets.baseline

# Audit findings
detect-secrets audit .secrets.baseline
```

**Strengths:**
- Baseline mode minimizes alert fatigue on legacy code
- Plugin system extensible to custom patterns
- Plaintext `.baseline` file enables auditability

**Weaknesses:**
- Keyword detection prone to false positives (any string near "secret" is flagged)
- Baseline must be manually maintained and audited

### GitHub Secret Scanning

**Platform-native detection** on GitHub.com (public and private repos).

**Coverage:** 100+ secret patterns from GitHub and vendor partners; custom patterns for Enterprise.

**Behavior:**
- Real-time scanning on push
- Automatic notification to user and admin
- Optional: Require remediation (push block) for private repos
- Webhook and REST API for custom workflows

**Alert lifecycle:**
```
Push detected → Pattern matched → (Optional) Verify with issuer
                              → Notify user → User dismisses or revokes
```

**Limitations:**
- Patterns controlled by GitHub (Enterprise can add custom)
- Public repo alerts sent to issuer for revocation (e.g., GitHub notifies Stripe)
- Organization settings override per-repo controls

---

## False Positives and Remediation

### Common False Positives

| Pattern | Trigger | Root Cause |
|---------|---------|-----------|
| UUIDs in code | High entropy string `550e8400-e29b-41d4-a716-446655440000` | Exact entropy match to secret thresholds |
| Example credentials | `password: "changeme123"` in docs | Keywords + modest entropy |
| Hashes (MD5, SHA) | Hex strings in test fixtures | Entropy overlap with real secrets |
| Git commit SHAs | Long hex string in history | Same appearance as hashes |
| Configuration schemas | `db_url: "postgres://user:pass@host/db"` | Generic database URL pattern |

### Suppression Strategies

**Inline comments (tool-specific):**
```javascript
// gitleaks:allow
const exampleToken = "ghp_1234567890abcdefghijklmnopqrstuvwxyz";
```

**Baseline exclusion (detect-secrets):**
```bash
detect-secrets audit .secrets.baseline
# Mark finding as OK (adds to allowlist)
```

**GitHub token allowlist (Enterprise):**
```json
{
  "patterns": [
    {
      "pattern": "gitleaks",
      "value": "1234567890abcdef",
      "reason": "Test fixture in hardcoded tests"
    }
  ]
}
```

**Path exclusion:**
```toml
[allowlist]
paths = ["docs/examples", ".github/fixtures"]
```

---

## Pre-Commit Integration

### Hook Workflow

```bash
#!/bin/bash
# Pre-commit hook

# Check staged files for secrets
gitleaks protect --staged

if [ $? -ne 0 ]; then
  echo "Secrets detected. Prevent commit." >&2
  exit 1
fi

exit 0
```

**Tradeoffs:**
- **Prevents accidents:** Stops accidental commits at developer machine
- **Friction:** Adds latency (500ms–2s per commit)
- **Bypass temptation:** `git commit --no-verify` defeats hook
- **Maintenance:** Hook must be distributed to all developers (pre-commit framework, Git template dir, setup script)

### Enforcement via CI

```yaml
# GitHub Actions
name: Secret Scanning
on: [push, pull_request]

jobs:
  scan:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
        with:
          fetch-depth: 0
      - name: TruffleHog Scan
        run: |
          pip install truffleHog
          trufflehog filesystem . --json > findings.json
          if [ -s findings.json ]; then
            cat findings.json
            exit 1
          fi
```

**Advantages:**
- Enforced regardless of local hooks
- Detects secrets in force-pushed history
- Can verify findings before failing build

**Disadvantages:**
- Post-commit detection (damage already done to repo history)
- Requires urgent remediation workflow

---

## Remediation Workflow: Post-Detection

### Timeline Pressure

Once a real secret is committed:
- **Assume compromise:** Attacker may have accessed the repo via GitHub API, mirrors, or GitHub's index crawlers
- **Immediate:** Revoke credential (rotate API key, reset password, invalidate token)
- **Hours:** Remove secret from git history (force-push or `git-filter-repo`)
- **Days:** Audit logs for unauthorized use of leaked credential

### Key Rotation vs. History Rewriting

**Immediate (minutes):**
1. Revoke credential in the issuer's system (GitHub, AWS console, database)
2. Create new credential
3. Update code to use new credential

```bash
# Example: AWS
# Console: Delete leaked access key
# CLI: Create new key
aws iam create-access-key --user-name automation

# Update code
sed -i 's/AKIA.*SECRET.*/NEW_KEY/g' .env
git add .env
git commit -m "Rotate AWS key (leaked key revoked)"
git push
```

**Follow-up (hours):**
1. Remove from git history using `git-filter-repo` or `git rebase`
2. Force-push to update references (repo history is changed)

```bash
# Remove secret from all history
git filter-repo --invert-paths --path secrets.txt

# Or remove lines containing pattern
git filter-repo --replace-text <(echo "s/AKIA[A-Z0-9]{16}.*//")

git push -f origin main
```

**Note:** Force-push has implications (breaks clones, requires `force_with_lease` or branch protection updates).

### Audit & Prevention

- Query issuer API: "Was this key ever used? From which IPs?"
- Check logs for unauthorized API calls, data access
- If compromised: Revoke related credentials (session tokens for same user)
- Implement detection: Monitor for future accidental commits

---

## Noise Reduction Techniques

### Tool Tuning

1. **Entropy threshold:** Raise from 3.0 to 4.0+ to exclude common false positives (UUIDs, hashes)
2. **Keyword context:** Require keywords like `secret`, `password`, `key` nearby (not just entropy)
3. **File type filtering:** Scan only risky extensions (`.env`, `.pem`, `.p12`) or specific directories
4. **Baseline mode:** Only alert on new findings, ignore legacy code

### Organizational Policy

1. **Examples in docs:** Colocate example credentials with "example/test" markers and path allowlists
2. **Test fixtures:** Centralize test data in sandboxed directories; exclude from scanning
3. **Rule review:** Monthly audit of false positives; disable noisy rules
4. **Alert triage:** Dedicated on-call person for secret alerts; route to security team

---

## See Also

- [Secrets Management](security-secrets-management.md) — Storage, rotation, Vault, dynamic secrets
- [Authentication Attacks](security-authentication-attacks.md) — Credential stuffing, brute force defense
- [GitHub Actions Security](devops-github-actions-deep.md) — Protecting secrets in CI/CD