# Configuration Management in Practice — 12-Factor, Environment Variables & Runtime Validation

## Overview

Configuration management separates application behavior from code, enabling different deployments (dev, staging, prod) to behave differently without recompilation. In practice, this means externalize every setting that changes between environments: database URLs, feature flags, logging levels, rate limits, timeouts, cache backends.

The challenge is not choosing between files and environment variables—it's building a **precedence chain** that lets teams use the right tool for each situation: committed defaults, deployment-time overrides, secret injection, and runtime discovery.

## The 12-Factor Config Principle

[12factor.net/config](https://12factor.net/config) establishes the foundation: store configuration in the environment, not in code or committed files.

**Why not committed files?** They embed deployment-specific values into the codebase, forcing either code branching (bad isolation) or secrets in git (catastrophic security failure).

**Why environment variables?** They:
- are set by the deployment runtime (container orchestrator, systemd, CI/CD, shell)
- survive across deployments without code changes
- are easily audited in deployment logs
- don't require file I/O at runtime
- simplify container and function-as-a-service workflows

**The catch:** Environment variables are strings. A database connection pool size of `"20"` arrives as a string, not an integer. The app must parse and validate. This is where runtime validation becomes critical.

## Configuration Sources & Precedence

Real systems layer multiple sources; the precedence chain determines what wins:

### File-Based Configuration

**Use for:** defaults, schema documentation, non-sensitive metadata.

Committed files (`config.yaml`, `config.json`, `app.properties`) establish defaults and make the schema reviewable in version control. Formats:

- **YAML**: Human-readable, good for humans authoring configs (Kubernetes, docker-compose rely on it)
- **JSON**: Machine-generated, strict validation via JSON Schema, widely tooled
- **TOML**: Simpler than YAML, good for INI-like hierarchies, fewer gotchas (used by Rust, Python, etc.)

**Anti-pattern:** Committing environment-specific `config.prod.yaml` files. This embeds production values in the repo and encourages separate logical paths per env.

**Better pattern:** Commit `config.defaults.yaml` with sensible defaults. Environment variables override specific keys at runtime.

### Environment Variables

**Use for:** overrides, deployment-specific metadata, secret references (not secrets themselves).

Typical precedence:
```
System environment → .env file → defaults in code
```

The `.env` pattern (popularized by `dotenv` and similar libs) loads from a project `.env` file for local development, but uses actual environment variables in production (set by Docker, Kubernetes, systemd, CI/CD).

**Why not commit `.env`?** It can leak secrets or environment-specific production values. Commit `.env.example` instead—a template showing required keys without values.

**Common mistake:** Treating `.env` as version-controlled configuration. It's a local development convenience, not a config management system.

### Secret Stores (Vault, AWS Secrets Manager)

**Use for:** credentials, API keys, encryption keys, tokens.

- **HashiCorp Vault**: Multi-cloud secret store, supports dynamic secrets (rotate credentials automatically), audit logging, encryption as a service
- **AWS Secrets Manager**: AWS-native, tight IAM integration, automatic rotation support, encrypted at rest
- **Azure Key Vault**: Similar to AWS, integrated with managed identities
- **Kubernetes Secrets**: Built into K8s, etcd-backed (encrypt etcd at rest), injected as env vars or volume mounts

These should NOT be committed. The app retrieves them at runtime via authenticated API calls or injection at deployment time.

### Remote Configuration Services

**Use for:** live config changes without restart (feature flags, rate limits, A/B test routing).

- **Consul**: Key-value store with watch support, service discovery, health checks
- **etcd**: Kubernetes' backing store, supports watches and transactions
- **Spring Cloud Config**: Centralized config with Git backend, refresh endpoints
- **Feature flag services** (LaunchDarkly, Unleash, Flagsmith): Specialized for feature-based routing

These add complexity—live changes introduce consistency problems (different services see different values momentarily). Use judiciously.

## Configuration Schema & Validation

Raw environment variables are strings. An app that treats `MAX_RETRIES="abc"` as a number will crash at runtime. Validation happens at application startup.

### Zod: TypeScript Runtime Validation

Zod is a TypeScript-first schema validation library that validates data against a declared schema and produces strongly typed results.

```typescript
import { z } from 'zod';

const ConfigSchema = z.object({
  PORT: z.coerce.number().min(1024).max(65535).default(3000),
  DATABASE_URL: z.string().url(),
  LOG_LEVEL: z.enum(['debug', 'info', 'warn', 'error']).default('info'),
  CACHE_TTL: z.coerce.number().optional(), // optional, undefined if absent
  FEATURES_ENABLED: z.string().transform(s => s.split(',')), // parse CSV
});

const config = ConfigSchema.parse(process.env);
// → config is now { PORT: number, DATABASE_URL: string, LOG_LEVEL: string, ... }
// If any field is invalid, throws ZodError with detailed messages
```

**Strengths:**
- Compile-time type inference from schema (no duplicate type definitions)
- Rich error messages on validation failure, suitable for logging
- Transformations (coerce strings to numbers, parse JSON, etc.)
- Composable schemas for modular config (DB config, cache config, etc.)

**Note on `z.coerce`:** Converts strings to numbers (from env vars). Always use for numeric env vars.

### Convict: Configuration Schema in Node.js

Convict predates Zod and remains widely used. It emphasizes schema-as-documentation:

```javascript
const convict = require('convict');

const config = convict({
  port: {
    doc: 'The port to bind the server to',
    format: 'port',
    default: 3000,
    env: 'PORT'
  },
  db_url: {
    doc: 'PostgreSQL connection string',
    format: 'url',
    env: 'DATABASE_URL'
  },
  log_level: {
    doc: 'Logging verbosity',
    format: ['debug', 'info', 'warn', 'error'],
    default: 'info',
    env: 'LOG_LEVEL'
  }
});

config.validate({ allowed: 'strict' }); // Fail if env vars don't match schema
```

**Strengths:**
- Schema is self-documenting (include `doc` for each field)
- Built-in format validators (port, url, email, etc.)
- Strict mode prevents unknown env vars
- Long-standing (stable API)

**Weaknesses:**
- Less elegant than Zod for TypeScript
- Errors less detailed

Both approaches solve the same problem: declare expected types and ranges at startup, validate immediately, fail fast if config is broken.

## Configuration Precedence Chains

A real application might use:

1. **Hardcoded defaults** in the app
2. **Committed `config.defaults.yaml`** (database pool size, timeouts, pagination limits)
3. **Environment variables** (deployment-specific: `DATABASE_URL`, `NODE_ENV`, `LOG_LEVEL`)
4. **Secret store** (API keys via Vault or Secrets Manager, injected as env vars)
5. **Feature flags** (live flags from a service or stored in config service)

Example loading order (pseudocode):

```
1. Start with in-code defaults
2. Overlay config.defaults.yaml if it exists
3. For each key in schema:
   - If env var is set, use it (overrides file)
   - Else if secret store has it, fetch and use it
   - Else use the default (or error if required)
4. At startup, validate entire config against schema
5. If validation fails, log all errors and refuse to start
```

**Anti-pattern:** Different logic per deployment. Don't have `if (env === 'prod') { ... }` configuration logic. The config system itself should be environment-agnostic.

## Secret Injection Patterns

Secrets require special handling because they must not be logged, stored in version control, or visible in deployment manifests.

### Vault Pattern: Authenticated Retrieval

App requests secrets from Vault at startup using authentication (AppRole, JWT, Kubernetes service account):

```
1. App starts with minimal credential (AppRole ID/Secret or K8s SA token)
2. App authenticates to Vault using that credential
3. Vault returns a token
4. App requests secrets using the token
5. Token expires; app can renew or fetch a new token
6. Secrets are never written to disk; they live only in application memory
```

**Advantage:** Secrets are never embedded in deployment manifests or Docker images.

**Cost:** Adds a runtime dependency (Vault must be available at startup or app fails).

### Secrets Manager Pattern: Injected Environment Variables

Deployment orchestrator (Docker, Kubernetes, ECS) retrieves secrets and injects them as environment variables before the app starts:

```
1. Orchestrator reads deployment manifest with secret references
2. Orchestrator fetches secret values from Secrets Manager (AWS Secrets Manager, K8s Secrets, etc.)
3. Orchestrator injects secrets as environment variables in the container/process
4. App starts and reads env vars (uses Zod or similar to validate)
```

**Advantage:** Simpler—app sees env vars, no extra runtime dependencies.

**Downside:** Secrets visible in process listing (`ps`); requires container/OS to isolate process environments.

**Kubernetes pattern:** Reference secrets in a Deployment manifest:

```yaml
env:
- name: DATABASE_PASSWORD
  valueFrom:
    secretKeyRef:
      name: db-secret
      key: password
```

Kubernetes injects the secret value before the pod starts.

## Config Validation on Deployment vs. Startup

### Validate at deployment time

CI/CD pipeline checks config before deploying:

```bash
# In CI:
CONFIG_SCHEMA=./config.schema.json
cat config.production.json | ajv validate -s $CONFIG_SCHEMA
```

**Pros:** Catches errors early; fail the deployment before updating infrastructure.

**Cons:** Requires committing environment-specific config (problematic for secrets).

### Validate at application startup

App calls `ConfigSchema.parse(process.env)` at startup, refuses to start if invalid:

```typescript
const config = ConfigSchema.parse(process.env);
logger.info({ config: sanitize(config) }, 'Config loaded');
```

**Pros:** Simple, secrets never committed, works across any deployment method.

**Cons:** Failures happen in production (though containers restart automatically, this can be slow).

**Best practice:** Combine both. Use CI to validate non-secret config; app startup validates all config.

## Configuration for Different Scales

### Single-Node / Development

Commit `config.defaults.yaml` with local dev defaults. `.env` file (gitignored) holds local overrides:

```bash
# .env.example
DATABASE_URL=postgresql://localhost/app_dev
LOG_LEVEL=debug

# .env (local, git-ignored)
DATABASE_URL=postgresql://user:pass@db.example.com/app_dev
LOG_LEVEL=trace
```

### Containerized / Kubernetes

No committed env-specific config. Image contains only `config.defaults.yaml`. At deployment:

```yaml
# deployment.yaml
spec:
  containers:
  - name: app
    env:
    - name: LOG_LEVEL
      value: info
    - name: DATABASE_URL
      valueFrom:
        secretKeyRef:
          name: db-credentials
          key: url
```

### Multi-Region / SaaS

Use a config service (Consul, Spring Cloud Config) indexed by region/tenant, or parametrize the default config:

```yaml
# config.defaults.yaml
database:
  region: ${AWS_REGION}
  instance: db.${AWS_REGION}.example.com
```

Environment variables are resolved during config loading.

## Common Issues and Trade-Offs

**Issue: "The same config file works locally but not in production."**
- Solution: Commit only defaults, never environment-specific values. Test config in CI with actual deployment values (use a separate `.env.ci` or mock the env vars).

**Issue: "We can't rotate secrets without redeploying."**
- Solution: Use Vault or a remote config service for secrets, not env vars. These support live rotation.

**Issue: ".env file grows to 200 variables and no one knows what's required."**
- Solution: Use a schema library (Zod, Convict) and enforce strict validation. Document each variable in the schema.

**Issue: "Feature flags are scattered across different config sources."**
- Solution: Use a dedicated feature flag service (LaunchDarkly, Unleash) instead of environment variables.

## See Also

- **configuration-management.md** — General overview of config sources, feature flags, secrets
- **security-secrets-management.md** — Deep dive on secrets handling and audit logging
- **architecture-twelve-factor.md** — Full 12-factor methodology
- **typescript-runtime-validation.md** — Runtime validation in TypeScript (Zod, class-validator, io-ts)