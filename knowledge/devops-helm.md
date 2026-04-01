# Helm — Kubernetes Package Manager

## Chart Structure

```
mychart/
  Chart.yaml          # metadata: name, version, appVersion, dependencies
  Chart.lock          # locked dependency versions
  values.yaml         # default configuration values
  values.schema.json  # optional JSON Schema for values validation
  .helmignore         # files to exclude from packaging
  templates/
    deployment.yaml
    service.yaml
    ingress.yaml
    _helpers.tpl      # named template definitions (partials)
    NOTES.txt         # post-install usage notes
    tests/
      test-connection.yaml
  charts/             # dependency chart archives (.tgz)
  crds/               # Custom Resource Definitions (applied before templates)
```

### Chart.yaml

```yaml
apiVersion: v2 # v2 for Helm 3
name: myapp
version: 1.2.3 # chart version (semver, bumped on chart changes)
appVersion: "4.5.6" # version of the app being deployed
description: My application
type: application # or "library"
keywords: [web, api]
home: https://example.com
maintainers:
  - name: Alice
    email: alice@example.com
dependencies:
  - name: postgresql
    version: "~12.1.0" # semver range
    repository: https://charts.bitnami.com/bitnami
    condition: postgresql.enabled
    alias: db
```

**version vs appVersion:** `version` is the chart packaging version (must change when templates change). `appVersion` is the application version being deployed (informational, shown in `helm list`).

## Template Language

Helm uses Go templates with Sprig functions. Templates render to Kubernetes YAML manifests.

### Built-in Objects

| Object          | Description                                                                  |
| --------------- | ---------------------------------------------------------------------------- |
| `.Values`       | Values from values.yaml + overrides                                          |
| `.Release`      | Release info: `.Name`, `.Namespace`, `.IsInstall`, `.IsUpgrade`, `.Revision` |
| `.Chart`        | Chart.yaml contents: `.Name`, `.Version`, `.AppVersion`                      |
| `.Capabilities` | Cluster capabilities: `.KubeVersion`, `.APIVersions`                         |
| `.Template`     | Current template: `.Name`, `.BasePath`                                       |
| `.Files`        | Access non-template files in chart                                           |

### Essential Functions

```yaml
# String functions
{{ .Values.name | upper }}
{{ .Values.name | quote }}          # wrap in double quotes
{{ .Values.name | default "myapp" }}
{{ .Values.name | trim | lower }}
{{ printf "%s-%s" .Release.Name .Chart.Name }}

# Required values (fail if missing)
{{ required "A database host is required" .Values.db.host }}

# Type conversion
{{ .Values.port | int }}
{{ .Values.enabled | toString }}

# YAML/JSON rendering
{{ .Values.annotations | toYaml | nindent 4 }}
{{ .Values.config | toJson }}

# Lookup existing cluster resources
{{ lookup "v1" "ConfigMap" "default" "my-config" }}

# Include files
{{ .Files.Get "config/app.conf" }}
{{ .Files.Glob "configs/*" }}
```

### Flow Control

```yaml
# if/else
{{- if .Values.ingress.enabled }}
apiVersion: networking.k8s.io/v1
kind: Ingress
# ...
{{- else if .Values.route.enabled }}
# OpenShift Route
{{- end }}

# with — changes scope (. becomes the value)
{{- with .Values.nodeSelector }}
nodeSelector:
  {{- toYaml . | nindent 2 }}
{{- end }}

# range — iterate
{{- range .Values.servers }}
- host: {{ .host }}
  port: {{ .port | default 8080 }}
{{- end }}

# range with index
{{- range $index, $val := .Values.items }}
  item-{{ $index }}: {{ $val }}
{{- end }}

# range over a map
{{- range $key, $val := .Values.env }}
  {{ $key }}: {{ $val | quote }}
{{- end }}
```

### Whitespace Control

`{{-` trims leading whitespace, `-}}` trims trailing whitespace. Critical for clean YAML output.

```yaml
# Without trim — produces blank lines
{{ if .Values.debug }}
  debug: true
{{ end }}

# With trim — clean output
{{- if .Values.debug }}
  debug: true
{{- end }}
```

## Named Templates (\_helpers.tpl)

```yaml
# templates/_helpers.tpl
{{- define "myapp.fullname" -}}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name .Chart.Name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}

{{- define "myapp.labels" -}}
helm.sh/chart: {{ printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" }}
app.kubernetes.io/name: {{ .Chart.Name }}
app.kubernetes.io/instance: {{ .Release.Name }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{- define "myapp.selectorLabels" -}}
app.kubernetes.io/name: {{ .Chart.Name }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}
```

Usage in templates:

```yaml
metadata:
  name: { { include "myapp.fullname" . } }
  labels: { { - include "myapp.labels" . | nindent 4 } }
spec:
  selector:
    matchLabels: { { - include "myapp.selectorLabels" . | nindent 6 } }
```

**`include` vs `template`:** Prefer `include` — it captures output as a string so you can pipe it (`| nindent 4`). `template` inserts inline and can't be piped.

### tpl Function

Render a string as a template (useful for values that contain template expressions):

```yaml
# values.yaml
greeting: "Hello {{ .Release.Name }}"

# template
{{ tpl .Values.greeting . }}  # renders: Hello my-release
```

## Hooks

Hooks run at specific lifecycle points. Implemented as regular templates with annotations.

```yaml
apiVersion: batch/v1
kind: Job
metadata:
  name: {{ .Release.Name }}-db-migrate
  annotations:
    "helm.sh/hook": pre-upgrade,pre-install
    "helm.sh/hook-weight": "-5"        # lower runs first
    "helm.sh/hook-delete-policy": hook-succeeded
spec:
  template:
    spec:
      restartPolicy: Never
      containers:
        - name: migrate
          image: {{ .Values.image.repository }}:{{ .Values.image.tag }}
          command: ["./migrate", "up"]
```

| Hook            | Timing                                               |
| --------------- | ---------------------------------------------------- |
| `pre-install`   | After templates render, before any resources created |
| `post-install`  | After all resources created                          |
| `pre-delete`    | Before any resources deleted                         |
| `post-delete`   | After all resources deleted                          |
| `pre-upgrade`   | After templates render, before any resources updated |
| `post-upgrade`  | After all resources updated                          |
| `pre-rollback`  | Before rollback                                      |
| `post-rollback` | After rollback                                       |
| `test`          | When `helm test` runs                                |

**Hook weights:** Integer string, lower values run first. Default is `0`. Use negative weights for ordering: `-10` runs before `-5`.

**Delete policies:** `hook-succeeded` (delete after success), `hook-failed` (delete after failure), `before-hook-creation` (delete previous hook before new one runs — most common).

## Dependencies

```bash
# Download dependencies to charts/ directory
helm dependency update ./mychart
helm dependency build ./mychart

# List dependencies
helm dependency list ./mychart
```

### Conditional Dependencies

```yaml
# Chart.yaml
dependencies:
  - name: redis
    version: "17.x.x"
    repository: https://charts.bitnami.com/bitnami
    condition: redis.enabled # toggle via values
    tags:
      - cache # toggle groups via tags
```

```yaml
# values.yaml
redis:
  enabled: true
tags:
  cache: true
```

### Library Charts

Type `library` in Chart.yaml. Contain only named templates (no renderable templates). Used for shared helpers across charts.

```yaml
# library chart Chart.yaml
apiVersion: v2
name: common
type: library
version: 1.0.0
```

## OCI Registries

Helm 3.8+ supports OCI (Open Container Initiative) registries for chart storage.

```bash
# Login to registry
helm registry login ghcr.io -u USERNAME

# Package and push
helm package ./mychart
helm push mychart-1.2.3.tgz oci://ghcr.io/myorg/charts

# Install directly from OCI
helm install myrelease oci://ghcr.io/myorg/charts/mychart --version 1.2.3

# Pull
helm pull oci://ghcr.io/myorg/charts/mychart --version 1.2.3
```

## Release Management

```bash
# Install
helm install myrelease ./mychart -f custom-values.yaml -n mynamespace --create-namespace

# Upgrade (or install if not exists)
helm upgrade --install myrelease ./mychart -f prod-values.yaml

# Rollback to previous revision
helm rollback myrelease 2

# List releases
helm list -n mynamespace
helm history myrelease

# Uninstall
helm uninstall myrelease -n mynamespace

# Dry run + debug (render templates without applying)
helm install myrelease ./mychart --dry-run --debug

# Template only (render to stdout without cluster)
helm template myrelease ./mychart -f values.yaml
```

### Values Hierarchy (Ascending Priority)

1. `values.yaml` in parent chart
2. `values.yaml` in subchart
3. `-f / --values` file passed to helm CLI
4. `--set` individual values on CLI
5. `--set-string` (forces string type)
6. `--set-file` (set value from file contents)
7. `--set-json` (set from JSON string)

```bash
helm install myrelease ./mychart \
  -f base.yaml \
  -f prod-overlay.yaml \
  --set image.tag=v2.0.1 \
  --set-string metadata.buildId="12345"
```

## Testing

### Built-in Tests

```yaml
# templates/tests/test-connection.yaml
apiVersion: v1
kind: Pod
metadata:
  name: {{ include "myapp.fullname" . }}-test
  annotations:
    "helm.sh/hook": test
spec:
  restartPolicy: Never
  containers:
    - name: wget
      image: busybox
      command: ['wget', '--spider', 'http://{{ include "myapp.fullname" . }}:{{ .Values.service.port }}']
```

```bash
helm test myrelease
```

### helm-unittest

Plugin for unit testing templates without a cluster:

```yaml
# tests/deployment_test.yaml
suite: deployment tests
templates:
  - deployment.yaml
tests:
  - it: should set replicas from values
    set:
      replicaCount: 5
    asserts:
      - equal:
          path: spec.replicas
          value: 5
  - it: should fail without image
    set:
      image.repository: null
    asserts:
      - failedTemplate: {}
```

### Linting

```bash
helm lint ./mychart                    # syntax + best practices
helm lint ./mychart -f prod-values.yaml  # lint with specific values
```

## Helmfile

Declarative spec for deploying multiple Helm releases:

```yaml
# helmfile.yaml
repositories:
  - name: bitnami
    url: https://charts.bitnami.com/bitnami

environments:
  production:
    values:
      - environments/production.yaml
  staging:
    values:
      - environments/staging.yaml

releases:
  - name: api
    namespace: { { .Environment.Name } }
    chart: ./charts/api
    values:
      - values/api/common.yaml
      - values/api/{{ .Environment.Name }}.yaml
    set:
      - name: image.tag
        value: { { requiredEnv "IMAGE_TAG" } }

  - name: redis
    namespace: { { .Environment.Name } }
    chart: bitnami/redis
    version: 17.11.6
    condition: redis.enabled
```

```bash
helmfile -e production sync     # apply all releases for production
helmfile -e staging diff        # show diff before applying
helmfile -e production destroy  # remove all releases
```
