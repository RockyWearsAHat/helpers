# Docker

## Image Layers and Architecture

Docker images are stacks of read-only filesystem layers. Each Dockerfile instruction creates a layer. Layers are cached and shared between images via content-addressable storage (SHA256 digests).

```
┌────────────────────┐
│  Container layer   │  ← read-write (ephemeral)
├────────────────────┤
│  COPY app/ .       │  ← layer 4
├────────────────────┤
│  RUN npm ci        │  ← layer 3
├────────────────────┤
│  COPY package*.json│  ← layer 2
├────────────────────┤
│  FROM node:20-slim │  ← layer 1 (base image layers)
└────────────────────┘
```

**Layer caching rule**: if a layer's instruction AND all parent layers are unchanged, the cached layer is reused. Once a cache miss occurs, all subsequent layers rebuild. Order instructions from least-changing to most-changing.

## Dockerfile Instructions

```dockerfile
# Base image — pin to specific version for reproducibility
FROM node:20.11-slim AS builder

# Metadata
LABEL maintainer="team@example.com"
LABEL org.opencontainers.image.source="https://github.com/org/repo"

# Environment variables
ENV NODE_ENV=production
# ARG is build-time only (not in final image)
ARG APP_VERSION=unknown

# Working directory (created if doesn't exist)
WORKDIR /app

# Copy with cache optimization — deps first, code second
COPY package.json package-lock.json ./
RUN npm ci --omit=dev

COPY . .
RUN npm run build

# Expose documents the port (doesn't publish it)
EXPOSE 3000

# Non-root user (security)
RUN addgroup --system app && adduser --system --ingroup app app
USER app

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
  CMD curl -f http://localhost:3000/healthz || exit 1

# CMD vs ENTRYPOINT
ENTRYPOINT ["node"]          # fixed executable
CMD ["dist/server.js"]       # default arguments (overridable)
# docker run myapp dist/worker.js  → runs "node dist/worker.js"
```

### CMD vs ENTRYPOINT

|              | CMD                                                          | ENTRYPOINT                                    |
| ------------ | ------------------------------------------------------------ | --------------------------------------------- |
| **Override** | `docker run img <new-cmd>` replaces CMD                      | `docker run img <args>` appends to ENTRYPOINT |
| **Forms**    | `CMD ["node", "app.js"]` (exec) or `CMD node app.js` (shell) | Same two forms                                |
| **Combined** | When both set, CMD provides default args to ENTRYPOINT       |                                               |
| **Use**      | Default command users might override                         | Fixed executable (CLI tools, wrappers)        |

**Prefer exec form** `["cmd", "arg"]` — shell form wraps in `/bin/sh -c` which prevents signal propagation (SIGTERM won't reach your process).

### COPY vs ADD

`COPY` — straightforward file copy. Use this by default.
`ADD` — also handles URL downloads and auto-extracts `.tar.gz`. Avoid for clarity; use `COPY` + explicit `RUN curl/tar` instead.

## Multi-Stage Builds

```dockerfile
# Stage 1: Build
FROM golang:1.22 AS builder
WORKDIR /src
COPY go.mod go.sum ./
RUN go mod download
COPY . .
RUN CGO_ENABLED=0 GOOS=linux go build -ldflags="-s -w" -o /app

# Stage 2: Runtime (minimal image)
FROM gcr.io/distroless/static-debian12:nonroot
COPY --from=builder /app /app
ENTRYPOINT ["/app"]
```

**Benefits**: build tools not in final image, dramatically smaller images (Go binary: 800MB builder → 5MB distroless), separate dependency caching.

### Common Multi-Stage Patterns

```dockerfile
# Copy specific artifacts from named stages
COPY --from=builder /app/dist ./dist
COPY --from=builder /app/node_modules ./node_modules

# Copy from external image (no build stage needed)
COPY --from=nginx:1.25 /etc/nginx/nginx.conf /etc/nginx/

# Test stage (run tests, don't include in final image)
FROM builder AS test
RUN npm test

FROM node:20-slim AS production
COPY --from=builder /app/dist ./dist
```

## BuildKit Features

BuildKit is Docker's modern build engine (default since Docker 23.0). Enable explicitly with `DOCKER_BUILDKIT=1`.

### Cache Mounts

```dockerfile
# Cache package manager downloads between builds
RUN --mount=type=cache,target=/root/.npm \
    npm ci

# apt cache
RUN --mount=type=cache,target=/var/cache/apt \
    --mount=type=cache,target=/var/lib/apt/lists \
    apt-get update && apt-get install -y curl
```

### Bind Mounts

```dockerfile
# Mount source without COPY (for build-time-only access)
RUN --mount=type=bind,source=package.json,target=/app/package.json \
    jq '.version' /app/package.json
```

### Secret Mounts

```dockerfile
# Mount secrets without baking into image layers
RUN --mount=type=secret,id=npmrc,target=/root/.npmrc \
    npm ci
# Build: docker build --secret id=npmrc,src=.npmrc .
```

### Buildx (Multi-Platform)

```bash
# Build for multiple architectures
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  --tag myapp:latest \
  --push .

# Create builder instance
docker buildx create --name multiarch --use
```

## Docker Compose

```yaml
# compose.yaml (v2 format, no "version:" needed)
services:
  app:
    build:
      context: .
      dockerfile: Dockerfile
      target: production # multi-stage target
      args:
        APP_VERSION: "1.2.3"
    ports:
      - "3000:3000"
    environment:
      DATABASE_URL: postgresql://postgres:secret@db:5432/app
    depends_on:
      db:
        condition: service_healthy
    deploy:
      replicas: 2
      resources:
        limits:
          cpus: "0.5"
          memory: 512M
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3000/healthz"]
      interval: 10s
      timeout: 5s
      retries: 3
    restart: unless-stopped
    volumes:
      - app-data:/app/data
    networks:
      - backend

  db:
    image: postgres:16-alpine
    environment:
      POSTGRES_PASSWORD_FILE: /run/secrets/db_password
      POSTGRES_DB: app
    volumes:
      - pgdata:/var/lib/postgresql/data
      - ./init.sql:/docker-entrypoint-initdb.d/init.sql:ro
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U postgres"]
      interval: 5s
    networks:
      - backend
    secrets:
      - db_password

volumes:
  pgdata:
  app-data:

networks:
  backend:
    driver: bridge

secrets:
  db_password:
    file: ./secrets/db_password.txt
```

```bash
docker compose up -d              # start detached
docker compose up -d --build      # rebuild images
docker compose down               # stop and remove containers
docker compose down -v            # also remove volumes
docker compose logs -f app        # follow logs
docker compose exec app sh        # shell into running container
docker compose ps                 # show running services
docker compose config             # validate and show resolved config
```

## Networking

| Driver      | Use Case                                                                                 |
| ----------- | ---------------------------------------------------------------------------------------- |
| **bridge**  | Default. Isolated network for containers on same host. DNS resolution by container name. |
| **host**    | Container shares host network stack. No isolation. Best performance.                     |
| **overlay** | Multi-host networking (Swarm/K8s). Encrypted communication.                              |
| **none**    | No networking. Full isolation.                                                           |
| **macvlan** | Container gets own MAC/IP on physical network. Appears as physical device.               |

```bash
docker network create --driver bridge mynet
docker run --network mynet --name api myapp
docker run --network mynet --name db postgres
# api can reach db via hostname "db"
```

**DNS**: Compose automatically creates a network where service names resolve. Custom bridge networks enable DNS. Default bridge network does NOT have DNS — use `--link` (deprecated) or custom networks.

## Storage

### Volumes vs Bind Mounts

|                 | Volumes                           | Bind Mounts                       |
| --------------- | --------------------------------- | --------------------------------- |
| **Managed by**  | Docker                            | Host filesystem                   |
| **Location**    | `/var/lib/docker/volumes/`        | Anywhere on host                  |
| **Portability** | Yes                               | Tied to host path                 |
| **Performance** | Native (Linux), optimized (macOS) | Native (Linux), slow (macOS)      |
| **Use case**    | Persistent data (DBs, uploads)    | Development (source code, config) |

```bash
# Named volume
docker volume create pgdata
docker run -v pgdata:/var/lib/postgresql/data postgres

# Bind mount (development)
docker run -v $(pwd)/src:/app/src:ro myapp  # :ro = read-only

# tmpfs (in-memory, ephemeral)
docker run --tmpfs /tmp:rw,noexec,nosuid myapp
```

## Security

### Image Security

```dockerfile
# Non-root user (CRITICAL for production)
RUN addgroup --gid 1001 app && adduser --uid 1001 --gid 1001 --disabled-password app
USER 1001

# Read-only filesystem
# docker run --read-only --tmpfs /tmp myapp

# Drop capabilities
# docker run --cap-drop ALL --cap-add NET_BIND_SERVICE myapp

# No new privileges
# docker run --security-opt no-new-privileges myapp
```

### Rootless Docker

```bash
# Run Docker daemon as non-root (no --privileged needed)
dockerd-rootless-setuptool.sh install
# Uses user namespaces, slirp4netns for networking
```

### Image Scanning

```bash
# Trivy (comprehensive, fast)
trivy image myapp:latest
trivy image --severity HIGH,CRITICAL myapp:latest
trivy fs --security-checks vuln,secret,config .

# Grype (Anchore)
grype myapp:latest
grype dir:. --only-fixed    # only show vulns with available fixes

# Docker Scout (built-in)
docker scout cves myapp:latest
docker scout recommendations myapp:latest
```

### Distroless Images

No shell, no package manager, no OS utilities — just your app and runtime dependencies.

```dockerfile
# Google distroless
FROM gcr.io/distroless/static-debian12           # static binaries (Go, Rust)
FROM gcr.io/distroless/base-debian12             # dynamically linked (C/C++)
FROM gcr.io/distroless/java21-debian12           # Java
FROM gcr.io/distroless/nodejs20-debian12         # Node.js
FROM gcr.io/distroless/python3-debian12          # Python

# Chainguard images (alternative, frequently updated)
FROM cgr.dev/chainguard/node:latest
FROM cgr.dev/chainguard/python:latest
```

### Image Size Comparison

| Base Image                   | Size        |
| ---------------------------- | ----------- |
| `ubuntu:22.04`               | ~77MB       |
| `debian:12-slim`             | ~74MB       |
| `alpine:3.19`                | ~7MB        |
| `node:20`                    | ~1.1GB      |
| `node:20-slim`               | ~200MB      |
| `node:20-alpine`             | ~130MB      |
| `gcr.io/distroless/nodejs20` | ~130MB      |
| `scratch`                    | 0MB (empty) |

## .dockerignore

```
.git
.github
node_modules
dist
*.md
.env*
.vscode
docker-compose*.yml
Dockerfile*
```

Without `.dockerignore`, the entire build context is sent to the Docker daemon. A `.git` directory alone can be hundreds of MB.

## Performance and Layer Patterns

### Layer Optimization

```dockerfile
# Suboptimal — each RUN is a layer, cache breaks on any change
RUN apt-get update
RUN apt-get install -y curl
RUN apt-get install -y git

# Better — single layer, clean up in same layer
RUN apt-get update && \
    apt-get install -y --no-install-recommends curl git && \
    rm -rf /var/lib/apt/lists/*
```

### Build Argument Patterns

```dockerfile
# Base image version as ARG
ARG NODE_VERSION=20
FROM node:${NODE_VERSION}-slim

# Conditional logic
ARG INSTALL_DEV=false
RUN if [ "$INSTALL_DEV" = "true" ]; then npm install; else npm ci --omit=dev; fi
```

### Container Resource Limits

```bash
docker run \
  --memory=512m \
  --memory-swap=512m \          # same as memory = no swap
  --cpus=1.5 \
  --pids-limit=100 \            # fork bomb protection
  myapp
```

## Useful Commands

```bash
# Build
docker build -t myapp:1.0 .
docker build -t myapp:1.0 --no-cache .
docker build --target builder -t myapp:builder .   # specific stage

# Inspect
docker image inspect myapp:latest
docker history myapp:latest                         # layer sizes
docker system df                                    # disk usage

# Cleanup
docker system prune                # unused containers, networks, dangling images
docker system prune -a --volumes   # aggressive cleanup (CAUTION)
docker builder prune               # BuildKit cache

# Debug
docker run --rm -it myapp:latest /bin/sh            # override entrypoint
docker run --rm -it --entrypoint /bin/sh myapp      # if ENTRYPOINT set
docker cp container:/app/logs ./logs                # copy files out
docker stats                                        # live resource usage
```
