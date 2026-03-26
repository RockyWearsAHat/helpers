# Open Source Practices — Contributing, Licensing, and Community

## How to Read a Project Before Contributing

### 1. Check the Vitals

```bash
# Is this project alive?
git log --oneline -5              # Last commits
gh issue list --state open -L 5   # Open issues
gh pr list --state open -L 5      # Open PRs
```

| Signal               | Healthy        | Warning         |
| -------------------- | -------------- | --------------- |
| Last commit          | < 3 months ago | > 1 year        |
| Open issues response | Days           | Months or none  |
| PR merge time        | Days to weeks  | Months or never |
| CI status            | Green badges   | Red or missing  |
| CONTRIBUTING.md      | Exists         | Missing         |

### 2. Read These Files (In Order)

1. **README.md** — What is this? How do I use it?
2. **CONTRIBUTING.md** — How do I contribute? What's the process?
3. **CODE_OF_CONDUCT.md** — Community expectations
4. **LICENSE** — What can I do with this code?
5. **.github/ISSUE_TEMPLATE/** — What info do maintainers want?
6. **.github/PULL_REQUEST_TEMPLATE.md** — PR expectations

### 3. Understand the Development Workflow

```
Fork → Clone → Branch → Commit → Push → Pull Request → Review → Merge
```

Some projects use different flows:

- **GitHub Flow**: Feature branches off main, PR to main
- **Git Flow**: develop, feature/_, release/_, hotfix/\* branches
- **Trunk-based**: Everyone commits to main (with feature flags)

## Making Your First Contribution

### Start With Good First Issues

```
Labels to look for:
  "good first issue"
  "help wanted"
  "beginner-friendly"
  "documentation"
  "easy"
```

### The Contribution Checklist

1. **Read CONTRIBUTING.md** (seriously, read the whole thing)
2. **Check if it's already being worked on** — search issues and PRs
3. **Comment on the issue** — "I'd like to work on this. Here's my approach." Wait for a response.
4. **Fork and branch** — `git checkout -b fix/issue-42-description`
5. **Write code** — follow existing style (don't reformulate the world)
6. **Write tests** — if the project has tests, yours should too
7. **Run CI locally** — don't rely on the remote CI to catch failures
8. **Write a good commit message** — explain WHY, not just what
9. **Open a PR** — Reference the issue, describe what you did, include screenshots if UI

### Writing Good PRs

```markdown
## What

Fix #42: Handle null user in profile endpoint

## Why

The profile endpoint crashes with a 500 when a deleted user's profile is accessed.
This is the most common error in our Sentry dashboard.

## How

Added a null check before accessing user properties. Returns 404 with a clear
error message when the user doesn't exist.

## Testing

- Added unit test for null user case
- Verified manually with deleted user ID
- Existing tests pass
```

### Responding to Code Review

- **Don't take it personally** — it's about the code, not you
- **Ask questions** if feedback is unclear
- **Push fixes as new commits** during review (easier to see changes)
- **Squash when approved** (if the project wants clean history)
- **Say thank you** — maintainers are usually volunteers

## Licensing — What You Need to Know

### Permissive Licenses (Do Almost Anything)

| License      | Key Requirements                            | Notable Users                 |
| ------------ | ------------------------------------------- | ----------------------------- |
| MIT          | Include license text                        | React, jQuery, Node.js, Rails |
| Apache 2.0   | Include license + NOTICE, patent grant      | Android, Kubernetes, Rust     |
| BSD 2-Clause | Include license text                        | FreeBSD, nginx (historically) |
| BSD 3-Clause | Include license text, no endorsement clause | Go standard library           |
| ISC          | Include license text (simplified MIT)       | OpenBSD, npm                  |

### Copyleft Licenses (Share-Alike)

| License | Key Requirements                      | Scope                      |
| ------- | ------------------------------------- | -------------------------- |
| GPL v2  | Derivative works must be GPL          | Entire program             |
| GPL v3  | + Patent protection, anti-tivoization | Entire program             |
| LGPL    | Copyleft for the library only         | The library, not your code |
| AGPL    | GPL + network use triggers copyleft   | Server-side code too!      |
| MPL 2.0 | Copyleft per-file (not per-project)   | Modified files only        |

### What This Means in Practice

```
MIT/Apache: Use it anywhere. Commercial, proprietary, whatever. Just keep the license.

GPL: If you distribute a program containing GPL code, you must release
     your entire program's source under GPL.
     DOES NOT apply to: using a GPL tool (compiler, editor),
     linking to GPL libraries as separate processes (debatable),
     using GPL code on your server without distributing.

AGPL: Same as GPL + if users interact with it over a network,
      you must release source. This is why some companies ban AGPL.

LGPL: You can link to LGPL libraries from proprietary code,
      but changes to the library itself must be released.
```

### License Compatibility

```
MIT → can be included in GPL projects     ✓
GPL → cannot be included in MIT projects  ✗
Apache 2.0 → compatible with GPL v3       ✓
Apache 2.0 → NOT compatible with GPL v2   ✗
```

### Choosing a License

```
Want maximum adoption? → MIT or Apache 2.0
Want patent protection? → Apache 2.0
Want contributions back? → GPL v3 or AGPL
Want file-level copyleft? → MPL 2.0
Library that proprietary code can use? → LGPL or MIT/Apache
Don't want any license? → That means ALL RIGHTS RESERVED (nobody can use it!)
```

## Maintaining an Open Source Project

### The Bus Factor Problem

If only one person understands the project, the project has a bus factor of 1. Mitigate:

- Document architecture decisions (ADRs)
- Have multiple maintainers with merge access
- Write contributor guides that actually help
- Automate everything that can be automated

### Issue Management

```markdown
## Bug Report Template

**Describe the bug:** Clear description of what's wrong
**To Reproduce:** Step-by-step instructions
**Expected behavior:** What should happen
**Environment:** OS, language version, package version
**Screenshots/Logs:** If applicable
```

### Release Management

```
Semantic Versioning: MAJOR.MINOR.PATCH
  MAJOR: Breaking changes (existing code may break)
  MINOR: New features (backwards compatible)
  PATCH: Bug fixes (backwards compatible)

Pre-release: 1.0.0-alpha.1, 1.0.0-beta.1, 1.0.0-rc.1
```

### Changelog Conventions

```markdown
## [1.2.0] - 2024-03-15

### Added

- Support for WebSocket connections (#42)

### Changed

- Improved error messages for auth failures

### Fixed

- Memory leak in connection pool (#38)

### Deprecated

- `connect()` without options (use `connect({})` instead)

### Removed

- Python 3.7 support

### Security

- Updated TLS certificate validation
```

## Governance Models

| Model      | Description                             | Examples                       |
| ---------- | --------------------------------------- | ------------------------------ |
| BDFL       | Benevolent Dictator For Life            | Python (was), Linux            |
| Core team  | Small group of maintainers              | Rust, React                    |
| Foundation | Non-profit governance                   | Apache, Linux Foundation, CNCF |
| Corporate  | Company controls, community contributes | Android, VS Code, Chromium     |
| Consensus  | Community-driven decisions              | Debian                         |

## Funding Open Source

| Method            | How It Works                                              |
| ----------------- | --------------------------------------------------------- |
| GitHub Sponsors   | Direct donations to maintainers                           |
| Open Collective   | Transparent budget for projects                           |
| Tidelift          | Companies pay for maintenance guarantees                  |
| Consulting        | Offer paid support/consulting around your OSS             |
| Dual licensing    | Free for OSS, paid for commercial (MySQL model)           |
| Open core         | Core is open, enterprise features are paid (GitLab model) |
| Foundation grants | Apply for grants (NLnet, Mozilla, Sovereign Tech Fund)    |

---

_"Open source is not about being free as in beer. It's about being free as in speech." — Richard Stallman. Though the beer part is nice too._
