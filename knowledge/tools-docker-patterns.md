# Docker Patterns

## Multi-Stage Builds

Multi-stage builds reduce final image size by separating build dependencies from runtime. Each `FROM` instruction creates a new build stage.

```dockerfile
FROM node:20-bookworm AS builder
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .
RUN npm run build

FROM node:20-slim
WORKDIR /app
COPY --from=builder /app/dist ./dist
COPY --from=builder /app/node_modules ./node_modules
COPY package*.json ./
EXPOSE 3000
CMD ["node", "dist/server.js"]
```

**Benefits**: Final image contains only runtime dependencies (node_modules + dist), not build tools (TypeScript compiler, dev dependencies, source). Can reduce 800MB → 200MB.

**Patterns**:
- **Builder stage**: Compile, transpile, generate assets
- **Test stage**: Run tests before packaging (fail fast)
- **Artifacts stage**: Copy only built artifacts to runtime stage
- **Multiple runtimes**: Java builder → distroless runtime (300MB → 50MB)

**Layer order in build stage**: Dependencies first (least-changing), source code last (most-changing) for cache hits.

## BuildKit Features

Docker BuildKit is the modern builder backend. Enable with `export DOCKER_BUILDKIT=1`.

**Key features**:
- **Parallel stage execution**: Stages with no dependencies build concurrently
- **Better caching**: Content-addressed layers; unchanged layers never rebuild
- **Inline cache**: `--cache-to=type=inline --cache-from=...` for CI/CD cache reuse across machines
- **External cache sources**: Reference `docker.io/org/repo:cache` as build cache
- **Secrets handling**: `--secret id=github_token` injects secrets without leaving them in layers
  ```dockerfile
  RUN --mount=type=secret,id=github_token \
    npm install --registry https://npm.pkg.github.com \
    --token="$(cat /run/secrets/github_token)"
  ```
- **SSH socket mounting**: `--mount=type=ssh` for SSH agent access during build
- **Cache mount**: `--mount=type=cache,target=/go/pkg/mod` persists cache across builds (Go dependencies)

**CI/CD integration**:
```bash
# GitHub Actions example
docker buildx build \
  --cache-from=type=reg,ref="$REGISTRY/cache:latest" \
  --cache-to=type=reg,ref="$REGISTRY/cache:latest",mode=max \
  --push \
  -t "$REGISTRY/app:latest" .
```

Buildx also enables **multi-platform builds**: `docker buildx build --platform linux/amd64,linux/arm64 -t app:latest .`

## Layer Caching Optimization

Layer cache invalidation is the root cause of slow builds. Understand the cache key:

**Cache key = Dockerfile instruction + cumulative hash of all parent layers**

Once a layer cache misses, all subsequent layers rebuild (even if unchanged).

**Optimization strategy**:
1. Base image (least-changing)
2. System dependencies (`apt-get update/install`)
3. Package manager setup (metadata, credentials, SSH keys)
4. Dependency files (`package.json`, `go.mod`, `requirements.txt`)
5. Dependency install (`npm ci`, `go mod download`, `pip install`)
6. Source code (COPY . .)
7. Build commands (`npm run build`)

**Anti-patterns**:
- `COPY . .` early: Any source change invalidates all downstream cache
- `RUN apt-get update && apt-get install` on separate lines: Updates line caches, installs use stale package lists
- Combining unrelated steps: "RUN npm ci && npm run build && npm run test" — one failure invalidates entire layer

**Cache busting techniques**:
- ARG for version pins: `ARG BUILDKIT_INLINE_CACHE=1` (consumed by cache key)
- .dockerignore: Exclude files that trigger spurious rebuilds
- Conditional build steps: `RUN if [ -f src/main.rs ]; then cargo build; fi`

## .dockerignore

Functions like .gitignore but for Docker builds. Reduces build context sent to daemon.

```dockerfile
# .dockerignore
.git
.gitignore
node_modules
npm-debug.log
.env
.env.local
.nyc_output
coverage
.idea
.vscode
*.log
```

**Impact**: On large repos, excluding node_modules can reduce build context from 500MB to 50MB. Speeds up both local builds and remote CI/CD.

## Docker Compose Patterns

### Networking

Compose creates a private bridge network (default name: `projectname_default`). Services communicate via service name as hostname.

```yaml
version: '3.9'
services:
  web:
    image: app:latest
    ports:
      - "8080:3000"
    depends_on:
      db:
        condition: service_healthy
    environment:
      DATABASE_URL: "postgresql://db:5432/app"
  
  db:
    image: postgres:16-alpine
    environment:
      POSTGRES_PASSWORD: secret
      POSTGRES_DB: app
    ports:
      - "5432:5432"  # expose for local psql access (optional)
    volumes:
      - db_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U postgres"]
      interval: 10s
      timeout: 5s
      retries: 5

volumes:
  db_data:
```

`depends_on` with `condition: service_healthy` waits for the health check to pass before starting dependent service.

### Secrets Handling

Compose doesn't support runtime secret injection (unlike Swarm/Kubernetes). Alternatives:

1. **Environment variables** (least secure): Visible in process list, logs
   ```yaml
   environment:
     API_KEY: ${API_KEY}  # from .env file
   ```
2. **Secrets file with proper permissions**:
   ```yaml
   services:
     app:
       environment:
         DATABASE_PASSWORD_FILE: /run/secrets/db_password
       secrets:
         - db_password
   secrets:
     db_password:
       file: ./secrets/db_password  # chmod 600
   ```
3. **Mount volume with restricted permissions**:
   ```yaml
   services:
     app:
       volumes:
         - type: bind
           source: /path/to/secrets
           target: /app/secrets
           read_only: true
   ```

Production: Use Docker Swarm secrets or Kubernetes + sealed-secrets, not Compose.

### Volume Patterns

```yaml
volumes:
  # Named volume (managed by Docker)
  db_data:
    driver: local
  
  # Bind mount (explicit path on host)
  # source: /mnt/data
  # target: /app/data

services:
  app:
    volumes:
      # Named volume
      - db_data:/var/lib/postgresql/data
      
      # Bind mount (for development)
      - ./src:/app/src:ro  # read-only
      
      # tmpfs (in-memory, ephemeral)
      - /app/cache
```

Development: Bind mount source code, mount volumes for data persistence.
Production: Use named volumes for stateful services; ephemeral containers for stateless.

## Health Checks

Health checks signal container state to orchestrators (Docker, Swarm, Kubernetes). Without them, Compose waits only for process start, not service readiness.

```dockerfile
HEALTHCHECK \
  --interval=30s \
  --timeout=10s \
  --start-period=40s \
  --retries=3 \
  CMD curl -f http://localhost:3000/health || exit 1
```

Parameters:
- `interval`: Check frequency (default 30s)
- `timeout`: How long to wait for check result (default 30s)
- `start-period`: Grace period before first check (app startup time) (default 0)
- `retries`: Consecutive failures to mark UNHEALTHY (default 3)

Exit codes: 0 = healthy, 1 = unhealthy, 2 = reserved.

**Best practices**:
- Implement /health endpoint (lightweight, read-only, no external dependencies)
- Set start-period to account for app boot time (prevents flapping)
- Use curl, wget, or custom script (not just exit code 0)
- Keep check fast (< timeout value)

## Rootless Containers

Rootless mode runs the Docker daemon as unprivileged user, reducing attack surface. Containers still run as root within their namespace (isolated from host).

**Setup** (Linux):
```bash
dockerd-rootless-setuptool.sh install
export DOCKER_HOST=unix://$XDG_RUNTIME_DIR/docker.sock
docker run --rm alpine id  # uid2 0 (root in container namespace) but unprivileged on host
```

**Tradeoffs**:
- **Pros**: Host compromise doesn't grant container-level privileges; reduced privilege escalation attack surface
- **Cons**: Performance overhead (~10-15%); port binding requires user namespace remapping; some volume mounts restricted

**Limitations**:
- Cannot expose ports < 1024 without privileged helper
- Systemd integration differs
- Some volume mount options unavailable

Production use: Rootless for infrastructure services; standard for user-facing workloads if performance is critical.

## Secrets Management

**Never**:
- Embed secrets in image (docker history reveals them)
- Pass secrets as environment variables in Dockerfile (`RUN export SECRET=...`)
- Log secret values

**Approaches**:

1. **BuildKit secrets** (build-time only):
   ```bash
   docker buildx build \
     --secret id=github_token,env=GITHUB_TOKEN \
     -t app:latest .
   ```
   ```dockerfile
   RUN --mount=type=secret,id=github_token npm install
   ```

2. **Runtime injection** (Docker):
   ```bash
   docker run \
     -e DATABASE_PASSWORD="$(cat /run/secrets/db_pass)" \
     app:latest
   ```
   Or use password manager integration (HashiCorp Vault, etc.)

3. **Volume mount** (for config files, certs):
   ```bash
   docker run -v ~/.ssh/id_rsa:/run/secrets/ssh_key:ro app:latest
   ```

4. **Docker Swarm secrets** (encrypted at rest):
   ```bash
   echo "secret_value" | docker secret create my_secret -
   docker service create \
     --secret my_secret \
     -e SECRET_FILE=/run/secrets/my_secret \
     app:latest
   ```

## Development Containers (devcontainer)

`.devcontainer/devcontainer.json` standardizes dev environment using Docker.

```json
{
  "name": "Node App",
  "image": "mcr.microsoft.com/devcontainers/typescript-node:18",
  "features": {
    "ghcr.io/devcontainers/features/docker-outside-of-docker:1": {}
  },
  "customizations": {
    "vscode": {
      "extensions": [
        "dbaeumer.vscode-eslint",
        "esbenp.prettier-vscode"
      ],
      "settings": {
        "eslint.validate": ["javascript", "typescript"]
      }
    }
  },
  "forwardPorts": [3000],
  "postCreateCommand": "npm install",
  "remoteUser": "node"
}
```

**Benefits**:
- Consistent environment across team (no "works on my machine")
- CI/CD parity (test environment matches development)
- Easy onboarding (clone + open in container)
- Zero local toolchain setup needed

**Ecosystem**: GitHub Codespaces, VS Code Dev Containers, JetBrains Remote Development all support devcontainer spec.

## Multi-Platform Builds

Build images for multiple architectures (amd64, arm64, armv7, etc.) with one command.

```bash
docker buildx build \
  --platform linux/amd64,linux/arm64,linux/arm/v7 \
  --tag myapp:latest \
  --push .
```

**Implementation**:
- Buildx uses QEMU emulation by default (slow, for testing)
- Production: Use native builders (GitHub Actions, GitLab CI provide native runners for multiple platforms)
- Manifest lists: Docker automatically creates multi-arch manifest pointing to platform-specific images

**Tradeoffs**:
- Build time: Separate image per platform (2-3x slower than single platform)
- Storage: All platform images pushed to registry
- Benefit: Single tag, pull gets correct platform automatically

**Use cases**: Distribute to users on different architectures (M1 Macs, ARM servers, etc.)

## See Also

- containers-orchestration, devops-docker, infrastructure-container-networking, security-container