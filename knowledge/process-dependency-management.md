# Process: Dependency Management — Updates, Vulnerabilities, and Supply Chain

## The Dependency Dilemma

Modern software is built on abstractions: libraries, frameworks, and open-source components. A typical application depends on dozens to hundreds of packages. Each dependency is a liability: bugs, security vulnerabilities, and unmaintained code can ripple through the entire system.

Dependency management answers: How do we keep dependencies current without breaking the application? How do we detect and respond to vulnerabilities? How do we know what we're shipping?

## Update Strategies

### Manual Updates

Developers periodically check for updates, test locally, merge PR, and deploy.

**Pros:**
- Full control; developers understand each update
- Can batch updates intelligently (e.g., only major versions in the next release)

**Cons:**
- Requires discipline; easy to neglect
- Manual work doesn't scale to hundreds of dependencies
- Time lag between updates available and updates applied (weeks or months)
- By the time you check, 10 new patch versions exist

### Renovate / Dependabot Automated Approach

Tools that run on a schedule, detect new versions, and open PRs automatically:

**Renovate (mend.io/renovate):**

```json
{
  "extends": ["config:base"],
  "packageRules": [
    {
      "matchUpdateTypes": ["patch"],
      "automerge": true,
      "autoApprove": true
    },
    {
      "matchUpdateTypes": ["minor"],
      "groupName": "minor dependencies",
      "schedule": ["before 3am on Monday"]
    },
    {
      "matchDatasources": ["npm"],
      "matchPackagePatterns": ["typescript", "eslint"],
      "groupName": "dev-tools",
      "assignees": ["@team-leads"]
    }
  ]
}
```

Creates PRs grouped by update type, automerges patches/minor versions after CI passes, and schedules majors for review.

**Dependabot (GitHub native):**

Simpler configuration; runs with GitHub Actions; auto-merges compatible updates. Less customizable than Renovate.

**Pros of automation:**
- Consistent cadence: you always see updates
- Early detection of breaking changes
- Less human burden; patches flow automatically
- Reduces "technical debt" of stale dependencies

**Cons:**
- Flood of PRs if not tuned (branches explode; CI load)
- "Update fatigue" if team feels harassed by PRs
- Requires strong test coverage; CI must catch breaking changes
- Some dependencies have poor release quality; need human judgment

### Hybrid Approach

Most mature teams run automated tools but with guardrails:

- **Patches auto-merge** (only bug fixes; safe by definition)
- **Minors reviewed and merged weekly or biweekly** (new features; low risk if semver is honored)
- **Majors flagged for explicit evaluation** (breaking changes; investigated before adoption)
- **Pinned/core dependencies tracked separately** (e.g., Node.js, database drivers; require more caution)

## Vulnerability Scanning and Response

### Detection Tools

**npm audit / yarn audit:** Built into package managers. Checks lockfile against CVE databases.

```bash
npm audit
# Vulnerabilities found: 3 high, 2 moderate ...
# Fix available by running: npm audit fix
```

**Snyk, Dependabot alerts, Sonatype Nexus Lifecycle:** Continuous scanning of dependencies for known CVEs. Dashboards, email alerts, PR creation.

**Supply-chain security tools (SBOM generation):** CycloneDX, SPDX. Generate a software bill of materials listing all transitive dependencies and their versions.

### Vulnerability Response

When a CVE is published (e.g., "OpenSSL 3.0.0 does not validate certificate chains"):

1. **Identification**: Automated scan alerts that your app uses the vulnerable version
2. **Assessment**: Is your code path affected? (Many CVEs are theoretical; you might not trigger them)
3. **Update**: Bump the dependency to a safe version
4. **Testing**: Run full test suite; verify no new regressions
5. **Deployment**: Prioritize based on severity and risk (critical: deploy today; low: next regular release)

**Severity ratings** (CVSS) guide urgency:
- Critical (9.0–10.0): Exploit remotely; no auth needed → deploy immediately
- High (7.0–8.9): Significant risk → deploy within 24–48 hours
- Medium (4.0–6.9): Exploit requires user interaction or weak conditions → plan into next release
- Low (0.1–3.9): Theoretical or requires unlikely scenarios → track; apply at normal cadence

### CVE Response Time Expectation

- Open-source projects: 24 hours to awareness, 1–2 weeks to fix
- Security-conscious organizations: 48 hours to assess, 72 hours to deploy patch
- Regulated environments (finance, healthcare): formal vulnerability management with SLAs

## Dependency Auditing and License Compliance

### Auditing for Issues

**SBOM (Software Bill of Materials):**

Formal list of all dependencies (direct and transitive) shipped in your application. Generatedby tools like syft, trivy, or scanning APIs.

```json
{
  "bomFormat": "CycloneDX",
  "specVersion": "1.4",
  "components": [
    {
      "type": "library",
      "name": "lodash",
      "version": "4.17.21",
      "vulnerabilities": [
        {
          "ref": "CVE-2021-23337",
          "source": "NVD"
        }
      ]
    }
  ]
}
```

**Use cases:**
- Compliance audits: "What's in the application?"
- Incident response: "Are we affected by this CVE?"
- Legal review: "What licenses are we shipping?"
- Supply-chain security: "Prove your dependency chain"

### License Compliance

Open-source licenses carry obligations (GPL, AGPL, Apache, MIT, BSD, etc.). Some are viral: if you use GPL code, your application must be open-source. Others are permissive (MIT, BSD): no obligations.

**Compliance scanning:** Tools check licenses of all dependencies (FOSSA, WhiteSource, Black Duck).

Policy examples:
- Permissive licenses (MIT, Apache, BSD) : OK
- Weak copyleft (LGPL): OK if used as a library only (not statically linked)
- Strong copyleft (GPL, AGPL): Red flag; often prohibited in commercial software
- Unknown license: Flag for manual review

## Breaking Change Detection

When a major version of a dependency is released, adoption requires assurance that your code still works.

### Dependency Scans for Breaking Changes

**Tools:** Renovate and Dependabot parse package metadata and detect breaking changes:

```
Left Pad: 1.3.0 → 1.4.0
Breaking: The default padding character changed from ' ' to '0'
At-Risk Code: Your app calls padLeft(num) relying on space padding
Recommendation: Review this PR carefully; test before merge
```

Some ecosystems (e.g., Java with Maven) have formal **compatibility metadata**; others require reading changelogs.

### Testing Breaking Changes

1. Update dependency in a feature branch
2. Run full test suite (e.g., Jest, pytest, JUnit)
3. If tests fail, review the breaking change and decide:
   - Update code to new API
   - Choose different library
   - Delay adoption until next release cycle

## Lockfile Management

**package-lock.json / yarn.lock / Pipenv.lock / Cargo.lock:**

Records exact versions and hashes of all dependencies at a point in time. Enables reproducible builds: two developers (or CI) install the same versions.

### Lockfile Discipline

**Commit lockfiles to version control.** Never run your application against a dependency you haven't locked down and verified.

**Shallow updates:** When a library patches a bug, consider updating:

```bash
npm upgrade lodash  # Updates package.json and package-lock.json
npm ci               # CI uses exact locked versions
```

**Do not commit unstable lockfiles.** Before committing dependency updates, tests must pass locally and in CI.

## Transitive Dependencies and Subtree Bloat

A transitive dependency is a package that your dependency depends on. You don't explicitly `import` it, but it's in your app.

```
Your app
└── Framework (direct)
    ├── Utilities (transitive, depth 1)
    │   └── More Utilities (transitive, depth 2)
    └── Parser (transitive, depth 1)
        └── Helper (transitive, depth 2)
```

Problems:
- **Bloat**: Your app ships 400 packages when you explicitly use 12
- **Version conflict**: Two transitive deps both need `lodash`, but different versions in range; package manager must resolve
- **Supply-chain risk**: Vulnerability in a deep transitive dep you've never heard of still breaks your app
- **Maintainability**: Hard to update a deep transitive dep if you don't know why it's there

**Mitigation:**
- Prefer smaller, focused packages (lodash vs. utility-belt)
- Review dependency trees regularly (`npm ls`, `cargo tree`)
- Use SBOM tools to audit what you're shipping
- Consider vendoring critical dependencies (copying source into your repo) for high-security codebases

## See Also

- [Supply Chain Security](security-supply-chain.md) — CVE tracking and secure artifact distribution
- [Package Managers](tools-package-managers.md) — npm, pip, Cargo internals
- [Vulnerability Scanning](security-vulnerability-scanning.md) — SAST, DAST, dependency scanning
- [Version Control Workflows](version-control-workflows.md) — branching strategies for dependency updates