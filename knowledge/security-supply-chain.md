# Software Supply Chain Security

## Overview

Supply chain attacks target the tools, libraries, build systems, and distribution mechanisms that produce software. A single compromised dependency can cascade to thousands of downstream consumers. Modern supply chain security combines inventory (SBOM), scanning, signing, provenance, and policy enforcement.

## Software Bill of Materials (SBOM)

### SPDX (ISO/IEC 5962:2021)

| Field               | Purpose                                              |
| ------------------- | ---------------------------------------------------- |
| `SPDXVersion`       | Format version (SPDX-2.3)                            |
| `DocumentNamespace` | Unique document URI                                  |
| `Package`           | Component name, version, supplier, download location |
| `Relationship`      | DEPENDS_ON, CONTAINS, BUILD_TOOL_OF                  |
| `ExternalRef`       | CPE, PURL, security advisory links                   |
| `LicenseConcluded`  | SPDX license expression                              |

SPDX supports tag-value, JSON, RDF, YAML, and XML serializations. SPDX 3.0 adds build profiles, AI/ML profiles, and security profile.

### CycloneDX (OWASP)

```xml
<bom xmlns="http://cyclonedx.org/schema/bom/1.5">
  <components>
    <component type="library">
      <name>lodash</name>
      <version>4.17.21</version>
      <purl>pkg:npm/lodash@4.17.21</purl>
      <hashes>
        <hash alg="SHA-256">abc123...</hash>
      </hashes>
    </component>
  </components>
  <dependencies>
    <dependency ref="pkg:npm/myapp@1.0.0">
      <dependency ref="pkg:npm/lodash@4.17.21"/>
    </dependency>
  </dependencies>
</bom>
```

CycloneDX supports BOM types: software, hardware, SaaSBOM, MLBOM, CBOM (cryptography), and VEX (vulnerability exploitability exchange).

### SBOM Generation Tools

| Tool                    | Ecosystems         | Output Formats             |
| ----------------------- | ------------------ | -------------------------- |
| `syft` (Anchore)        | All major          | SPDX, CycloneDX, Syft JSON |
| `trivy`                 | Containers, FS     | SPDX, CycloneDX            |
| `cdxgen`                | Polyglot           | CycloneDX                  |
| `sbom-tool` (Microsoft) | .NET, npm, pip, Go | SPDX                       |
| `cyclonedx-cli`         | Conversion/merge   | CycloneDX                  |

### Package URL (PURL)

Standard for identifying packages across ecosystems:

```
pkg:type/namespace/name@version?qualifiers#subpath
pkg:npm/%40angular/core@16.2.0
pkg:pypi/requests@2.31.0
pkg:maven/org.apache.logging.log4j/log4j-core@2.20.0
pkg:golang/github.com/gin-gonic/gin@v1.9.1
pkg:oci/alpine@sha256:abc123?repository_url=ghcr.io
```

## Dependency Scanning

### Scanner Comparison

| Tool           | Type          | Databases         | Ecosystem Coverage              |
| -------------- | ------------- | ----------------- | ------------------------------- |
| **Dependabot** | GitHub-native | GHSA, NVD         | npm, pip, Maven, Go, Rust, etc. |
| **Renovate**   | Self-hostable | Multiple          | 50+ managers, monorepo-aware    |
| **Snyk**       | SaaS/CLI      | Snyk vuln DB      | Code, deps, containers, IaC     |
| **Trivy**      | OSS CLI       | NVD, GHSA, vendor | FS, containers, IaC, K8s        |
| **Grype**      | OSS CLI       | NVD, GHSA, vendor | SBOM-native, container-aware    |

### Dependabot Configuration

```yaml
# .github/dependabot.yml
version: 2
updates:
  - package-ecosystem: npm
    directory: "/"
    schedule:
      interval: weekly
    open-pull-requests-limit: 10
    groups:
      dev-dependencies:
        dependency-type: development
        update-types: [minor, patch]
    ignore:
      - dependency-name: "aws-sdk"
        update-types: ["version-update:semver-major"]
    security-updates-only: false
```

### Renovate Advanced Patterns

```json
{
  "extends": ["config:recommended", "group:monorepos"],
  "packageRules": [
    {
      "matchUpdateTypes": ["minor", "patch"],
      "automerge": true,
      "automergeType": "branch"
    },
    {
      "matchPackageNames": ["typescript"],
      "allowedVersions": ">=5.0.0 <6.0.0"
    }
  ],
  "vulnerabilityAlerts": { "enabled": true, "labels": ["security"] },
  "lockFileMaintenance": {
    "enabled": true,
    "schedule": ["before 5am on monday"]
  }
}
```

## Lockfile Integrity

### Why Lockfiles Matter

- Pin exact versions (transitive included)
- Record integrity hashes for tamper detection
- Ensure reproducible installs across environments

### Integrity Verification

```bash
# npm — verify lockfile matches package.json
npm ci                    # strict install from lockfile
npm audit signatures      # verify registry signatures

# pip — hash-checking mode
pip install --require-hashes -r requirements.txt

# Go — module verification
GONOSUMCHECK="" go mod verify
GOFLAGS=-mod=vendor go build ./...
```

### Attack Vector: Lockfile Manipulation

Attackers can submit PRs that modify lockfiles to inject malicious packages. Defenses:

- Review lockfile diffs carefully in PRs
- Use `npm ci` (not `npm install`) in CI
- Enable `package-lock=true` enforcement
- Verify checksums: `npm audit signatures`

## Supply Chain Attack Patterns

### Typosquatting

Publishing malicious packages with names similar to popular ones:

- `lodahs` → `lodash`
- `colorsss` → `colors`
- `crossenv` → `cross-env`

**Defenses**: scope packages (`@org/pkg`), use lockfiles, enable namespace reservation, verify downloads/age.

### Dependency Confusion

Exploiting package managers that check public registries before private ones:

```
Private registry: @company/auth-utils@1.0.0
Public registry:  auth-utils@99.0.0  ← attacker publishes
```

Package manager installs public `99.0.0` because version is higher.

**Defenses**:

- Scoped packages (`@company/auth-utils`)
- `.npmrc` registry mapping per scope
- Claim placeholder names on public registries
- Use `--registry` and `registry` configs explicitly

### Malicious Maintainer / Account Takeover

- `event-stream` (2018): maintainer handed off to attacker
- `ua-parser-js` (2021): account compromise, crypto miner injected
- `node-ipc` (2022): maintainer inserted protestware

**Defenses**: review new maintainers, pin versions, audit `postinstall` scripts, use `--ignore-scripts`.

## Code Signing & Verification

### Sigstore Ecosystem

```
┌──────────────┐    ┌─────────┐    ┌───────────┐
│   Developer  │───▶│  Fulcio  │───▶│  Rekor    │
│  (OIDC ID)   │    │  (CA)    │    │ (Log)     │
└──────────────┘    └─────────┘    └───────────┘
       │                                  │
       ▼                                  ▼
  Sign artifact              Immutable transparency
  with ephemeral key         log entry (proof)
```

| Component   | Role                                                   |
| ----------- | ------------------------------------------------------ |
| **Fulcio**  | Issues short-lived certificates bound to OIDC identity |
| **Rekor**   | Immutable transparency log for signatures              |
| **cosign**  | CLI for signing/verifying containers and blobs         |
| **Gitsign** | Sign git commits with Sigstore                         |

### Container Image Signing with cosign

```bash
# Sign (keyless — uses OIDC identity)
cosign sign ghcr.io/org/app@sha256:abc123

# Sign with key
cosign sign --key cosign.key ghcr.io/org/app@sha256:abc123

# Verify
cosign verify --certificate-identity user@example.com \
  --certificate-oidc-issuer https://accounts.google.com \
  ghcr.io/org/app@sha256:abc123

# Attach SBOM to image
cosign attach sbom --sbom sbom.cdx.json ghcr.io/org/app@sha256:abc123

# Verify SBOM attachment
cosign verify-attestation --type cyclonedx \
  ghcr.io/org/app@sha256:abc123
```

### npm Provenance

```bash
# Publish with provenance (requires GitHub Actions OIDC)
npm publish --provenance
# Package appears with "Provenance" badge on npmjs.com
npm audit signatures  # verify all installed packages
```

## Reproducible Builds

### Principles

1. **Same source → same binary** regardless of build environment
2. **Eliminate non-determinism**: timestamps, filesystem ordering, locale, UUIDs
3. **Verification**: anyone can rebuild and compare output hash

### Common Non-Determinism Sources

| Source                    | Mitigation                       |
| ------------------------- | -------------------------------- |
| Timestamps in archives    | `SOURCE_DATE_EPOCH`, `--mtime`   |
| File ordering in archives | Sort file lists before archiving |
| Embedded build paths      | Build in fixed path (`/build/`)  |
| Compiler randomization    | Fixed seeds, deterministic flags |
| Floating dependencies     | Lockfiles, pinned versions       |

### SOURCE_DATE_EPOCH

```bash
export SOURCE_DATE_EPOCH=$(git log -1 --format=%ct)
# Build tools that respect this: tar, zip, gcc, go, docker
```

## SLSA Framework

### Levels

| Level      | Requirements                                    | Trust             |
| ---------- | ----------------------------------------------- | ----------------- |
| **SLSA 1** | Build process exists and produces provenance    | Documentation     |
| **SLSA 2** | Hosted build service, authenticated provenance  | Tamper-resistant  |
| **SLSA 3** | Hardened build platform, unforgeable provenance | Tamper-proof      |
| **SLSA 4** | Two-person review, hermetic/reproducible builds | Maximum assurance |

### Provenance Schema (SLSA)

```json
{
  "_type": "https://in-toto.io/Statement/v1",
  "subject": [{ "name": "myapp", "digest": { "sha256": "abc123..." } }],
  "predicateType": "https://slsa.dev/provenance/v1",
  "predicate": {
    "buildDefinition": {
      "buildType": "https://github.com/actions/runner",
      "externalParameters": { "workflow": ".github/workflows/release.yml" },
      "resolvedDependencies": [
        { "uri": "git+https://github.com/org/app@refs/tags/v1.0.0" }
      ]
    },
    "runDetails": {
      "builder": { "id": "https://github.com/actions/runner/github-hosted" }
    }
  }
}
```

### GitHub Artifact Attestation

```yaml
# .github/workflows/release.yml
jobs:
  build:
    permissions:
      id-token: write
      contents: read
      attestations: write
    steps:
      - uses: actions/attest-build-provenance@v1
        with:
          subject-path: dist/myapp
```

```bash
# Verify locally
gh attestation verify dist/myapp --owner org
```

## Registry Security

### Container Registry Hardening

- Enable content trust / image signing enforcement
- Use digest-based references (`@sha256:...`) not tags
- Scan images on push (Trivy, Snyk Container)
- Set retention policies for untagged images
- Enable vulnerability scanning (ECR, GCR, ACR built-in)
- Restrict push access with RBAC
- Use private registries with network policies

### Admission Control

```yaml
# Kubernetes — require signed images (Kyverno)
apiVersion: kyverno.io/v1
kind: ClusterPolicy
metadata:
  name: verify-image-signatures
spec:
  validationFailureAction: Enforce
  rules:
    - name: verify-cosign
      match:
        resources:
          kinds: [Pod]
      verifyImages:
        - imageReferences: ["ghcr.io/org/*"]
          attestors:
            - entries:
                - keyless:
                    subject: "https://github.com/org/*"
                    issuer: "https://token.actions.githubusercontent.com"
```

## Supply Chain Security Checklist

| Area            | Action                                                       |
| --------------- | ------------------------------------------------------------ |
| **Inventory**   | Generate SBOMs for all releases                              |
| **Scanning**    | Automated dependency vulnerability scanning in CI            |
| **Lockfiles**   | Enforce lockfile usage, review lockfile diffs                |
| **Signing**     | Sign artifacts and container images                          |
| **Provenance**  | SLSA Level 2+ provenance for build artifacts                 |
| **Registries**  | Private registries, access control, scanning on push         |
| **Updates**     | Automated dependency updates with grouped PRs                |
| **Scripts**     | Audit install scripts, use `--ignore-scripts` where possible |
| **Namespacing** | Scoped packages, dependency confusion prevention             |
| **Monitoring**  | Alert on new dependency additions, maintainer changes        |
| **Policy**      | Admission control for signed/verified images in prod         |
