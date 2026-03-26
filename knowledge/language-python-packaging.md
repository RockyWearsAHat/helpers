# Python Packaging — setuptools, pyproject.toml, Build Backends & Distribution

## Overview

Python packaging evolved from setuptools' ad-hoc `setup.py` scripts toward declarative, backend-agnostic standards. **PEP 517** (2017) introduced pluggable build backends; **PEP 518** standardized build requirements in `pyproject.toml`; **PEP 621** (2021) moved all package metadata to `pyproject.toml`. Modern projects declare once, work with any backend.

## Core Concepts

### The Build System (PEP 517)

A build backend transforms source code into **wheels** (binary distributions) and **sdists** (source distributions). Popular backends:

- **setuptools** — the default. Handles most C extensions, legacy compatibility.
- **hatchling** — simple, fast, zero-dependency (built into Hatch).
- **flit** — lightweight; requires Python 3.7+.
- **maturin** — Rust/Python hybrids; compiles Rust to binary wheels.
- **pdm-backend** — PDM's simple backend.

The backend is declared in `pyproject.toml`:

```toml
[build-system]
requires = ["setuptools>=61", "wheel"]
build-backend = "setuptools.build_meta"
```

### Package Metadata (PEP 621)

Modern standard: all metadata in `pyproject.toml`, not `setup.py`:

```toml
[project]
name = "mylib"
version = "0.1.0"
description = "A brief description"
authors = [{name = "Author", email = "email@example.com"}]
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
```

The `version` field enables dynamic versioning (read from `__init__.py` or a separate file), though static versions are preferred for reproducibility.

### Wheels vs Sdist

- **Wheel** (`.whl`) — pre-built, binary format. Contains compiled C extensions, metadata, and bytecode. Installs instantly; no compilation.
- **Sdist** (`.tar.gz`) — source archive. Contains source code, build script, package metadata. Requires build step on installation.

Publish both: wheels for speed and reliability, sdists for source auditability.

## Package Distribution: setuptools, poetry, pdm, uv

### setuptools

The venerable default. `setup.py` contains a `setup()` call with package metadata. Modern projects use `pyproject.toml` + `setuptools.build_meta`:

```python
# setup.py (legacy, not needed with pyproject.toml)
from setuptools import setup
setup(name="mylib", version="0.1.0")
```

```toml
# pyproject.toml (modern)
[build-system]
requires = ["setuptools>=61", "wheel"]
build-backend = "setuptools.build_meta"

[tool.setuptools.packages]
find = {}  # Auto-discover packages
```

Handles C extensions well; slower dependency resolution.

### poetry

Dependency lock file management + unified build. `poetry.lock` pins all transitive dependencies (reproducible installs). Poetry is a poet's tool: opinionated, beautiful, slower for large monorepos.

```toml
[tool.poetry]
name = "mylib"
version = "0.1.0"
dependencies = ["requests >= 2.28"]

[[tool.poetry.source]]
name = "internal"
url = "https://pypi.internal.example.com/simple"
priority = "primary"
```

**Tradeoff:** Excellent for single packages; awkward for monorepos (one `poetry.lock` per package).

### pdm

Like poetry but more flexible. Supports PEP 582 (local `__pypackages__` for offline install) and project-relative virtual envs. Simpler TOML format:

```toml
[project]
dependencies = ["requests"]

[tool.pdm.dev-dependencies]
test = ["pytest"]
```

**Tradeoff:** Smaller ecosystem; emerging tooling.

### uv

Fast Rust-based pip replacement. Resolves dependencies in milliseconds (vs pip's minutes on large projects). No lock file by default; can generate one:

```sh
uv pip install -r requirements.txt  # Fast
uv pip compile pyproject.toml -o requirements.txt  # Lock
```

**Emerging winner** for monorepos and CI/CD (minimal overhead). Conservative: compatible with pip's ecosystem.

### conda

Channel-based package manager for scientific/data science. Manages non-Python dependencies (BLAS, MKL, CUDA). Slower, heavier, separate from PyPI:

```sh
conda install -c conda-forge numpy scipy  # From conda-forge channel
```

**Tradeoff:** Overkill for pure-Python packages; essential for data science (Numba, PyTorch, TensorFlow depend on BLAS).

## Virtual Environments

Isolate project dependencies from system Python.

```sh
python -m venv venv        # Create
source venv/bin/activate   # macOS/Linux
venv\Scripts\activate.bat  # Windows

pip install -r requirements.txt
```

**Modern tooling:**
- **poetry**: `poetry install` creates venv automatically.
- **pdm**: `pdm install` (PEP 582 or `.venv`).
- **uv**: `uv venv` + `uv pip install`.

## Dependency Specification

Pins constrain acceptable versions.

```toml
# Exact version (fragile; pinning all transitive deps is better)
requests = "2.28.1"

# Compatible release
requests = "~= 2.28"  # >= 2.28, == 2.*

# Loose range
requests = ">= 2.28, < 3.0"

# Exclude ranges
typing-extensions = ">= 3.7.4, != 3.8, != 3.9"

# Pre-release allowed
mylib = ">= 1.0a1"

# Multiple indexes / extras
pydantic = {version = ">= 2.0", extras = ["email"]}
cryptography = {version = ">= 40", index = "internal"}
```

**Lock files** (`.lock`, `poetry.lock`) pin all transitive dependencies for reproducibility. Commit to version control for production environments.

## Publishing to PyPI

1. **Build wheels + sdist:**

```sh
pip install build
python -m build
# Output: dist/mylib-0.1.0.tar.gz, dist/mylib-0.1.0-py3-none-any.whl
```

2. **Upload:**

```sh
pip install twine
twine upload dist/*
# Or: poetry publish, pdm publish, uv pip publish
```

3. **Authenticate** via API token (not password):

```toml
# ~/.pypirc
[pypi]
username = __token__
password = pypi-AgEIcHlwaS5vcmcvA...
```

4. **Test locally first:**

```sh
twine check dist/*
# Or: inspect the wheel with zipfile library
```

## Best Practices

- **PEP 440 versioning:** `MAJOR.MINOR.PATCH` (semver-like), e.g., `2.0.0a1.post0.dev3`.
- **Avoid`*` in dependencies:** Never pin `*` in production. Lock files are the social contract.
- **Separate runtime and dev dependencies.** Use `[project.optional-dependencies]` for test, docs, dev tools.
- **Use project references (monorepos):** setuptools (local file paths), poetry (path dependencies), pdm (editable installs).
- **Automate builds in CI.** No manual wheel creation; CI/CD handles it.
- **Test both sdist and wheel installations.** Wheels often miss files.

## See Also

- **language-python.md** — Python idioms and PEP 8
- **language-python-async.md** — Async patterns
- **developer-tools-landscape.md** — Build system comparisons across languages