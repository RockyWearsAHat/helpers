# Code Documentation — JSDoc, Javadoc, Docstrings & Self-Documenting Code

## The Documentation Problem

Code documentation serves multiple audiences with conflicting needs:

- **API consumers** need to understand _what_ the function does and _how_ to use it without reading implementation
- **Maintainers** need to understand _why_ choices were made and _when_ the code might fail
- **Reviewers** need sufficient context to verify correctness without reading related modules
- **Your future self** needs reminders about non-obvious implementation details and edge cases

Without documentation, code literacy decays: new team members waste days deciphering intent, refactoring becomes risky because nobody remembers constraints, and architectural decisions get lost when people leave.

The tension: **over-documentation clutters the code; under-documentation creates orphaned modules.**

---

## Language-Specific Documentation Systems

### JavaScript: JSDoc

JSDoc comments document using structured tag annotations over functions, classes, and method signatures. Tools like IDEs and documentation generators parse these to produce:

- Type hints visible in code editors (autocomplete, parameter checking)
- Searchable API documentation
- Type checking warnings in strict mode

**Format:**

```javascript
/**
 * Fetches a user by ID from the database.
 * @param {number} userId - The numeric user ID
 * @param {Object} options - Configuration options
 * @param {boolean} [options.cache=true] - Whether to use cached results
 * @returns {Promise<User>} Resolves with the user object; rejects with Error if not found
 * @throws {ValidationError} If userId is not a positive integer
 * @example
 * // Fetch a user without caching
 * const user = await fetchUser(42, { cache: false });
 * @deprecated Use fetchUserV2 instead; this function will be removed in v3.0
 */
```

**Common tags:**
- `@param` — function parameter (type, name, description)
- `@returns` — return value and type
- `@throws` — exception types
- `@example` — usage example (often executable in documentation generators)
- `@deprecated` — marks function as no longer recommended
- `@private` — hides from IDE autocomplete and documentation
- `@async` — marks function as asynchronous
- `@see`, `@link` — cross-references to related functions

**Discipline:** JSDoc works because the comment structure is machine-readable. Consistency matters: If you skip `@param` for one argument, IDEs can't infer types for that argument. Many teams enforce JSDoc coverage in CI (tools like `eslint-plugin-jsdoc`).

### Java: Javadoc

Javadoc comments use block tags similar to JSDoc but apply to classes, interfaces, methods, and fields. The Javadoc tool generates HTML documentation directly from source.

**Format:**

```java
/**
 * Calculates the compound interest for a given principal, rate, and time period.
 *
 * <p>This method uses the formula: A = P(1 + r/n)^(nt)
 *
 * @param principal the initial amount in dollars (must be positive)
 * @param annualRate the annual interest rate as a decimal (e.g., 0.05 for 5%)
 * @param years the number of years to compound (must be non-negative)
 * @param compoundsPerYear how many times interest is compounded annually (1, 2, 4, or 12)
 * @return the final amount after interest is calculated
 * @throws IllegalArgumentException if principal <= 0 or compoundsPerYear is invalid
 * @since 1.2
 * @author Jane Doe
 */
public double calculateCompoundInterest(
    double principal, double annualRate, int years, int compoundsPerYear) {
```

**Key conventions:**
- First sentence is a summary (shown in API overview)
- `<p>` tags separate paragraphs in the description
- `{@link ClassName#methodName}` creates hyperlinks in generated docs
- `{@code snippet}` renders code monospace without interpretation
- `@param`, `@return`, `@throws` document the contract
- `@deprecated` marks obsolete members
- JEP 467 (Java 21+) allows Markdown instead of HTML/tags

**Best practice:** Document the _contract_, not the implementation. "Returns the sum of all positive integers in the array" is better than "Iterates through array, checks if > 0, adds to accumulator."

### Python: Docstrings

Python docstrings (the first string literal in a module, class, function, or method) serve as inline documentation accessible via `help()` and IDEs. Three standard styles exist:

#### Google Style

Clean, readable format using section headers:

```python
def calculate_average(numbers: list[float], exclude_outliers: bool = False) -> float:
    """Calculate the arithmetic mean of a list of numbers.
    
    Optionally removes values beyond 1.5 standard deviations from the mean
    before calculation (excludes extreme outliers).
    
    Args:
        numbers: A list of numeric values. May not be empty.
        exclude_outliers: If True, removes statistical outliers. Default is False.
    
    Returns:
        The arithmetic mean as a float.
    
    Raises:
        ValueError: If the list is empty or contains non-numeric values.
        TypeError: If exclude_outliers is not a boolean.
    
    Example:
        >>> calculate_average([1, 2, 3, 4, 5])
        3.0
        >>> calculate_average([1, 2, 100], exclude_outliers=True)
        1.5
    """
```

#### NumPy Style

Structured format popular in scientific Python:

```python
def convolve(signal: np.ndarray, kernel: np.ndarray) -> np.ndarray:
    """Compute the discrete convolution of two signals.
    
    Parameters
    ----------
    signal : np.ndarray, shape (n,)
        The input signal.
    kernel : np.ndarray, shape (k,)
        The convolution kernel. Must be shorter than signal.
    
    Returns
    -------
    np.ndarray, shape (n + k - 1,)
        The convolved signal.
    
    Raises
    ------
    ValueError
        If kernel is longer than signal.
    
    Notes
    -----
    Uses FFT for efficiency when both signals are large.
    
    References
    ----------
    .. [1] Cooley, J.W., Tukey, J.W. (1965). "An algorithm for the machine
           calculation of complex Fourier series"
    
    Examples
    --------
    >>> convolve([1, 0, 1], [1, 1])
    array([1, 1, 1, 1])
    """
```

#### Sphinx/reStructuredText

Academic/formal style using reStructuredText markup:

```python
def fibonacci(n):
    """Return the nth Fibonacci number.
    
    :param n: position in the Fibonacci sequence (0-indexed)
    :type n: int
    :returns: the nth Fibonacci number
    :rtype: int
    :raises ValueError: if n is negative
    """
```

**Pattern:** Google and NumPy styles are machine-parseable by Sphinx (via the `napoleon` extension). They're more readable in raw form than reStructuredText. Google style is easier for simple functions; NumPy style scales better for functions with many parameters.

### Rust: Doc Comments

Rust's rustdoc tool generates documentation from doc comments (`///` for items, `//!` for crate/module documentation).

**Format:**

```rust
/// Computes the greatest common divisor using the Euclidean algorithm.
///
/// # Arguments
///
/// * `a` - First integer (can be negative; absolute value is used)
/// * `b` - Second integer (can be negative; absolute value is used)
///
/// # Returns
///
/// The GCD of the absolute values of `a` and `b`.
///
/// # Panics
///
/// Panics if both `a` and `b` are zero.
///
/// # Examples
///
/// ```
/// assert_eq!(gcd(48, 18), 6);
/// assert_eq!(gcd(-48, 18), 6);
/// ```
pub fn gcd(mut a: i32, mut b: i32) -> i32 {
    a = a.abs();
    b = b.abs();
    while b != 0 {
        let temp = b;
        b = a % b;
        a = temp;
    }
    a
}
```

**Key feature:** Doc comments can include **executable code examples**. Running `cargo test --doc` verifies examples still compile and pass — preventing documentation rot. This is unique to Rust and catches stale code quickly.

**Conventions:**
- `#` headers for sections (`# Arguments`, `# Returns`, `# Panics`)
- Code examples prefixed with ` ```
- Links using backticks: `SomeType`, `module::function`
- Module-level docs: `//!` at the top of the file

### Go: Godoc

Go's godoc tool treats the first comment before a package, type, func, or variable as its documentation. Unlike other systems, godoc **has no special syntax**—it's just comments.

**Format:**

```go
// Package math provides basic mathematical functions.
package math

// Sqrt returns the square root of x using Newton's method.
// It panics if x is negative.
func Sqrt(x float64) (float64, error) {
    if x < 0 {
        return 0, fmt.Errorf("cannot take square root of %v", x)
    }
    // Newton's method: x_{n+1} = (x_n + S/x_n) / 2
    guess := x
    for i := 0; i < 10; i++ {
        guess = (guess + x/guess) / 2
    }
    return guess, nil
}
```

**Go conventions:**
- Comment sentences start with the identifier name: "Sqrt returns", not "Returns"
- Indented code blocks for examples
- No structured tags; just prose
- Simplicity is intentional—forces clear writing

The minimalism works because Go's standard library uses consistent vocabulary, and godoc's output is plain HTML (easy to read).

---

## Self-Documenting Code vs. Comments

The false dichotomy: "Well-named code doesn't need comments."

This conflates two different problems:
1. **What the code does** — revealed by reading the code's structure
2. **Why the code exists** — lost unless explicitly stated

### Code Clarity Heuristics

Good code reveals _what_ it does. Examples:

**Poor clarity:**
```python
# Bad
for i in range(len(users)):
    if users[i][2] > 18:
        process_user(users[i])
```

**Good clarity:**
```python
for user in users:
    age = user.age
    if age > legal_adult_threshold:
        process_user(user)
```

Even better:
```python
adult_users = [u for u in users if u.age >= LEGAL_ADULT_THRESHOLD]
for user in adult_users:
    process_user(user)
```

Ordering: Use clear variable names and structures _first_. Then add comments only for context that can't be inferred from the code.

### When Code Cannot Self-Document

**Non-obvious algorithmic choices:**
```rust
// Skip zero-length intervals to avoid division by zero in velocity calculation.
// This is safe because the kernel ensures all intervals have been pre-validated.
if self.start == self.end { continue; }
```

**Business logic rationale:**
```javascript
// Customers in Nevada are exempt from state sales tax per SB 498 (2023).
// This exemption expires Dec 31, 2024; add a TODO to revert.
const isNevada = address.state === 'NV';
const exemptFromSalesTax = isNevada && isCustomerInDatabase(customer.id);
```

**Performance trade-offs:**
```python
# Using lazy evaluation instead of eager because historical analysis shows
# 85% of queries filter before page 3. Eager loading wastes ~200ms per query.
results = lazy_paginate_results(query, page_size=20)
```

**Constraint boundaries:**
```go
// UTF-8 strings have a max length of 64KB in this protocol (2-byte length prefix).
// Longer strings silently truncate; see RFC 4242, section 3.
func SendString(conn net.Conn, s string) error {
```

**When not to comment:** Repeating what the code says is waste:
```javascript
// Bad
// Check if count is greater than zero
if (count > 0) {
  // Decrement count
  count--;
}

// Good: variable names already tell the story
if (count > 0) {
  count--;
}
```

---

## Documentation Generation Tools

### Sphinx (Python)

Sphinx generates documentation from docstrings, Markdown, and reStructuredText. It's the de facto standard for Python APIs.

**Key features:**
- Reads Google/NumPy/reStructuredText docstrings via `sphinx.ext.napoleon`
- Generates searchable HTML, PDF, and ePub
- Cross-references across modules (`py:func:`, `py:class:`)
- Integrates with `ReadTheDocs` for free hosting

**Workflow:**
```bash
sphinx-quickstart docs/
# Edit docs/conf.py to enable napoleon, autodoc extensions
# Generate: make html
```

### TypeDoc (TypeScript/JavaScript)

TypeDoc generates HTML documentation from TypeScript and JSDoc comments:

```bash
npm install -g typedoc
typedoc --out docs src/
```

Outputs gorgeous, searchable HTML with type signatures automatically extracted from TypeScript.

### Cargo Doc (Rust)

Generates docs from rustdoc comments directly:
```bash
cargo doc --open
```

Automatically includes dependency docs; navigable by type and module. The Rust ecosystem standard.

### Godoc (Go)

Built into the Go toolchain:
```bash
go doc <package>
godoc -h  # Starts a local web server
```

Minimal but effective—reflects Go's philosophy.

### Javadoc (Java)

Standard Java tool:
```bash
javadoc -d docs src/**/*.java
```

Generates navigable HTML with class hierarchies and cross-references. Older than most modern tooling, but still used extensively in enterprise.

---

## API Surface Documentation

Beyond function signatures, document the **contract** your API provides:

### Error Semantics

```python
def fetch_user(user_id: int) -> User:
    """Retrieve a user by ID.
    
    Returns:
        The User object.
    
    Raises:
        NotFoundError: If no user with the given ID exists.
        AuthenticationError: If credentials are invalid (HTTP 401).
        RateLimitError: If more than 100 requests per minute (HTTP 429).
    """
```

### Concurrency Guarantees

```go
// Range is not thread-safe. Concurrent calls to Range will panic.
// Use NewThreadSafeRange for concurrent access.
func (r *Range) Iterate(fn func(int)) {
```

### Side Effects

```rust
/// Consumes the builder and returns a Config.
/// All subsequent calls to this builder will panic.
pub fn build(self) -> Config {
```

### Deprecated Paths

```javascript
/**
 * @deprecated Since v2.0. Use `calculateSum()` instead.
 * This function will be removed in v3.0.
 */
function sum(numbers) {
```

### Versioning & Stability

```python
def experimental_streaming_api(data: Iterator) -> AsyncGenerator:
    """EXPERIMENTAL: This API is unstable and may change without notice.
    
    Expected to stabilize in version 3.0. Feedback welcome at github.com/project/issues.
    """
```

---

## Documentation Maintenance Discipline

Documentation rots because:
1. **Tests don't exercise examples** — examples become stale and don't compile
2. **Code changes without documentation updates** — signature changes break examples
3. **No ownership** — nobody is assigned responsibility for keeping docs fresh

### Three Prevention Strategies

**1. Test documentation examples:**
- Rust: `cargo test --doc` runs doc comment examples
- Python doctests: `pytest --doctest-modules` or `python -m doctest`
- TypeScript/JavaScript: extract examples and run in CI

**2. Version examples with code:**
Store examples alongside code, not in separate doc files. Link them, don't duplicate them.

**3. Mark documentation as owned:**
In code review, ask: "Does this PR update the docs?" Make doc updates as mandatory as code reviews. Use code owners files (`CODEOWNERS`) if applicable.

---

## See Also

- [process-documentation.md](process-documentation.md) — how to structure documentation discipline
- [technical-writing-patterns.md](technical-writing-patterns.md) — Diátaxis framework for docs organization
- [clean-code.md](clean-code.md) — naming and readability principles
- [api-documentation.md](api-documentation.md) — REST/GraphQL API documentation patterns