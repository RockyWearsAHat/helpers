# Clean Architecture / Hexagonal / Ports & Adapters

## The Core Principle

**Dependencies point inward.** Outer layers depend on inner layers. Inner layers know nothing about outer layers. Business logic never imports from frameworks, databases, or HTTP libraries.

```
┌─────────────────────────────────────────┐
│  Frameworks & Drivers (DB, Web, UI)     │
│  ┌─────────────────────────────────┐    │
│  │  Interface Adapters (Controllers,│    │
│  │  Gateways, Presenters)          │    │
│  │  ┌─────────────────────────┐    │    │
│  │  │  Application (Use Cases) │    │    │
│  │  │  ┌─────────────────┐    │    │    │
│  │  │  │  Entities       │    │    │    │
│  │  │  │  (Domain Logic) │    │    │    │
│  │  │  └─────────────────┘    │    │    │
│  │  └─────────────────────────┘    │    │
│  └─────────────────────────────────┘    │
└─────────────────────────────────────────┘
```

## Clean Architecture Layers

### Entities (Domain Layer)

Enterprise business rules. Pure domain objects with no framework dependencies. Could be shared across multiple applications.

```python
class Account:
    def __init__(self, account_id: AccountId, balance: Money):
        self._id = account_id
        self._balance = balance

    def withdraw(self, amount: Money) -> list[DomainEvent]:
        if amount > self._balance:
            raise InsufficientFundsError(self._id, amount, self._balance)
        self._balance = self._balance.subtract(amount)
        return [FundsWithdrawn(self._id, amount)]
```

No SQLAlchemy models. No Pydantic schemas. No framework annotations. Pure Python (or Java, TypeScript, etc.).

### Use Cases (Application Layer)

Application-specific business rules. Orchestrate entities and call ports. Each use case is a single operation.

```python
class TransferFundsUseCase:
    def __init__(self, account_repo: AccountRepository, event_bus: EventBus):
        self._account_repo = account_repo  # Port (interface)
        self._event_bus = event_bus        # Port (interface)

    def execute(self, command: TransferFunds) -> TransferResult:
        source = self._account_repo.find_by_id(command.source_id)
        target = self._account_repo.find_by_id(command.target_id)

        events = source.withdraw(command.amount)
        events += target.deposit(command.amount)

        self._account_repo.save(source)
        self._account_repo.save(target)

        for event in events:
            self._event_bus.publish(event)

        return TransferResult(success=True)
```

### Interface Adapters

Convert data between use case format and external format. Controllers, presenters, gateways.

```python
# HTTP Controller (adapter for primary/driving port)
class AccountController:
    def __init__(self, transfer_use_case: TransferFundsUseCase):
        self._transfer = transfer_use_case

    def post_transfer(self, request: HttpRequest) -> HttpResponse:
        command = TransferFunds(
            source_id=AccountId(request.json["from"]),
            target_id=AccountId(request.json["to"]),
            amount=Money(Decimal(request.json["amount"]), request.json["currency"]),
        )
        result = self._transfer.execute(command)
        return HttpResponse(200, {"status": "ok"})
```

### Frameworks & Drivers

The outermost layer: database drivers, web frameworks, message broker clients. Configuration and wiring.

## Hexagonal Architecture (Ports & Adapters)

### Ports

Interfaces defined by the application. Two types:

**Primary (Driving) Ports**: How the outside world talks to the application. Defined as use case interfaces.

```python
class TransferFunds(Protocol):
    """Primary port — driven by external actors (UI, API, CLI)."""
    def execute(self, command: TransferFundsCommand) -> TransferResult: ...
```

**Secondary (Driven) Ports**: How the application talks to the outside world. Defined as repository/service interfaces.

```python
class AccountRepository(Protocol):
    """Secondary port — application drives this to access data."""
    def find_by_id(self, account_id: AccountId) -> Account | None: ...
    def save(self, account: Account) -> None: ...

class EventBus(Protocol):
    """Secondary port — application drives this to publish events."""
    def publish(self, event: DomainEvent) -> None: ...
```

### Adapters

Concrete implementations of ports:

```python
# Primary adapter: REST API
class FastAPIAccountAdapter:
    def __init__(self, transfer_port: TransferFunds):
        self._transfer = transfer_port

    @app.post("/transfers")
    def create_transfer(self, body: TransferRequest):
        return self._transfer.execute(body.to_command())

# Primary adapter: CLI
class CLIAccountAdapter:
    def __init__(self, transfer_port: TransferFunds):
        self._transfer = transfer_port

    def run(self, args):
        command = TransferFundsCommand(args.source, args.target, args.amount)
        self._transfer.execute(command)

# Secondary adapter: PostgreSQL
class PostgresAccountRepository:
    def __init__(self, session: Session):
        self._session = session

    def find_by_id(self, account_id: AccountId) -> Account | None:
        row = self._session.query(AccountRow).get(str(account_id))
        return self._to_domain(row) if row else None

# Secondary adapter: In-memory (for testing)
class InMemoryAccountRepository:
    def __init__(self):
        self._accounts: dict[AccountId, Account] = {}

    def find_by_id(self, account_id: AccountId) -> Account | None:
        return self._accounts.get(account_id)
```

### Hexagonal Diagram

```
┌─────────────────────────────────────────────────┐
│                                                 │
│  REST ──→ [Primary Port] ──→ Application ──→ [Secondary Port] ──→ PostgreSQL
│  CLI  ──→ [Primary Port] ──↗   Core      ──→ [Secondary Port] ──→ Redis
│  gRPC ──→ [Primary Port] ──↗             ──→ [Secondary Port] ──→ Kafka
│                                                 │
│            Driving Side        Domain        Driven Side
│            (left)                             (right)
└─────────────────────────────────────────────────┘
```

## Dependency Injection

The glue that connects ports to adapters. Wiring happens at the composition root (application startup).

```python
# Composition root — the ONLY place that knows all concrete implementations
def create_app():
    # Infrastructure
    db_session = create_session(DATABASE_URL)
    kafka_producer = create_producer(KAFKA_BROKERS)

    # Secondary adapters
    account_repo = PostgresAccountRepository(db_session)
    event_bus = KafkaEventBus(kafka_producer)

    # Use cases (wired with ports)
    transfer_use_case = TransferFundsUseCase(account_repo, event_bus)

    # Primary adapters
    controller = AccountController(transfer_use_case)

    return FastAPI(routes=[controller.routes])
```

## Comparison: Clean vs Hexagonal vs Onion

| Aspect      | Clean Architecture                          | Hexagonal                                         | Onion                                                |
| ----------- | ------------------------------------------- | ------------------------------------------------- | ---------------------------------------------------- |
| Origin      | Robert C. Martin (2012)                     | Alistair Cockburn (2005)                          | Jeffrey Palermo (2008)                               |
| Core idea   | Dependency rule (inward)                    | Ports and adapters                                | Layers around domain core                            |
| Layers      | Entities, Use Cases, Adapters, Frameworks   | Application core + Adapters                       | Domain, Domain Services, Application, Infrastructure |
| Emphasis    | Separation of concerns, testability         | Symmetry (driving = driven), swappable adapters   | Domain model at the center                           |
| Key insight | Details depend on policies, not the reverse | Application has distinct driving and driven sides | Infrastructure is an outer concern                   |

They're more similar than different. All enforce: domain at the center, dependencies point inward, infrastructure is pluggable.

## Practical Implementation

### Project Structure (Python)

```
src/
  domain/                    # Entities, value objects, domain events, domain services
    account.py
    money.py
    events.py
  application/               # Use cases, port interfaces
    ports/
      account_repository.py  # Protocol/ABC
      event_bus.py           # Protocol/ABC
    transfer_funds.py        # Use case
  infrastructure/            # Adapters
    persistence/
      postgres_account_repo.py
      in_memory_account_repo.py
    messaging/
      kafka_event_bus.py
    web/
      fastapi_controller.py
  main.py                    # Composition root
```

### Project Structure (TypeScript)

```
src/
  domain/
    entities/Account.ts
    value-objects/Money.ts
    events/FundsWithdrawn.ts
  application/
    ports/AccountRepository.ts    # Interface
    ports/EventBus.ts             # Interface
    use-cases/TransferFunds.ts
  infrastructure/
    persistence/PrismaAccountRepo.ts
    messaging/SQSEventBus.ts
    web/ExpressController.ts
  main.ts
```

## Testing Strategy

The primary benefit: domain logic tested in complete isolation.

| Layer       | Test Type             | Dependencies                         | Speed   |
| ----------- | --------------------- | ------------------------------------ | ------- |
| Domain      | Unit tests            | None (pure logic)                    | Instant |
| Application | Unit tests with mocks | Mocked ports                         | Instant |
| Adapters    | Integration tests     | Real infrastructure (Testcontainers) | Slower  |
| Full system | E2E tests             | Everything real                      | Slowest |

```python
# Domain test — zero dependencies
def test_withdraw_insufficient_funds():
    account = Account(AccountId("1"), Money(50, "USD"))
    with pytest.raises(InsufficientFundsError):
        account.withdraw(Money(100, "USD"))

# Use case test — mocked ports
def test_transfer_funds():
    repo = InMemoryAccountRepository()
    bus = FakeEventBus()
    repo.save(Account(AccountId("src"), Money(200, "USD")))
    repo.save(Account(AccountId("dst"), Money(50, "USD")))

    use_case = TransferFundsUseCase(repo, bus)
    result = use_case.execute(TransferFunds("src", "dst", Money(100, "USD")))

    assert result.success
    assert repo.find_by_id(AccountId("src")).balance == Money(100, "USD")
    assert repo.find_by_id(AccountId("dst")).balance == Money(150, "USD")
    assert len(bus.published) == 2
```

## When to Use (and When Not To)

### Use When

- Domain logic is complex and the core business differentiator
- Multiple entry points (web, CLI, message consumers, scheduled jobs)
- Long-lived application that will evolve significantly
- Need to swap infrastructure (change databases, message brokers)
- Team size warrants clear boundaries

### Overengineering When

- Simple CRUD with little domain logic — a framework-native approach is fine
- Prototypes and MVPs — move fast, refactor later if it survives
- Scripts and small tools — not every Python script needs hexagonal architecture
- Team of one on a small project — the indirection isn't worth it

**Start simple. Extract ports/adapters when you feel the pain of coupling, not before.**
