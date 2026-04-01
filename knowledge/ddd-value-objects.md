# DDD Value Objects — Immutability, Equality, Validation & Type Safety

## What is a Value Object?

A **value object** represents a concept that is defined entirely by its value, not by identity. Two money values of $10 are equal; two Customer entities with the same name are different people.

**Core properties:**
- **Identity-less:** Equality based on content, not instance identity
- **Immutable:** Cannot change after creation
- **Self-validating:** Enforce invariants during construction
- **Type-safe:** Encode domain semantics (Money vs. int; Email vs. String)

```java
// Value object: two instances with same content are equal
Money m1 = Money.of(10, USD);
Money m2 = Money.of(10, USD);
assert m1.equals(m2);  // true; same value

// Entity: identity matters
Customer c1 = new Customer("Alice", 1L);
Customer c2 = new Customer("Alice", 1L);  // Different identity, despite same fields
assert !c1.equals(c2);  // false; different objects with different IDs
```

**Why value objects matter:**
- Domain clarity: model captures domain concepts, not just strings and numbers
- Type safety: "Money.of(10)" is obviously different from "10"
- Invariant enforcement: invalid values never exist (e.g., negative price)
- Immutability safety: pass value objects freely without worrying about modification

## Immutability

Value objects are immutable: once created, they cannot change. Operations produce new instances:

```java
public class Money {
  private final BigDecimal amount;
  private final Currency currency;
  
  public Money(BigDecimal amount, Currency currency) {
    if (amount.signum() < 0) {
      throw new InvalidMoneyException("Amount cannot be negative");
    }
    this.amount = amount;
    this.currency = currency;
  }
  
  // Operations return new instances
  public Money add(Money other) {
    if (!this.currency.equals(other.currency)) {
      throw new CurrencyMismatchException();
    }
    return new Money(this.amount.add(other.amount), this.currency);
  }
  
  public Money multiply(int factor) {
    return new Money(this.amount.multiply(BigDecimal.valueOf(factor)), this.currency);
  }
  
  // Getters expose immutable values
  public BigDecimal getAmount() { return amount; }
  public Currency getCurrency() { return currency; }
}

// Usage
Money price = Money.of(10, USD);
Money discounted = price.multiply(0.9);  // Returns new instance
assert price.equals(Money.of(10, USD));  // Original unchanged
```

**Consequences of immutability:**
- Thread-safe by design; no synchronization needed
- Can be cached/reused safely
- Predictable semantics; no hidden state changes
- Cost: more object allocations (typically negligible)

## Equality by Value

Two value objects are equal if their content is equal, regardless of reference identity:

```java
public class Email implements ValueObject<Email> {
  private final String address;
  
  public Email(String address) {
    if (!isValidEmail(address)) {
      throw new InvalidEmailException();
    }
    this.address = address;
  }
  
  @Override
  public boolean equals(Object other) {
    if (!(other instanceof Email)) return false;
    return this.address.equals(((Email) other).address);
  }
  
  @Override
  public int hashCode() {
    return address.hashCode();
  }
}

// Two instances, same value → equal
Email e1 = new Email("alice@example.com");
Email e2 = new Email("alice@example.com");
assert e1.equals(e2);  // true
assert e1 != e2;       // Different references, but value-equal
```

**Why not reference equality?** Reference identity is accidental; domain equality is semantic. "alice@example.com" is the same email address whether loaded from DB, received in a request, or constructed in code.

**Implement carefully:**

| Mistake                     | Problem                                          | Solution                           |
| ----- ---------------------- | ------------------------------------------------ | ---------------------------------- |
| **Omit hashCode()**         | Breaks HashMap/HashSet; unpredictable behavior  | Implement alongside equals()       |
| **Inconsistent hash/equals** | Objects equal but hash differently               | Derive hash from same fields       |
| **Use reference equality**  | `e1 == e2` fails for identical values            | Override equals() for value-based  |
| **Mutable fields in hash**  | Field changes after insertion; lost in HashMap   | Only hash immutable fields         |

## Self-Validating

Value objects enforce invariants during construction. Invalid states never exist:

```java
public class Quantity {
  private final int value;
  
  public Quantity(int value) {
    if (value <= 0) {
      throw new InvalidQuantityException("Quantity must be positive");
    }
    this.value = value;
  }
  
  public Quantity add(Quantity other) {
    return new Quantity(this.value + other.value);
  }
}

// Invariant is guaranteed
Quantity q = new Quantity(5);  // OK
Quantity invalid = new Quantity(-1);  // Exception; never created

// Type system prevents misuse
Quantity q1 = new Quantity(10);
int q2 = 10;
// q1 + q2;  // Compile error; prevents mixing Quantity with int
```

**Benefits:**
- Null checks unnecessary (`q.add(other)` always safe if q != null)
- No defensive programming at use sites
- Failures fast and obvious (at value construction, not later)

## Type-Safe Primitives

Encode domain semantics in types instead of primitive strings/numbers:

```java
// WITHOUT value objects: confusing, prone to errors
public void transferMoney(String from, String to, int amount) {
  // What currency? Is amount in cents or dollars?
  // What if someone passes negative amount?
}

// WITH value objects: clear and safe
public void transferMoney(AccountId from, AccountId to, Money amount) {
  // Semantics are explicit
  // Invalid values rejected at construction
}

// More examples
public class Email {
  private String address;
  public Email(String address) { /* validate */ }
}

public class Address {
  private String street, city, postalCode;
  public Address(String street, String city, String postalCode) { /* validate */ }
}

public class UserId {
  private final UUID id;
  public UserId(UUID id) {
    if (id == null) throw new NullPointerException();
    this.id = id;
  }
}

// API is self-documenting
public void sendWelcomeEmail(UserId user, Email email) { }
// vs.
public void sendWelcomeEmail(String userId, String email) { }
```

## Persistence Strategies

Value objects must be persisted; strategies depend on context.

**Strategy 1: Embedded in Entity (ORM)**

```java
@Entity
public class Customer {
  @Id private Long id;
  
  @Embedded  // Money is stored in same table
  private Money creditLimit;
  
  @Embedded
  private Email email;
}

// SQL: single customers table with currency, amount, email columns
```

**Strategy 2: Separate Table with Foreign Key**

```java
@Entity
public class Order {
  @Id private Long id;
  
  @OneToOne
  private OrderTotal orderTotal;  // Separate row in order_totals table
}

@Entity
@Table(name = "order_totals")
public class OrderTotal {
  @Id private Long id;
  private BigDecimal amount;
  private String currency;
}
```

**Strategy 3: Serialized (JSON/Protobuf)**

```java
@Entity
public class Order {
  @Id private Long id;
  
  @Convert(converter = MoneyConverter.class)  // Stored as JSON in DB
  private Money total;
}

public class MoneyConverter implements AttributeConverter<Money, String> {
  @Override
  public String convertToDatabaseColumn(Money money) {
    return JsonSerialize(money);  // {"amount": 100.00, "currency": "USD"}
  }
  
  @Override
  public Money convertToEntityAttribute(String dbData) {
    return JsonDeserialize(dbData);
  }
}
```

**Strategy 4: Domain Repository Reconstruction**

With event sourcing, value objects are reconstructed from events:

```java
Order order = new Order();
order.apply(new OrderCreated(
  orderId = "O1",
  total = Money.of(100, USD),
  timestamp = Instant.now()
));
// Value object (Money) reconstructed as part of aggregate replay
```

## Value Object Collections

Collections of value objects should themselves be immutable:

```java
public class ShoppingCart {
  private final List<CartItem> items;  // Unmodifiable
  
  public ShoppingCart(List<CartItem> items) {
    this.items = Collections.unmodifiableList(new ArrayList<>(items));
  }
  
  public ShoppingCart addItem(CartItem item) {
    List<CartItem> newItems = new ArrayList<>(items);
    newItems.add(item);
    return new ShoppingCart(newItems);  // Returns new instance
  }
  
  public List<CartItem> getItems() {
    return items;  // Safe; already unmodifiable
  }
}

// Usage
ShoppingCart cart1 = new ShoppingCart(List.of(item1, item2));
ShoppingCart cart2 = cart1.addItem(item3);  // New instance; cart1 unchanged
```

## Functional Core/Imperative Shell

Value objects form the **functional core**: pure functions, no side effects, deterministic:

```java
// Functional core (value objects only)
public Money applyDiscount(Money price, Discount discount) {
  BigDecimal factor = BigDecimal.ONE.subtract(discount.getRate());
  return price.multiply(factor);
}

// Imperative shell (mutable state, I/O)
public void processOrder(Order order) {
  Money discountedPrice = applyDiscount(order.getTotal(), order.getDiscount());
  order.setTotal(discountedPrice);  // Mutable
  repository.save(order);  // Side effect: persistence
  emailService.send(order);  // Side effect: sends email
}
```

This separation makes core logic testable and composable.

## Builder Pattern for Complex Value Objects

For value objects with many fields, use a builder:

```java
public class Address {
  private final String street;
  private final String city;
  private final String state;
  private final String postalCode;
  private final String country;
  
  private Address(Builder builder) {
    this.street = builder.street;
    this.city = builder.city;
    this.state = builder.state;
    this.postalCode = builder.postalCode;
    this.country = builder.country;
    validate();  // Invariants after construction
  }
  
  public static class Builder {
    private String street;
    private String city;
    private String state;
    private String postalCode;
    private String country;
    
    public Builder streetAddress(String street) { this.street = street; return this; }
    public Builder city(String city) { this.city = city; return this; }
    // ... other fields
    
    public Address build() {
      return new Address(this);
    }
  }
  
  private void validate() {
    if (street == null || city == null || postalCode == null) {
      throw new InvalidAddressException();
    }
  }
}

// Usage
Address addr = new Address.Builder()
  .streetAddress("123 Main St")
  .city("Springfield")
  .state("IL")
  .postalCode("62701")
  .country("USA")
  .build();
```

See also: [immutability patterns](paradigm-immutability.md), [aggregates](ddd-aggregate-design.md)