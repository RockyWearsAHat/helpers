# Git Branching Strategies — Models, Naming Conventions, and Team Patterns

## Branching Model Archetypes

Branching strategy describes how branches are created, maintained, and integrated. Different models suit different team structures and release cadences.

### Trunk-Based Development

A single branch (`main` or `trunk`) is the shared integration point. All developers commit frequently—typically within 24 hours—using short-lived feature branches (< 2 days) or committing directly to trunk with feature flags.

**Workflow:**

```
main: A → B → C → D (multiple merges/day)
```

**Preconditions:**

- Strong CI/CD pipeline (tests run automatically on every commit)
- High test coverage (unit, integration, e2e)
- Mature feature flag infrastructure (hide incomplete work)
- Small, disciplined commits (easy to revert individual changes)
- High trust and communication within team

**Characteristics:**

- Always-deployable main branch
- Simple history (minimal branching)
- Fast integration (less frequent merge conflicts)
- Continuous deployment possible

**Drawbacks:**

- Requires feature flags (add complexity)
- Demands rigor: bad commits affect everyone immediately
- Difficult in regulated environments (audit trails, staged releases)
- Less suitable for teams without CI/CD maturity

**Team context:** Startups, SaaS companies, teams with strong DevOps culture, continuous deployment shops. Google, Facebook, and Amazon use variants.

### GitHub Flow

A refined trunk-based model for web applications. Emphasizes simple pull requests and CI/CD discipline.

**Workflow:**

1. Branch from `main`
2. Make commits
3. Open pull request (triggers CI)
4. Code review and CI pass → merge to `main`
5. Deploy `main` immediately (or automatically)

```
main: ─────────────────────────────────────────
         ↑     ↑     ↑
         PR1   PR2   PR3 (merges)
```

Each branch is ephemeral—deleted after merge. `main` is always production-ready.

**Preconditions:**

- Auto-deploy to production on merge (or one-click deploy)
- Strong test coverage
- Automated monitoring and rollback
- Small PRs (< 400 lines of meaningful changes)

**Characteristics:**

- Very simple (easier to teach)
- Encourages frequent deployments
- PR-driven (supports distributed review)
- Fast feedback loop

**Team context:** SaaS teams, web startups, teams practicing continuous deployment. GitHub itself uses this model.

### Git Flow (Gitflow)

A structured, multi-branch model with explicit roles for different branch types. Optimized for versioned software with explicit release cycles.

**Branch types:**

- `main` — production releases only, tagged with version
- `develop` — integration branch for the next release
- `feature/*` — feature branches (from `develop`, merged back to `develop`)
- `release/*` — release preparation (from `develop`, merged to `main` and back to `develop`)
- `hotfix/*` — emergency production fixes (from `main`, merged to `main` and `develop`)

**Workflow:**

```
main:     ────┬──────────┬──────────  (tags: v1.0, v2.0)
             v1.0        v2.0
develop:  ────┼────┬────┬┼──────────
             (back) features
```

**Preconditions:**

- Explicit release cycles (versions released quarterly or annually)
- Multiple versions in maintenance simultaneously
- Backporting fixes to old versions

**Characteristics:**

- Clear separation of concerns (feature, release, hotfix tracks)
- Supports parallel release maintenance
- Long-lived branches (develop, release braches) increase merge conflict risk

**Drawbacks:**

- Complex (more branches, more merges)
- Slower integration (features sit on feature branches longer)
- Merge conflicts between long-lived branches become frequent
- Difficult to debug which features went into which release

**Team context:** Mobile apps (versioned releases), packaged products, enterprise software. Banks, software vendors, teams with formal release processes.

### Release Branches (Simplified Gitflow)

A middle ground between trunk-based and full Gitflow.

**Workflow:**

```
main:      A ─→ B ─→ v1.0 (tag)
             ↓ (fork)
release/1.0: B → C (fixes) → v1.0 tag (merge back to main)
             ↓ (develop continues)
main:      A ─→ B ─→ D ─→ E ─→ v2.0
```

When ready to release, create `release/X.Y` from main. Main continues receiving new features. Bug fixes on release branch are cherry-picked back to main.

**Characteristics:**

- Simpler than Gitflow (only one release branch active at a time)
- Supports bug-fix-only releases (no new features)
- Main always receives the next development version's code

**Team context:** Software with regular minor releases, maintenance branches for old versions. Open source projects that need LTS versions.

### Trunk-Based with Release Tags

Extreme simplicity: trunk only, tag releases, backport fixes.

```
main: A → B → C → D(tag:v1.0) → E → F → G(tag:v1.1)
```

When a bug must be fixed in v1.0, either:

1. Fix in main, cherry-pick back to v1.0 (if applicable)
2. Manually patch the release tag

**Characteristics:**

- Minimal context switching
- Simple history
- Requires tooling to manage backports

**Team context:** Very small teams, projects with infrequent releases, or projects where old versions receive no maintenance.

## Branch Naming Conventions

Names communicate purpose, linking branches to issue tracking and enabling automation.

### Common Patterns

- **Feature branches**: `feature/user-auth`, `user-login`, `feat/oauth2`
- **Bug fixes**: `bugfix/login-crash`, `fix/payment-timeout`
- **Release branches**: `release/1.2.0`, `release/2024-q1`
- **Hotfix branches**: `hotfix/security-xss`, `emergency/production-bug`
- **Work-in-progress**: `wip/refactor-database`, `experimental/caching`
- **Task branches**: `task/update-dependencies`, `chore/lint-config`

### Automation-Friendly Naming

**Format:** `<type>/<ticket-id>-<short-description>`

```
feature/PROJ-1234-add-two-factor-auth
bugfix/GH-5678-null-pointer-exception
```

**Benefits:**

- CI/CD can extract ticket ID for linking
- Allows automatic branch cleanup on PR merge
- Search and grep friendly
- Clearly communicates purpose

### Repository Defaults

Many teams adopt all-lowercase, hyphens-not-underscores conventions:

```
✓ feature/user-registration
✗ Feature/UserRegistration
✗ feature_user_registration
```

This avoids filesystem and GitHub rendering quirks.

## Long-Lived vs Short-Lived Branches

**Short-lived branches** (< 1 week):

- Created for a specific task
- Frequently rebased or merged
- Deleted after integration
- Reduce merge conflicts
- Enable rapid iteration

**Long-lived branches** (weeks to months):

- Maintained indefinitely (or for a release cycle)
- Serve as integration points for features
- Increase merge conflict risk as changes accumulate
- Typical in multi-version maintenance (main, develop, release/1.0, release/2.0)

**Trade-off:** Short-lived branches require disciplined integration (PRs, CI), while long-lived branches are easier to maintain in isolation but harder to merge back.

## Branch Deletion and Cleanup

After merging a feature branch, delete it:

```bash
git branch -d feature/add-auth    # Delete local
git push origin --delete feature/add-auth   # Delete remote
```

**Why delete:**

- Keeps namespace clean (prevents 100s of stale branches)
- Reduces confusion about what's being worked on
- New work starts from main (always up-to-date)

GitHub and GitLab have settings to auto-delete branches on PR merge.

**Retention:**

Some teams keep `release/*` and `hotfix/*` branches for historical record, but delete feature branches aggressively.

## Team Communication and Convention

Branching strategy is a **social contract**, not a technical enforcer. Effectiveness depends on team adoption.

### Documentation

Document in `CONTRIBUTING.md` or wiki:

- Which model you use (trunk-based, GitHub Flow, Gitflow)
- Naming conventions
- Who can delete branches
- When merges vs. rebases are appropriate
- How long to keep branches before deletion

### Discovery and Tooling

- **Branch protection rules**: Require PR review, CI passing before merge
- **CI/CD integration**: Auto-run tests on PR creation
- **Pre-commit hooks**: Validate branch names locally
- **Merge strategy**: Enforce `--no-ff` (always create merge commit) or allow fast-forward, depending on preference

### Hybrid Strategies

Most teams adapt the model to their context. Example:

- Trunk-based for daily development
- Release branches for stable versions
- Hotfix branches for emergency production fixes
- Feature flags for incomplete work on main

## Remote Tracking Branches

Git maintains local tracking branches for each remote branch:

```bash
git branch -r   # Shows origin/main, origin/develop, etc.
git fetch       # Updates remote tracking branches
git pull        # Fetch + merge; equiv. to `fetch` + `merge origin/main`
```

**Workflow implications:**

- Always `git fetch` before creating a new branch to avoid basing work on stale upstream
- `git branch --track` sets up local branch to track remote (for easy pull)
- `git push -u origin feature` sets upstream for future `git push` without arguments

---

_Sources: Atlassian Git Tutorials (Branching Patterns), Martin Fowler (Patterns for Managing Source Code Branches), GitHub Flow documentation, Git Flow and Gitflow papers_