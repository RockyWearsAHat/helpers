# DevOps & CI/CD

## Continuous Integration (CI)

**Core principle:** Every developer integrates code into the shared mainline frequently (at least daily). Every integration triggers automated build + test.

**CI pipeline essentials:**

1. **Compile/build** — Catch syntax errors immediately.
2. **Lint** — Enforce style and catch common bugs (ESLint, Pylint, ShellCheck, clippy).
3. **Unit tests** — Fast, isolated, cover business logic.
4. **Integration tests** — Verify component interactions.
5. **Security scanning** — SAST (static analysis), dependency vulnerability scanning.
6. **Artifact creation** — Build the deployable artifact once. Deploy the same artifact everywhere.

**CI best practices:**

- Keep the build fast (< 10 minutes target). Parallelize tests.
- Fix broken builds immediately — a broken mainline blocks everyone.
- Run the full pipeline on every pull request before merge.
- Never skip tests to "ship faster."

## Continuous Delivery (CD)

**Continuous Delivery**: Every commit that passes CI is _deployable_ to production. Deployment is a manual decision.
**Continuous Deployment**: Every commit that passes CI _automatically deploys_ to production. No human gate.

**Deployment strategies:**
| Strategy | Description | Risk | Rollback |
|----------|-------------|------|----------|
| Rolling | Replace instances gradually | Medium | Stop rollout, roll back |
| Blue/Green | Two identical environments, switch traffic | Low | Switch back to old env |
| Canary | Route small % of traffic to new version | Low | Route back to old version |
| Feature Flags | Code deployed but gated behind flags | Very Low | Toggle flag off |
| Recreate | Stop old, start new (downtime) | High | Redeploy old version |

## The Twelve-Factor App

Methodology for building SaaS applications (Heroku, 2011). Still the gold standard.

| Factor                 | Principle                                                        |
| ---------------------- | ---------------------------------------------------------------- |
| I. Codebase            | One codebase tracked in VCS, many deploys                        |
| II. Dependencies       | Explicitly declare and isolate dependencies                      |
| III. Config            | Store config in environment variables                            |
| IV. Backing Services   | Treat databases, queues, caches as attached resources            |
| V. Build, Release, Run | Strictly separate build and run stages                           |
| VI. Processes          | Execute the app as stateless processes                           |
| VII. Port Binding      | Export services via port binding                                 |
| VIII. Concurrency      | Scale out via the process model                                  |
| IX. Disposability      | Fast startup, graceful shutdown                                  |
| X. Dev/Prod Parity     | Keep development, staging, and production as similar as possible |
| XI. Logs               | Treat logs as event streams (stdout)                             |
| XII. Admin Processes   | Run admin/management tasks as one-off processes                  |

## Infrastructure as Code (IaC)

**Principle:** All infrastructure (servers, networks, databases, DNS, load balancers) is defined in version-controlled code. No manual changes.

**Tools by layer:**
| Layer | Tools |
|-------|-------|
| Cloud provisioning | Terraform, Pulumi, CloudFormation, CDK |
| Configuration management | Ansible, Chef, Puppet, Salt |
| Container orchestration | Kubernetes, Docker Compose, Nomad |
| Service mesh | Istio, Linkerd, Consul Connect |

**IaC best practices:**

- **Idempotent:** Running the same code twice produces the same result.
- **Immutable infrastructure:** Don't patch servers. Build new images, replace old ones.
- **State management:** Lock Terraform state. Use remote backends (S3, GCS).
- **Modular:** Reuse modules for common patterns (VPC, ECS cluster, RDS instance).
- **Drift detection:** Regularly check that actual infrastructure matches the code.

## Containerization

**Docker best practices:**

- Use minimal base images (`alpine`, `distroless`, `scratch` for Go).
- Multi-stage builds — build in one stage, copy artifacts to a slim runtime image.
- Don't run as root. Use `USER` directive.
- Pin dependency versions. Don't use `latest` tag in production.
- One process per container.
- Use `.dockerignore` to exclude unnecessary files.
- Layer ordering matters — put frequently changing layers last for cache efficiency.

**Kubernetes essentials:**

- **Pod**: Smallest deployable unit (one or more containers).
- **Deployment**: Manages pod replicas and rolling updates.
- **Service**: Stable network endpoint for a set of pods.
- **ConfigMap/Secret**: Externalized configuration.
- **Ingress**: HTTP routing and TLS termination.
- **HPA (Horizontal Pod Autoscaler)**: Auto-scale based on metrics.
- **Readiness/Liveness probes**: Health checks for traffic routing and restart decisions.

## GitOps

**Principle:** Git is the single source of truth for both application code and infrastructure. Changes are applied by syncing git state to the cluster.

**Workflow:**

1. Developer opens PR with infrastructure/config change.
2. PR reviewed and merged.
3. GitOps operator (ArgoCD, Flux) detects the change.
4. Operator applies the change to the cluster.
5. Drift detected? Operator reconciles (reapplies git state).

**Benefits:** Audit trail (git history), rollback (git revert), consistency, declarative.

## Environment Management

- **Dev → Staging → Production** (minimum).
- Config per environment via env vars or config files (never hardcoded).
- Use secrets management (Vault, AWS Secrets Manager, SOPS) — never commit secrets.
- Database migrations run as part of the deployment pipeline, not manually.
- Feature flags decouple deployment from release.

---

_Sources: The Twelve-Factor App (Adam Wiggins), Google SRE Book, Accelerate (Forsgren/Humble/Kim), Kubernetes documentation, Terraform documentation_
