# Python Packaging Ecosystem: Tools, Virtual Environments, and Modern Workflows

## Overview

Python's packaging landscape is fragmented. Where npm has one dominant tool, Python has evolved through setuptools → pip → Poetry → Hatch → uv, each solving different problems. Understanding the distinction between environment management (venv, conda, pyenv), package management (pip, Poetry), build backends (setuptools, hatchling, pdm-backend), and project managers (Hatch, PDM) is essential for modern Python development.

## Virtual Environments: Isolation Mechanisms

### venv (Standard Library, Python 3.3+)

The built-in, lightweight virtual environment tool. Creates an isolated Python environment per project.

```bash
python3 -m venv myproject-env
source myproject-env/bin/activate  # macOS/Linux
# myproject-env\Scripts\activate   # Windows
```

**Advantages**:
- Zero dependencies (built into Python)
- Lightweight (fast to create)
- Part of Python standard library (stable API)

**Disadvantages**:
- Can't switch Python versions mid-project
- Requires manual activation
- Platform-specific activation syntax

### virtualenv (Third-party, 1996–present)

Pre-dates venv and remains actively maintained. Offers features venv lacks:

```bash
virtualenv myenv
virtualenv myenv --python=3.10  # Switch Python version
```

**Advantages**:
- Supports multiple Python versions
- Faster creation than venv
- Cross-platform activation
- Active development

**Disadvantage**:
- Extra dependency (though well-maintained)

### conda (Conda Inc.)

Multi-language environment manager. Not just Python—handles system-level dependencies, C libraries, and non-Python tools.

```bash
conda create -n myenv python=3.11 numpy scipy
conda activate myenv
```

**Advantages**:
- Solves system dependencies (anaconda/miniconda ecosystems)
- Pre-built binaries for scientific packages (NumPy, SciPy)
- Compatible with channels (conda-forge for non-official packages)
- Handles R, SQL, C++ alongside Python

**Disadvantages**:
- Much larger download footprint (1-2 GB)
- Slower environment creation
- Separate package ecosystem (PyPI vs. conda-forge)
- Overkill for pure Python projects

### pyenv (Python Version Manager)

Installs and manages multiple Python versions, not virtual environments.

```bash
pyenv install 3.10.0 3.11.5 3.12.0
pyenv versions
pyenv global 3.11.5           # System-wide default
pyenv local 3.10.0            # Project-specific
```

**Use case**: Work on multiple projects requiring different Python versions.

**Common pairing**: `pyenv` + `venv`:
```bash
pyenv local 3.10.5  # Project uses Python 3.10.5
python -m venv env  # Create venv from that Python
```

## Package Managers: The Evolution

### pip (pip Installs Packages, ~2014–present)

The default package installer. Reads `requirements.txt` or `setup.py` and installs from PyPI.

**Basic usage**:
```bash
pip install requests>=2.28
pip install -r requirements.txt
pip freeze > requirements.txt  # Export current deps
```

**Limitations**:
- No dependency lock file (until pip 21.1's in-development support)
- Can't manage dev vs. production deps cleanly
- Slow resolver (fixed in pip 20.3)
- No project scaffolding

### Pipenv (Kenneth Reitz, ~2017)

Attempted to be Python's Bundler (Ruby). Combines pip + virtualenv + lock files.

```bash
pipenv install requests
pipenv install --dev pytest     # Dev dependency
pipenv graph                    # Show dependency tree
```

**Features**:
- Automatic virtual env creation
- Pipfile + Pipfile.lock (semantic dependency declaration)
- Dev dependency distinction

**Why it fell out of favor**:
- Slower than alternatives (deep resolution)
- Abandoned maintenance intermittently
- Overcomplicated for simple use cases

### Poetry (Sébastien Eustace, ~2018–present)

Modern, opinionated Python project manager. Uses `pyproject.toml` for everything.

```toml
[tool.poetry]
name = "mylib"
version = "0.1.0"
description = "My library"
authors = ["Author <email@example.com>"]

[tool.poetry.dependencies]
python = "^3.10"
requests = "^2.28"

[tool.poetry.group.dev.dependencies]
pytest = "^7.0"
black = "^23.0"

[tool.poetry.scripts]
mycli = "mylib.cli:main"  # Executable entry point
```

**Poetry's strengths**:
- Declarative, centralized `pyproject.toml`
- Dependency resolution via backtracking (similar to Pip 20.3+)
- `poetry.lock` for reproducible installs
- Built-in packaging and publishing (poetry publish to PyPI)
- Clear separation: dev vs. runtime deps

**Criticisms**:
- **Slow dependency resolution** (backtracking solver can timeout on complex graphs)
- Heavy (larger binary than alternatives)
- Lock file can drift if manually edited
- Non-standard (not part of Python packaging spec PEPs)

### PDM (Python Development Master, ~2021–present)

Fast, modern alternative designed around PEP 517/518/621 standards.

```toml
[project]
name = "mylib"
version = "0.1.0"
dependencies = ["requests>=2.28"]

[project.optional-dependencies]
dev = ["pytest"]

[build-system]
requires = ["pdm-backend"]
build-backend = "pdm.backend"
```

**PDM advantages**:
- Strict PEP compliance (uses standard -backend system)
- Fast C-based resolver
- Minimal disk footprint
- Central lock file cache (shared across projects)
- Separated concerns: package metadata vs. tool config

**Adoption**: Growing but niche compared to Poetry.

### Hatch (PyPA, ~2021–present)

Official Python Packaging Authority's modern project manager. Focused on simplicity and standards compliance.

```toml
[project]
name = "mylib"
version = "0.1.0"
dependencies = ["requests>=2.28"]

[tool.hatch.envs.dev]
dependencies = ["pytest"]
scripts = {"test" = "pytest"}

[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"
```

**Hatch strengths**:
- PyPA-backed (de facto standard)
- Minimal, focused tool (not monolithic)
- Fast build backend (hatchling)
- Explicit environment management (unlike Poetry's auto-magic)

**Trade-off**: Requires more manual configuration than Poetry.

### uv (Astral, ~2024–present)

Rust-based package manager from the creators of Ruff. Aims for extreme speed.

```bash
uv pip install requests
uv sync            # Uses pyproject.toml + uv.lock
uv run python script.py
uv venv myenv      # Virtual environment management
```

**uv's innovation**:
- **10-100x faster** than pip (Rust implementation, parallel downloads)
- Drop-in pip replacement (`uv pip install`)
- Unified CLI: package management, venv creation, script running
- Supports `pyproject.toml` and requirements files
- Emerging as the speed standard

**Still evolving**: Not yet feature-complete for all use cases (e.g., editable installs).

## pyproject.toml: The Standard

PEP 621 unified Python packaging metadata into `pyproject.toml`, replacing scattered `setup.py`, `setup.cfg`, and `MANIFEST.in`.

### Modern Structure

```toml
[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"

[project]
name = "mylib"
version = "0.1.0"
description = "A library"
readme = "README.md"
requires-python = ">=3.10"
dependencies = [
    "requests>=2.28",
    "pydantic[email]>=2.0",
]

[project.optional-dependencies]
dev = ["pytest>=7.0", "black", "mypy"]
docs = ["sphinx", "sphinx-rtd-theme"]

[project.urls]
Homepage = "https://github.com/user/mylib"
Documentation = "https://mylib.readthedocs.io"

[project.scripts]
mycli = "mylib.cli:main"

[tool.black]
line-length = 88

[tool.pytest.ini_options]
minversion = "7.0"
```

### Version Management Inside pyproject.toml

**Static**:
```toml
version = "0.1.0"
```

**Dynamic** (read from file or attribute):
```toml
[tool.hatch.version]
path = "src/mylib/__init__.py"
pattern = '__version__ = "(?P<version>[^"]+)"'
```

Dynamic versioning enables single-source-of-truth but sacrifices reproducibility.

## Dependency Groups and Dependency Ranges

### Dependency Ranges

Python uses similar semver conventions as Node.js:

```toml
"requests>=2.28.0"        # At least 2.28.0
"requests<3"              # Before version 3
"requests>=2.28,<3"       # Between 2.28 and 3
"requests~=2.28"          # Patch-level bumps (≥2.28, <2.99)
"requests==2.28.1"        # Exact version (reproducible)
```

### Optional Dependencies and Groups

```toml
[project.optional-dependencies]
dev = ["pytest", "black"]
docs = ["sphinx"]
all = ["pytest", "black", "sphinx"]  # Convenience group

[project.dependencies]
requests = ">=2.28"  # Always required
```

**Install optionals**:
```bash
pip install mylib[dev,docs]
pip install mylib[all]
```

### Poetry's Dependency Groups (Different Convention)

```toml
[tool.poetry.group.dev.dependencies]
pytest = "^7.0"

[tool.poetry.group.docs.dependencies]
sphinx = "^5.0"
```

Not yet standardized in PEPs; Poetry-specific.

## Lockfiles: Requirements.txt vs. Lock Files

### requirements.txt (Simple)

```
requests==2.28.1
urllib3==1.26.8
charset-normalizer==2.0.12
```

Manually maintained or generated via `pip freeze`. Deterministic but doesn't capture full resolution tree.

### Poetry.lock / uv.lock / PDM.lock (Full Resolution)

```yaml
[[package]]
name = "requests"
version = "2.28.1"
description = "HTTP library"
category = "main"
python-versions = ">=3.7, <4"
files = [
    {file = "requests-2.28.1-py3-none-any.whl", hash = "sha256:..."}
]

[package.dependencies]
charset-normalizer = ">=2,<3"
idna = ">=2.5,<4"
```

Captures **all metadata**: transitive dependencies, exact URLs, hashes for verification.

**Best practice**:
- Commit lock file to version control
- Use `poetry install` / `uv sync` / `pdm sync` to install from lock (reproducible)
- Regenerate lock with `poetry lock` / `uv lock` / `pdm lock` when updating dependencies

## Virtual Environment Best Practices

### Project Structure

```
myproject/
├── pyproject.toml         # Package definition + tool config
├── .python-version        # pyenv local config
├── venv/ (or .venv/)      # Virtual environment (in .gitignore)
├── src/
│   └── mylib/
│       ├── __init__.py
│       └── main.py
├── tests/
│   └── test_main.py
└── README.md
```

### Development Workflow

```bash
# One-time setup
pyenv install 3.11.5
pyenv local 3.11.5
python -m venv venv
source venv/bin/activate

# First time
pip install -e ".[dev]"  # Editable install + dev deps

# Or with Poetry
poetry install

# Daily work
poetry run pytest
poetry run mylib
```

### CI/CD and Reproducibility

```bash
# CI should always use lock files
poetry install --no-directory  # Don't update lock
uv sync                        # Use uv.lock exactly
pdm sync                       # Use pdm.lock exactly

# Never use `pip install` without lock files in CI/CD
```

## Dependency Updates and Security

### Tools for Scanning

- **pip-audit** — Check for known vulnerabilities
- **safety** — Database of Python security advisories
- **Dependabot** (GitHub) — Automated PR creation for updates

```bash
pip-audit --fix
uv pip compile requirements.txt --upgrade
```

### Guidelines

1. **Distinguish patch updates** (safe) from minor/major (review required)
2. **Test all dependency updates** in CI before merging
3. **Use lock files** to freeze known-good versions
4. **Regenerate locks** regularly (weekly/monthly) to catch security updates

## Key Takeaways

1. **Virtual environments** (venv) isolate dependencies per project; use them always
2. **Python version management** (pyenv) enables multi-version workflows
3. **Package managers** (Poetry, PDM, uv, Hatch) each solve different optimization criteria
   - Poetry: Opinionated, all-in-one
   - PDM: Standards-compliant, fast
   - uv: Rust-fast, emerging standard
   - Hatch: PyPA-backed, minimal
4. **pyproject.toml** is the modern single source of truth for project metadata
5. **Lock files** are mandatory for reproducible installs; always commit them
6. **Dependency ranges** should be relaxed enough to allow patches but strict enough for safety
7. **Optional dependencies** enable flexible installs (dev, docs, all)

## See Also

- [language-python-packaging.md](language-python-packaging.md) — Build backends and distribution (wheels vs. sdists)
- [language-python.md](language-python.md) — Python conventions and imports
- [process-dependency-management.md](process-dependency-management.md) — Dependency updates and vulnerability scanning