# Software Supply Chain Security — SLSA, Sigstore, SBOM & Provenance

## Overview

Supply chain security addresses threats to the tools, libraries, build systems, and distribution channels that produce software. A compromised or malicious dependency can cascade to thousands of downstream consumers. Modern supply chain security combines **inventory** (SBOM), **provenance** (where did code come from?), **signing** (prove authenticity), and **policy enforcement** (verify before deployment).

## SLSA Framework: Levels & Controls

**SLSA** (Supply-chain Levels for Software Artifacts) is a gradated security framework defining maturity levels for build integrity, dependency strength, and provenance.

### SLSA Levels

| Level | Build Integrity | Dependency | Provenance | Control |
|-------|-----------------|-----------|-----------|---------|
| **L1** | Scripted build exists; documented process | Track direct deps | Digest provenance exists | Best-effort |
| **L2** | Hermetic build; version-controlled scripts | Pinned versions | Signed provenance; auditable | Automated guardrail |
| **L3** | Reproducible builds; isolated build env | Transitive deps strong | Countersigned provenance; strict audit | Mandatory gate |
| **L4** | Hardened build system; immutable artifacts | All deps cryptographically secure | Hardware-backed signing; formal verification | Air-gapped hardened infra |

**Hermetic build** = build reproducible from source alone; no local state, network calls, or implicit dependencies.

**Reproducible build** = same source → same artifact bit-for-bit.

### SLSA Controls at Each Level

**L1–L3 focus on:**
- Build environment isolation
- Source version control
- Dependency pinning
- Provenance generation and signing
- Audit logging

**L4 adds:**
- Hardware-based attestation (TPM, secure enclave)
- Formal verification of build configuration
- Offline signing ceremonies
- Post-build artifact immutability

## Sigstore Ecosystem: Keyless Signing

Sigstore provides **keyless signing** for software artifacts. Developers authenticate via OIDC (GitHub, Google) instead of managing long-lived cryptographic keys. This dramatically simplifies key rotation and recovery.

### Architecture

```
Developer (GitHub, Google OAuth)
    ↓
Fulcio — Issues short-lived cert bound to OIDC identity
    ↓
cosign — Signs artifact; uploads signature + cert
    ↓
Rekor — Immutable transparency log; proves signature existed
    ↓
Verifier — Checks signature against cert + Rekor log
```

### Components

| Component | Role |
|-----------|------|
| **Fulcio** | CA issuing short-lived certificates (5–10 min) bound to OAuth identity |
| **Rekor** | Transparency log; stores/verifies signed entries; enables audit trail |
| **cosign** | CLI tool for signing/verifying containers and binaries |
| **Gitsign** | Sign git commits with Sigstore |
| **cosign-gha** | GitHub Actions integration; auto-provision OIDC tokens |

### Container Image Signing with cosign

```bash
# Sign with OIDC (GitHub Actions)
cosign sign ghcr.io/org/app@sha256:abc123

# Verify signature
cosign verify --certificate-identity user@example.com \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  ghcr.io/org/app@sha256:abc123

# Attach SBOM to image
cosign attach sbom --sbom sbom.cdx.json ghcr.io/org/app@sha256:abc123

# Verify attestation (SBOM, provenance)
cosign verify-attestation --type cyclonedx \
  ghcr.io/org/app@sha256:abc123
```

**Key insight:** Certificate is ephemeral; validity verified against Rekor log. No cert revocation lists needed.

## In-Toto Attestations

In-toto records **link metadata**: evidence that an action (build, test, sign) happened, who did it, on what inputs/outputs. Chained together, links form a **supply chain model** proving correctness of the software production process.

### Link Structure

```json
{
  "signed": {
    "_type": "link",
    "name": "build",
    "materials": { "src/main.c": { "sha256": "abc..." } },
    "byproducts": { "return-value": 0, "stdout": "..." },
    "environment": { "OS": "Linux" },
    "command": ["make", "build"],
    "products": { "app.bin": { "sha256": "def..." } }
  },
  "signatures": [
    { "keyid": "alice-key", "sig": "..." }
  ]
}
```

### Supply Chain Layout

A layout file (JSON) defines the expected build steps and authorization:

```json
{
  "steps": [
    {
      "name": "clone",
      "retval": 0,
      "expected_products": { "src/**": { "sha256": "..." } }
    },
    {
      "name": "build",
      "functionaries": ["alice"],
      "expected_products": { "app.bin": { "sha256": "..." } }
    }
  ]
}
```

Artifacts can be verified against the layout to prove compliance with expected steps.

## SBOM (Software Bill of Materials)

An SBOM is a machine-readable inventory of software components, versions, licenses, and vulnerabilities.

### SPDX Format

```json
{
  "spdxVersion": "SPDX-2.3",
  "creationInfo": { "created": "2025-03-26T00:00:00Z", "creators": ["Tool: cyclonedx-cli"] },
  "packages": [
    {
      "SPDXID": "SPDXRef-Package",
      "name": "myapp",
      "version": "1.0.0",
      "supplier": "Organization: Company",
      "downloadLocation": "https://github.com/org/app",
      "filesAnalyzed": false
    }
  ],
  "relationships": [
    {
      "spdxElementId": "SPDXRef-Package",
      "relatedSpdxElement": "SPDXRef-lodash",
      "relationshipType": "DEPENDS_ON"
    }
  ]
}
```

### CycloneDX Format

CycloneDX is XML-based, OWASP-maintained. Better support for SaaSBOM, MLBOM, and VEX (Vulnerability Exploitability eXchange).

```xml
<bom xmlns="http://cyclonedx.org/schema/bom/1.5">
  <components>
    <component type="library">
      <name>lodash</name>
      <version>4.17.21</version>
      <purl>pkg:npm/lodash@4.17.21</purl>
    </component>
  </components>
</bom>
```

### SBOM Generation

- **Language/package managers** — `npm sbom`, `pip install cyclonedx-bom`, `cargo sbom`
- **Container images** — `syft`, `grype`, `trivy sbom`
- **Build-time** — Generate during CI/CD; attach to artifacts as provenance
- **Dependency scanning** — Combine with CVE databases to identify vulnerable components

## Dependency Review & Verification

### Pull Request Dependency Review

GitHub, GitLab, and Gitea support **dependency review** in PRs: automatic detection of new/updated dependencies with vulnerability scanning.

```yaml
# GitHub Action
- uses: actions/dependency-review-action@v4
```

### Supply Chain Verification Checks

1. **Signature verification** — Check all dependencies have valid signatures (cosign, GPG)
2. **Provenance verification** — Validate SLSA attestations; reject L0 artifacts
3. **License scanning** — Flag copyleft/restricted licenses before merging
4. **SBOM comparison** — Detect unexpected new dependencies; flag transitive vulns
5. **Pinning enforcement** — Reject version ranges; require lock files

### npm Provenance

npm registry now supports `--publish-provenance` with GitHub Actions OIDC:

```bash
npm publish --provenance
```

Published packages show a "Provenance" badge linking to GitHub Actions run that built them.

## End-to-End Supply Chain Example

```
1. Developer pushes code → GitHub (signed with Sigstore)
2. GitHub Actions runs CI:
   - Build artifact (hermetic, pinned deps)
   - Generate SBOM (syft)
   - Generate in-toto link (build metadata)
3. cosign signs artifact digest + SBOM
4. Rekor logs signature (immutable proof)
5. Image pushed to registry with cosign attestations
6. Deployment verifies:
   - Signature valid (Rekor log)
   - Provenance SLSA L3+
   - SBOM matches expectations
   - No critical vulns
7. Deploy only if all checks pass
```

## Trade-offs & Maturity

- **SBOM adoption:** Essential now; standards (SPDX/CycloneDX) converging
- **Keyless signing:** Eliminates key management burden; adoption ramping (GitHub Actions OIDC)
- **In-toto overhead:** Valuable for high-assurance systems (financial, defense); overkill for low-risk projects
- **SLSA L4:** Achievable but expensive; L2–L3 practical for most orgs

## See Also

- [Security — Container Scanning](security-container.md)
- [DevOps — CI/CD Patterns](devops-cicd-patterns.md)
- [Tools — Package Managers](tools-package-managers.md)