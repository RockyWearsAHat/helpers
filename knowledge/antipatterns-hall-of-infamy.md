# Infamous Antipatterns — Bugs So Bad They Became Jokes

## Why This Exists

Every antipattern here is a real, recurring pattern in production codebases. They're so common and so destructive that the developer community turned them into shared vocabulary — jokes that compress years of collective pain into a single phrase. Knowing the name helps you spot the pattern instantly.

---

## Architecture Antipatterns

### Spaghetti Code

Code with tangled, unpredictable control flow. No clear structure. Every function calls every other function. Tracing execution requires a whiteboard and three days.

**The joke**: "This isn't code, it's a Jackson Pollock painting."

**What it looks like**: 2000-line functions. `goto` or deeply nested callbacks. Global state mutated from 47 different locations. Circular dependencies between modules.

**The fix**: Extract functions. Reduce coupling. Invert dependencies. If you can't describe what a module does in one sentence, it does too much.

### Big Ball of Mud

The entire system is spaghetti code. No architecture at all — just accretion of features bolted on over years. Everyone's afraid to touch it because nobody understands it.

**The joke**: "Our architecture diagram is just a photo of the dumpster behind the building."

**Real-world frequency**: The most common software architecture in production. Most systems end up here. It's the default destination when nobody actively fights entropy.

### God Object / God Class

One class that knows everything, does everything, and everything depends on it. 10,000 lines. 200 methods. Imported in every file.

**The joke**: "We don't have microservices. We have one service and it's very angry."

**Signs**: `Manager`, `Handler`, `Processor`, `Utils`, `Helper` in the class name. If the class name is vague, it probably does too much.

### Lava Flow

Dead code that nobody dares remove because "it might be important" or "someone might need it." Accumulated over years. Undocumented. Untested. Possibly load-bearing.

**The joke**: "We don't delete code here. We just comment it out and put a date on it. From 2014."

**The fix**: If it's in source control, it's recoverable. Delete it. If tests pass, it wasn't needed.

### Golden Hammer

"We used [favorite technology] and it worked great last time, so we'll use it for everything."

**Classic examples**:

- Using a relational database as a message queue
- Writing everything in one language because the team only knows one language
- Solving every problem with microservices (or solving every problem with a monolith)

**The joke**: "When all you have is a hammer, everything looks like a microservice."

---

## Code-Level Antipatterns

### Cargo Cult Programming

Copying code or patterns without understanding why they work. Magic incantations preserved because removing them might break something.

**The joke**: "Why is there a `sleep(100)` here?" "Don't touch that. Last person who removed it was fired."

**Signs**: Code blocks that "must be there" but nobody can explain why. Patterns from tutorials applied without understanding the problem they solve. Configuration copied from Stack Overflow with values nobody understands.

### Copy-Paste Programming

Instead of abstracting common logic, copy the code and modify it slightly. Now you have 17 copies of the same bug.

**The joke**: "DRY? We're Write Everything Twice. Actually, Write Everything Seventeen Times."

**The fix**: When you copy-paste, you're identifying a reusable abstraction. Extract it. But: copying twice is fine. Three times, extract.

### Premature Optimization

Optimizing code before you have evidence it's slow. Spending a week making a function 10ns faster when it's called once per request and the database query takes 200ms.

**The joke**: "Premature optimization is the root of all evil" — Donald Knuth.
The full quote continues: "Yet we should not pass up our opportunities in that critical 3%." People always forget the second part.

**Signs**: Hand-rolled data structures instead of standard library. Bitwise tricks in business logic. Caching everything (including things that change every request). Choosing a "fast" language for a CRUD app.

### Stringly Typed Programming

Using strings for everything instead of proper types. Statuses as strings. Configs as strings. Errors as strings. Types as strings.

**The joke**: "Our type system is `string`. Everything is a `string`."

```python
# Stringly typed
user["role"] = "admin"      # Typo: "adimn" compiles fine
user["status"] = "actve"    # Runtime bug

# Properly typed
class Role(Enum):
    ADMIN = "admin"
    USER = "user"
user.role = Role.ADMIN      # Typo: won't compile
```

### Magic Numbers / Magic Strings

Unexplained literal values scattered throughout code.

```python
# What does 86400 mean? What does 3 mean?
if elapsed > 86400:
    retry(max_attempts=3)

# Self-documenting
SECONDS_PER_DAY = 86400
MAX_RETRIES = 3
if elapsed > SECONDS_PER_DAY:
    retry(max_attempts=MAX_RETRIES)
```

### Boolean Blindness

Functions that take multiple boolean arguments. Impossible to read at the call site.

```python
# What do these booleans mean?
create_user("Alice", True, False, True, False)

# Alternative: use named parameters, enums, or option objects
create_user("Alice", admin=True, verified=False, active=True, newsletter=False)
```

---

## Process Antipatterns

### "Works On My Machine"

Code passes locally but fails in CI, staging, or production. The developer's machine has special configuration, environment variables, cached data, or dependencies that aren't documented.

**The joke**: Gave rise to Docker's original marketing: "Works on my machine? Then we'll ship your machine."

**The fix**: Containerize. Use deterministic builds. Test in CI with clean environments. Never manually install dependencies.

### Shotgun Surgery

One change requires touching 20 files. A single field name change means updating models, serializers, views, tests, migrations, API docs, client code, and configs.

**The joke**: "I changed a button color and somehow the billing system broke."

**Opposite**: Divergent Change (one module changed for many different reasons). Both indicate poor separation of concerns.

### Resume-Driven Development

Choosing technology based on what looks good on a resume rather than what solves the problem.

**The joke**: "We rebuilt our todo app with Kubernetes, a message queue, two databases, and a machine learning pipeline. For our 50 users."

### Boat Anchor

Code, frameworks, or infrastructure kept around because "we might need it someday." You won't. It's dead weight that costs maintenance.

**The joke**: YAGNI — "You Ain't Gonna Need It." You thought you'd need that abstract factory factory? You didn't.

### Accidental Complexity

Complexity that comes from your tools and choices rather than the problem itself.

**The joke**: "We spent 4 months building the deployment pipeline. The app took 2 weeks."

**Essential complexity** = inherent to the problem domain. **Accidental complexity** = introduced by your implementation choices. Most codebases are 90% accidental complexity.

---

## The Classics

### The Daily WTF (Worse Than Failure)

**Enterprise FizzBuzz**: What happens when simple problems are "architected" with enterprise patterns. AbstractSingletonProxyFactoryBean is a real Spring class name.

**The Inner-Platform Effect**: Building a configurable system so general that you've just reimplemented the programming language / database / OS it runs on, but worse. "We built our own query language so non-programmers could... wait, they still need a programmer to use it."

**Second System Effect** (Fred Brooks): The second version of a successful system is always over-engineered and bloated. "Version 1 was lean and worked. Version 2 took three years and did everything badly."

### Falsehoods Programmers Believe

These are so common they've become canonical lists:

**Names**: A person has a first name and a last name. Names are unique. Names use ASCII. Names don't change. (All false.)

**Time**: There are 24 hours in a day. A minute has 60 seconds. Time zones are whole-hour offsets. Daylight savings is consistent. (All false.)

**Addresses**: Every address has a street number. Zip codes are numeric. Countries have states/provinces. (All false.)

**Phone numbers**: Phone numbers have area codes. A phone number uniquely identifies a person. Phone numbers are numeric. (All false.)

**Geography**: Countries have one time zone. Country borders don't change. Country names are unique. (All false.)

---

## Legacy Code Antipatterns

### Dependency Hell

Mutually incompatible version requirements. Package A needs X≥2.0, package B needs X<2.0. Welcome to Saturday afternoon.

**Language-specific names**: DLL Hell (Windows), JAR Hell (Java), node_modules (JavaScript — it's not an antipattern, it's a lifestyle).

### Technical Debt

Shortcuts taken knowingly with a plan to fix later. Except "later" never comes. The interest compounds. Eventually velocity drops to zero because every change fights the accumulated debt.

**The joke**: "We'll fix it in the next sprint." (The next sprint has been running for three years.)

**Reality**: Some technical debt is strategic and deliberate. Most is unintentional and accumulates silently.

### Jenga Code

The system is so fragile that every change risks toppling it. No tests. No documentation. Key logic in one person's head (who left).

**Signs**: Deploy day requires a war room. "Don't deploy on Fridays" is policy. Rollback is the most tested procedure you have.

---

## The Meta-Antipattern

### Not Invented Here (NIH)

Refusing to use existing solutions because "we can build it better." You can't. Use the library, the framework, the service. Your job is to solve your business problem, not reimplement HTTP parsing.

**The opposite antipattern**: **Invented Here** — refusing to build anything because "there's probably a library for that." There isn't always. Sometimes you need 10 lines of code, not a 50MB dependency.

**The balance**: Use proven solutions for solved problems. Build custom solutions for your unique problems.

---

_Sources: AntiPatterns (Brown et al.), The Daily WTF (thedailywtf.com), Refactoring (Fowler), "Falsehoods Programmers Believe" series, A Philosophy of Software Design (Ousterhout), developer folklore and collective memory_
