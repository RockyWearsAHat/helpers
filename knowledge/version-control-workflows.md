# Version Control Workflows

## Branching Strategies

### Trunk-Based Development

All developers commit to a single branch (`main`/`trunk`) frequently (at least daily). Short-lived feature branches (< 2 days) are acceptable. Feature flags hide incomplete work.

**Best for:** Teams with strong CI/CD, continuous deployment, high trust environments.
**Pros:** Always-deployable main branch, simple history, fast integration.
**Cons:** Requires feature flags, good test coverage, and disciplined small commits.

### GitHub Flow

Simple workflow: `main` is always deployable. Create feature branches, open pull requests, merge after review.

1. Branch from `main`
2. Make commits
3. Open PR
4. Review + CI passes
5. Merge to `main`
6. Deploy `main`

**Best for:** SaaS, web apps, continuous deployment teams.

### Git Flow (Gitflow)

Structured branching model with long-lived branches:

- `main` — production releases (tagged)
- `develop` — integration branch
- `feature/*` — feature work (branched from develop)
- `release/*` — release preparation
- `hotfix/*` — production emergency fixes

**Best for:** Versioned software, mobile apps, packaged products with explicit release cycles.
**Cons:** Complex, slow integration, merge conflicts between long-lived branches.

### Release Branches

Simpler than Gitflow. Cut a `release/X.Y` branch when ready to stabilize. Main continues to receive new features. Cherry-pick fixes between release and main.

## Commit Conventions

### Conventional Commits

Structured commit messages that enable automated changelogs and semantic versioning:

```
<type>(<scope>): <description>

[optional body]

[optional footer(s)]
```

**Types:** `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`.

**Examples:**

```
feat(auth): add OAuth2 login support
fix(api): handle null response from payment gateway
docs(readme): update installation instructions
refactor(db): extract query builder into separate module
perf(search): add index on users.email column
```

Breaking changes: add `!` after type or `BREAKING CHANGE:` in footer.

### Good Commit Messages

1. **Subject line**: Imperative mood, < 72 chars. "Add feature" not "Added feature" or "Adds feature".
2. **Body** (when needed): Explain _why_, not _what_. The diff shows what changed.
3. **One logical change per commit**. Don't mix refactoring with feature work.
4. **Reference issue/ticket numbers**: `Fixes #42`, `Closes PROJ-123`.

## Semantic Versioning (SemVer)

Format: `MAJOR.MINOR.PATCH`

- **MAJOR**: Breaking changes (incompatible API changes).
- **MINOR**: New features (backward-compatible additions).
- **PATCH**: Bug fixes (backward-compatible fixes).

Pre-release: `1.0.0-alpha.1`, `2.0.0-beta.3`, `1.0.0-rc.1`.

**Rules:**

- Start at 0.1.0 for initial development. Anything goes before 1.0.0.
- Once you hit 1.0.0, the public API is defined and versioning matters.
- Never modify a released version — always increment.

## Pull Request Conventions

1. **Keep PRs small** — < 400 lines of meaningful changes. Large PRs get rubber-stamped.
2. **Write a clear description** — what changed, why, how to test, screenshots for UI.
3. **One concern per PR** — don't mix bug fixes, features, and refactoring.
4. **Self-review before requesting** — read your own diff first.
5. **Link to issues/tickets** — provide context for reviewers.
6. **Include tests** — PR without tests for changed behavior is incomplete.
7. **Respond to all comments** — resolve or explain why you won't change.

## Git Conventions

- **Pull with rebase**: `git pull --rebase` keeps history linear.
- **Don't rewrite shared history**: Never `--force` push to branches others are using.
- **Use .gitignore**: Never commit build artifacts, dependencies, IDE configs, or secrets.
- **Sign commits**: GPG-sign for verified authorship in sensitive projects.
- **Stash or commit before switching branches**: Don't carry uncommitted work around.
- **Use `git bisect`** to find which commit introduced a bug.
- **Tags for releases**: Always tag release commits with the version number.

---

_Sources: Atlassian Git Tutorials, Martin Fowler (Branching Patterns), GitHub Flow documentation, Conventional Commits specification (conventionalcommits.org), semver.org_
