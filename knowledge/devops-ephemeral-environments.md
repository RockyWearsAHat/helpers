# Ephemeral Environments — On-Demand Preview, Development & Integration Spaces

## Overview

**Ephemeral environments** are short-lived, on-demand copies of production infrastructure provisioned for specific workflows: pull request review, feature testing, developer experimentation, or integration testing. Unlike static staging/QA environments that live forever, ephemeral environments are created, used, and destroyed within hours or minutes—reducing infrastructure costs, eliminating configuration drift, and giving developers their own isolated playground.

The separation of concerns: **local development** (your machine), **ephemeral** (shared cloud sandbox, per-PR or per-developer), and **persistent** (staging, production) maps to different developer needs.

## Design Patterns

### Namespace-per-Branch / Namespace-per-PR

Deploy a full application copy to its own Kubernetes namespace for each PR or feature branch. Git webhooks trigger deployment pipelines.

**Mechanics:**
- CI/CD controller watches for PR events: creates namespace `pr-{number}` or `branch-{slug}`, deploys Helm chart with environment-specific overrides
- DNS or ingress routing: `preview-pr-42.example.com` points to `pr-42/ingress-controller`
- Database isolation: lightweight copy (schema clone, anonymized production data, or fresh test data)
- Automatic cleanup: namespace destroyed on PR merge or after TTL

**Trade-offs:**
- ✓ Production-identical infrastructure; full integration testing possible
- ✓ Easy debugging: developers are not fighting shared resource contention
- ✗ Resource multiplication: 20 open PRs = 20 clusters' worth of pods
- ✗ Data management challenge: production data at scale can't be cloned per-PR
- ✗ Slow feedback: manifests → CI → deployment → ingress DNS propagation can take 2-5 min

### Cluster-Shared with Namespace Isolation

Single long-running cluster; ephemeral namespaces share nodes but are network-isolated and have resource quotas.

**Mechanics:**
- Faster deployment (nodes already up)
- Network policies and RBAC restrict cross-namespace contact
- Pod resource requests/limits prevent one namespace from starving others
- No per-PR DNS; access via port-forward or shared ingress with path-based routing

**Trade-offs:**
- ✓ Lower resource cost; fast startup
- ✓ Trivial to scale (just add namespaces)
- ✗ Noisy neighbors: resource contention possible if quotas aren't strict
- ✗ Single cluster failure cascades to all features

### Container-Image-per-Commit

Build container images for each commit; stage them in a registry. Deploy orchestration selects which image(s) to run.

**Mechanics:**
- Push triggers `docker build` → push to registry with tag `pr-42-abc123def`
- Deployment manifests reference that tag, avoiding retagging `latest`
- Rollback is trivial: point back to a prior tag
- Immutability aids debugging: re-run the exact same image

**Prerequisite:** Registry is the source of truth, not Git. Multiple services can co-exist because they're tagged uniquely.

## Local Development Alternatives

Two patterns offer similar isolation without the cloud cost.

### Telepresence - Remote Debugging in the Cluster

Telepresence intercepts traffic to a remote Kubernetes service, forwarding it to a local process. Reverse proxying: your laptop's port 5000 runs your service; the cluster sees remote calls arrive there.

**Use case:** You're developing a single microservice; the other 99 services live in the remote cluster.

**Mechanism:**
- `telepresence connect` → installs local SOCKS proxy → routes K8s-bound traffic to remote cluster
- `telepresence intercept myservice --port 5000:5000` → traffic destined for `myservice:5000` redirects to localhost:5000
- Your local code runs unmolested; you see real traffic, real databases, real dependencies

**Trade-offs:**
- ✓ Single service runs locally (IDE, hot reload, debugger attachment)
- ✓ Real traffic and data from the cluster
- ✓ No Docker/container knowledge required
- ✗ Latency: all cluster dependencies are remote calls
- ✗ State is shared with teammates (if they trigger a bug, your local instance sees it)
- ✗ Network-dependent: WiFi flakes = debugging session dies

### Tilt / Skaffold - Synchronized Local Dev

Both tools automate the edit→build→deploy cycle for a single developer or small team.

**Tilt** (Python/Starlark-based):
- `Tiltfile` declares services, build steps, triggers (on file change)
- Watches for edits; on change: rebuild image, push to local registry, update K8s manifest, wait for rollout
- Web UI shows build logs, resource health, logs from running pods
- Integrates with local Minikube or Docker Desktop K3s

**Skaffold** (YAML-based):
- Similar workflow: watch → build → deploy; plugin architecture
- Better CI/CD integration (can run in pipeline without dashboard)
- Supports multiple build strategies (Docker, Buildpack, Bazel, custom)
- File sync (instead of always rebuilding images for small changes)

**Trade-offs:**
- ✓ Feedback loop < 10 sec for most code changes
- ✓ Full app runs locally (cheaper, offline-capable)
- ✓ Deterministic (your changes, you see them first)
- ✗ Requires local K8s cluster (Minikube, Kind, Docker Desktop)
- ✗ Memory/CPU overhead on laptop (4-8GB RAM typical)
- ✗ Database/state reset requires manual work or custom init scripts

## Cloud-Based Development Environments

### Gitpod / GitHub Codespaces

Ephemeral container-based development environments, preconfigured per project, launched from Git branches.

**Gitpod:**
- `.gitpod.yml` declares base image, init tasks (npm install, migrations), ports to expose, VS Code extensions
- Click "Open in Gitpod" on PR; redirects to `gitpod.io/#branch-url`
- Full VS Code in browser; full file system and Docker daemon inside
- Builds on top of Docker/Linux container semantics

**GitHub Codespaces:**
- GitHub-native equivalent; `.devcontainer` configuration (Docker Compose or image + features)
- Tight integration with GitHub UI; one-click launch from PR or branch
- Storage persists across sessions (not ephemeral by default, but can be deleted)
- Billing: per-minute compute + storage

**Trade-offs:**
- ✓ Onboarding: contributor clones repo → clicks button → has full dev env in 5 min
- ✓ No local Docker/K8s knowledge required
- ✓ Consistent across team (everyone's image is identical)
- ✗ Latency & bandwidth: all I/O goes through browser/SSH tunnel
- ✓ Cost: pay per-minute (scaling down to 0 when not in use is handled automatically by cloud provider)
- ✗ Networking: outbound requests from Codespace may be rate-limited by ISPs/CDNs

## Data Management for Ephemeral Environments

### Challenge

Production databases range from MB to TB. Spinning up a daily snapshot per PR is:
1. Expensive (storage, backup/restore time)
2. Slow (restores can take minutes)
3. Risky (real data in ephemeral spaces)

### Strategies

**Schema + Seed Data:**
- Copy schema only (DDL); populate with fixed test fixtures
- Fast, reproducible, privacy-safe
- Downside: tests don't exercise real data distributions (skewed indices, large tables)

**Production Data Anonymization:**
- Snapshot production; run anonymization job (PII removal, data scrambling)
- Deployed ephemeral env gets anonymized copy
- Trade-off: one day old, not real-time, still large

**Lightweight Cloning (when supported):**
- Some databases (PostgreSQL, MySQL) support Copy-on-Write clones
- Physical copy is lazy; clone is ready in seconds, minimal storage overhead
- As you write changes, deltas are tracked
- Cleanup: drop the clone, discard deltas

**Synthetic Data Generation:**
- Factory or scenario-based generators produce realistic test data
- Fast, safe, but doesn't catch bugs that depend on real distribution
- Use in combination with targeted production snapshot for critical flows

## Infrastructure-as-Code for Ephemeral Deployment

### Terraform / Pulumi

Define ephemeral environments as code; instantiate them on demand.

**Pattern:**
```
# variables: pr_number, branch_name, image_tag
# main.tf: provision RDS subnet, namespace, deployment, ingress
# taint/destroy on PR merge/TTL expiration
```

**Advantages:**
- Reproducible (same .tf file produces same infra)
- Debugging: `terraform state show` reveals what was provisioned
- Cost tracking per environment tag

### Helm + Values Overrides

For Kubernetes, parameterize Helm values for environment-specific settings.

```yaml
# values-pr.yaml
replicas: 1
resources:
  requests: {cpu: 100m, memory: 128Mi}  # minimal
database: pr42-preview  # separate DB
redis_cache: false  # disable for speed
```

Reduces drift; Git tracks all values used.

## Cost Optimization

### Resource Quotas & Right-Sizing

- Ephemeral workloads don't need HA (1 replica, no redundancy)
- CPU requests 1/5 of production, memory similarly reduced
- 1 shared RDS instance with parameter groups per env (not per-PR DB)
- Disable expensive features: Redis caching, ElasticSearch, auto-scaling

### Automatic Cleanup

- TTL: 4 hours for interactive development, 24 hours for CI/CD runs
- Webhook: delete namespace/env on PR close or merge
- Unattended: cron job deletes environments not accessed for N hours

**Cost impact:** 30 active PRs × 8 GB each × $0.001/GB/hr ≈ $240/month. Reduce to 2 GB per env (remove caches, minimize DB copies) ≈ $80/month.

### Shared Infrastructure

- Single cluster, shared node pool, isolated by namespace + network policy
- Shared data stores (PostgreSQL parameter groups, Redis partitions, S3 prefixes)
- Amortizes fixed costs (EKS cluster, NAT gateway) across many environments

## Operational Considerations

### Debugging

**Logs:**
- Aggregate ephemeral env logs separately (namespace suffix in log labels)
- Retention: 7 days (shorter than production, cheaper)
- Search by PR number / branch name

**Port-Forward / Direct Access:**
```bash
kubectl port-forward -n pr-42 pod/api-xyz 5000:5000
# localhost:5000 now talks to remote pod
curl -v http://localhost:5000/health
```

**Headless Debugging:**
- Services in ephemeral namespaces can make outbound calls to production databases (read-only credentials)
- Use separate DB connections from production to avoid accidental writes

### Monitoring & Alerts

**Disable production-style alerts** for ephemeral environments:
- No PagerDuty/Slack notifications for transient outages
- Metrics still flow to central store for post-mortem analysis

**Keep narrow scope:** alert on obvious failures (pod CrashLoop, Out of Memory) but not on slow queries or high latency.

## Related Topics

See also: [Progressive Delivery](progressive-delivery.md) (canary releases, traffic shifting), [Kubernetes Security](devops-kubernetes-security.md) (namespace isolation, network policies), [GitOps Patterns](infrastructure-gitops-patterns.md) (declarative deployment), [CI/CD Patterns](devops-cicd-patterns.md) (trigger automation).