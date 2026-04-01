# Repository & Unit of Work Patterns — Data Access Abstraction & Transaction Scope

The Repository and Unit of Work patterns abstract data access logic and manage transaction boundaries. They're part of clean architecture's data layer and are central to domain-driven design.

## Repository Pattern

**Intent:** Mediate between the domain and data mapping layers by presenting a collection-like interface for accessing domain objects.

### Core Concept

Instead of scattered SQL queries throughout the codebase, a Repository encapsulates data access. The application talks to the repository as if it were a collection:

```python
class UserRepository:
    def add(self, user: User):
        # INSERT logic
        pass
    
    def remove(self, user_id: UserId):
        # DELETE logic
        pass
    
    def find_by_id(self, id: UserId) -> User:
        # SELECT logic
        pass
    
    def find_all_active(self) -> list[User]:
        # SELECT with WHERE logic
        pass

# Usage
users = user_repository.find_all_active()
user_repository.remove(user.id)
```

### Push Back: Repository in CRUD Applications

The pattern assumes domain objects are rich (containing business logic) and persistence is secondary. In simple CRUD applications where the domain is thin, a Repository may add unnecessary abstraction. A thin DAO (Data Access Object) returning DTOs (Data Transfer Objects) might be simpler:

```python
# Simple CRUD: DAO returning DTOs
user_dto = user_dao.get_by_id(123)  # Returns dict or DTO

# Rich domain: Repository returning domain objects
user = user_repository.find_by_id(UserId(123))  # Returns domain User object
user.promote_to_admin()  # Business logic
user_repository.save(user)  # Persist changed state
```

Repositories assume the latter; DAOs assume the former.

## Repository vs. DAO

**DAO (Data Access Object)**
- Returns tables row-by-row, often as DTOs or maps
- Maps database structure directly
- Focus on data retrieval
- Often mirrors table names: `UserDAO`, `OrderDAO`

**Repository**
- Returns domain objects
- Encapsulates query logic relevant to the domain
- Focus on "what does the domain need?" not "what does the database store?"
- Named after domain concepts: `UserRepository`, `OrderRepository`

Example:

```python
# DAO approach: Low-level, structural
users_data = user_dao.find_where("age > :age", age=18)
# Returns [{"id": 1, "name": "Alice", ...}, ...]

# Repository approach: High-level, semantic
active_users = user_repository.find_all_adults()
# Returns [User(id=1, name=Alice, ...), ...]
```

## Specification Pattern

Repositories often separate query logic using Specifications:

```python
class Specification(ABC):
    @abstractmethod
    def is_satisfied_by(self, obj) -> bool:
        pass

class ActiveUserSpecification(Specification):
    def is_satisfied_by(self, user) -> bool:
        return user.is_active and user.verified

class UserRepository:
    def find_by_spec(self, spec: Specification) -> list[User]:
        # Convert specification to SQL WHERE clause
        # Or filter in memory if small dataset
        pass

# Usage
active = user_repository.find_by_spec(ActiveUserSpecification())
```

Benefits: Specifications are reusable, testable, and decouple query construction from persistence.

## Query Objects

When queries become complex, extract them into objects:

```python
class FindUsersQuery:
    def __init__(self, min_age: int = None, country: str = None, tier: str = None):
        self.min_age = min_age
        self.country = country
        self.tier = tier

class UserRepository:
    def find(self, query: FindUsersQuery) -> list[User]:
        # Build WHERE clause from query object
        # SELECT * FROM users WHERE ...
        pass

# Usage
premium_users_in_us = user_repository.find(
    FindUsersQuery(min_age=21, country="US", tier="premium")
)
```

Query objects make complex searches readable and reusable.

## Unit of Work Pattern

**Intent:** Maintain a list of objects affected by a business transaction and coordinate the writing of changes and the resolution of concurrency problems.

### Problem It Solves

Without Unit of Work:

```python
user = user_repository.find_by_id(1)
user.email = "new@example.com"
# Did we save? Did this persist?
```

Did calling `find_by_id()` fetch from database or cache? Is `email = "new@example.com"` tracked? When does it persist? These questions are murky.

### Unit of Work Solution

```python
unit_of_work = UnitOfWork()
user_repo = unit_of_work.users  # Repository via UoW

user = user_repo.find_by_id(1)
user.email = "new@example.com"

# Unit of Work tracks that user was modified
unit_of_work.commit()  # Persists all changes in one transaction
```

The Unit of Work tracks:
- **New objects** — Need INSERT
- **Dirty objects** — Modified and need UPDATE
- **Removed objects** — Need DELETE

Committing executes all queries in a single transaction.

### Transaction Boundary Management

```python
class UnitOfWork:
    def __init__(self, connection):
        self.connection = connection
        self.users = UserRepository(self)
        self.orders = OrderRepository(self)
        self._new_objects = []
        self._dirty_objects = []
        self._removed_objects = []
    
    def register_new(self, obj):
        self._new_objects.append(obj)
    
    def register_dirty(self, obj):
        if obj not in self._dirty_objects:
            self._dirty_objects.append(obj)
    
    def register_removed(self, obj):
        self._removed_objects.append(obj)
    
    def commit(self):
        try:
            self.connection.begin_transaction()
            self._insert_new()
            self._update_dirty()
            self._delete_removed()
            self.connection.commit()
        except Exception:
            self.connection.rollback()
            raise

# Usage
with UnitOfWork(connection) as uow:
    user = uow.users.find_by_id(1)
    order = uow.orders.find_by_id(100)
    user.email = "new@example.com"
    order.status = "shipped"
    uow.commit()  # Both persist atomically
```

## Generic Repository Pitfall

When teams create a generic `Repository<T>` class, they often aim for reusability but achieve fragile over-abstraction:

```python
# Problematic generic repository
class Repository(Generic[T]):
    def find_all(self) -> list[T]: ...
    def find_by_id(self, id) -> T: ...
    def save(self, obj: T) -> None: ...
    def delete(self, obj: T) -> None: ...

class UserRepository(Repository[User]):
    pass

# Problem: How do you express domain queries?
# users = user_repository.find_all_active_in_region("US")
# This doesn't fit the generic interface
```

The generic repository works for trivial CRUD but fails when domain logic is more nuanced. The fix: Accept that repositories are specific to their aggregates and encode domain queries directly:

```python
class UserRepository:
    def find_all(self) -> list[User]: ...
    def find_by_id(self, id: UserId) -> User: ...
    def find_all_active_in_region(self, region: str) -> list[User]:
        # Domain-specific query
        pass
    def save(self, user: User) -> None: ...
```

## Entity Framework and ORM Patterns

ORMs like Entity Framework blur the line between repositories and Unit of Work:

```csharp
// C# / Entity Framework
using (var context = new ApplicationDbContext())
{
    var user = context.Users.Find(1);
    user.Email = "new@example.com";
    context.SaveChanges();  // Implicit Unit of Work
}
```

The DbContext is both repository (provides `Users`, `Orders` collections) and Unit of Work (tracks changes, coordinates commit). Convenient for simple apps but can obscure transaction boundaries in complex flows.

### DbContext as Repository Anti-pattern

```csharp
// Bad: DbContext as repository, no clear transaction boundary
public UserService(ApplicationDbContext db) {
    this.db = db;
}

public void ProcessUser(int id) {
    var user = db.Users.Find(id);
    // ... multiple operations, unclear when persisted
    db.SaveChanges();  // Hidden transaction boundary
}
```

### Explicit Unit of Work Better

```csharp
// Better: Explicit unit of work
public UserService(IUnitOfWork unitOfWork) {
    this.unitOfWork = unitOfWork;
}

public void ProcessUser(int id) {
    var user = unitOfWork.Users.Find(id);
    // ...
    unitOfWork.Commit();  // Clear transaction boundary
}
```

## Repository in Clean Architecture

In Clean Architecture's data layer:

```
Domain Layer (innermost)
  ↓
Application Layer (use cases)
  ↓
Interface Adapters (repositories, controllers)
  ↓
Frameworks & Drivers (ORM, database)
```

The domain layer defines repository *interfaces*. The data layer implements them:

```python
# Domain layer: Interface only
class UserRepository(ABC):
    @abstractmethod
    def find_by_id(self, id: UserId) -> User: ...

# Data layer: Implementation
class SqlUserRepository(UserRepository):
    def find_by_id(self, id: UserId) -> User:
        # SQL query
        pass
```

This maintains clean unidirectional dependencies: domain knows nothing about SQL, UI, or infrastructure.

## Related Patterns

- **DAO** — Simpler, row-oriented. Use for thin CRUD applications.
- **Active Record** — Domain objects self-persist. Simpler than Repository but couples domain to persistence.
- **Data Mapper** — Separate mapper layer. Repository uses Data Mapper for persistence.
- **Query Object** — Encapsulates complex queries. Complements Repository.
- **Specification** — Adds reusable query criteria. Complements Repository.

## Anti-patterns and Pitfalls

**Repository as convenience method collection** — Adding methods to repository just because they're convenient (`find_by_name_and_age()`, `find_by_region_and_tier()`). This leads to proliferation and maintenance burden.

**Leaky repositories** — Repository reveals SQL or ORM details (returns raw SQL results, exposes join logic in the interface). Should hide persistence mechanics.

**Repository per table, not per aggregate** — Organizing repositories by database table rather than domain concept. Violates DDD principles.

**No transaction boundary** — Using repositories without explicit commit/rollback makes data consistency unclear.

**Generic repository as silver bullet** — Over-generalizing repositories into a generic `<T>` base class often backfires.

## Modern Perspectives

The Repository pattern remains relevant in domain-driven design and clean architecture. However:

- ORMs have become sophisticated enough that explicit repositories sometimes feel redundant (see Entity Framework DbContext).
- Some microservice architectures bypass repositories in favor of direct ORM usage or event sourcing.
- GraphQL and query languages sometimes make repository query methods less central (clients compose queries).

The pattern's core insight—*isolate data access concerns from domain logic*—remains valuable even as its implementation varies.