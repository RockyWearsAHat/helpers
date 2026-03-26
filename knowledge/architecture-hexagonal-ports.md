# Hexagonal Architecture — Ports, Adapters, and Testing Isolation

Hexagonal architecture (also called ports and adapters) is a structural pattern for designing applications where the core business logic remains independent of external concerns. Rather than organizing code in strict layers, hexagon treats the domain as a central hexagon surrounded by adapters that translate between external systems and ports (interfaces) the domain provides.

## Core Concepts

### Ports

A **port** is an interface the domain defines to communicate with the outside world. Ports represent a contract of what the domain needs from external systems, not implementation details.

| Port Type | Description | Example |
|-----------|-------------|---------|
| **Driving/Primary** | Inbound—represents how the outside world drives the domain | HTTP controller, CLI, scheduled job |
| **Driven/Secondary** | Outbound—represents how the domain calls external systems | Database, email service, payment API |

The domain defines the port interface; the adapter provides the concrete implementation. This *inverts* the typical dependency: the adapter depends on the port, not the other way around.

```
Domain (hexagon):
  - PaymentProcessor (port interface)
  - InventoryManager (port interface)
  
Adapters (outside):
  - StripePaymentAdapter implements PaymentProcessor
  - PostgresInventoryAdapter implements InventoryManager
```

### Adapters

An **adapter** translates between external technology and the domain's port interface. Adapters are the outermost layer—they know about frameworks, protocols, and third-party libraries. The domain knows nothing about them.

**Key properties:**
- Each adapter implements exactly one port
- Adapters contain no business logic
- Multiple adapters can implement the same port (swap implementations easily)
- Adapters handle error translation, serialization, authentication, etc.

**Example: Two implementations of a Users port**

```python
# Domain port
class UserRepository(ABC):
    @abstractmethod
    def find_by_id(self, user_id: str) -> User | None: ...
    @abstractmethod
    def save(self, user: User) -> None: ...

# PostgreSQL adapter
class PostgresUserRepository(UserRepository):
    def __init__(self, connection_pool):
        self.pool = connection_pool
    
    def find_by_id(self, user_id: str) -> User | None:
        stmt = "SELECT * FROM users WHERE id = %s"
        result = self.pool.execute(stmt, (user_id,))
        if not result:
            return None
        return self._row_to_user(result[0])
    
    def save(self, user: User) -> None:
        stmt = "INSERT INTO users (...) VALUES (...) ON CONFLICT ..."
        self.pool.execute(stmt, (...))

# In-memory adapter (for testing)
class MemoryUserRepository(UserRepository):
    def __init__(self):
        self.users: dict[str, User] = {}
    
    def find_by_id(self, user_id: str) -> User | None:
        return self.users.get(user_id)
    
    def save(self, user: User) -> None:
        self.users[user.id] = user
```

The domain never knows which implementation is used. Testing uses the in-memory adapter; production uses the Postgres adapter.

## Dependency Rule

The **dependency rule** is the foundation of hexagonal architecture:

> Dependencies point inward. The domain has no imports from adapters or frameworks.

In practice:
- Domain code imports domain code only
- Adapters import domain ports and external libraries
- Domain never imports adapters or framework code

This creates a testable boundary: mock/replace any adapter without touching domain logic.

```
Violates dependency rule:
  class OrderService:
      def create_order(self, data) -> Order:
          user = requests.get(f"https://api.com/users/{data['user_id']}")
          # Depends on HTTP directly; untestable in isolation

Follows dependency rule:
  class OrderService:
      def __init__(self, user_provider: UserProvider):
          self.user_provider = user_provider
      
      def create_order(self, data) -> Order:
          user = self.user_provider.get(data['user_id'])
          # UserProvider is a port; inject any adapter
```

## Testing with Adapters

The entire value of hexagonal architecture for testing is the ability to replace production adapters with test doubles.

### Testing Patterns

**1. Unit Test with Mock Adapters**
```python
def test_order_creation_succeeds():
    user_repo = MemoryUserRepository()
    user_repo.save(User(id="123", name="Alice"))
    
    payment_processor = MockPaymentProcessor(always_succeed=True)
    order_service = OrderService(user_repo, payment_processor)
    
    order = order_service.create_order(user_id="123", amount=100)
    
    assert order.status == "completed"
    assert payment_processor.processed_amount == 100
```

**2. Integration Test with Real One Adapter**
```python
def test_order_persists_to_database(pg_connection):
    # Real database adapter
    order_repo = PostgresOrderRepository(pg_connection)
    
    # Mock payment (avoid real charges)
    payment_processor = MockPaymentProcessor(always_succeed=True)
    
    order_service = OrderService(order_repo, payment_processor)
    order = order_service.create_order(user_id="123", amount=100)
    
    # Verify it actually persisted
    retrieved = order_repo.find_by_id(order.id)
    assert retrieved.status == "completed"
```

**3. End-to-End with Test Container**
```python
def test_full_flow_with_testcontainers():
    # Real PostgreSQL in Docker
    with postgres_container() as db:
        order_repo = PostgresOrderRepository(db.connection)
        
        # Real or mock payment depending on test intent
        payment_processor = StripePaymentProcessor(api_key="test_key")
        
        order_service = OrderService(order_repo, payment_processor)
        # ... full scenario test
```

The key: swap adapters without rewriting tests. Core logic is tested once against each adapter type.

## Project Structure

Common structures for hexagonal projects:

**By Layer (Vertical Slices)**
```
src/
  domain/
    models/              # User, Order, Payment entities
      order.py
      user.py
    services/            # OrderService, UserService
      order_service.py
    ports/               # Interfaces
      order_repository.py
      payment_processor.py
  adapters/
    primary/             # Driving adapters
      http_api/
        order_controller.py
      cli/
        cli_interface.py
    secondary/           # Driven adapters
      postgres/
        order_repository.py
        user_repository.py
      stripe/
        payment_processor.py
      email/
        email_sender.py
  application/          # Dependency injection, bootstrapping
    container.py
```

**By Feature (Horizontal Slices)**
```
src/
  orders/
    models/
    services/
    ports/
  users/
    models/
    services/
    ports/
  adapters/
    primary/
      http_api/
        orders_controller.py
        users_controller.py
    secondary/
      postgres/
        orders_repository.py
        users_repository.py
```

Choose whichever keeps related code locatable. Vertical slices (by layer) make dependency direction clear; horizontal slices (by feature) make bounded contexts obvious.

## Comparison to Other Architectures

### vs. Clean Architecture

Clean architecture explicitly defines layers: Entities → Use Cases → Interface Adapters → Frameworks. Hexagonal is less prescriptive about internal layers—it only cares that the domain is isolated. Clean architecture is a specialized case of hexagonal.

### vs. Onion Architecture

Onion architecture organizes in concentric rings: Domain Model → Domain Services → Application Services → Infrastructure → Presentation. Hexagonal doesn't mandate rings; it focuses on directions (inward dependency). Onion is more prescriptive about layer count.

### vs. N-Layer Architecture

N-layer (Presentation → Business → Data) enforces horizontal layering. Hexagonal adds the key rule: no dependency cycles. N-layer often has Business depending on Data depending on Database; hexagonal inverts this (Database ports, not dependencies).

**The real advantage:** Hexagonal doesn't force you to put domain logic in a controller or service class. You're free to organize domain code however makes sense (models, aggregates, use cases, operations) as long as it stays isolated.

## Common Pitfalls

**1. Port Leakage — Technical Ports vs Business Ports**

Wrong:
```python
# HTTPRequest is a technical detail, not a business concept
class AuthService:
    def authenticate(self, request: HTTPRequest) -> AuthToken:
        # ...
```

Right:
```python
# What the business actually needs: credentials
class AuthService:
    def authenticate(self, username: str, password: str) -> AuthToken:
        # ...
```

Design ports around business capabilities, not frameworks.

**2. Too Many Adapters for One Thing**

Wrong:
```python
class PaymentAdapter:       # violates single responsibility
    def charge_with_stripe(self, ...): ...
    def charge_with_paypal(self, ...): ...
    def charge_with_bitcoin(self, ...): ...
```

Right:
```python
class PaymentProcessor(ABC):  # one port
    def charge(self, ...): ...

class StripePaymentAdapter(PaymentProcessor): ...
class PaypalPaymentAdapter(PaymentProcessor): ...
class BitcoinPaymentAdapter(PaymentProcessor): ...
```

One port, many adapters. Adapters are implementation choices.

**3. Huge Adapters with Business Logic**

Adapters should contain glue code only: serialization, error translation, retries. If you're writing conditionals and validation in an adapter, move it to the domain.

**4. Ignoring Adapter Composition**

Real systems have chains: HTTP request → deserializer → validator → domain service → query adapter → response serializer. Each step is a responsibility. Don't make one adapter do it all.

## When to Use Hexagonal Architecture

**Ideal for:**
- Systems with complex business logic worth protecting
- Applications with multiple external integrations (databases, APIs, message queues)
- Teams doing test-driven development
- Projects needing to swap implementations (e.g., monolith to microservices)

**Overkill for:**
- Simple CRUD apps with minimal logic
- Prototype/research code
- Single-person projects with no testing

The complexity of hexagonal architecture is justified by the testability and flexibility it provides. Don't pay the cost unless you're reaping the benefits.

## Implementation Checklist

- [ ] Define domain ports as interfaces/abstract classes
- [ ] Implement adapters without importing domain code into adapters (dependency inversion)
- [ ] Create test adapters for all driven ports
- [ ] Use dependency injection to wire adapters at application startup
- [ ] Write unit tests with mock adapters
- [ ] Write integration tests swapping one real adapter at a time
- [ ] Document which port each adapter implements
- [ ] Ensure all domain logic remains testable without running external systems

See also: [architecture-clean-hexagonal.md](architecture-clean-hexagonal.md), [patterns-dependency-injection.md](patterns-dependency-injection.md), [testing-philosophy.md](testing-philosophy.md)