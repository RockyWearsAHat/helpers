# Contract Testing — Consumer-Driven Contracts & Verification Strategies

## The Contract Testing Premise

Microservices architectures distribute systems across independently deployable services. Integration tests that spin up all services become fragile and slow. Contract testing offers an alternative: verify that each service pair's interactions match expected contracts, in isolation.

A **contract** is a specification of how one service (consumer) expects another (provider) to behave. Examples: "GET /user/:id returns JSON with fields name, email"; "publish_order event has schema {...}".

**Consumer-driven contracts** shift ownership: consumers define what they need from providers. Providers must verify they satisfy consumer contracts before deployment. This prevents silent breakage: if a consumer depends on a field that a provider removes, the deployment fails immediately—detected by automated verification, not production failures.

## Consumer-Driven Contracts (CDC) Workflow

### Phase 1: Consumer Defines Expected Interactions

Consumer team writes tests defining how they call the provider:

```
Given: I need to fetch user profile
When: I GET /api/users/42
Then: Response status is 200
  AND response.name exists
  AND response.email exists
```

This is not a mock—it's a specification bundled with an executable example. The consumer generates a **contract artifact** (typically JSON or plain text) documenting the interaction.

### Phase 2: Publish Contract

The contract is stored in a shared repository or broker (Pact Broker, GitHub, artifact registry). Provider teams subscribe to this artifact.

### Phase 3: Provider Verifies Contract

Provider team retrieves the contract and writes a verification test:

```
Given: consumer expects GET /api/users/42 → 200 with name, email
When: provider's actual implementation handles this request
Then: contract is satisfied (assertion passes) or fails (provider must change code)
```

If provider can't satisfy the contract, they must:
- Negotiate with consumer (contract requires unreasonable behavior)
- Implement the contract
- Contact consumer if removal is necessary (rarely)

### Phase 4: Gate Deployments

- **Consumer deploy**: Can deploy if provider contract is satisfied (provider's verification passed)
- **Provider deploy**: Can deploy if all consumer contracts still pass

This prevents dependency hell: consumer can't deploy if provider won't support it, and provider can't deploy if it breaks consumers.

## Contract Types & Specification Formats

### Request-Response Contracts

HTTP synchronous calls. Consumer specifies expected request and response.

Most common for REST APIs; easiest to specify and verify.

### Message/Event Contracts

Asynchronous events published to queues or topics. Consumer specifies expected message schema and content.

Example: "inventory.reserved event includes order_id, item_ids, quantities."

Provider must ensure published events match this schema. More challenging than request-response because message brokers don't enforce schemas; mismatches surface when consumer deserializes.

### Schema-Based Contracts

Separate from behavioral tests; schemas act as machine-readable contracts:

- **OpenAPI/Swagger**: RESTful API contracts
- **AsyncAPI**: Async message contracts
- **gRPC proto**: Protocol buffer definitions
- **GraphQL schema**: Query/mutation signatures

Tools validate that implementations match schemas. Schema-first approach: define contract, generate code stubs, implement.

Advantage: contracts are data-only, language-agnostic. Disadvantage: schemas are passive; they don't capture complex state-dependent behavior or temporal constraints.

### Bidirectional Contracts

Some tools support contracts flowing both directions: consumer defines expectations, provider defines capabilities, tools verify both align. Advantages:
- Less negotiation; explicit mismatches surfaced early
- Detects breaking changes more reliably

Less commonly used; most teams use unidirectional (consumer-driven) flow.

## Contract Testing vs. Integration Testing

| Aspect | Contract Testing | Integration Testing |
|--------|------------------|-------------------|
| **Scope** | Two services in isolation (mocked boundary) | Full system of services running |
| **Speed** | Fast (mocks, no I/O) | Slow (real coordination, deployment) |
| **Brittleness** | Lower (focused on contract boundary) | Higher (many moving parts) |
| **Coverage** | Contract-specific behavior | End-to-end workflows |
| **Maintenance** | Contracts as first-class; changes negotiated | Tests often brittle; chasing system state |
| **Time to feedback** | Seconds | Minutes or hours |

**Best practice layering:**

```
Unit tests (single service)
    ↓
Contract tests (service pairs, mocked boundary)
    ↓
Smoke tests (all services, minimal scenarios)
    ↓
E2E tests (production-like workflows, rare)
```

Contract tests replace most integration tests. E2E tests are reserved for critical user journeys, not per-API-pair verification.

## Pact: Consumer-Driven Contract Testing

**Pact** is the de facto standard framework. Workflow:

1. **Consumer test** writes interactions (mocks provider)
2. **Consumer generates pact file** (JSON documenting all interactions)
3. **Pact Broker** hosts pact files
4. **Provider's CI** retrieves pact, verifies against actual implementation
5. **Deployment gates** enforce contract satisfaction

**Strengths:**
- Code-first (tests drive contract generation)
- Language-agnostic (pact files are standardized JSON)
- Mature tooling ecosystem; available for JVM, JavaScript, Python, Go, Ruby, .NET, etc.
- Pact Broker provides contract versioning and deployment tracking

**Gotchas:**
- Equivalent mutant problem: consumer tests pass but don't exercise all code paths in provider
- Requires discipline: consumers must write thorough tests, providers must verify thoroughly
- Pact tests don't catch network-level issues or middleware failures (auth, rate-limiting, etc.)

## Versioning & Breaking Changes

Contracts evolve. Managing breaking changes without cascading failures:

### Semantic Versioning of Contracts

- **Major**: Breaking changes (field removed, type changed)
- **Minor**: Backward-compatible additions (new field, new endpoint)
- **Patch**: Bug fixes

### Compatibility Strategies

**Additive fields (safe):**
Consumer doesn't break if provider adds fields. Provider can add optional fields without breaking consumers.

**Field removal (breaking):**
If consumer relies on a field, provider can't remove it without updating consumer first. Contract versioning tracks this; deployment gates prevent premature removal.

**Type changes (breaking):**
Changing uint to string breaks consumer deserialization. Requires either dual-support period (accept both types) or coordinated deployment.

**Best practice:**
- Prefer optional fields (nullable, with defaults)
- Use additive changes (new endpoints, new fields) for minor version bumps
- Deprecate before removal (add deprecation warning in contract; give timeline)
- Coordinate breaking changes via contract negotiation, not silent deployments

## Pact Broker & Deployment Coordination

**Pact Broker** is a central service storing pact files and metadata. Features:

- **Versioning**: Track pact evolution across service releases
- **Can-I-Deploy**: Query endpoint answering "is it safe to deploy X?" (returns true if all contracts verified)
- **Webhooks**: Trigger provider verification when consumer pacts change
- **Tags**: Mark release branches, environments (main, staging, production)

Workflow:
```
Consumer publishes pact tagged "main"
    ↓
Pact Broker notifies provider
    ↓
Provider CI runs verification; passes or fails
    ↓
Consumer queries can-i-deploy before merging; blocks if verification failed
```

Orchestrates the deployment decision without requiring all services to be running.

## Microservices Testing Pyramid with Contracts

Traditional test pyramid:

```
        / E2E \
       /-------\
      / Integ. \
     /---------\
    / Unit Tests \
   /-------------\
```

With contract tests:

```
           / E2E (rare) \
          /------------\
         / Contract tests \
        /----------------\
       /    Unit tests    \
      /-------------------\
```

E2E tests shrink dramatically because contract tests cover service-pair interactions comprehensively. E2E tests handle only critical user journeys and edge cases that span more than two services.

## Spring Points & Failure Modes

### Async Message Contracts are Hard

Event schemas don't capture temporal ordering or causality. "Event A published before Event B" can't be expressed in schema. Real failures: consumer assumes ordering, provider changes publish order. Solution: test temporal contracts explicitly or use choreography frameworks.

### Schema Drift

Consumer code and contract schema desync silently. Consumer expects field X but stops reading it; provider stops sending it. Both tests pass; production breaks when consumer actually uses field X. Mitigation: property-based testing on consumers; contract generation from code (schema-first approach less vulnerable).

### Incomplete Contract Coverage

High contract score but provider has unreachable code or untested path. Contract specifies happy path; error handling untested. Real failures appear when dependent services go down or return errors. Mitigation: include error scenarios in contracts; test both success and failure paths.

### Network & Middleware Outside Contract Scope

Contract tests mock the network. Real failures: TLS cert expired, rate-limiter rejects requests, load balancer misconfigured. Contracts don't catch these. Mitigation: add smoke tests with real network; contract tests + integration smoke tests.

## See Also

- [Testing Philosophy](testing-philosophy.md) — Test purposes and feedback loops
- [Testing Integration & E2E](testing-integration-e2e.md) — Relationship between test types
- [Architecture Microservices](architecture-microservices.md) — Service boundaries and coupling
- [Testing Advanced Patterns](testing-advanced-patterns.md) — Property-based testing, chaos engineering