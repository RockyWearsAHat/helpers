# .NET Ecosystem — Application Frameworks

## Overview

The **.NET ecosystem** encompasses application frameworks, data access, real-time communication, and cloud-native tools. **ASP.NET Core** is the modern web server framework (replacing ASP.NET Framework 4.x). **Entity Framework Core** is the ORM. **Blazor** enables C# in the browser via WebAssembly. **SignalR** provides real-time bidirectional communication. Collectively, they form an opinionated, integrated stack for building full-stack applications in C#.

## ASP.NET Core — Web Application Framework

**ASP.NET Core** is a lightweight, cross-platform web framework. Applications are built as middleware pipelines.

### Middleware Pipeline

```csharp
// Program.cs (what was Startup.cs)
var builder = WebApplication.CreateBuilder(args);

// Register services (dependency injection)
builder.Services.AddScoped<OrderService>();
builder.Services.AddSingleton<LoggingService>();
builder.Services.AddControllers();
builder.Services.AddCors();

var app = builder.Build();

// Build middleware pipeline
if (app.Environment.IsDevelopment()) {
    app.UseDeveloperExceptionPage();
}

app.UseHttpsRedirection();
app.UseCors("AllowAllOrigins");
app.UseRouting();
app.UseAuthentication();
app.UseAuthorization();

app.MapControllers(); // Wire up controller routes
app.MapGet("/health", () => "OK"); // Minimal API endpoint

app.Run();
```

Middleware executes in order:

```
Request → UseCors → UseAuthentication → UseAuthorization → MapControllers → Response
```

Each middleware wraps the next, forming a chain. Custom middleware:

```csharp
app.Use(async (context, next) => {
    // Before request
    context.Response.Headers.Add("X-Custom", "value");
    
    await next(); // call next middleware
    
    // After response
    context.Response.StatusCode = 200;
});
```

### Dependency Injection

ASP.NET Core has built-in DI. Services are registered in `Program.cs`:

```csharp
builder.Services.AddScoped<IOrderRepository, OrderRepository>();      // New per request
builder.Services.AddSingleton<IConfigService, ConfigService>();       // One instance entire app
builder.Services.AddTransient<IMailService, MailService>();           // New every time

// Construct controllers automatically; dependencies injected
[ApiController]
[Route("api/[controller]")]
public class OrdersController : ControllerBase {
    private readonly IOrderRepository _repo;
    private readonly IOrderService _svc;
    
    public OrdersController(IOrderRepository repo, IOrderService svc) {
        _repo = repo;
        _svc = svc;
    }
}
```

## Minimal APIs

**Minimal APIs** reduce boilerplate for simple endpoints:

```csharp
app.MapGet("/api/users/{id}", 
    async (int id, OrderContext db) => 
        await db.Orders.FindAsync(id))
    .WithName("GetOrder")
    .WithOpenApi();

app.MapPost("/api/orders", 
    async (Order order, OrderContext db) => {
        db.Orders.Add(order);
        await db.SaveChangesAsync();
        return Results.Created($"/api/orders/{order.Id}", order);
    })
    .Produces<Order>(StatusCodes.Status201Created)
    .WithName("CreateOrder")
    .WithOpenApi();
```

Replaces verbose controller classes for simple CRUD operations. Generates OpenAPI (Swagger) documentation automatically.

## Entity Framework Core — ORM & Migrations

**Entity Framework Core** (EF Core) is the modern ORM for .NET, supporting relational and NoSQL databases.

### DbContext & Entities

```csharp
public class User {
    public int Id { get; set; }
    public string Email { get; set; }
    public ICollection<Order> Orders { get; set; }
    public DateTime CreatedAt { get; set; }
}

public class OrderContext : DbContext {
    public DbSet<User> Users { get; set; }
    public DbSet<Order> Orders { get; set; }
    
    protected override void OnConfiguring(DbContextOptionsBuilder options) {
        options.UseSqlServer("Server=localhost;Database=mydb;");
    }
    
    protected override void OnModelCreating(ModelBuilder modelBuilder) {
        modelBuilder.Entity<User>()
            .HasMany(u => u.Orders)
            .WithOne(o => o.User)
            .HasForeignKey(o => o.UserId);
        
        modelBuilder.Entity<User>()
            .HasIndex(u => u.Email)
            .IsUnique();
    }
}
```

### LINQ Queries

```csharp
// LINQ-to-SQL: becomes a WHERE clause
var users = db.Users
    .Where(u => u.Email.Contains("@company.com"))
    .OrderBy(u => u.CreatedAt)
    .Take(10)
    .ToList();

// Compiled queries (performance)
var query = EF.CompileAsyncQuery((OrderContext db, string email) =>
    db.Users.Where(u => u.Email == email).First()
);
var user = await query(db, "alice@test.com");

// Projections (partial data, efficient)
var userEmails = db.Users
    .Select(u => new { u.Id, u.Email })
    .ToList();

// Includes (eager loading to avoid N+1)
var usersWithOrders = db.Users
    .Include(u => u.Orders)
    .Where(u => u.Orders.Count > 0)
    .ToList();
```

### Migrations

Schema changes are version-controlled:

```bash
dotnet ef migrations add AddUserCreatedAt
dotnet ef database update
```

```csharp
// Generated migration: 20240101000000_AddUserCreatedAt.cs
public partial class AddUserCreatedAt : Migration {
    protected override void Up(MigrationBuilder migrationBuilder) {
        migrationBuilder.AddColumn<DateTime>(
            name: "CreatedAt",
            table: "Users",
            type: "datetime2",
            nullable: false,
            defaultValue: new DateTime(2024, 1, 1, 0, 0, 0, 0, DateTimeKind.Utc));
    }
    
    protected override void Down(MigrationBuilder migrationBuilder) {
        migrationBuilder.DropColumn(name: "CreatedAt", table: "Users");
    }
}
```

EF Core supports SQL Server, PostgreSQL, SQLite, Cosmos DB, and others.

## Blazor — UI Framework (C# in Browser)

**Blazor** lets you write interactive user interfaces in C#, running in the browser via WebAssembly or on the server via SignalR.

### Blazor Server

Executes component logic on the server; WebSocket connection syncs UI state to browser.

```csharp
// Pages/Counter.razor
@page "/counter"
@rendermode InteractiveServer

<h1>Counter</h1>
<p>Current count: @count</p>
<button class="btn btn-primary" @onclick="IncrementCount">Click me</button>

@code {
    private int count = 0;
    
    private void IncrementCount() {
        count++;
    }
}
```

**Advantages**: Full .NET available (no WASM size limit), no separate JavaScript needed.
**Disadvantages**: Requires persistent connection, higher latency, server-side resource usage.

### Blazor WebAssembly

Compiles C# to WebAssembly; runs entirely in browser.

```csharp
// Same .razor component
// But uses @rendermode InteractiveWebAssembly

// Calls server API
@inject HttpClient http

@code {
    private async Task LoadData() {
        data = await http.GetFromJsonAsync<Data>("/api/data");
    }
}
```

**Advantages**: No server needed for UI logic, can work offline, traditional SPA.
**Disadvantages**: Large .NET runtime download, secrets can't be stored (client-side code), startup time.

### Hybrid Rendering

Blazor can mix server and WebAssembly components in the same app.

## SignalR — Real-Time Communication

**SignalR** enables server → client push notifications and bidirectional RPC.

```csharp
public class ChatHub : Hub {
    public async Task SendMessage(string user, string message) {
        await Clients.All.SendAsync("ReceiveMessage", user, message);
    }
    
    public override async Task OnConnectedAsync() {
        await Clients.All.SendAsync("UserJoined", Context.User.Identity.Name);
        await base.OnConnectedAsync();
    }
}

// Startup
app.MapHub<ChatHub>("/chatHub");
```

```csharp
// Client (JavaScript)
const connection = new signalR.HubConnectionBuilder()
    .withUrl("/chatHub")
    .withAutomaticReconnect()
    .build();

connection.on("ReceiveMessage", (user, message) => {
    console.log(`${user}: ${message}`);
});

connection.start();
connection.invoke("SendMessage", "Alice", "Hello!");
```

SignalR handles reconnection, scaling, and backplane for multi-server deployments.

## .NET MAUI — Cross-Platform Desktop & Mobile

**.NET Multi-platform App UI (MAUI)** is a framework for building Windows, macOS, iOS, and Android apps from shared C# code.

```csharp
public partial class MainPage : ContentPage {
    public MainPage() {
        InitializeComponent();
    }
    
    private void OnCounterClicked(object sender, EventArgs e) {
        CounterLabel.Text = $"Current count: {count}";
    }
}

// XAML UI
<ContentPage
    xmlns="http://schemas.microsoft.com/dotnet/2021/maui"
    Title="My App">
    <VerticalStackLayout Padding="30" Spacing="10">
        <Label Text="Welcome!" FontSize="32" />
        <Button Text="Click Me" Clicked="OnCounterClicked" />
        <Label x:Name="CounterLabel" />
    </VerticalStackLayout>
</ContentPage>
```

## Aspire — Cloud-Native Development

**.NET Aspire** is an opinionated framework for building resilient, observable, cloud-native applications. It provides a project template with service discovery, configuration, health checks, logging, and observability wired in.

```csharp
// AppHost project (orchestrates services)
var builder = DistributedApplication.CreateBuilder(args);

var postgres = builder.AddPostgres("postgres");
var api = builder.AddProject<Projects.Api>("api")
    .WithReference(postgres);

var web = builder.AddProject<Projects.Web>("web")
    .WithReference(api);

builder.Build().Run();
```

Generates `.NET Aspire dashboards` for observability: metrics, logs, traces, resources in one UI.

## AOT Compilation

**.NET now supports Ahead-Of-Time (AOT) compilation** via NativeAOT, creating standalone executables without the .NET runtime.

```bash
dotnet publish -c Release --self-contained -r win-x64 /p:PublishAot=true
# Creates myapp.exe (single file, no .NET needed)
```

**Benefits**: Fast startup (milliseconds), smaller footprint, deployable for IoT/serverless.
**Tradeoffs**: Trimming and AOT-friendly code required; reflection must be explicit; smaller ecosystem.

## Ecosystem Cohesion & Tradeoffs

The .NET ecosystem is particularly well-integrated:

- **Language + runtime**: C# is strongly integrated with the runtime; language features (records, nullable reference types, pattern matching) support framework idioms
- **DI + ORM + Web**: ASP.NET Core DI, EF Core, Minimal APIs compose seamlessly; less glue code than Java
- **Real-time**: SignalR is first-class; fewer third-party dependencies for websockets
- **Unified platform**: Windows, Linux, macOS, WebAssembly, iOS, Android from one toolchain

Tradeoffs:
- **Platform lock-in perception**: Microsoft-controlled ecosystem, though open-source. Smaller than Java/JavaScript ecosystems.
- **Ecosystem maturity**: Smaller library collection than Java or Node.js.
- **Learning curve**: C# syntax and async/await patterns take mastery; concepts like async, Span<T>, records are powerful but dense.

## Notable Omissions vs. Java Spring

ASP.NET Core doesn't have direct equivalents for:

- **Distributed systems patterns**: No built-in service discovery or circuit breakers (community libraries exist)
- **Microservices maturity**: Spring Cloud is more mature than equivalent .NET libraries
- **Configuration servers**: No Consul/etcd equivalents; must use third-party tools

Conversely, ASP.NET Core leads in:

- **Development velocity**: Minimal APIs and hot reload reduce friction
- **Type system**: Nullable reference types, records, and discriminated unions prevent entire classes of bugs
- **Async-first**: Async is ingrained in the framework; better default than Spring's request/thread model

**See also**: language-csharp, runtime-dotnet, api-rest-maturity, database-sql-fundamentals, observability-distributed-tracing, cloud-orchestration-containers