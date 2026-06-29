# C# Conventions and Idioms (.NET)

## Modern C# (12/13 — .NET 8/9)

C# has evolved dramatically. Modern C# is concise, performant, and expressive.

## Records & Immutability

```csharp
// Records — immutable reference types with value semantics
public record User(string Name, string Email, int Age);

// Auto-generates: constructor, Equals, GetHashCode, ToString, Deconstruct
var user = new User("Alice", "alice@test.com", 30);
var (name, email, age) = user;  // Deconstruction

// Non-destructive mutation (with-expressions)
var admin = user with { Name = "Admin" };

// Record structs (value types)
public readonly record struct Point(double X, double Y);

// Positional records with validation
public record Port(int Value)
{
    public Port
    {
        if (Value is < 0 or > 65535)
            throw new ArgumentOutOfRangeException(nameof(Value));
    }
}
```

## Pattern Matching

```csharp
// Switch expressions with patterns
string Classify(object obj) => obj switch
{
    int n when n > 0 => "positive",
    int n when n < 0 => "negative",
    int              => "zero",
    string s         => $"string of length {s.Length}",
    null             => "null",
    _                => $"unknown: {obj.GetType()}"
};

// Property patterns
decimal CalculateDiscount(Order order) => order switch
{
    { Total: > 1000, CustomerType: "premium" } => order.Total * 0.20m,
    { Total: > 500 }                           => order.Total * 0.10m,
    { CustomerType: "premium" }                => order.Total * 0.05m,
    _                                          => 0m
};

// List patterns (C# 11+)
string Describe(int[] arr) => arr switch
{
    []           => "empty",
    [var single] => $"one element: {single}",
    [var first, .., var last] => $"first: {first}, last: {last}"
};

// Relational patterns
string Grade(int score) => score switch
{
    >= 90 => "A",
    >= 80 => "B",
    >= 70 => "C",
    >= 60 => "D",
    _     => "F"
};
```

## Nullable Reference Types

```csharp
// Enable in project: <Nullable>enable</Nullable>
#nullable enable

string name = "Alice";      // Non-null — compiler guarantees it
string? nickname = null;     // Explicitly nullable

// Null-conditional + null-coalescing
int length = nickname?.Length ?? 0;
string display = nickname ?? "Anonymous";

// Null-forgiving (suppress warning when you know better)
string definitelyNotNull = GetValue()!;  // Use sparingly
```

## LINQ

```csharp
// Query syntax (SQL-like)
var adults = from u in users
             where u.Age >= 18
             orderby u.Name
             select new { u.Name, u.Email };

// Method syntax (preferred for most cases)
var adults = users
    .Where(u => u.Age >= 18)
    .OrderBy(u => u.Name)
    .Select(u => new { u.Name, u.Email });

// Common LINQ operations
users.First(u => u.Id == id);           // Throws if not found
users.FirstOrDefault(u => u.Id == id);  // Returns null/default
users.Any(u => u.Active);               // Boolean check
users.All(u => u.Verified);             // All match?
users.GroupBy(u => u.Role);             // Group by key
users.ToDictionary(u => u.Id);          // To dictionary
users.Distinct().ToList();              // Deduplicate
users.Chunk(100);                       // Batch (C# 11+)
users.DistinctBy(u => u.Email);         // Dedupe by property
```

## Async/Await

```csharp
// Async method
public async Task<User> GetUserAsync(string id, CancellationToken ct = default)
{
    var response = await _httpClient.GetAsync($"/users/{id}", ct);
    response.EnsureSuccessStatusCode();
    return await response.Content.ReadFromJsonAsync<User>(ct)
        ?? throw new InvalidOperationException("Null response");
}

// Parallel async
public async Task<(User, Config, List<Post>)> LoadDashboardAsync(string userId)
{
    var userTask = GetUserAsync(userId);
    var configTask = GetConfigAsync();
    var postsTask = GetPostsAsync(userId);

    await Task.WhenAll(userTask, configTask, postsTask);
    return (userTask.Result, configTask.Result, postsTask.Result);
}

// IAsyncEnumerable (async streams)
public async IAsyncEnumerable<User> GetUsersAsync(
    [EnumeratorCancellation] CancellationToken ct = default)
{
    await foreach (var batch in _db.StreamBatchesAsync(ct))
    {
        foreach (var user in batch)
            yield return user;
    }
}

// Always pass CancellationToken. Always use ConfigureAwait(false) in libraries.
```

## Dependency Injection (Built-in)

```csharp
// Registration
builder.Services.AddScoped<IUserRepository, UserRepository>();
builder.Services.AddSingleton<ICacheService, RedisCacheService>();
builder.Services.AddTransient<IEmailService, SmtpEmailService>();

// Constructor injection
public class UserService(IUserRepository repository, ILogger<UserService> logger)
{
    public async Task<User?> GetAsync(string id)
    {
        logger.LogInformation("Fetching user {UserId}", id);
        return await repository.FindByIdAsync(id);
    }
}

// Primary constructors (C# 12) — shown above. No need for field assignments.
```

## Primary Constructors (C# 12)

```csharp
public class UserService(IUserRepository repo, ILogger<UserService> logger)
{
    public Task<User?> GetAsync(string id) => repo.FindByIdAsync(id);
}

// Works with structs too
public struct Distance(double meters)
{
    public double Kilometers => meters / 1000;
    public double Miles => meters / 1609.344;
}
```

## Collection Expressions (C# 12)

```csharp
int[] numbers = [1, 2, 3, 4, 5];
List<string> names = ["Alice", "Bob"];
Span<byte> bytes = [0xFF, 0x00, 0xAB];

// Spread operator
int[] combined = [..first, ..second, 42];
```

## Raw String Literals (C# 11)

```csharp
string json = """
    {
        "name": "Alice",
        "age": 30
    }
    """;

// Interpolated raw strings
string query = $"""
    SELECT * FROM users
    WHERE name = '{name}'
    AND age > {minAge}
    """;
```

## Error Handling

```csharp
// Result pattern (instead of exceptions for expected failures)
public readonly record struct Result<T>
{
    public T? Value { get; }
    public string? Error { get; }
    public bool IsSuccess => Error is null;

    public static Result<T> Success(T value) => new() { Value = value };
    public static Result<T> Failure(string error) => new() { Error = error };
}

// Guard clauses with throw expressions
public void Process(string input) =>
    _ = input ?? throw new ArgumentNullException(nameof(input));
```

## Tooling

| Tool                      | Purpose                   |
| ------------------------- | ------------------------- |
| **dotnet format**         | Code formatting           |
| **Roslyn analyzers**      | Built-in static analysis  |
| **SonarAnalyzer**         | Deep static analysis      |
| **xUnit** / **NUnit**     | Testing                   |
| **Moq** / **NSubstitute** | Mocking                   |
| **BenchmarkDotNet**       | Performance benchmarking  |
| **Rider** / **VS**        | IDEs with deep C# support |

---

_Sources: C# Language Reference (Microsoft), .NET documentation, Effective C# (Bill Wagner), C# in Depth (Jon Skeet)_
