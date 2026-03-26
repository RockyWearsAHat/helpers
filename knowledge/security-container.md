# Container Security — Image Scanning, Runtime Isolation, and Supply Chain

## Overview

Container security spans the full lifecycle: **build time** (image scanning, signing), **deployment time** (access control, secrets), and **runtime** (process isolation, monitoring). No single layer is sufficient; defense-in-depth requires that compromise at one layer is constrained and detected by others.

---

## Image Scanning and Vulnerability Analysis

Container images are composed of base image + application layers. Each layer may contain known vulnerabilities (CVEs). Scanning detects them before deployment.

### Static Image Scanning

Analyzing image layers without running the container:

1. **Unpack image:** Extract filesystem layers
2. **Identify packages:** Parse package managers (apt, rpm, apk, npm, pip) to extract installed packages and versions
3. **Match against vulnerability database:** Compare package versions against CVE feeds (NVD, Debian Security Tracker, etc.)
4. **Report vulnerabilities:** List CVEs with severity, patches available, transitive dependencies

**Tools:** Trivy, Anchore Grype, Aqua Microscanner, Clair, Snyk. Most are free/open-source with optional commercial analysis.

**Limitations:**
- Only discovers known CVEs. Zero-days in unanalyzed packages are missed.
- Database has lag: CVEs take days/weeks to appear in feeds
- Transitive dependencies: Scanning npm packages requires parsing package-lock.json, recursing through dependencies (exponential). Some tools use SBoM (Software Bill of Materials) to skip recursive crawl.
- False positives: Some scanners flag CVEs that don't affect your specific code path (vulnerable function not called).

### SBOM (Software Bill of Materials)

Formal inventory of software components (libraries, versions, licenses, checksums). Formats: SPDX, CycloneDX.

Standard SBOM generated at build time and included in image (or registry metadata). Scanning tools use SBOM instead of re-analyzing packages, improving speed and accuracy.

**Adoption:** Increasingly required in regulated industries and government contracts (e.g., SLSA framework, EO secure software development).

### Scanning Frequency

- **Build time:** Mandatory; reject images with critical CVEs before publishing to registry
- **Deployment time:** Re-scan before deploying to production (new CVEs may have been discovered since build)
- **Runtime:** Continuous scanning while containers are running (threats emerge daily)

---

## Container Image Signing and Attestation

Image is tamperable: registry could be compromised, supply chain intercepted. Signing and verification ensure provenance.

### Container Signing (Sigstore Cosign)

**Cosign** (from Sigstore project): Sign images using private keys, verify using public keys.

```
cosign sign --key cosign.key gcr.io/myapp/image:v1
cosign verify --key cosign.pub gcr.io/myapp/image:v1
```

**Flow:**
1. Build image, compute digest (SHA256 of image content)
2. Sign digest with private key, store signature in image registry (as separate artifact linked to image)
3. Verifier: fetch image, compute digest, fetch signature, verify using public key

**Storage:** Signatures are stored in OCI registry (same registry as image). Single point of entry for supply chain.

**Keyless signing:** Newer approach uses **OpenID Connect (OIDC)** identity. Sign with OIDC token (temporary, issued by OIDC provider like GitHub or Google), verify token chain instead of static key. Enables temporary credentials without managing long-lived keys.

### Supply Chain Attestation (SLSA)

SLSA (Supply chain Levels for Software Artifacts) framework defines provenance evidence levels. Cosign can attach attestations:

```
cosign attest --predicate predicate.json gcr.io/myapp/image:v1
```

Predicate contains build metadata: who built it, build steps, dependencies, build environment. Verifier can inspect full build history.

---

## Runtime Security: Process Isolation

Containers share a kernel with the host and other containers. Isolation is enforced via **Linux namespaces** and **cgroups**, not hypervisors.

### Linux Namespaces

Kernel feature partitioning system resources; each namespace has isolated view.

**Types:**
- **PID namespace:** Process tree isolation. Each container has its own PID 1 (init), unaware of processes outside its namespace.
- **Network namespace:** Network interface isolation. Container has own network stack, IP, routing. Accessed via virtual ethernet.
- **Mount namespace:** Filesystem isolation. Container can't see mounts outside its namespace. Root filesystem is provided by image.
- **IPC namespace:** Inter-process communication isolation (shared memory, message queues).
- **UTS namespace:** Hostname and domainname isolation. Container sees its own hostname.
- **User namespace:** User ID mapping. Container runs as "root" (UID 0) but is mapped to unprivileged user on host (UID 1000000 or higher). Container cannot privileged escalate beyond its host mapping.

**Limitation:** Kernel is shared. If attacker finds kernel exploit (e.g., `dirty cow` CVE), escapes container namespace and gains host code execution.

### cgroups (Control Groups)

Resource limits: CPU, memory, I/O. Prevent one container from starving others.

```
docker run --cpus 1 --memory 512m
```

Limits that container to 1 CPU and 512 MB memory. Exceeding limits results in throttling or OOM kill.

### Rootless Containers

By default, container runs as root inside its namespace (UID 0 in container namespace). Rootless mode:

1. Run container runtime (Docker, Podman) as unprivileged user
2. Use user namespace mapping: container UID 0 maps to host UID 100000 (unprivileged)
3. Container thinks it's root, but host OS prevents privileged operations

**Benefit:** Even if container namespace is breached, attacker has unprivileged host identity (cannot `iptables`, `modprobe`, direct hardware access).

**Trade-off:** Some applications assume UID 0 has full privileges; rootless may break them. Requires newer kernel (5.10+).

---

## Seccomp Profiles

Secure Computing Mode: **system call (syscall) allowlist/blocklist.**

Default Docker seccomp profile blocks ~44 dangerous syscalls out of 300+:
- `ptrace` (process tracing, debugger escape)
- `reboot` (halt kernel)
- `swap_on` (I/O manipulation)
- `syslog` (kernel logging exposure)
- Module operations (`init_module`, `delete_module`)

Custom profiles can restrict further:

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
      "names": ["read", "write", "open", "close", "exit"],
      "action": "SCMP_ACT_ALLOW"
    }
  ]
}
```

This profile blocks all syscalls except `read`, `write`, `open`, `close`, `exit`. Application cannot fork, network operations, or file stat checks.

**Tuning:** Balance: too restrictive breaks app; too loose defeats purpose. Profiling tools (strace, seccomp-tracer) identify which syscalls an app uses.

**Limitations:** Profile cannot distinguish context (e.g., "allow write but only to file X"). Kernel refuses all syscalls not in allow list.

---

## AppArmor and SELinux

Mandatory access control (MAC) frameworks. More powerful than seccomp (context-aware), higher complexity.

### AppArmor

**Profile-based:** Define what a process can do in YAML-like syntax.

```
/usr/bin/curl {
  /etc/ssl/certs/* r,
  /usr/lib** r,
  /tmp/** rw,
  network inet stream,
  capability net_raw,
}
```

Curl can read /etc/ssl/certs/, /usr/lib, read/write /tmp, create network sockets, and use NET_RAW capability.

**Advantage:** Simple syntax, easier to audit than SELinux.

**Limitation:** Less granular than SELinux. Cannot distinguish "read /etc/passwd (denied)" vs. "read any file matching /etc/pass* (allowed)" easily.

### SELinux

**Label + policy based:** Every file, process, socket has security label (context). Policies define allowed transitions (process A can read file with label B).

```
type httpd_t;
type httpd_sys_rw_content_t;

allow httpd_t httpd_sys_rw_content_t:file { read write };
```

httpd process (label httpd_t) can read/write files labeled httpd_sys_rw_content_t.

**Advantage:** Fine-grained, comprehensive. Widely deployed (Red Hat, Fedora, CentOS).

**Disadvantage:** Cryptic syntax, steep learning curve. Policy debugging is complex (denials hidden in audit logs).

**Kubernetes:** Security Profiles Operator (SPO) simplifies AppArmor/SELinux deployment in K8s clusters.

---

## Linux Capabilities

Root privilege (UID 0) is monolithic: can do anything. Linux capabilities split this into granular permissions.

Standard capabilities:
- `CAP_NET_BIND_SERVICE`: Bind to ports < 1024
- `CAP_SYS_ADMIN`: Huge catch-all (mount, namespace operations, iptables)
- `CAP_NET_ADMIN`: Network configuration
- `CAP_CHOWN`: Change file ownership
- `CAP_DAC_OVERRIDE`: Bypass file permission checks

Container defaults: Most containers start with a set of useful capabilities (bind ports, network setup). Dangerous ones are dropped.

```
docker run --cap-drop=SYS_ADMIN --cap-add=NET_BIND_SERVICE
```

**Trade-off:** Application needs specific capability; if denied, it fails. Tuning is per-workload.

---

## Secrets Management in Containers

Containers need credentials (API keys, passwords, TLS certs). Secrets must not be baked into image.

### Anti-patterns

**Secret in Dockerfile:** `RUN echo "API_KEY=secret" >> .bashrc` → Secret is in image layers, accessible to anyone with image.

**Secret as environment variable:** `docker run -e API_KEY=secret` → Visible in process listing (`env` command), environment files.

### Better Approaches

**1. Secrets stored outside container:**
- Use secrets management service (Vault, AWS Secrets Manager, Google Secret Manager)
- Application retrieves at startup with temporary credentials (OIDC token, short-lived API key)
- Credential is not in image or container, only short-lived token needed to fetch it

**2. Kubernetes Secrets:**
```
kubectl create secret generic db-secret --from-literal=password=mypass
```

Mount secret as file: 
```yaml
volumes:
- name: db-secret
  secret:
    secretName: db-secret
containers:
- name: app
  volumeMounts:
  - name: db-secret
    mountPath: /run/secrets
```

Limitation: Secrets stored in etcd by default (unencrypted or encrypted with static key on master node). Not a recommended final solution without additional hardening (encryption at rest with external key management).

**3. Workload identity (OIDC + federation):**
- Container runs with OIDC token (signed by cluster, identity is workload name + namespace + pod ID)
- External service accepts OIDC token, exchanges for temporary credential (AWS SigV4 signature, GCP token)
- No credential stored; only temporary tokens exchanged

Example: Pod in Kubernetes with OIDC identity → exchange for AWS role session → access S3.

---

## Pod Security Standards (Kubernetes)

Kubernetes Pod Security Standards (PSS) define security profiles:

- **Restricted:** Strict security settings. No privileged containers, read-only root FS, dropped capabilities, seccomp profile required, runAsNonRoot.
- **Baseline:** Minimal restrictions, prevent obvious privilege-escalation.
- **Privileged:** No restrictions (legacy).

Namespace-level policy:

```yaml
apiVersion: v1
kind: Namespace
metadata:
  name: my-app
  labels:
    pod-security.kubernetes.io/enforce: restricted
```

Pods in restricted namespace must comply or are rejected at admission.

---

## Network Policies

Kubernetes NetworkPolicy restricts pod-to-pod communication (east-west traffic).

Default: All pods can communicate. NetworkPolicy changes to **deny-by-default, allow explicit whitelist.**

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: deny-frontend
spec:
  podSelector:
    matchLabels:
      role: frontend
  policyTypes:
  - Ingress
  ingress:
  - from:
    - podSelector:
        matchLabels:
          role: client
    ports:
    - protocol: TCP
      port: 8080
```

Frontend pods accept ingress only from pods labeled role=client on port 8080. All other traffic is denied.

**Limitation:** NetworkPolicy is enforced by CNI plugin (Calico, Cilium, etc.). Not all CNI plugins support it; must be explicitly enabled.

---

## Admission Controllers and Policy Enforcement

Kubernetes admission controllers intercept object creation/modification and enforce policies before persistence.

**Pod Security Policy (PSP):** Deprecated, replaced by Pod Security Standards.

**Kyverno:** Kubernetes-native policy engine. Can enforce seccomp, AppArmor, image signing verification, disallow privileged containers, etc.

```yaml
apiVersion: kyverno.io/v1
kind: ClusterPolicy
metadata:
  name: require-nonroot
spec:
  validationFailureAction: enforce
  rules:
  - name: check-runAsNonRoot
    match:
      resources:
        kinds:
        - Pod
    validate:
      message: "securityContext.runAsNonRoot must be true"
      pattern:
        spec:
          securityContext:
            runAsNonRoot: true
```

Any Pod missing `runAsNonRoot: true` is rejected.

---

## Container Escape and Breakout

Container security is layered. Breakthrough at any layer does not necessarily grant host compromise if other layers hold.

### Exploit Scenarios

1. **Exploit kernel vulnerability:** Attacker finds CVE affecting shared kernel → code execution on host with container's capability set (limited by capabilities, namespaces, seccomp)
2. **Escape from namespace:** If attacker's container runs with excessive capabilities (e.g., `CAP_SYS_ADMIN` + `CAP_NET_ADMIN`), they may break namespace boundaries and interact with host directly
3. **Privileged containers:** Containers with `--privileged` flag run with all capabilities and no isolation → effectively root on host

### Defense-in-Depth

Multiple layers must fail:
1. **Image from untrusted source:** Signed and scanned images prevent malware pre-planted in image
2. **Application vulnerability:** Seccomp profile restricts dangerous syscalls
3. **Privilege escalation:** Rootless containers, dropped capabilities prevent privilege escalation
4. **Kernel escape:** Recent kernel patches, restricted syscalls make exploitation harder (but not impossible)
5. **Runtime monitoring:** Detect anomalous behavior (process spawning other processes, unexpected network connections) and kill container

---

## Supply Chain: Build-to-Deploy

Secure container lifecycle:

1. **Source code:** Signed commits, code review before merge
2. **Build:** CI/CD pipeline builds image deterministically, runs tests, scans for vulnerabilities
3. **Image signing:** Image is signed by CI/CD with key managed by infrastructure (not developer personal key)
4. **Registry scanning:** Registry re-scans image after push (new CVEs may have appeared)
5. **Deployment policy:** Admission control verifies image signature, enforces PSS, denies unsigned images
6. **Runtime monitoring:** Detect anomalous behavior, correlate with threat intel

**Sigstore integration:** Entire chain can be signed and verified via transparent ledger (Rekor), enabling auditability.

---

## See Also

- [containers-orchestration.md](containers-orchestration.md) — Docker and Kubernetes fundamentals
- [cloud-aws-containers.md](cloud-aws-containers.md) — AWS container services (ECR, ECS, EKS)
- [security-supply-chain.md](security-supply-chain.md) — SBOM, SLSA, and supply chain integrity
- [security-best-practices.md](security-best-practices.md) — Defense-in-depth principles