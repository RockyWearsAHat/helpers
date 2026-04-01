# Python Conventions and Idioms

## The Zen of Python (PEP 20)

```
Beautiful is better than ugly.
Explicit is better than implicit.
Simple is better than complex.
Complex is better than complicated.
Flat is better than nested.
Sparse is better than dense.
Readability counts.
Special cases aren't special enough to break the rules.
Although practicality beats purity.
Errors should never pass silently.
Unless explicitly silenced.
In the face of ambiguity, refuse the temptation to guess.
There should be one-- and preferably only one --obvious way to do it.
Although that way may not be obvious at first unless you're Dutch.
Now is better than never.
Although never is often better than *right* now.
If the implementation is hard to explain, it's a bad idea.
If the implementation is easy to explain, it may be a good idea.
Namespaces are one honking great idea -- let's do more of those!
```

## PEP 8 Style Guide (Key Conventions)

### Naming

- `snake_case` for functions, methods, variables, modules.
- `PascalCase` for classes.
- `UPPER_SNAKE_CASE` for constants.
- `_private` for internal use (single leading underscore).
- `__mangled` for name mangling (double underscore — rare, avoid unless needed).

### Formatting

- 4 spaces per indent (never tabs).
- 79 characters max line length (99 is common in practice).
- 2 blank lines before top-level definitions. 1 blank line between methods.
- Imports at the top: stdlib → third-party → local (separated by blank lines).
- Use absolute imports over relative imports.

### Formatting tools (automate style consistency):

- **Black**: Opinionated formatter that eliminates style debates.
- **isort**: Sorts imports automatically.
- **Ruff**: Linter + formatter. Extremely fast (Rust-based). Can replace flake8, isort, and more.

## Type Hints (Modern Python 3.10+)

```python
# Basic types
def greet(name: str) -> str:
    return f"Hello, {name}"

# Collections (3.10+ built-in generics)
def process(items: list[int]) -> dict[str, int]:
    return {"total": sum(items)}

# Union types (3.10+ pipe syntax)
def parse(value: str | int) -> float:
    return float(value)

# Optional (same as X | None)
def find(key: str) -> str | None:
    return cache.get(key)

# TypeAlias
type UserId = int
type Handler = Callable[[Request], Response]

# TypedDict
class Config(TypedDict):
    host: str
    port: int
    debug: bool
```

**Type hints are strongly encouraged in new code.** Running `mypy` or `pyright` in CI catches type errors early.

## Dataclasses & Modern Classes

```python
from dataclasses import dataclass, field

@dataclass
class User:
    name: str
    email: str
    age: int
    roles: list[str] = field(default_factory=list)

# Immutable (frozen)
@dataclass(frozen=True)
class Point:
    x: float
    y: float

# Python 3.11+ slots for memory efficiency
@dataclass(slots=True)
class Event:
    name: str
    timestamp: float
```

**Prefer dataclasses over plain classes or namedtuples** for data containers. Use `attrs` for more advanced features.

**Pydantic** for validation + serialization (API models, config):

```python
from pydantic import BaseModel, Field

class CreateUser(BaseModel):
    name: str = Field(min_length=1, max_length=100)
    email: str
    age: int = Field(ge=0, le=150)
```

## Error Handling

```python
# Be specific with exceptions
try:
    result = process(data)
except ValueError as e:
    logger.warning("Invalid data: %s", e)
    return default
except ConnectionError as e:
    logger.error("Service unavailable: %s", e)
    raise ServiceUnavailableError from e

# NEVER do this:
try:
    something()
except Exception:  # Catches everything including KeyboardInterrupt-adjacent
    pass  # Silently swallowed — a debugging nightmare

# Custom exceptions
class AppError(Exception):
    """Base exception for the application."""

class NotFoundError(AppError):
    """Resource not found."""

class ValidationError(AppError):
    """Input validation failed."""
```

## Context Managers

```python
# File handling (always use with)
with open("data.json") as f:
    data = json.load(f)

# Custom context manager
from contextlib import contextmanager

@contextmanager
def timer(label: str):
    start = time.perf_counter()
    yield
    elapsed = time.perf_counter() - start
    logger.info("%s took %.3fs", label, elapsed)

with timer("database query"):
    results = db.query(sql)
```

## Async/Await (asyncio)

```python
import asyncio
import aiohttp

async def fetch_url(session: aiohttp.ClientSession, url: str) -> str:
    async with session.get(url) as response:
        return await response.text()

async def fetch_all(urls: list[str]) -> list[str]:
    async with aiohttp.ClientSession() as session:
        tasks = [fetch_url(session, url) for url in urls]
        return await asyncio.gather(*tasks)

# Python 3.12+ TaskGroup (structured concurrency)
async def process_all(items: list[Item]) -> list[Result]:
    results = []
    async with asyncio.TaskGroup() as tg:
        for item in items:
            tg.create_task(process_item(item))
    return results
```

## Modern Python Features (3.10–3.13)

### Structural pattern matching (3.10)

```python
match command:
    case ["quit"]:
        sys.exit(0)
    case ["move", x, y]:
        move_to(int(x), int(y))
    case ["say", *words]:
        print(" ".join(words))
    case _:
        print(f"Unknown command: {command}")
```

### Exception groups (3.11)

```python
try:
    async with asyncio.TaskGroup() as tg:
        tg.create_task(risky_op_1())
        tg.create_task(risky_op_2())
except* ValueError as eg:
    for exc in eg.exceptions:
        handle_value_error(exc)
except* TypeError as eg:
    for exc in eg.exceptions:
        handle_type_error(exc)
```

### Performance annotations (3.13)

```python
# Free-threaded Python (no GIL) — experimental in 3.13
# JIT compiler — experimental in 3.13
```

## Project Structure

```
my_project/
├── pyproject.toml          # Project config (replaces setup.py, setup.cfg)
├── src/
│   └── my_package/
│       ├── __init__.py
│       ├── core.py
│       ├── models.py
│       └── utils.py
├── tests/
│   ├── conftest.py         # Shared pytest fixtures
│   ├── test_core.py
│   └── test_models.py
├── .python-version         # Pin Python version
└── README.md
```

**Use `pyproject.toml`** for all config (not setup.py). Single source of truth for dependencies, tool config, and metadata.

**Virtual environments:** Strongly recommended. `uv` (fast, Rust-based) or `venv` are common choices. Installing packages globally risks version conflicts.

## Testing with pytest

```python
import pytest

def test_add():
    assert add(2, 3) == 5

@pytest.mark.parametrize("input,expected", [
    ("hello", "HELLO"),
    ("world", "WORLD"),
    ("", ""),
])
def test_upper(input, expected):
    assert input.upper() == expected

@pytest.fixture
def db_session():
    session = create_session()
    yield session
    session.rollback()

def test_create_user(db_session):
    user = create_user(db_session, name="Alice")
    assert user.id is not None
```

---

_Sources: PEP 8, PEP 20, PEP 484, PEP 526, PEP 612, PEP 695, Python documentation, Effective Python (Brett Slatkin), Fluent Python (Luciano Ramalho), Real Python_
