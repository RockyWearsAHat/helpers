# Dev Containers — Configuration, Lifecycle, and Development Workflows

## Overview

**Dev containers** are standardized, reproducible development environments defined in code. A `devcontainer.json` file specifies a Docker image, extensions, dependency installation, and initialization scripts. Developers commit this file to version control; teammates get identical environments in VS Code, GitHub Codespaces, or any Dev Containers–compatible runtime.

This decouples the development environment from the host machine. No more "works on my machine" — environment setup becomes auditable and versioned alongside source code.

## The devcontainer.json Configuration

The `devcontainer.json` file lives in `.devcontainer/` at the repository root. Core fields:

```json
{
  "name": "My Project",
  "image": "mcr.microsoft.com/devcontainers/python:3.11",
  "features": {
    "ghcr.io/devcontainers/features/git:latest": {},
    "ghcr.io/devcontainers/features/docker-in-docker:latest": {}
  },
  "customizations": {
    "vscode": {
      "extensions": ["ms-python.python", "ms-vscode.makefile-tools"],
      "settings": {
        "python.linting.enabled": true
      }
    }
  },
  "forwardPorts": [8000, 8080],
  "mounts": ["source=${localEnv:HOME}/.ssh,target=/root/.ssh,readonly"],
  "remoteUser": "vscode"
}
```

### Key Configuration Sections

**Image vs Dockerfile**: Specify a prebuilt image (recommended for speed) or build from a `Dockerfile` in `.devcontainer/`. Prebuilt images from `mcr.microsoft.com/devcontainers/` include common tools.

**Features**: Reusable modules that install and configure software (Git, Docker, Node.js, database CLIs). Features are composable — specify multiple without writing custom scripts. The `devcontainers/features` repository curates official features; community members publish alternatives to GitHub Container Registry (GHCR) or Docker Hub.

**customizations**: Configure IDE behavior. Under `vscode`, list extensions to auto-install and VS Code settings to apply. Other IDEs (JetBrains, Vim) can have their own `customizations` subsections.

**forwardPorts**: Map container ports to localhost. Enables accessing development servers and databases from the host.

**mounts**: Bind volumes. Common patterns: mount SSH keys read-only, mount Docker daemon sockets for DinD (Docker-in-Docker), mount host package caches to speed dependency installation.

**remoteUser**: Which user runs in the container. Default is `root`; `vscode` is a non-root user included in Microsoft's official images.

## Lifecycle Scripts

Scripts execute in sequence when the container starts. Order and timing matter:

1. **initializeCommand**: Runs on the host *before* the container starts. Useful for cloning submodules, pre-staging files, or validating prerequisites.

2. **onCreateCommand**: Runs once when the container is first created, before the user connects. Install global dependencies, build heavy artifacts. Single command; use `&&` to chain.

3. **postCreateCommand**: Runs after `onCreateCommand`, before user connection. Often used for final setup or pre-caching.

4. **postStartCommand**: Runs every time the container *restarts* (not just first-time creation). Lightweight startup tasks: start background services, initialize databases.

5. **postAttachCommand**: Runs after the user *connects* via IDE. Greet the user, display status, run quick validations.

Example progression:
```json
{
  "initializeCommand": "git submodule update --init",
  "onCreateCommand": "pip install -r requirements.txt && npm install",
  "postCreateCommand": "npm run build",
  "postStartCommand": "docker-compose up -d",
  "postAttachCommand": "npm test"
}
```

**Performance caveat**: `onCreateCommand` and `postCreateCommand` block container readiness. Keep them lean. Heavy builds belong in CI/CD or in a build layer of the Dockerfile.

## Features System

Features are packaged environments. Instead of a shell script, use the features registry:

```json
{
  "features": {
    "ghcr.io/devcontainers/features/python:1": {
      "version": "3.11"
    },
    "ghcr.io/devcontainers/features/node:1": {
      "version": "18"
    },
    "ghcr.io/devcontainers/features/rust:1": {}
  }
}
```

Features are layered: each feature installs on top of the base image. Order may matter for dependencies. Microsoft's feature repository is the canonical source; community features on GHCR often replicate or extend official ones.

## Docker-Based Development Workflow

### Local Development (VS Code Dev Containers)

1. Open the project folder in VS Code.
2. Click "Reopen in Container" (Command Palette: "Dev Containers: Reopen in Container").
3. VS Code builds the image (cached after first run) and connects.
4. The container appears transparent: file tree shows internal files, terminal runs inside the container.

The container persists between sessions. Modifications inside persist.

### GitHub Codespaces

Codespaces run the same `devcontainer.json` on GitHub's infrastructure. Developers start a Codespace from the repo, get the configured environment in their browser (VS Code Web or desktop client).

**Key differences from local Dev Containers**:
- Container runs on GitHub-managed hardware.
- Automatic resource limits (CPU, RAM, storage).
- Billing by compute hours (generous free tier for public repos, 60 hours/month for private).
- Environment variables and secrets sourced from GitHub secrets (not local `.env` files).

## Multi-Container Development (Docker Compose)

For projects with databases, APIs, and other services, use `docker-compose.yml`:

```json
{
  "dockerComposeFile": "docker-compose.yml",
  "service": "app",
  "workspaceFolder": "/workspace"
}
```

The `service` field specifies which container in the Compose file is the development container (where the IDE connects). Other services (PostgreSQL, Redis) start alongside.

**Startup order**: Compose does not guarantee service readiness, only container startup. Use `depends_on` with health checks:

```yaml
services:
  postgres:
    image: postgres:15
    healthcheck:
      test: ["CMD", "pg_isready"]
      interval: 10s
      timeout: 5s
      retries: 5
  app:
    depends_on:
      postgres:
        condition: service_healthy
```

In dev container lifecycle scripts, avoid assuming services are ready. Use `until` loops or client CLI polling to verify.

## Environment Variables and Secrets

### Environment Variables

Define in `devcontainer.json`:

```json
{
  "remoteEnv": {
    "NODE_ENV": "development",
    "DATABASE_URL": "postgres://user:password@postgres:5432/db"
  }
}
```

Reference host environment variables:
```json
{
  "remoteEnv": {
    "SSH_AUTH_SOCK": "${localEnv:SSH_AUTH_SOCK}"
  }
}
```

### Secrets in Codespaces

GitHub Codespaces injects repository secrets as environment variables at container runtime. Define in the repository settings under "Secrets and variables" → "Codespaces". They're available inside Codespaces automatically; local Dev Containers cannot access them.

Workaround for local dev: Create a `.env` file (gitignored), then:
```json
{
  "remoteEnv": {
    "DATABASE_PASSWORD": "${localEnv:DATABASE_PASSWORD}"
  }
}
```

## GPU Support

Dev containers can expose NVIDIA GPUs to the container using `--gpus` runtime flags. Support depends on the host and container orchestrator:

**Local Dev Containers**: Requires Docker Desktop GPU support (macOS, Windows) or NVIDIA Docker runtime (Linux). Pass GPU access via mounts or runtime configuration.

**Codespaces**: GPU instances available on request (enterprise feature). Contact GitHub support to enable GPU-equipped runners.

Most development workflows don't require GPU development containers. Reserve for ML/AI projects with on-device training.

## Performance Optimization

### Build Layer Caching

Docker caches layers based on `RUN` instruction content. Order expensive operations last:

```dockerfile
FROM python:3.11
RUN apt-get update && apt-get install -y build-essential  # Cached if unchanged
COPY . /app                                                # Invalidates if source changes
RUN pip install -e .                                      # Re-runs on any source change
```

Better approach: Cache dependency installation:
```dockerfile
FROM python:3.11
RUN apt-get update && apt-get install -y build-essential
COPY requirements.txt /tmp/
RUN pip install -r /tmp/requirements.txt  # Cached unless requirements.txt changes
COPY . /app
RUN pip install -e .
```

### Volume Mount Performance

Bind-mounting source code from the host can be slow on macOS and Windows (Docker Desktop limitation). Options:

1. **Accept the latency** for small projects.
2. **Use named volumes** instead of bind mounts (slower than native but faster than bind mount passthrough).
3. **Copy source into the container** during build and use source control workflows (push/pull from host) — eliminates live-sync but is faster for large codebases.

### Container Startup Speed

- Use prebuilt images instead of building Dockerfile from scratch.
- Keep `onCreateCommand` and `postCreateCommand` minimal.
- Cache dependency resolutions: Docker BuildKit supports cache mounts for package managers.

## Security Considerations

### Running as Non-Root

The `remoteUser` field allows running as a non-root user (e.g., `vscode`). Recommended for production-like environments. Trade-off: some tools (systemd, network config) require root.

```json
{
  "remoteUser": "vscode"
}
```

### SSH Key Management

Mount SSH keys read-only:
```json
{
  "mounts": ["source=${localEnv:HOME}/.ssh,target=/root/.ssh,readonly"]
}
```

Alternatively, forward the SSH agent from the host:
```json
{
  "mounts": ["source=${localEnv:SSH_AUTH_SOCK},target=/ssh-agent,readonly"],
  "remoteEnv": {
    "SSH_AUTH_SOCK": "/ssh-agent"
  }
}
```

### Dockerfile Security

Scan images for vulnerabilities. Use official base images (e.g., `mcr.microsoft.com`) — they're regularly patched.

Avoid storing secrets in image layers (they're leaked in history). Use build secrets:
```dockerfile
RUN --mount=type=secret,id=npmtoken npm ci --registry https://npm.example.com
```

## Common Patterns

### Monorepo with Shared Dev Container

Multiple services in one repo can share a dev container config, or each can have its own under service-specific directories:

```
.devcontainer/
  devcontainer.json         # Shared
services/
  api/.devcontainer/        # Service-specific overrides
  web/.devcontainer/
```

### Development vs Production Parity

Use the same base image in dev containers and production deployments. Tag it consistently:

```json
{
  "image": "myregistry/myapp:20260325-alpine"
}
```

This ensures developers and CI/CD use identical environments.

### Quick Environment Switching

Use variables in the devcontainer.json:

```json
{
  "image": "mcr.microsoft.com/devcontainers/python:${VERSION:-3.11}"
}
```

In Codespaces, configure vars under "Codespaces variables" or pass them during creation.

## Relationship to Other Tools

**Dev Containers vs Nix**: Nix also provides reproducible environments but uses a functional package manager. Dev Containers are more accessible to teams not already using Nix; both can coexist.

**Dev Containers vs Docker Compose**: Compose orchestrates multiple services; Dev Containers is an IDE integration layer. Dev Containers often *uses* Compose (`dockerComposeFile` field).

**Dev Containers vs Vagrant**: Vagrant predates containers and uses VMs instead of Docker. Dev Containers are lighter-weight; Vagrant sees less adoption in new projects.

## See Also

- [containers.dev](https://containers.dev) — official specification
- [devcontainers/features](https://github.com/devcontainers/features) — curated feature registry
- [VS Code Dev Containers extension](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers)
- [GitHub Codespaces documentation](https://docs.github.com/en/codespaces)
- Docker Compose for orchestration patterns