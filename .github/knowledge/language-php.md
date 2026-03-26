# PHP Best Practices (Modern PHP 8.3+)

## Modern PHP Is Not 2005 PHP

PHP has undergone a massive transformation. Modern PHP is typed, performant, and well-structured. If your mental model is `mysql_query()` and `register_globals`, update it.

## Type System

```php
// Strict types — ALWAYS enable
declare(strict_types=1);

// Typed properties, parameters, and return types
class User
{
    public function __construct(
        public readonly string $name,
        public readonly string $email,
        public readonly int $age,
        public readonly Role $role = Role::User,
    ) {}
}

// Union types (PHP 8.0)
function parse(string|int $value): float
{
    return (float) $value;
}

// Intersection types (PHP 8.1)
function save(Countable&Iterator $collection): void { ... }

// DNF types (PHP 8.2) — Disjunctive Normal Form
function process((Countable&Iterator)|null $data): void { ... }

// Never return type (PHP 8.1) — function doesn't return
function abort(string $message): never
{
    throw new RuntimeException($message);
}

// Nullable types
function find(int $id): ?User  // Same as User|null
{
    return $this->repository->find($id);
}
```

## Enums (PHP 8.1)

```php
// Basic enum
enum Status
{
    case Active;
    case Inactive;
    case Suspended;
}

// Backed enum (with values)
enum Role: string
{
    case Admin = 'admin';
    case User = 'user';
    case Viewer = 'viewer';

    public function isPrivileged(): bool
    {
        return $this === self::Admin;
    }
}

$role = Role::from('admin');        // Throws on invalid
$role = Role::tryFrom('unknown');   // Returns null on invalid
```

## Readonly Properties & Classes

```php
// Readonly properties (PHP 8.1)
class Config
{
    public function __construct(
        public readonly string $host,
        public readonly int $port,
        public readonly bool $debug = false,
    ) {}
}

// Readonly classes (PHP 8.2) — all properties are readonly
readonly class Point
{
    public function __construct(
        public float $x,
        public float $y,
    ) {}
}
```

## Match Expression

```php
// match is strict (===), exhaustive, and returns a value
$result = match ($status) {
    'active' => 'User is active',
    'inactive', 'suspended' => 'User is not active',
    default => throw new InvalidArgumentException("Unknown status: $status"),
};

// No type coercion (unlike switch)
match (true) {
    $age >= 65 => 'senior',
    $age >= 18 => 'adult',
    $age >= 13 => 'teenager',
    default    => 'child',
};
```

## Named Arguments

```php
// Readable function calls
$user = new User(
    name: 'Alice',
    email: 'alice@test.com',
    age: 30,
    role: Role::Admin,
);

// Skip optional parameters
htmlspecialchars($string, double_encode: false);
```

## Fibers (PHP 8.1)

```php
// Cooperative concurrency (used by async frameworks like ReactPHP, Amp, Revolt)
$fiber = new Fiber(function (): void {
    $value = Fiber::suspend('hello');
    echo "Got: $value\n";
});

$result = $fiber->start();        // Returns 'hello'
$fiber->resume('world');          // Prints "Got: world"
```

## Error Handling

```php
// Custom exception hierarchy
class AppException extends RuntimeException {}
class NotFoundException extends AppException {}
class ValidationException extends AppException
{
    public function __construct(
        public readonly array $errors,
        string $message = 'Validation failed',
    ) {
        parent::__construct($message);
    }
}

// Typed catch blocks
try {
    $user = $this->findOrFail($id);
} catch (NotFoundException $e) {
    return response()->json(['error' => 'Not found'], 404);
} catch (ValidationException $e) {
    return response()->json(['errors' => $e->errors], 422);
}

// Null-safe operator
$street = $user?->getAddress()?->getStreet();
```

## Collections & Functional Patterns

```php
// Array functions (functional style)
$adults = array_filter($users, fn(User $u) => $u->age >= 18);
$names = array_map(fn(User $u) => $u->name, $users);
$total = array_reduce($orders, fn(float $sum, Order $o) => $sum + $o->total, 0.0);

// Arrow functions (PHP 7.4)
$double = fn(int $n): int => $n * 2;

// First-class callable syntax (PHP 8.1)
$lengths = array_map(strlen(...), $strings);

// Spread operator
function sum(int ...$numbers): int
{
    return array_sum($numbers);
}
sum(...[1, 2, 3]);
```

## PSR Standards

| PSR        | Purpose                                              |
| ---------- | ---------------------------------------------------- |
| **PSR-1**  | Basic coding standard                                |
| **PSR-4**  | Autoloading standard (namespace → directory mapping) |
| **PSR-7**  | HTTP message interfaces                              |
| **PSR-11** | Container interface (DI)                             |
| **PSR-12** | Extended coding style (superseded by PER)            |
| **PER-CS** | PHP Evolving Recommendation Coding Style (current)   |
| **PSR-14** | Event dispatcher                                     |
| **PSR-15** | HTTP server request handlers (middleware)            |
| **PSR-18** | HTTP client                                          |

## Project Structure (Laravel/Symfony Style)

```
src/
├── Controller/
├── Service/
├── Repository/
├── Entity/ (or Model/)
├── DTO/
├── Exception/
├── Event/
└── ValueObject/
tests/
├── Unit/
├── Integration/
└── Functional/
composer.json
phpunit.xml
phpstan.neon
```

## Tooling

| Tool                                   | Purpose                                 |
| -------------------------------------- | --------------------------------------- |
| **Composer**                           | Dependency management                   |
| **PHPStan** / **Psalm**                | Static analysis (use level max)         |
| **PHP-CS-Fixer** / **PHP_CodeSniffer** | Code formatting                         |
| **PHPUnit** / **Pest**                 | Testing                                 |
| **Rector**                             | Automated refactoring & upgrades        |
| **Xdebug** / **SPX**                   | Debugging & profiling                   |
| **OPcache**                            | Bytecode caching (production essential) |

## Key Rules

1. **`declare(strict_types=1)`** in every file.
2. **Type everything**: parameters, returns, properties.
3. **Use PHPStan at maximum level** in CI.
4. **Never use `@` error suppression** or `extract()`.
5. **Use prepared statements** for SQL (PDO or ORM). Never concatenate user input into queries.
6. **Autoload via Composer PSR-4.** No manual `require` chains.

---

_Sources: PHP documentation, PHP-FIG PSR Standards, Laravel/Symfony documentation, PHP: The Right Way, Modern PHP (Josh Lockhart)_
