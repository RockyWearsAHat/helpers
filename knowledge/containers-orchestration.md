# Containers & Orchestration — Docker, Kubernetes, and Beyond

## Docker Fundamentals

### Image vs Container

- **Image**: Read-only template. A filesystem snapshot + metadata. Built from a Dockerfile.
- **Container**: Running instance of an image. Has its own writable layer, network, and process space.
- **Registry**: Image storage (Docker Hub, GitHub Container Registry, ECR, GCR).

### Dockerfile Patterns

```dockerfile
# Use specific tags, not :latest (reproducibility)
FROM node:20-alpine AS builder

# Set working directory
WORKDIR /app

# Copy dependency files first (better layer caching)
COPY package.json package-lock.json ./
RUN npm ci --production

# Copy source code (changes more often — separate layer)
COPY src/ ./src/

# Multi-stage build: smaller final image
FROM node:20-alpine
WORKDIR /app
COPY --from=builder /app ./

# Non-root user (security)
RUN addgroup -g 1001 appgroup && adduser -u 1001 -G appgroup -D appuser
USER appuser

# Use exec form (PID 1 gets signals properly)
CMD ["node", "src/index.js"]
```

### Layer Caching Rules

1. Each Dockerfile instruction creates a layer
2. If a layer's instruction hasn't changed AND all previous layers are cached → cached
3. Once a cache miss occurs, ALL subsequent layers are rebuilt
4. **Order matters:** Put rarely-changing instructions first (dependencies before source code)

### Docker Compose

```yaml
# docker-compose.yml
services:
  app:
    build: .
    ports:
      - "3000:3000"
    environment:
      - DATABASE_URL=postgres://db:5432/myapp
    depends_on:
      db:
        condition: service_healthy
    volumes:
      - ./src:/app/src # Dev: mount source for hot reload

  db:
    image: postgres:16-alpine
    environment:
      POSTGRES_DB: myapp
      POSTGRES_PASSWORD_FILE: /run/secrets/db_password
    volumes:
      - pgdata:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U postgres"]
      interval: 5s
      timeout: 5s
      retries: 5

volumes:
  pgdata:
```

### Essential Docker Commands

```bash
# Build
docker build -t myapp:1.0 .
docker build --no-cache -t myapp:1.0 .    # Force rebuild

# Run
docker run -d -p 3000:3000 --name myapp myapp:1.0
docker run -it --rm ubuntu bash            # Interactive, remove on exit
docker run -v $(pwd):/app myapp:1.0        # Mount volume

# Inspect
docker ps                                  # Running containers
docker ps -a                               # All containers
docker logs -f myapp                       # Follow logs
docker exec -it myapp sh                   # Shell into container
docker inspect myapp                       # Full metadata
docker stats                               # Resource usage

# Cleanup
docker system prune -a                     # Remove everything unused
docker image prune                         # Remove dangling images
docker volume prune                        # Remove unused volumes
```

### Docker Networking

```
bridge (default)  Containers on same bridge can communicate by name
host              Container shares host network (no port mapping needed)
none              No networking
overlay           Multi-host networking (Swarm/Kubernetes)
```

```bash
# Create a network
docker network create mynet
docker run --network mynet --name api myapi
docker run --network mynet --name db postgres
# api can reach db at hostname "db"
```

## Kubernetes (k8s) Core Concepts

### Architecture

```
Control Plane:
  ├── API Server (kubectl talks to this)
  ├── etcd (distributed key-value store — cluster state)
  ├── Scheduler (assigns Pods to Nodes)
  └── Controller Manager (reconciliation loops)

Worker Nodes:
  ├── kubelet (manages Pods on this node)
  ├── kube-proxy (network rules, service routing)
  └── Container Runtime (containerd, CRI-O)
```

### Key Resources

#### Pod (smallest deployable unit)

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: myapp
spec:
  containers:
    - name: app
      image: myapp:1.0
      ports:
        - containerPort: 3000
      resources:
        requests:
          memory: "128Mi"
          cpu: "250m"
        limits:
          memory: "256Mi"
          cpu: "500m"
      livenessProbe:
        httpGet:
          path: /health
          port: 3000
        initialDelaySeconds: 10
        periodSeconds: 5
      readinessProbe:
        httpGet:
          path: /ready
          port: 3000
```

#### Deployment (manages Pod replicas with rolling updates)

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: myapp
spec:
  replicas: 3
  selector:
    matchLabels:
      app: myapp
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxSurge: 1
      maxUnavailable: 0
  template:
    metadata:
      labels:
        app: myapp
    spec:
      containers:
        - name: app
          image: myapp:1.0
```

#### Service (stable network endpoint for Pods)

```yaml
apiVersion: v1
kind: Service
metadata:
  name: myapp
spec:
  selector:
    app: myapp
  ports:
    - port: 80
      targetPort: 3000
  type: ClusterIP # Internal only (default)
  # type: LoadBalancer  # External LB (cloud provider)
  # type: NodePort      # Expose on every node's IP
```

#### Ingress (HTTP routing)

```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: myapp
spec:
  rules:
    - host: myapp.example.com
      http:
        paths:
          - path: /
            pathType: Prefix
            backend:
              service:
                name: myapp
                port:
                  number: 80
```

### ConfigMaps and Secrets

```yaml
# ConfigMap: non-sensitive config
apiVersion: v1
kind: ConfigMap
metadata:
  name: app-config
data:
  LOG_LEVEL: "debug"
  API_URL: "https://api.example.com"

# Secret: sensitive data (base64 encoded, NOT encrypted at rest by default!)
apiVersion: v1
kind: Secret
metadata:
  name: app-secrets
type: Opaque
data:
  DB_PASSWORD: cGFzc3dvcmQxMjM=  # base64 of "password123"
```

### Essential kubectl Commands

```bash
# Context
kubectl config get-contexts
kubectl config use-context my-cluster

# View resources
kubectl get pods -o wide
kubectl get deployments
kubectl get services
kubectl get all -n my-namespace
kubectl describe pod myapp-abc123

# Logs & debugging
kubectl logs myapp-abc123 -f          # Follow logs
kubectl logs myapp-abc123 -c sidecar  # Specific container
kubectl exec -it myapp-abc123 -- sh   # Shell into pod
kubectl port-forward svc/myapp 3000:80  # Local port forward

# Apply & manage
kubectl apply -f deployment.yaml
kubectl delete -f deployment.yaml
kubectl rollout status deployment/myapp
kubectl rollout undo deployment/myapp   # Rollback
kubectl scale deployment myapp --replicas=5

# Debug
kubectl get events --sort-by=.metadata.creationTimestamp
kubectl top pods                        # Resource usage
```

## Container Security Checklist

1. **Non-root user**: `USER 1001` in Dockerfile. Running as root expands the attack surface significantly.
2. **Read-only filesystem**: `readOnlyRootFilesystem: true` in k8s security context
3. **No privileged mode**: `privileged: false`
4. **Drop capabilities**: `drop: ["ALL"]`, add only what's needed
5. **Scan images**: Trivy, Grype, Snyk for vulnerability scanning
6. **Minimal base images**: `alpine`, `distroless`, or `scratch`
7. **No secrets in images**: Use secrets management, not ENV in Dockerfile
8. **Network policies**: Restrict pod-to-pod communication
9. **Resource limits**: Prevent noisy neighbors and container escape via resource exhaustion
10. **Image signing**: Cosign/Notary for supply chain security

## Volumes & Storage

### Docker Volumes

```bash
# Named volume (Docker manages location)
docker volume create mydata
docker run -v mydata:/app/data myapp

# Bind mount (host directory)
docker run -v /host/path:/container/path myapp

# tmpfs (in-memory, not persisted)
docker run --tmpfs /app/tmp myapp
```

### Kubernetes Persistent Volumes

```yaml
# PersistentVolumeClaim
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: data-pvc
spec:
  accessModes: ["ReadWriteOnce"]
  resources:
    requests:
      storage: 10Gi
  storageClassName: standard
```

## Local Development Tools

| Tool            | Purpose                                            |
| --------------- | -------------------------------------------------- |
| Docker Desktop  | Docker + k8s on Mac/Windows                        |
| Rancher Desktop | Open-source alternative to Docker Desktop          |
| minikube        | Local k8s cluster (VM-based)                       |
| kind            | k8s in Docker containers (fast)                    |
| k3d             | k3s (lightweight k8s) in Docker                    |
| Tilt            | Dev workflow: auto-rebuild & deploy on file change |
| Skaffold        | Google's dev workflow tool for k8s                 |
| Lens            | Kubernetes IDE (GUI)                               |

---

_Containers solve "works on my machine." Kubernetes solves "works on my cluster." Neither solves "works in my head."_
