# Engineering Judgment — The Taste That Separates Good From Great

Raw knowledge is necessary but not sufficient. What makes a great engineer is _judgment_ — knowing which knowledge to apply, when, and how much. This document isn't about what tools exist. It's about how to choose.

---

## Tradeoff Thinking

### There Are No Best Practices, Only Tradeoffs

Every "best practice" is actually "best under certain assumptions." When the assumptions change, the practice is wrong.

```
"Always use microservices"
  → True assumption: large team, independent deployment needed
  → False assumption: 3-person startup, single product
  → Judgment: Monolith first. Extract services when team/scale demands it.

"Always write tests"
  → True assumption: long-lived code, team collaboration, complex logic
  → False assumption: throwaway prototype, exploring feasibility
  → Judgment: Skip tests for spikes. Write them when you keep the code.

"Never use global state"
  → True assumption: concurrent code, large codebase, many contributors
  → False assumption: single-threaded CLI tool, 200 lines
  → Judgment: A module-level config dict is fine for a small script.
```

**The skill:** Identify the hidden assumptions, evaluate whether they hold in YOUR context, then decide.

### The Five Tensions

Every engineering decision involves tension between competing values. You can't maximize all of them. The art is knowing which to prioritize for THIS project, at THIS stage.

```
                    SIMPLICITY
                        ▲
                        │
        FLEXIBILITY ◄───┼───► PERFORMANCE
                        │
                        ▼
                    CORRECTNESS ◄───► SPEED OF DELIVERY
```

| Tension                     | Left Extreme                           | Right Extreme                     | Healthy Middle                                                      |
| --------------------------- | -------------------------------------- | --------------------------------- | ------------------------------------------------------------------- |
| Simplicity vs Flexibility   | Rigid, one-use code                    | Abstract framework for everything | Simple code with extension points only where proven needed          |
| Correctness vs Speed        | Ship nothing, perfect                  | Ship broken, fast                 | Ship correct core, accept rough edges in non-critical paths         |
| Performance vs Readability  | Unreadable but fast                    | Clear but slow                    | Readable code with targeted optimization where profiling shows need |
| DRY vs Clarity              | One function for 5 different use cases | Copy-paste everywhere             | Deduplicate when the _concept_ is the same, not just the characters |
| Abstraction vs Concreteness | Everything is an interface             | Everything is hardcoded           | Abstract at proven variation points                                 |

### How to Make Technical Decisions

**Step 1: Name the constraints.** What can't change? Budget, timeline, team size, existing systems, compliance requirements.

**Step 2: Name the unknowns.** What don't you know yet? User behavior, traffic patterns, data volume, integration requirements.

**Step 3: Make unknowns smaller.** Can you prototype? Can you measure? Can you ask someone who's done it? Spend a day answering unknowns before committing to a direction.

**Step 4: Choose the most reversible option.** When uncertain, pick the path that's easiest to change. Monorepo vs multirepo? Start with monorepo — easier to split later. Microservices vs monolith? Start with monolith — easier to extract later.

**Step 5: Document the WHY.** The decision itself will be obvious in the code. What won't be obvious is why you chose it over alternatives. A 5-line comment or ADR (Architecture Decision Record) saves weeks of future re-debating.

```markdown
## ADR-0003: Use PostgreSQL instead of MongoDB

**Status:** Accepted
**Context:** We need a database for order management. Data is relational (orders → items → products). Team knows SQL.
**Decision:** PostgreSQL with JSONB columns for flexible attributes.
**Alternatives considered:**

- MongoDB: More flexible schema but we'd lose ACID transactions across orders/payments. Team has no MongoDB experience.
- MySQL: Viable but PostgreSQL JSONB gives us schema flexibility without sacrificing SQL.
  **Consequences:** We need a migration strategy. Schema changes require migrations (acceptable given data is relational).
```

---

## Taste in Code — What "Good" Looks Like

### The Readability Principle

Code is read 10x more often than it is written. Optimize for the reader.

**The reader is future-you, six months from now**, having forgotten everything about this project. Or a teammate seeing this code for the first time. Every decision you make should ask: "Will the reader understand why?"

```python
# Clever but hostile to readers:
result = {k: v for d in [defaults, overrides] for k, v in d.items() if v is not None}

# Clear and kind to readers:
result = {**defaults}
for key, value in overrides.items():
    if value is not None:
        result[key] = value
```

Both work. The second one doesn't require the reader to pause and decode a nested comprehension. That pause, multiplied across a codebase, is enormous.

### Naming Is Design

A name that requires a comment is a failed name. A name that describes the _intent_ (not the implementation) is a good name.

```python
# Implementation-focused names
def process(d):        # process what? d is what?
    temp = []
    for x in d:
        if x['a'] > 0:
            temp.append(x)
    return temp

# Intent-focused names
def filter_active_users(users):
    return [user for user in users if user['login_count'] > 0]
```

**Naming heuristics:**

- Functions: verb phrase describing what it does → `calculate_tax()`, `send_notification()`
- Booleans: question that's answered yes/no → `is_valid`, `has_permission`, `should_retry`
- Collections: plural noun describing the elements → `active_users`, `pending_orders`
- Counts: `num_` or `_count` prefix/suffix → `num_retries`, `error_count`
- Handlers: `on_` or `handle_` prefix → `on_click`, `handle_payment_failed`

### Function Size and Shape

A function should do ONE thing. How do you know? **If you can extract a part of it and give it a meaningful name that isn't just "part_one" or "rest_of_the_logic," it's doing more than one thing.**

```python
# Doing too many things:
def process_order(order):
    # Validate
    if not order.items:
        raise ValueError("Empty order")
    if order.total < 0:
        raise ValueError("Negative total")

    # Calculate tax
    tax_rate = get_tax_rate(order.shipping_state)
    tax = order.subtotal * tax_rate

    # Charge payment
    charge = stripe.Charge.create(amount=order.total + tax, ...)

    # Send confirmation
    send_email(order.customer.email, template="confirmation", order=order)

    # Update inventory
    for item in order.items:
        decrease_stock(item.product_id, item.quantity)

    return charge

# Each extracted function has a clear, meaningful name:
def process_order(order):
    validate_order(order)
    tax = calculate_tax(order)
    charge = charge_payment(order, tax)
    send_confirmation(order)
    update_inventory(order)
    return charge
```

### Error Handling Philosophy

Errors are not exceptional — they're part of the program's behavior. Handle them explicitly.

**The hierarchy:**

1. **Make it impossible:** Use types to prevent errors at compile time (strongest)
2. **Return errors as values:** Result types, error codes (Rust, Go, functional style)
3. **Throw exceptions:** For truly unexpected failures (Python, Java, C#)
4. **Let it crash:** For transient failures in supervised systems (Erlang/Elixir)
5. **Ignore it:** Only if failure literally doesn't matter (logging, optional analytics)

```rust
// Level 1: Impossible (type system prevents negative amounts)
struct PositiveAmount(f64);
impl PositiveAmount {
    fn new(value: f64) -> Option<Self> {
        if value > 0.0 { Some(Self(value)) } else { None }
    }
}

// Level 2: Explicit error return
fn withdraw(account: &mut Account, amount: PositiveAmount) -> Result<Receipt, InsufficientFunds> {
    if account.balance < amount.0 {
        return Err(InsufficientFunds { available: account.balance });
    }
    account.balance -= amount.0;
    Ok(Receipt { /* ... */ })
}
```

**On error messages:** Include the context needed to fix the problem.

```
Vague:    "Error: connection failed"
Helpful:  "Error: connection to database at db.prod.internal:5432 failed after 3 retries (last error: connection refused). Check that the database is running and the host/port are correct."
```

---

## Choosing Technologies

### The Boring Technology Thesis

> "Choose boring technology." — Dan McKinley

You get approximately 3 "innovation tokens" per project. Spend them on things that give you competitive advantage. For everything else, use the most well-understood, well-documented, battle-tested option.

```
Innovation tokens well spent:
  ✓ Novel algorithm that is your core product
  ✓ New framework that 10x's your team's productivity (proven by spike)
  ✓ Custom infrastructure for a genuine scaling cliff

Innovation tokens wasted:
  ✗ New database because it's trendy (when a well-understood option is sufficient)
  ✗ Custom deployment system (when standard CI/CD covers the need)
  ✗ Rewrite in a faster language because "performance" (profiling should come first)
```

### Technology Evaluation Checklist

Before adopting a technology, answer these questions:

| Question                       | Why It Matters                                                                          |
| ------------------------------ | --------------------------------------------------------------------------------------- |
| **Who maintains it?**          | Single person = bus factor 1. Company = may pivot/paywall. Foundation = usually stable. |
| **How old is it?**             | < 2 years = expect breaking changes. 5+ years = battle-tested.                          |
| **What's the community like?** | Stack Overflow answers? Active Discord? GitHub issues responded to?                     |
| **What's the escape path?**    | If this doesn't work out, how hard is it to migrate away?                               |
| **Does the team know it?**     | Unknown tech = 3-6 month productivity penalty while learning.                           |
| **What breaks at scale?**      | Every technology has a cliff. Where is it? Is that before or after your likely scale?   |

### Database Selection Intuition

Three archetypes cover the majority of use cases:

```
Relational (e.g., PostgreSQL, MySQL):
              "I need structured data with strong consistency."
              Relational data, ACID transactions, flexible querying.
              Most applications start here. Move away only when
              hitting a specific limitation.

In-Memory (e.g., Redis, Memcached):
              "I need sub-millisecond access."
              Caching, sessions, rate limiting, real-time leaderboards, pub/sub.
              In-memory stores trade durability for speed.

Event Streaming (e.g., Kafka, Pulsar):
              "I need systems to talk asynchronously."
              Event streaming, decoupled microservices, audit logs, data pipelines.
              Durable, replayable, high-throughput.
```

Everything else is a specialization of these three needs: structured storage, fast access, decoupled communication.

---

## Scaling Judgment

### The Scaling Ladder

Don't solve scaling problems you don't have. But know the ladder so you can climb it when needed:

```
Step 1: One server (handles more than you think)
        A single modern server: 64 cores, 256GB RAM, NVMe SSD
        Can handle: 10,000+ req/sec, millions of rows, thousands of concurrent users
        This covers 90% of applications. Seriously.

Step 2: Read replicas + caching
        Database reads slow? Read from replicas. Add Redis cache.
        Now you handle 10x the reads without touching your code.

Step 3: CDN + static asset optimization
        Images, JS, CSS served from edge locations.
        Reduces load on your servers by 50-80%.

Step 4: Background job processing
        Move slow work (emails, reports, image processing) to async queues.
        User gets fast response. Work happens in background.

Step 5: Database sharding or service extraction
        NOW you might need horizontal scaling.
        Split by natural boundaries (tenant, region, date range).

Step 6: Microservices
        Last resort, not first choice. Each service = operational overhead.
        Extract only when a monolith component genuinely needs independent scaling.
```

### The "One Beefy Server" Rule

Before designing a distributed system, calculate whether one machine handles your load:

```
Your traffic: 100 requests/second
One server can handle: 10,000 requests/second
You need: 1 server

Your data: 500GB
One server disk: 4TB NVMe
You need: 1 server

Your concurrent users: 5,000
One server with async runtime: 100,000+ concurrent connections
You need: 1 server
```

Many companies scale to millions of users on a handful of well-optimized servers. Distributed systems have enormous operational costs (network failures, consistency challenges, debugging difficulty). Don't pay those costs until the math forces you to.

---

## Code Review Judgment

### What to Actually Look For

**Not this:** "You should add a comment here." / "This variable name could be better."
**This:** Pattern-level issues that indicate misunderstanding or will cause future problems.

**The review hierarchy (most to least important):**

1. **Correctness bugs** — Will this produce wrong results? Race conditions? Off-by-one? Null dereference?
2. **Security issues** — SQL injection? Unvalidated input? Exposed secrets? Missing auth checks?
3. **Design problems** — Wrong abstraction? Tight coupling? Responsibility in wrong place?
4. **Performance cliffs** — Not "could be faster" but "will be O(n²) on production data"
5. **Maintainability** — Will the next person understand this? Are there traps?
6. **Style** — Automate with formatters. Don't waste human review on whitespace.

### When to Push Back and When to Let Go

**Push back hard on:** Correctness bugs, security holes, architectural decisions that are expensive to reverse.

**Let go (or add a TODO):** Style preferences, minor naming disagreements, "I would have done it differently but this works fine."

**The "two-way door" test:** If this decision is easily reversible (renaming, refactoring, changing a config), let it go. Ship it. Change it later if needed. If it's hard to reverse (database schema, public API, data migration), debate it now.

---

## Debugging Judgment

### Where Bugs Actually Are

```
90% of bugs are in YOUR code, not the library/framework/OS.
9% of bugs are in how you USE the library (wrong assumptions, missing config).
1% of bugs are in the library itself.

Debug in that order. Don't blame the framework until you've read the docs twice.
```

### The Fastest Debugging Technique

**Rubber duck debugging works because explaining the problem forces you to think sequentially** instead of jumping between hypotheses.

But there's a faster version:

```
1. State what you EXPECT to happen. (Be precise.)
2. State what ACTUALLY happens. (Be precise.)
3. The bug is in the gap between 1 and 2.
4. Find the earliest point where expectation diverges from reality.
   (Binary search: check the middle of the pipeline first.)
```

### When To Stop Debugging and Start Over

**Sunk cost fallacy in debugging:** "I've spent 6 hours on this approach, I can't give up now."

**Reality check:** If you've been stuck for > 2 hours on the same bug:

1. Step away for 15 minutes (physically leave your desk)
2. Explain the problem to someone (or a rubber duck)
3. Try a completely different approach
4. If it's a configuration issue, start from a known-good state and add changes one at a time

The fresh approach often works in 30 minutes. The 6 hours were wasted by assumption blindness.

---

## Project Judgment

### The Three Phases Every Project Goes Through

```
Phase 1: EXPLORATION (unknown unknowns)
  Goal: Learn what you're building
  Strategy: Prototype, spike, experiment. Code quality LOW. Throwaway.
  Time: 10-20% of project

Phase 2: CONSTRUCTION (known unknowns → known knowns)
  Goal: Build the thing
  Strategy: Clean architecture, tests, code review. Code quality HIGH.
  Time: 60-70% of project

Phase 3: HARDENING (polish + reliability)
  Goal: Make it production-ready
  Strategy: Edge cases, error handling, monitoring, docs, load testing.
  Time: 20-30% of project
```

**The mistake:** Skipping Phase 1 (building the wrong thing with perfect code) or skipping Phase 3 (shipping a fragile product).

### Estimating Work

**Hofstadter's Law:** It always takes longer than you expect, even when you take into account Hofstadter's Law.

**Practical estimation:**

1. Gut estimate: "3 days"
2. Break into tasks: Each task ≤ 4 hours
3. Sum the tasks: "Actually 5 days"
4. Multiply by 1.5-2x for unknowns: "8-10 days"
5. The answer is usually closer to step 4 than step 1

**The 90-90 rule:** "The first 90% of the code accounts for the first 90% of the development time. The remaining 10% of the code accounts for the other 90% of the development time."

That last 10% (error handling, edge cases, integration, deployment) always takes longer than you think.

### When to Refactor

**Refactor BEFORE adding a feature** if the current structure makes the feature awkward. Not after. Not as a separate project. Right before.

```
Want to add feature X.
Current code makes X hard to add.
  → Refactor the specific code that blocks X.
  → NOW add X cleanly.

Common pitfalls:
  - Adding X awkwardly with plans to refactor "later" (which often never happens)
  - Refactoring everything while you're in there (scope creep)
  - Better: Refactor just enough to make X clean.
```

This is the **Boy Scout Rule** applied with judgment: "Leave the code better than you found it" — but only the code you're touching, and only enough to serve your current purpose.

---

## The Meta-Skill: Knowing What You Don't Know

The most dangerous engineer is the one who doesn't know what they don't know. The most effective engineer is the one who does.

**Intellectual humility checklist:**

- "Am I sure this is the right approach, or do I just know this approach?"
- "Have I talked to someone who's done this at 10x our scale?"
- "What's the thing I'm most wrong about in this design?"
- "If this fails, what will the failure mode be?"

**When to ask for help:**

- You've been stuck for more than 1 hour with no forward progress
- You're making a decision that's expensive to reverse
- You're working in a domain you don't have experience in (security, compliance, networking)
- You don't know what you don't know (that's when you need a second perspective most)

The best engineers aren't the ones who know everything. They're the ones who know what matters, know what they don't know, and build systems simple enough that the next person can understand and improve them.
