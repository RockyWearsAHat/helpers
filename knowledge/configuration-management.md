# Configuration Management — Sources, Hierarchies, Feature Flags & Secrets

## Overview

Configuration management is the practice of externalizing application behavior from code, enabling different deployments (dev, staging, prod) to behave differently without recompilation. It bridges infrastructure, application code, and operational requirements, spanning environment variables, configuration files, remote services, and runtime feature switches.

Unlike secrets (credentials, keys, tokens), configuration is application behavior metadata: database hosts, feature toggles, logging levels, pagination limits. Conflating the two creates security debt.

## Configuration Sources & Hierarchy

Configuration typically flows from multiple overlapping sources, creating a merge order that determines which value wins:

**File-based**: Committed config files (e.g., `config.yaml`, `app.properties`) provide versioned, reviewable defaults. Many teams use language-specific formats (YAML for flexibility, TOML for simplicity, JSON for tooling). File configs are searchable and diff'able but require deployment/restart to change.

**Environment variables**: Set by the deployment runtime (Kubernetes, Docker, systemd, shell), overriding file-based config. Useful for CI/CD and container orchestration. Downside: hard to version, audit, or search across deployments; can bloat with hundreds of variables.

**Remote services**: Consul, etcd, Spring Cloud Config, or proprietary services hold live config. Enables hot reload without deployment. Trade-off: external dependency, eventual consistency, and discovery complexity.

**In-memory defaults**: Code-level defaults ship with the application, used if no external source provides a value.

**Command-line flags**: CLI tools often accept `--config-file` or individual settings like `--port 8080`. Useful for testing and scripting but rarely used for server applications.

Typical precedence (highest to lowest): command-line flags → environment variables → remote config service → config file → code defaults. Each layer should document its purpose.

## 12-Factor Configuration Principles

The 12-factor app methodology (https://12factor.net/) codifies environment-driven config:

1. **Strict separation**: Config (varies per environment) must not appear in code. A build artifact should work unchanged across dev/staging/production with only config changes.
2. **Environment variables**: The recommended externaliza method. Language-agnostic, supported everywhere (containers, CI, OS).
3. **No grouping by environment**: Rather than separate `config.dev.yaml` and `config.prod.yaml`, use a single `config.yaml` with `${ENV}` substitutions or a single source of truth (e.g., Consul) that returns environment-specific values.

The philosophy is: **identical codebase, swapped configuration**. This enforces that deployments differ only in config, not in hidden environment checks like `if (process.env.NODE_ENV === 'production')`.

Trade-offs: environment variables scale poorly (hundreds of vars become unwieldy); require careful naming conventions (`APP_DB_HOST`, `APP_DB_PORT`); and are harder to structure than hierarchical formats (YAML trees express relationships more clearly).

## Dotenv Patterns & Local Development

`.env` files are local development convenience: a text file with `KEY=VALUE` pairs, loaded into the environment at startup. Popular in Node.js (dotenv package), Python (python-dotenv), Ruby (dotenv gem), and Go (godotenv).

**Best practices:**
- `.env` (local, gitignored): developer's personal overrides
- `.env.example` (versioned): template documenting required variables and safe defaults
- Never commit `.env` with real secrets or production values
- Load early in application startup, before any config reads
- `.env` values should only supplement, not override already-set environment variables (respect CI/CD and container-provided values)

**Limitations**: dotenv is a local-only convenience and doesn't scale to teams. Secrets are often kept in `.env` regardless of the stated pattern, creating leaks.

## Configuration Validation & Type Safety

Configuration errors should fail fast at startup, not hours later in production. Modern approaches:

**Schema validation**: Define configuration structure (required keys, types, ranges) and validate at load time. JSON Schema, YAML schemas (via tools), or language-specific libraries (Zod for TypeScript, Pydantic for Python, struct tags for Go).

**Typed configuration objects**: Parse config into typed data structures. A YAML file becomes a Python dataclass or Go struct, catching type mismatches immediately.

**Fail-safe defaults vs. required keys**: Some values should have sensible defaults (port: 8080); others (database URL) should be required. Schema should express this.

**Runtime assertion**: Check invariants: "replica count must be ≥ 1", "log level must be DEBUG|INFO|WARN|ERROR". Database connection tests at startup verify the configured host is reachable.

## Feature Flags & Runtime Switches

Feature flags (feature toggles, feature gates, feature switches) enable runtime control of behavior without deployment. Use cases:

- **Gradual rollout**: Enable a new feature for 5% of users, ramp to 100%.
- **A/B testing**: Experiments targeting userID % 2 == 0.
- **Circuit breaking**: Disable a flaky external service call.
- **Configuration without redeploy**: Toggle logging, caching, or query modes.

**Types of flags:**

*Release flags*: Permanently tied to a code path. After the feature stabilizes, remove the flag and clean up dead code. Lifetime: weeks to months.

*Experiment flags*: Temporary, tied to a metric (A/B test outcome). Either roll back (feature disabled forever) or clean up the flag (feature always enabled). Lifetime: days to weeks.

*Ops flags*: Permanent, operational control (enable debug logging, disable cache). Lifetime: indefinite.

**Flag services**: Large teams use dedicated platforms:
- **LaunchDarkly**: SaaS, well-integrated, expensive at scale.
- **Unleash**: Open-source, self-hosted, good dashboard.
- **Flagsmith**: SaaS or self-hosted, lightweight.
- **Simple backends**: Redis, database, Consul (lower-level control, more engineering).

Flag services typically offer:
- Dashboard UI for toggling flags in real time
- Targeting rules (by user ID, custom attributes, percentage rollout)
- Analytics (which flags affected performance)
- Audit logs (who changed what, when)

**Pitfalls**: Old flags left in code create cognitive load ("what is this flag about?"). Teams often lack discipline to clean them up. Regular audits and documented retirement dates help.

## Secrets vs. Configuration

**Configuration** changes per environment: database host, log level, cache strategy. It is per-deployment metadata. It can be versioned, diff'ed, and often logged.

**Secrets** are credentials: passwords, API keys, certificates, tokens. They must never appear in version control, logs, or error messages. They require encryption at rest and transmitted over secure channels. They should be rotated regularly.

**The mistake**: Treating secrets like configuration. Storing API keys in `.env` files committed to git, logging full connection strings, embedding credentials in config files. This is the leading cause of credential leaks.

**Best practices:**
- Store secrets in dedicated secret management systems: HashiCorp Vault, AWS Secrets Manager, Azure Key Vault, Kubernetes Secrets, 1Password.
- Separate secret loading from config loading. The application requests secrets separately, often with authentication (e.g., IAM roles in AWS).
- Rotate secrets regularly (passwords every 90 days, API keys quarterly).
- Audit who accessed what secret, when.
- Never log secrets. Implement careful scrubbing of logs before shipping to logging systems.
- Use unique secrets per service/environment. Never reuse a production API key across multiple services.

## Configuration Reload (Hot Reload)

Restarting the application to pick up configuration changes is slow for long-running services and causes brief unavailability. Hot reload (zero-downtime config updates) is valuable for operational flags but risky for structural config (database URLs, pool sizes).

**Patterns:**

*File watcher*: Application watches for changes to `config.yaml`, reloads, and re-validates. Fast but requires the process to have filesystem access and permission to read.

*Signal handler*: Application listens for `SIGHUP` (or a webhook), reloads config. Requires the deployment system to coordinate the signal delivery.

*Polling remote service*: Application polls etcd or Consul every N seconds for config changes, detects deltas, applies updates.

*Event-driven*: Remote service pushes config changes via webhook or Pub/Sub (Kafka, RabbitMQ). More complex but responsive.

**Risks**: a corrupted config reload crashes the app; a bad value propagates instantly. Mitigation: validate before applying, support rollback, feature-flag risky changes.

## Configuration Structure & Discovery

Large applications accumulate dozens of config values. Organization matters:

**Hierarchical structure** (YAML/TOML): Express relationships. A database config object contains `host`, `port`, `pool_size`.

```yaml
database:
  primary:
    host: db.example.com
    port: 5432
    pool_size: 20
  replica:
    host: db-read.example.com
    port: 5432
```

**Naming conventions** (environment variables): Descriptive prefixes and delimiters.
```
APP_DB_PRIMARY_HOST=db.example.com
APP_DB_PRIMARY_PORT=5432
APP_CACHE_TTL_SECONDS=3600
```

**Documentation**: README or config schema documenting each value, its purpose, valid ranges, and examples. Auto-generated docs (from schema) reduce drift.

**Discovery tools**: Some teams export config as structured outputs (JSON, YAML) so ops/debugging tools can query available values and their types.

## Use Cases & Trade-offs

| Source | Best For | Trade-offs |
|--------|----------|-----------|
| Code defaults | Sensible fallbacks | Requires recompile to change |
| Config file | Base configuration | Requires restart; hard to search at scale |
| Environment variables | Container orchestration, CI/CD | Unstructured; scales poorly; hard to audit |
| Remote service | Live tuning, team sharing | Added dependency; eventual consistency |
| Feature flags | Gradual rollout, A/B testing | Tool complexity; flag debt if not cleaned up |

## See Also

- **API Design** — Configuration patterns in SDK design
- **Secrets Management** — Distinguishing and securing credentials
- **Infrastructure as Code** — Declarative configuration of infrastructure
- **12-Factor App** — Complete methodology for deployable applications