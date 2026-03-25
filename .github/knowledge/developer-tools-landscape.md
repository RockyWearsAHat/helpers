# Developer Tools Landscape — Editors, Build Systems, and Everything Else

## Code Editors & IDEs

### The Big Three (2024-2026)
| Editor | Strengths | Ecosystem |
|--------|-----------|-----------|
| **VS Code** | Extensions, Copilot integration, remote dev, free | 50,000+ extensions, dominant market share |
| **JetBrains** (IntelliJ, PyCharm, etc.) | Deep language understanding, refactoring, built-in tools | Paid (Community editions free), best for Java/Kotlin |
| **Neovim** | Speed, modal editing, infinite customization, terminal-native | Lua config, LSP support, treesitter, lazy.nvim |

### Emerging / Specialized
| Editor | What It Is |
|--------|-----------|
| Zed | GPU-accelerated, written in Rust, collaborative, fast |
| Helix | Terminal editor, Kakoune-inspired selection-first, built-in LSP |
| Cursor | VS Code fork with deep AI integration |
| Windsurf | AI-native IDE (Codeium) |
| Sublime Text | Lightweight, fast, paid |
| Emacs | The eternal editor. Org mode. Magit. An operating system pretending to be an editor. |

### Language Server Protocol (LSP)
```
Editor  ←→  Language Server
               (runs separately)

One protocol, any editor + any language server = universal tooling
Invented by Microsoft for VS Code, now universal.
```
**What LSP provides:** Auto-complete, go-to-definition, find references, rename, diagnostics, code actions, hover info, formatting.

**Key language servers:**
- TypeScript: `typescript-language-server`
- Python: `Pylance` (VS Code) / `pyright` (any editor)
- Rust: `rust-analyzer`
- Go: `gopls`
- C/C++: `clangd`
- Java: `Eclipse JDT Language Server`

## Build Systems

### Language-Specific
| Language | Build Tool | Config File |
|----------|-----------|-------------|
| JavaScript/TS | npm/pnpm/yarn | package.json |
| Python | pip, poetry, uv, hatch | pyproject.toml |
| Rust | Cargo | Cargo.toml |
| Go | go build (built-in) | go.mod |
| Java | Gradle / Maven | build.gradle / pom.xml |
| C/C++ | CMake, Make, Meson, Ninja | CMakeLists.txt, Makefile |
| Swift | Swift Package Manager / Xcode | Package.swift |
| C# | dotnet / MSBuild | .csproj |

### Monorepo Build Tools
| Tool | Language Ecosystem | Key Feature |
|------|-------------------|-------------|
| Turborepo | JavaScript/TypeScript | Incremental builds, remote caching |
| Nx | JavaScript/TypeScript | Affected detection, computation caching |
| Bazel | Any (Google) | Hermeticity, reproducibility, massive scale |
| Pants | Python, Go, Java | Ergonomic Bazel alternative |
| Rush | JavaScript/TypeScript | Microsoft's monorepo manager |
| Lerna | JavaScript | Package publishing (now with Nx) |

### Task Runners
```bash
# Make (universal, been around since 1976)
make build       # Run the 'build' target
make test        # Run the 'test' target

# Just (modern Make alternative, simpler syntax)
just build
just test

# Task (Go-based, YAML config)
task build
```

## Package Managers

### JavaScript
```bash
npm install          # Node Package Manager (default)
pnpm install         # Performance npm (hard links, saves disk)
yarn install         # Facebook's alternative
bun install          # Zig-based runtime + package manager (fast)
```

### Python
```bash
pip install package          # Traditional (use with venv!)
poetry install               # Dependency resolution + lockfile
uv pip install package       # Rust-based, 10-100x faster than pip
pdm install                  # PEP 582, no virtualenv needed
```

### System-Level
```bash
brew install tool            # macOS (Homebrew)
apt install tool             # Debian/Ubuntu
dnf install tool             # Fedora/RHEL
pacman -S tool               # Arch Linux
nix-env -i tool              # NixOS/Nix (reproducible!)
```

## CI/CD Tools

### Platforms
| Platform | Hosted By | Key Feature |
|----------|----------|-------------|
| GitHub Actions | GitHub | Tight GitHub integration, marketplace |
| GitLab CI | GitLab | Built into GitLab, auto-DevOps |
| CircleCI | CircleCI | Fast, good caching, orbs |
| Jenkins | Self-hosted | Infinitely extensible, complex |
| Buildkite | Hybrid | Run agents on your infra, SaaS dashboard |
| Drone | Self-hosted | Container-native, simple YAML |
| Dagger | Any | CI pipelines as code (Go/Python/TS SDK) |

### GitHub Actions Example
```yaml
name: CI
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 20
      - run: npm ci
      - run: npm test
      - run: npm run lint
```

## Linters & Formatters

### The Key Distinction
- **Linter**: Finds bugs and enforces conventions (ESLint, Pylint, clippy)
- **Formatter**: Enforces consistent style automatically (Prettier, Black, rustfmt)
- **Best practice**: Use BOTH. Format on save, lint in CI.

### By Language
| Language | Linter | Formatter |
|----------|--------|-----------|
| JavaScript/TS | ESLint, Biome | Prettier, Biome |
| Python | Ruff, Pylint, Flake8 | Ruff, Black |
| Rust | clippy | rustfmt |
| Go | go vet, golangci-lint | gofmt (non-negotiable) |
| Shell | ShellCheck | shfmt |
| CSS | Stylelint | Prettier |
| Ruby | RuboCop | RuboCop |
| Java | SpotBugs, Error Prone | google-java-format |
| C/C++ | clang-tidy | clang-format |

### Ruff — The Python Game-Changer
```bash
# 10-100x faster than Flake8 + isort + pyupgrade + autoflake + ...
ruff check .            # Lint
ruff format .           # Format (Black-compatible)
ruff check --fix .      # Auto-fix issues
```
Written in Rust. Replaces 10+ Python tools. Near-instant for large codebases.

## Version Managers

### Language Runtimes
```bash
# Node.js
nvm install 20         # Node Version Manager
fnm install 20         # Fast Node Manager (Rust, faster)
volta install node@20  # Volta (deterministic, per-project)

# Python
pyenv install 3.12     # Python version manager
uv python install 3.12 # uv manages Python versions too

# Ruby
rbenv install 3.3.0    # rbenv
rvm install 3.3.0      # RVM

# Java
sdkman install java 21-tem  # SDKMAN (Java, Kotlin, Gradle, etc.)

# Universal
asdf install nodejs 20   # One tool for many languages
mise install node@20     # Rust-based asdf alternative (faster)
```

## Observability & Monitoring

### The Three Pillars
1. **Logs**: Structured events with context
2. **Metrics**: Numeric measurements over time (counters, gauges, histograms)
3. **Traces**: Request path through distributed systems (spans)

### Tools
| Category | Tools |
|----------|-------|
| Logs | ELK (Elasticsearch+Logstash+Kibana), Loki+Grafana, Datadog |
| Metrics | Prometheus+Grafana, Datadog, New Relic |
| Traces | Jaeger, Zipkin, Datadog APM, Honeycomb |
| All-in-one | Datadog, Grafana Cloud, New Relic, Splunk |
| Error tracking | Sentry, Bugsnag, Rollbar |
| Status pages | Statuspage, Betteruptime, Instatus |

### OpenTelemetry (OTel)
The vendor-neutral standard for instrumentation. One SDK → export to any backend.
```python
# Auto-instrument a Flask app
from opentelemetry.instrumentation.flask import FlaskInstrumentor
FlaskInstrumentor().instrument_app(app)
# Now every request generates traces automatically
```

## Security Scanning

| Tool | What It Scans |
|------|--------------|
| Dependabot / Renovate | Dependency vulnerabilities (auto-PR updates) |
| Snyk | Dependencies + code + containers + IaC |
| Trivy | Container images, filesystems, git repos |
| Semgrep | Custom static analysis rules (like grep but AST-aware) |
| CodeQL | Deep semantic analysis (GitHub's engine) |
| Gitleaks / truffleHog | Secrets in git history |
| OWASP ZAP | Dynamic application security testing (DAST) |
| Bandit | Python security linter |
| gosec | Go security linter |

## Documentation Tools

| Tool | Language/Framework | Output |
|------|-------------------|--------|
| JSDoc / TypeDoc | JavaScript/TypeScript | HTML from comments |
| Sphinx | Python (RST/Markdown) | HTML, PDF, ePub |
| rustdoc | Rust | HTML (built into cargo) |
| GoDoc / pkg.go.dev | Go | HTML from code |
| Javadoc | Java | HTML from comments |
| MkDocs (Material) | Any (Markdown) | Beautiful static sites |
| Docusaurus | Any (MDX/React) | Versioned docs sites |
| mdBook | Any (Markdown) | Rust book-style docs |

---

*The best tool is the one that disappears. If you're fighting your tools instead of solving problems, switch tools.*
