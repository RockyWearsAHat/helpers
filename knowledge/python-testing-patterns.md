# Python Testing Patterns: pytest, Mocking, Property-Based, and Advanced Strategies

## pytest Framework Fundamentals

`pytest` is Python's dominant testing framework. Tests are simple functions starting with `test_`:

```python
def test_addition():
    assert 1 + 1 == 2

def test_string_equality():
    assert "hello".upper() == "HELLO"
```

Run with `pytest` or `pytest tests/` to discover and execute all test functions.

**Why pytest over unittest:**
- Simpler syntax (plain assert vs `self.assertEqual`)
- Fixture injection instead of setUp/tearDown
- Powerful parametrization for testing many inputs
- Extensive ecosystem of plugins
- Better error messages and introspection

## Fixtures: Reusable Test Building Blocks

Fixtures are functions that provide data or set up state for tests. They're injected by name:

```python
import pytest

@pytest.fixture
def user():
    return {"name": "Alice", "email": "alice@example.com"}

def test_user_name(user):
    assert user["name"] == "Alice"

def test_user_email(user):
    assert user["email"] == "alice@example.com"
```

**Fixture scopes:**
- `function` (default) — created and destroyed per test function
- `class` — created once per test class
- `module` — created once per module
- `session` — created once for the entire test run

```python
@pytest.fixture(scope="session")
def database_connection():
    conn = connect("database://...")
    yield conn
    conn.close()
```

**Fixture dependencies:**
```python
@pytest.fixture
def admin_user(user):
    user["role"] = "admin"
    return user

def test_admin_can_delete(admin_user):
    assert admin_user["role"] == "admin"
```

**Parametrized fixtures** generate multiple test instances from a fixture:

```python
@pytest.fixture(params=[1, 2, 5, 10])
def count(request):
    return request.param

def test_count_is_positive(count):
    assert count > 0  # Test runs 4 times
```

## Parametrize: Testing Multiple Inputs

`@pytest.mark.parametrize` runs the same test with different inputs:

```python
import pytest

@pytest.mark.parametrize("input,expected", [
    (1, 2),
    (2, 4),
    (3, 6),
])
def test_double(input, expected):
    assert input * 2 == expected
```

This generates three test cases: `test_double[1-2]`, `test_double[2-4]`, `test_double[3-6]`.

**Multiple parametrize decorators create a Cartesian product:**

```python
@pytest.mark.parametrize("x", [1, 2])
@pytest.mark.parametrize("y", [10, 20])
def test_add(x, y):
    assert x + y > 0  # Runs 4 times: (1,10), (1,20), (2,10), (2,20)
```

**Indirect parametrization** passes values to a fixture:

```python
@pytest.fixture
def connection(request):
    db = request.param
    return connect(db)

@pytest.mark.parametrize("connection", ["postgresql", "mysql"], indirect=True)
def test_query(connection):
    result = connection.query("SELECT 1")
    assert result is not None
```

## Markers: Organizing and Filtering Tests

Markers tag tests for selective execution:

```python
import pytest

@pytest.mark.slow
def test_integration():
    time.sleep(10)

@pytest.mark.skip(reason="Not implemented yet")
def test_future_feature():
    pass

@pytest.mark.xfail(reason="Known bug in issue #42")
def test_known_failure():
    assert False  # This failure is expected and reported separately

@pytest.mark.skipif(sys.platform == "win32", reason="Unix only")
def test_unix_path():
    pass
```

Run `pytest -m slow` to run only slow tests, or `pytest -m "not slow"` to skip them.

**Custom markers:**

```python
# conftest.py
import pytest

@pytest.mark.database
def test_db_query():
    pass

# Run: pytest -m database
```

## conftest.py: Shared Fixtures and Configuration

`conftest.py` in the test root defines fixtures and hooks available to all tests:

```python
# tests/conftest.py
import pytest

@pytest.fixture
def api_client():
    return APIClient("http://localhost:8000")

@pytest.fixture
def sample_data():
    return {"users": [{"id": 1, "name": "Alice"}]}
```

Tests in any subdirectory can use these fixtures:

```python
# tests/integration/test_api.py
def test_list_users(api_client, sample_data):
    response = api_client.get("/users")
    assert response.json() == sample_data["users"]
```

**pytest hooks** in conftest.py customize test execution:

```python
# tests/conftest.py

def pytest_configure(config):
    """Called before test collection."""
    config.addinivalue_line("markers", "database: marks tests as database tests")

def pytest_runtest_setup(item):
    """Called before each test function."""
    if "database" in item.keywords:
        # Skip if database is unavailable
        if not database_available():
            pytest.skip("Database not available")

def pytest_runtest_makereport(item, call):
    """Called after each test phase."""
    if call.excinfo and "database" in item.keywords:
        # Log database state for debugging
        log_database_state()
```

## unittest.mock: Patching and Spying

`unittest.mock` allows replacing parts of your code with mock objects:

```python
from unittest.mock import patch, MagicMock, call

# Patch a function
@patch("myapp.external_service.get_user")
def test_user_lookup(mock_get_user):
    mock_get_user.return_value = {"id": 1, "name": "Alice"}
    
    result = lookup_user(1)
    assert result["name"] == "Alice"
    mock_get_user.assert_called_once_with(1)

# Patch a class
@patch("myapp.database.Connection")
def test_query(mock_conn_class):
    mock_conn = MagicMock()
    mock_conn_class.return_value = mock_conn
    mock_conn.query.return_value = [{"id": 1}]
    
    result = run_query()
    assert len(result) == 1
```

**MagicMock features:**
- Auto-creates attributes and methods on access
- Records all calls via `call_args`, `call_args_list`, `call_count`
- Assertions: `assert_called_once()`, `assert_called_with(...)`, `assert_has_calls(...)`
- `side_effect` for custom behavior or raising exceptions

```python
mock = MagicMock()
mock.side_effect = ValueError("Network error")
with pytest.raises(ValueError):
    mock()  # Raises ValueError

mock.side_effect = [1, 2, 3]
mock()  # Returns 1
mock()  # Returns 2
```

**Spec for type safety:**

```python
from unittest.mock import create_autospec

# Ensure mock matches the real class's signature
mock_user = create_autospec(User)
mock_user.name = "Alice"
mock_user.get_email.return_value = "alice@example.com"

# This raises TypeError: spec'd mock object doesn't have 'nonexistent_method'
# mock_user.nonexistent_method()
```

**Context manager patching:**

```python
from unittest.mock import patch

def test_tempfile():
    with patch("tempfile.mkdtemp") as mock_mkdtemp:
        mock_mkdtemp.return_value = "/tmp/fake"
        tmpdir = get_temporary_directory()
        assert tmpdir == "/tmp/fake"
```

## Hypothesis: Property-Based Testing

`hypothesis` generates random test inputs and checks that properties always hold:

```python
from hypothesis import given, strategies as st

@given(st.lists(st.integers()))
def test_sort_is_idempotent(xs):
    """Sorting twice gives the same result."""
    assert sorted(sorted(xs)) == sorted(xs)

@given(st.lists(st.integers(min_value=-100, max_value=100)))
def test_sort_is_ordered(xs):
    """Result is in ascending order."""
    result = sorted(xs)
    for a, b in zip(result, result[1:]):
        assert a <= b
```

**When a property fails, hypothesis shrinks the input to the minimal failing case:**

```python
# If this fails on [500, -3000, 1, 42, ...]
# Hypothesis shrinks it to [1, -1] or similar
```

**Custom strategies:**

```python
from hypothesis import given, strategies as st

user_strategy = st.fixed_dictionaries({
    "name": st.text(min_size=1),
    "age": st.integers(min_value=0, max_value=150),
    "email": st.emails(),
})

@given(user_strategy)
def test_user_creation(user):
    assert len(user["name"]) > 0
    assert 0 <= user["age"] <= 150
```

## Coverage: Measuring Test Completeness

`pytest-cov` measures code coverage — the percentage of code lines executed by tests:

```bash
pytest --cov=myapp --cov-report=html
```

This generates an HTML report showing which lines are covered and which are missed.

**Coverage limitations:**
- 100% line coverage does not mean no bugs
- Missing branches: `if x: ...` might execute but not `else: ...`
- Uncovered error paths (exceptions, rare conditions)

**Use coverage as a metric, not a goal.** Aim for coverage of critical paths (business logic, error handling), not coverage for its own sake.

## Async Testing

Testing async functions requires special setup:

```python
import pytest
import asyncio

@pytest.mark.asyncio
async def test_fetch_user():
    user = await get_user(1)
    assert user["name"] == "Alice"

# Or use pytest-asyncio fixture
@pytest.fixture
def event_loop():
    loop = asyncio.get_event_loop_policy().new_event_loop()
    yield loop
    loop.close()

@pytest.mark.asyncio
async def test_concurrent_requests():
    results = await asyncio.gather(
        get_user(1),
        get_user(2),
        get_user(3),
    )
    assert len(results) == 3
```

## Snapshot Testing

Snapshot tests capture the current output and alert on changes:

```python
# pip install pytest-snapshot

def test_user_repr(snapshot):
    user = User(name="Alice", email="alice@example.com")
    snapshot.assert_match(repr(user))
```

First run creates `test_user_repr.ambr` with the output. Subsequent runs compare; if output changes, the test fails and shows the diff.

**Use for:**
- API responses (ensure no unintended changes)
- Complex object serialization
- Generated code or configuration

## tox and nox: Test Automation Across Environments

`tox` runs tests across multiple Python versions and dependency sets:

```ini
# tox.ini
[tox]
envlist = py39, py310, py311, lint, type

[testenv]
deps = pytest pytest-cov
commands = pytest --cov=myapp

[testenv:lint]
commands = ruff check myapp

[testenv:type]
commands = mypy myapp
```

`nox` is a similar tool using Python configuration:

```python
# noxfile.py
import nox

@nox.session(python=["3.9", "3.10", "3.11"])
def tests(session):
    session.install("pytest", "pytest-cov")
    session.run("pytest", "--cov=myapp")

@nox.session
def lint(session):
    session.install("ruff")
    session.run("ruff", "check", "myapp")
```

## pytest Plugins: Extending Testing

Popular extensions:
- `pytest-cov` — code coverage
- `pytest-xdist` — parallel test execution
- `pytest-timeout` — fail tests that exceed time limit
- `pytest-mock` — cleaner mock syntax via `mocker` fixture
- `pytest-cases` — lightweight parametrization
- `pytest-asyncio` — async test support
- `pytest-randomly` — randomize test order to catch hidden dependencies
- `pytest-benchmark` — performance regression detection

```python
# pytest-mock
def test_external_call(mocker):
    mock = mocker.patch("myapp.external_service.get")
    mock.return_value = "data"
    assert get() == "data"

# pytest-benchmark
def test_performance(benchmark):
    result = benchmark(expensive_function, arg1, arg2)
    assert result is not None
```

## Testing Best Practices

**Fast feedback loop:** Tests should complete in seconds, not minutes. Use mocks for slow I/O.

**Isolation:** Each test should be independent. Don't rely on test execution order or shared state.

**Clarity:** Test names should describe what is being tested: `test_user_creation_with_invalid_email_raises_error`.

**Arrange-Act-Assert:** Structure tests in three phases:
```python
def test_invoice_total():
    # Arrange
    invoice = Invoice([
        Line(quantity=2, price=50),
        Line(quantity=1, price=100),
    ])
    
    # Act
    total = invoice.calculate_total()
    
    # Assert
    assert total == 200
```

**Avoid implementation details:** Test behavior, not internal state. Mock external dependencies, not your own code.

## See Also

- Debugging: pytest verbose output with `-vv`, `-s` for print statements
- CI/CD: GitHub Actions, pytest in continuous integration
- Test data: factories (factory_boy), realistic data generation (Faker)