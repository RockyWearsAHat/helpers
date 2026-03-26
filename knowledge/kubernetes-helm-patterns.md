# Kubernetes Helm Patterns — Chart Design, Values, Templating, and Deployment Orchestration

## Overview

Helm is a package manager for Kubernetes, automating templating, multi-resource deployment, and release management. A Helm **chart** bundles templates (rendered to YAML manifests), values (configuration), and metadata. Helm's strength is in managing complexity: large applications reduce to a single `helm install` command with a few value overrides.

However, Helm can enforce tight coupling or create brittle templating logic if patterns are not thoughtful. This note focuses on sustainable chart design patterns, not Helm basics.

## Chart Structure Philosophy

A chart is:

```
mychart/
  Chart.yaml              # Metadata
  Chart.lock              # Dependency lock file
  values.yaml             # Default configuration
  values.schema.json      # JSON Schema (optional, for validation)
  templates/
    deployment.yaml
    service.yaml
    _helpers.tpl          # Reusable template snippets
    NOTES.txt             # Post-install instructions
    tests/
      test-connection.yaml
  charts/                 # Vendored dependency charts
  crds/                   # Custom Resource Definitions
  README.md
```

**Chart.yaml** should use `apiVersion: v2` (Helm 3). The `version` field is the chart packaging version (bumped when templates change); `appVersion` is informational (the application version being deployed). These serve different audiences: operators care about chart version; end users care about appVersion.

**Templates** are Go templates with Sprig functions, rendered into Kubernetes YAML. **No plain YAML in templates.** All manifests should use the templating system.

## Values and Configuration Hierarchy

**values.yaml** is the chart's single source of configuration defaults. Its structure should mirror your application's runtime concepts, not Kubernetes concepts.

### Anti-Pattern: Kubernetes-Centric Values

```yaml
# Avoid this
deployment:
  replicas: 3
  strategy:
    type: RollingUpdate
pod:
  restartPolicy: Always
  terminationGracePeriodSeconds: 30
container:
  imagePullPolicy: IfNotPresent
```

This mirrors `apiVersion: v1 / kind: Deployment` fields, forcing users to understand Kubernetes internals. Fragile: a minor API change breaks the tree.

### Pattern: Application-Centric Values

```yaml
# Better
app:
  name: myapp
  version: 1.2.3
  replicas: 3

image:
  repository: myapp
  tag: "1.2.3"
  pullPolicy: IfNotPresent

config:
  logLevel: info
  databaseUrl: postgresql://localhost:5432/myapp
  cacheExpiry: 3600

resources:
  requests:
    cpu: 100m
    memory: 128Mi
  limits:
    cpu: 500m
    memory: 512Mi

ha:
  enabled: true
  affinity: pod-anti-affinity
```

Values describe the application and its operational mode (HA, resources, logging). The templates translate these into Kubernetes YAML.

### Values Schema and Validation

Use **values.schema.json** (JSONSchema draft 7) to validate values before rendering. This catches typos and invalid configurations early.

```json
{
  "$schema": "https://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "app": {
      "type": "object",
      "properties": {
        "replicas": {
          "type": "integer",
          "minimum": 1,
          "maximum": 100
        },
        "logLevel": {
          "type": "string",
          "enum": ["debug", "info", "warn", "error"]
        }
      },
      "required": ["replicas", "logLevel"]
    }
  }
}
```

Schema validation is performed by `helm lint` and some deployment tools before rendering.

## Templating Patterns

### Template Functions and Pipelines

Helm templates use Go's `{{ }}` syntax with Sprig functions. Templates are verbose; use functions to reduce duplication.

**Example: Generating labels**

Instead of repeating labels in every template:

```yaml
# _helpers.tpl
{{- define "mychart.labels" -}}
app: {{ .Values.app.name }}
version: {{ .Chart.Version }}
heritage: Helm
{{- end }}

# deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  labels:
    {{- include "mychart.labels" . | nindent 4 }}
```

**Pipeline semantics:** `{{ .Values.replicas | int }}` pipes the value through the `int` function. Common pipes: `quote`, `upper`, `int`, `list`, `default`.

**Conditionals:** `{{ if .Values.ha.enabled }}` guards sections of YAML. Be cautious: conditional YAML blocks can create invalid manifests if not careful with indentation.

### The Template Indentation Trap

Helm templates require meticulous indentation to generate valid YAML. A common pattern:

```yaml
metadata:
  labels:
    # This MUST align with YAML indentation
    {{ include "mychart.labels" . | nindent 4 }}
```

`nindent` (newline + indent) correctly indents multi-line template output. Forgetting it produces invalid YAML. Use `helm lint` and `helm template` before installing.

### Range Loops and Iteration

```yaml
# Loop over a list of ports
ports:
{{- range .Values.ports }}
  - name: {{ .name }}
    containerPort: {{ .port }}
{{- end }}
```

Be careful with whitespace. The `-` in `{{-` strips leading whitespace; `-}}` strips trailing. Essential for preventing blank lines in the rendered YAML.

## Dependency Management

Helm charts can depend on other charts (databases, message queues, monitoring). Dependencies are defined in **Chart.yaml**:

```yaml
dependencies:
  - name: postgresql
    version: "~12.1.0"
    repository: https://charts.bitnami.com/bitnami
    condition: postgresql.enabled
    tags:
      - backend-deps
    alias: db
```

**Semantic versioning ranges:**
- `~12.1.0` — Patch updates only (12.1.x)
- `^12.1.0` — Minor + patch (12.x.x)
- `>=12.0.0, <13.0.0` — Range syntax

`helm repo update` fetches the latest chart versions. `helm dependency update` downloads and locks specific versions into `Chart.lock` and the `charts/` directory.

### Conditional Dependency Activation

The `condition` field allows disabling dependencies from values:

```yaml
# values.yaml
postgresql:
  enabled: false  # Disables postgresql dependency, uses external DB
db:
  externalUrl: postgresql://external-db:5432/app
```

**Pattern:** Use conditions for optional services (monitoring, tracing, databases) that end users might provide externally.

### Tags for Grouped Dependencies

```yaml
tags:
  - observability
```

Allows installing only observability dependencies: `helm install --wait --set tags.observability=true myrelease mychart`.

## Lifecycle Hooks

Helm **hooks** run at specific points in the release lifecycle. Defined in manifests with annotations:

```yaml
# Migration before upgrade
apiVersion: batch/v1
kind: Job
metadata:
  name: db-migration
  annotations:
    helm.sh/hook: pre-upgrade
    helm.sh/hook-weight: "-5"
    helm.sh/hook-delete-policy: before-hook-creation,hook-succeeded
spec:
  template:
    spec:
      containers:
        - name: migrate
          image: myapp:1.2.3
          command: ["./migrate.sh"]
      restartPolicy: Never
```

**Supported hooks:**
- `pre-install`: Before resources are created
- `post-install`: After resources are created
- `pre-upgrade`: Before upgrade
- `post-upgrade`: After upgrade
- `pre-delete`: Before delete
- `post-delete`: After delete
- `pre-rollback`, `post-rollback`: Around rollbacks

**Hook weight** (integer) controls execution order within the same hook. Lower numbers execute first. Useful for ensuring database migrations run before the app deployment.

**Hook deletion policy:** Controls cleanup after the hook completes. Common: `hook-succeeded` (delete if successful), `before-hook-creation` (delete old hook before running new one).

## Library Charts

A **library chart** (`type: library` in Chart.yaml) defines reusable templates but is not deployed directly. Useful for enforcing organization-wide standards.

```yaml
# library-chart/Chart.yaml
apiVersion: v2
type: library
name: myorg-library
version: 1.0.0
```

Library charts define templates (e.g., `_deployment.tpl`, `_service.tpl`) that application charts include:

```yaml
# application-chart/templates/deployment.yaml
{{- include "myorg-library.deployment" . }}
```

## Chart Testing

Helm includes a test hook type for validating deployments:

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: myapp-test
  annotations:
    helm.sh/hook: test
spec:
  containers:
    - name: test
      image: myapp:1.2.3
      command: ["./health-check.sh"]
  restartPolicy: Never
```

`helm test <release>` runs test pods and reports success/failure. Tests should be lightweight and quick (< 1 minute typical). They validate the deployment didn't silently break, not comprehensive application testing.

## Packaging and Distribution

### Packaging a Chart

```bash
helm package mychart/  # Creates mychart-1.2.3.tgz
helm repo index .      # Creates index.yaml for Helm repository
```

### OCI-Based Registries

Modern Helm (3.8+) supports OCI registries (Docker Hub, container registries) for chart distribution:

```bash
helm push mychart/ oci://registry.example.com/myorg
helm pull oci://registry.example.com/myorg/mychart
```

**Advantage over traditional Helm repositories:** Uses existing container infrastructure, standard authentication, and namespacing. Treats charts as versioned artifacts like container images.

## Helmfile and Multi-Chart Deployment

**Helmfile** (separate tool) manages multiple Helm releases declaratively:

```yaml
# helmfile.yaml
releases:
  - name: postgres
    namespace: infrastructure
    chart: bitnami/postgresql
    version: 12.1.0
    values:
      - postgresql.enabled: true

  - name: myapp
    namespace: production
    chart: ./charts/myapp
    values:
      - postgresql.enabled: false
        db.externalUrl: postgresql://postgres:5432/app
    hooks:
      - events: ["presync"]
        showlogs: true
        command: "bash"
        args: ["./pre-install-validation.sh"]
```

`helmfile sync` deploys all releases. Useful for managing entire environments (dev, staging, prod) from a single declaration.

## Kustomize vs. Helm

**Kustomize** is a Kubernetes-native templating tool (builtin: `kubectl kustomize`). It overlays base YAML manifests with strategic merge patches.

**Helm:** Full package manager; values-driven; lifecycle management (install, upgrade, delete, test, rollback).

**Kustomize:** Patch-based; no package concept; Kubernetes-first; used for GitOps (ArgoCD, Flux).

**When to choose:**
- **Helm:** Multi-environment deployments with values-driven configuration, release management, external dependencies
- **Kustomize:** Overlaying base manifests, GitOps-native workflows, multiple variants of the same application

Not mutually exclusive; Helm charts can use Kustomize internally or be deployed via ArgoCD (which uses Kustomize for final rendering).

## Mental Model

Helm is about **parametrization, templating, and orchestration**. A well-designed chart:

1. **Values mirror application concepts**, not Kubernetes fields
2. **Templates are DRY**, using helpers and includes
3. **Dependencies are optional and conditional**
4. **Hooks automate complex deployment sequences**
5. **Schemas validate early**

Poor chart design leads to either overly rigid charts (hard-coded values) or incomprehensibly flexible ones (nested template logic). Aim for the middle: straightforward values, templates that translate values to manifests, documented assumptions.