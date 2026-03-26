# Decorator & Middleware Patterns — Wrapping, Composition & Cross-Cutting Concerns

The Decorator and Middleware patterns solve related but distinct problems. Both use wrapping and composition to add behavior without modifying original objects, but Decorator operates on individual objects while Middleware operates on request/response pipelines.

## Decorator Pattern (Structural)

**Intent:** Attach additional responsibilities to an object dynamically. Decorators provide a flexible alternative to subclassing for extending functionality.

### Core Structure

**Component Interface** — Defines the contract. Both concrete components and decorators implement it.

**Concrete Component** — The base object being decorated. Provides core functionality.

**Decorator** — Wraps a component, implements the same interface, and delegates to it. Adds behavior before, after, or around the delegated call.

**Stacking:** Multiple decorators can wrap each other, forming a chain: `Decorator3(Decorator2(Decorator1(Component)))`.

### Example: Notification System

```python
class Notifier:
    def send(self, message: str):
        print(f"Basic notification: {message}")

class EmailDecorator(Notifier):
    def __init__(self, notifier):
        self.notifier = notifier
    
    def send(self, message):
        self.notifier.send(message)
        print(f"  + Email: {message}")

class SlackDecorator(Notifier):
    def __init__(self, notifier):
        self.notifier = notifier
    
    def send(self, message):
        self.notifier.send(message)
        print(f"  + Slack: {message}")

# Usage
notifier = Notifier()
notifier = EmailDecorator(notifier)
notifier = SlackDecorator(notifier)
notifier.send("Hello")  # Basic + Email + Slack
```

### Strengths

- **Single Responsibility:** Each decorator handles one concern.
- **Runtime composition:** Mix and match behaviors without creating exponential subclass hierarchies.
- **Flexible ordering:** Stack decorators in any order (though some orderings may be meaningless).
- **Open/Closed Principle:** New decorators added without modifying existing components.

### Weaknesses

- **Complexity:** Stack of decorators is harder to understand than a subclass.
- **Order dependency:** Decorator ordering sometimes matters (encryption then compression vs. compression then encryption produces different results).
- **Debugging difficulty:** Call stacks are deep; identity checks (instanceof) fail because actual type is wrapped.
- **Verbose creation code:** Building a decorated object requires multiple constructor calls.

## Middleware Pattern (Architectural)

Middleware operates on request/response pipelines, not individual objects. Each middleware layer wraps the next, intercepts requests/responses, and can modify or pass them through.

### HTTP Middleware (Express, Koa)

```javascript
// Express middleware
app.use((req, res, next) => {
    console.log(`${req.method} ${req.path}`);
    next();  // Pass to next middleware
});

app.use((req, res, next) => {
    if (!req.headers.authorization) {
        res.status(401).send('Unauthorized');
        return;
    }
    next();  // Pass to next middleware
});

app.get('/api/data', (req, res) => {
    res.json({ data: 'secret' });
});
```

Middleware stack execution:
```
Request -> Logging Middleware -> Auth Middleware -> Handler -> Response
          (logs request)       (checks header)
```

**Response pipeline is reversed:**
```
Response <- Logging Middleware <- Auth Middleware <- Handler
          (logs response)
```

### Koa's Upstream/Downstream Pattern

Koa makes the pipeline explicit:

```javascript
app.use(async (ctx, next) => {
    console.log('Request started');
    await next();  // Yields to downstream middleware
    console.log('Response finished');
});

app.use(async (ctx, next) => {
    const start = Date.now();
    await next();
    const duration = Date.now() - start;
    ctx.set('X-Response-Time', `${duration}ms`);
});

app.use(ctx => {
    ctx.body = 'Hello World';
});
```

This cascading pattern is more composable and transparent than Express's callback model.

### Django Middleware

Django middleware operates on request/response cycles but at the framework level:

```python
class LoggingMiddleware:
    def __init__(self, get_response):
        self.get_response = get_response
    
    def __call__(self, request):
        # Process request (upstream)
        logger.info(f"{request.method} {request.path}")
        
        response = self.get_response(request)
        
        # Process response (downstream)
        logger.info(f"Status {response.status_code}")
        return response
```

### ASP.NET Core Middleware

```csharp
app.Use(async (context, next) =>
{
    await next.Invoke();
    context.Response.Headers.Add("X-Custom-Header", "value");
});

app.UseAuthentication();
app.MapControllers();
```

## Function Composition and Decorators

In functional languages and modern JavaScript, decorators and middleware become function composition:

```python
def add_logging(f):
    def wrapper(*args, **kwargs):
        print(f"Calling {f.__name__}")
        result = f(*args, **kwargs)
        print(f"Result: {result}")
        return result
    return wrapper

@add_logging
def multiply(a, b):
    return a * b

multiply(3, 4)  # Logs before and after
```

Stacking decorators is composition:

```python
@add_logging
@add_timing
@add_caching
def expensive_operation():
    pass
```

**TypeScript Decorators (Experimental)**

```typescript
function WithLogging(target: any, propertyKey: string, descriptor: PropertyDescriptor) {
    const originalMethod = descriptor.value;
    descriptor.value = function(...args: any[]) {
        console.log(`Calling ${propertyKey}`);
        return originalMethod.apply(this, args);
    };
    return descriptor;
}

class DataService {
    @WithLogging
    fetchData() {
        return "data";
    }
}
```

## Cross-Cutting Concerns

Both patterns excel at addressing cross-cutting concerns—aspects of functionality that cut across multiple components:

- **Logging:** Every function/request should log? Add a decorator/middleware.
- **Authentication:** Protect certain endpoints? Add auth middleware.
- **Caching:** Memoize expensive computations? Decorator.
- **Rate limiting:** Throttle API endpoints? Middleware.
- **Error handling:** Wrap all handlers with try/catch? Middleware or decorator.
- **Compression:** Compress all HTTP responses? Middleware.

### Middleware for Concerns

Middleware is better when:
- The concern applies to all or most requests
- The concern needs to see the full request/response cycle
- Order matters and should be explicit

### Decorators for Concerns

Decorators are better when:
- The concern applies to specific objects/methods
- Fine-grained control is needed
- You're extending existing code without framework integration

## Comparison: Decorator vs. Middleware

| Aspect | Decorator | Middleware |
|--------|-----------|-----------|
| Scope | Individual objects | Request/response pipeline |
| Coupling | Wraps a component object | Wraps a handler/next layer |
| Stacking | Manual, in application code | Declarative, in framework config |
| Reversal | Must handle explicitly | Framework handles (upstream/downstream) |
| Best for | Single concerns on objects | Cross-cutting pipeline concerns |
| Example | Compression decorator on data | Compression middleware on HTTP |

## Pitfalls and Anti-patterns

**Over-decoration:** Too many layers of decorators make code hard to follow. A function with 10 decorators is unmaintainable.

**Order confusion:** Decorators applied in different order produce different results. Document order carefully.

**Middleware spaghetti:** Too many middleware layers with implicit dependencies. A bug in middleware #3 breaks middleware #7.

**No error propagation:** Middleware silently catches errors or decorators fail to propagate exceptions.

**Performance blindness:** Each layer adds overhead. Excessive wrapping causes noticeable latency.

**Lost type information:** Decorated functions may lose static type information or IDE hints.

## Related Patterns

- **Strategy:** Changes *algorithm selection*. Decorator adds *behavior*.
- **Adapter:** Converts one interface to another. Decorator preserves interface and adds behavior.
- **Proxy:** Controls access; Decorator adds responsibility. Proxy's interface often differs; Decorator's doesn't.
- **Observer:** Reacts to events. Decorator wraps objects.
- **Chain of Responsibility:** Handlers form a chain that can stop processing. Middleware always continues (via `next()`).

## Modern Usage

Decorators are ubiquitous in web frameworks (Spring, ASP.NET, NestJS) for route handlers. Middleware is fundamental to HTTP frameworks, event loops (Node.js, Python ASGI), and microservice architectures. Functional composition via decorators/wrappers dominates in Python (FastAPI), JavaScript (Next.js middleware), and functional languages.

The pattern's core insight—*separate core logic from cross-cutting concerns through wrapping*—remains as relevant as ever.