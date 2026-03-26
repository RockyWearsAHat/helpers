# Container Security: Image Hardening, Base Images, and Runtime Isolation

## Minimal Base Images: Reducing Attack Surface

Smaller base images mean fewer packages, fewer CVEs, faster startup, smaller storage/bandwidth. But tradeoff is reduced tools for debugging/operations.

### Hierarchy by Size and Feature Set

| Image        | Size  | Use Case                                 | Debugging                      |
| ------------ | ----- | ---------------------------------------- | ------------------------------ |
| `scratch`    | 0KB   | Static binaries only (Go)               | None; layers on FROM scratch   |
| `distroless` | 1-20MB| Applications + libc; excludes shell, apt | Limited; use ephemeral debug pod |
| `alpine`     | 5-7MB | Minimal Linux; `apk` package manager    | `sh`, `apk add` available       |
| `busybox`    | 1-2MB | Minimal POSIX utilities; shell included | `/bin/sh`                       |
| `debian`     | 50-80MB| Full OS + apt package manager           | Full debugging toolchain        |
| `ubuntu`     | 70+MB | Standard development Linux              | Everything included            |

**distroless images** (Google's) contain only application + libc + glibc; no shell, no package manager, no debugging utilities. Reduces attack surface and CVE count dramatically.

Example Dockerfile Multi-stage using distroless:

```dockerfile
FROM golang:1.21-alpine AS builder
WORKDIR /app
COPY go.mod go.sum ./
RUN go mod download
COPY . .
RUN CGO_ENABLED=0 GOOS=linux GOARCH=amd64 go build -o app main.go

# Final image: distroless base for Go applications
FROM gcr.io/distroless/base-debian12

COPY --from=builder /app/app /app
USER nonroot:nonroot
ENTRYPOINT ["/app"]
```

**Scanning distroless images:**
```bash
trivy image gcr.io/distroless/base-debian12
# Usually reports 0 vulns because base layers are minimal and actively maintained
```

### Alpine: Lightweight but Different

Alpine uses `musl` libc (not glibc). Issues:
- Some binaries compiled against glibc fail on Alpine (timezone handling, DNS resolution, locale)
- Larger final image sizes than distroless (includes shell, curl, wget for debugging)
- CVEs can be slower to patch (smaller volunteer team)

Use Alpine for: building blocks (CI tools, utilities), development environments. Prefer distroless for production application images.

### scanners-detect base image quality:

```bash
# Trivy identifies base image and known vulnerabilities
trivy image --severity HIGH,CRITICAL myapp:latest

# Grype (Anchore) provides similar analysis
grype myapp:latest -o table
```

---

## Dockerfile Security Patterns

### 1. Multi-Stage Builds: Separate Build and Runtime

```dockerfile
# Stage 1: Build
FROM node:20-bookworm AS builder
WORKDIR /build
COPY package.json package-lock.json ./
RUN npm ci
COPY . .
RUN npm run build && npm prune --production

# Stage 2: Runtime (final image)
FROM node:20-alpine
WORKDIR /app
# Copy only necessary artifacts from builder
COPY --from=builder --chown=node:node /build/dist ./dist
COPY --from=builder --chown=node:node /build/node_modules ./node_modules
COPY --from=builder --chown=node:node /build/package.json ./

# Non-root user
USER node

EXPOSE 3000
HEALTHCHECK --interval=30s --timeout=3s --retries=3 \
  CMD node -e "require('http').get('http://localhost:3000/health', (r)=>r.statusCode==200 ? process.exit(0) : process.exit(1))"

CMD ["node", "dist/server.js"]
```

**Benefits:**
- Builder image discarded; only final image shipped (no build tools, no source code)
- Dev dependencies (mocha, webpack, ts-node) don't ship to production
- Smaller container = faster deployments, less attack surface

### 2. Layer Caching Optimization

```dockerfile
FROM node:20-alpine

WORKDIR /app

# Layer 1: Dependencies (cached unless package.json changes)
COPY package.json package-lock.json ./
RUN npm ci

# Layer 2: Source code (frequent changes, invalidates this layer only)
COPY . .

# Layer 3: Build
RUN npm run build

USER node
EXPOSE 3000
CMD ["node", "dist/server.js"]
```

**Anti-pattern (rebuilds all layers on any change):**

```dockerfile
COPY . .                    # Copy all code
RUN npm ci && npm run build # Rebuild everything
```

### 3. Non-Root User

```dockerfile
# Create dedicated user and group
RUN addgroup --system --gid 1000 app && \
    adduser --system --uid 1000 --ingroup app app

WORKDIR /app
COPY --chown=app:app . .

USER app  # Switch before ENTRYPOINT

ENTRYPOINT ["node", "app.js"]
```

**Why:** Compromised process runs as non-root, cannot modify system binaries, cannot access privileged resources. Privilege escalation is harder (must exploit kernel vulnerability).

### 4. Read-Only Root Filesystem

In Kubernetes or Docker, mount root as read-only:

```yaml
# Kubernetes Pod spec
spec:
  securityContext:
    readOnlyRootFilesystem: true
  containers:
  - name: app
    volumeMounts:
    - name: tmp
      mountPath: /tmp  # Application scratch space (writable)
    - name: cache
      mountPath: /app/cache
  volumes:
  - name: tmp
    emptyDir: {}
  - name: cache
    emptyDir: {}
```

Dockerfile must support this:

```dockerfile
RUN mkdir -p /app /tmp && \
    chmod 1777 /tmp

WORKDIR /app
```

**Benefits:** Prevents malware persistence (cannot modify binaries), limits damage from exploits (can only write to mounted emptyDir volumes).

### 5. Health Checks

```dockerfile
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
  CMD curl -f http://localhost:3000/health || exit 1
```

Docker and orchestrators use health checks to restart unhealthy containers. Prevents zombie processes that respond but don't work.

---

## Runtime Security: Linux Capabilities, seccomp, AppArmor, SELinux

### Linux Capabilities: Granular Privilege Model

By default, containers run with all `root` capabilities. Drop unnecessary ones to reduce privilege if container is compromised.

```dockerfile
FROM ubuntu:22.04
RUN apt-get update && apt-get install -y nginx
EXPOSE 80 443
```

By default, this container has CAP_SYS_ADMIN, CAP_NET_ADMIN, CAP_SYS_RESOURCE, etc. — power to modify kernel, network, resource limits.

```bash
# Run container, dropping most capabilities (Docker default: drops SYS_ADMIN, NET_ADMIN, etc.)
docker run --cap-drop=ALL --cap-add=NET_BIND_SERVICE nginx
# nginx needs CAP_NET_BIND_SERVICE to bind port 80; everything else dropped
```

**Kubernetes Pod spec:**

```yaml
spec:
  containers:
  - name: app
    securityContext:
      runAsNonRoot: true
      runAsUser: 1000
      capabilities:
        drop:
        - ALL
        add:
        - NET_BIND_SERVICE  # only if app binds to port < 1024
      allowPrivilegeEscalation: false
```

**Common capabilities:**
- `CAP_NET_BIND_SERVICE` — bind to ports < 1024
- `CAP_SYS_ADMIN` — mount filesystems, modify namespaces (almost never needed in containers)
- `CAP_NET_ADMIN` — modify network routes, configure interfaces
- `CAP_SYS_PTRACE` — attach debuggers (useful for troubleshooting; security risk)

### seccomp: System Call Filtering

seccomp (secure computing mode) restricts which system calls a process can use. Container image can include a seccomp profile.

```json
{
  "defaultAction": "SCMP_ACT_ERRNO",
  "defaultErrnoRet": 1,
  "archMap": [
    {
      "architecture": "SCMP_ARCH_X86_64",
      "subArchitectures": ["SCMP_ARCH_X86", "SCMP_ARCH_X32"]
    }
  ],
  "syscalls": [
    {
      "names": [
        "open", "read", "write", "close", "stat", "fstat",
        "mmap", "mprotect", "clone", "execve", "exit", "exit_group"
      ],
      "action": "SCMP_ACT_ALLOW"
    }
  ]
}
```

**Apply via Docker:**

```bash
docker run --security-opt seccomp=/path/to/profile.json myapp
```

**Kubernetes:**

```yaml
spec:
  securityContext:
    seccompProfile:
      type: Localhost
      localhostProfile: app-seccomp.json
```

**Benefit:** If container is compromised, attacker can only call whitelisted syscalls. Blocks many privilege escalation techniques (load kernel modules, access raw sockets, etc.).

### AppArmor and SELinux: Mandatory Access Control

**AppArmor** (Ubuntu/Debian): Profile defines resource access rules (file read/write perms, capability access, network).

```bash
# AppArmor profile: /etc/apparmor.d/docker-ns
profile docker-ns flags=(attach_disconnected, mediate_deleted) {
  deny /proc/sysrq-h rwkl,
  deny /proc/kcore rwkl,
  deny /proc/mem rwkl,
  deny /proc/kmem rwkl,
  deny /sys/firmware/** rwkl,
  allow /proc/*/status r,
  allow /proc/*/stat r,
}
```

Docker loads profile:

```bash
docker run --security-opt apparmor=docker-ns myapp
```

**SELinux** (RHEL/CentOS): Similar to AppArmor; different syntax and enforcement model. Most Kubernetes clusters use it in permissive mode (log violations, don't enforce) to avoid compatibility issues.

---

## Image Scanning and Vulnerability Management

### 1. Build-Time Scanning (CI Pipeline)

```yaml
# GitHub Actions example
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: docker/build-push-action@v5
        with:
          context: .
          push: false
          tags: myapp:latest
          outputs: type=docker,dest=/tmp/image.tar

      - uses: anchore/scan-action@v3
        with:
          image: myapp:latest
          registry-username: ${{ secrets.REGISTRY_USER }}
          registry-password: ${{ secrets.REGISTRY_PASS }}

      - run: |
          if grep -q "CRITICAL\|HIGH" grype-report.json; then
            echo "::error::High-risk vulnerabilities found"
            exit 1
          fi
```

**Fail build if CRITICAL or HIGH vulns detected.** This prevents shipping vulnerable images.

### 2. Registry-Time Scanning (Continuous)

Registry (Docker Hub, ECR, Harbor) scans images after push. New CVE discovered? Registry re-scans old images and alerts.

```bash
# AWS ECR: enable automatic image scanning
aws ecr put-image-scanning-configuration \
  --repository-name myapp \
  --image-scan-config scanOnPush=true
```

Kubernetes admission controller can block image pull if image has CRITICAL vulns:

```yaml
apiVersion: admissionregistration.k8s.io/v1
kind: ValidatingWebhookConfiguration
metadata:
  name: image-security-webhook
webhooks:
- name: image-security.example.com
  clientConfig:
    service:
      name: image-security-webhook
      namespace: default
      path: "/validate"
  rules:
  - operations: ["CREATE", "UPDATE"]
    apiGroups: [""]
    apiVersions: ["v1"]
    resources: ["pods"]
```

Webhook fetches image scan results before pod launch; blocks if CRITICAL vulns found.

### 3. Runtime Scanning (Falco)

Falco monitors running containers for suspicious syscalls and process behavior.

```yaml
# Kubernetes DaemonSet
spec:
  containers:
  - name: falco
    image: falcosecurity/falco:latest
    securityContext:
      privileged: true
    volumeMounts:
    - name: docker
      mountPath: /var/run/docker.sock
    - name: cgroup
      mountPath: /hostfs/sys/fs/cgroup
```

Falco rules detect:

```yaml
- rule: Unauthorized Process Execution
  desc: Detect execution of unexpected binaries
  condition: >
    spawned_process and
    container and
    container.image.repository = "myapp" and
    not proc.name in (
      app, node, npm, python, java
    )
  output: >
    Unexpected process spawned
    (user=%user.name proc=%proc.name
    image=%container.image.repository)
  priority: WARNING
```

Alert on runtime anomalies immediately; log to SIEM for incident response.

---

## Supply Chain Security: Signing and Verification

### Image Signing with Cosign

```bash
# Generate key pair
cosign generate-key-pair

# Sign image during build
cosign sign --key cosign.key gcr.io/myapp/image:v1

# Verify before deployment
cosign verify --key cosign.pub gcr.io/myapp/image:v1
```

Kubernetes admission controller enforces signature verification:

```yaml
apiVersion: admissionregistration.k8s.io/v1
kind: ClusterPolicy
metadata:
  name: image-signature-verification
spec:
  validationFailureAction: enforce
  rules:
  - name: Check image signature
    match:
      resources:
        kinds:
        - Pod
    verifyImages:
    - imageReferences:
      - "gcr.io/myapp/*"
      attestors:
      - name: signature-check
        entries:
        - keys:
            publicKeys: |
              -----BEGIN PUBLIC KEY-----
              MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAE...
              -----END PUBLIC KEY-----
```

### SBOM (Software Bill of Materials) Generation

```bash
# Generate SBOM during build
syft gcr.io/myapp/image:v1 -o cyclonedx-json > sbom.json

# Attach to image
cosign attach sbom --sbom sbom.json gcr.io/myapp/image:v1
```

SBOM includes all libraries, versions, licenses. Supply chain tooling uses SBOM for:
- CVE scanning (faster than re-analyzing layers)
- License compliance scanning
- Provenance tracking
- Dependency analysis

---

## Practical Hardening Checklist

- [ ] Use distroless or Alpine base image; justify if using larger image
- [ ] Multi-stage build; dev dependencies not in final image
- [ ] Run as non-root user (UID >= 1000)
- [ ] Drop all capabilities except those required (CAP_DROP=ALL, CAP_ADD if needed)
- [ ] Enable seccomp profile for sandboxing
- [ ] Read-only root filesystem with writable /tmp and /var/tmp
- [ ] Health checks in Dockerfile (HEALTHCHECK instruction)
- [ ] No secrets in image; use external secret management
- [ ] Scan at build time (reject CRITICAL/HIGH vulns before push)
- [ ] Scan in registry and at deployment time (continuous re-checking)
- [ ] Sign images and verify signatures at deployment
- [ ] Include SBOM for supply chain traceability
- [ ] Monitor running containers with Falco for anomalous behavior