# Problem-Solving Methodology — How to Attack Any Problem From Scratch

This is the document about _process_ — how elite developers actually work through problems they've never seen before. Not "here's the pattern to apply" but "here's how to FIND the right approach when you don't know what it is yet."

---

## The Universal Problem-Solving Loop

Every good problem-solving process follows this loop, whether you're debugging a segfault, designing an API, or building a new feature:

```
UNDERSTAND → PLAN → EXECUTE → VERIFY → REFLECT
     ↑                                    │
     └────────────────────────────────────┘
```

Most people skip straight to EXECUTE. That's why most people waste time.

### Step 1: UNDERSTAND — What Are You Actually Solving?

**Spend 20% of your time here.** It feels slow. It saves 50% of total time.

**The Five Whys of Requirements:**

```
User says: "I need a button that exports to CSV"
  Why? "So I can email reports to my boss"
  Why email? "Because she needs weekly summaries"
  Why weekly? "Because she tracks month-over-month trends"
  Why CSV? "That's what she asked for"
  Actual need: A scheduled report with trend visualization.
  The CSV button is one possible solution, not the requirement.
```

**Requirements checklist:**

- What does "done" look like? (acceptance criteria)
- What are the inputs? What are the outputs?
- What are the edge cases? (empty input, massive input, malformed input, concurrent access)
- What are the constraints? (latency, memory, compatibility, security)
- What does failure look like? (and how do we recover?)

**The restatement test:** Restate the problem in your own words. If the requester says "no, that's not what I meant," you just saved yourself from building the wrong thing.

### Step 2: PLAN — Design Before You Type

**Think about WHAT before HOW.**

```
Level 1: What are the major components?
Level 2: How do they interact? (data flow, control flow)
Level 3: What's the interface of each component? (inputs, outputs, errors)
Level 4: What's the implementation of each component?

Most people start at Level 4. Start at Level 1.
```

**Sketch it out:** Whiteboard, paper, or a quick ASCII diagram. Force yourself to draw the data flow. If you can't draw it, you don't understand it yet.

```
User Request
     │
     ▼
[API Gateway] ──auth──► [Auth Service]
     │
     ▼
[Order Service] ──stock check──► [Inventory Service]
     │
     ▼
[Payment Service] ──charge──► [Stripe API]
     │
     ▼
[Notification Service] ──email──► [Customer]
```

Even this rough sketch reveals questions: What if the stock check passes but payment fails? Do we need to reserve inventory? What's the rollback flow?

### Step 3: EXECUTE — Build Incrementally

**The cardinal rule: Get something working end-to-end FIRST, then iterate.**

```
BAD order (big bang):
  1. Build perfect data model
  2. Build all API endpoints
  3. Build all business logic
  4. Build UI
  5. Wire everything together
  6. Hope it works (it won't)

GOOD order (walking skeleton):
  1. Hardcoded response through entire stack (UI → API → DB → response)
  2. Replace hardcoded data model with real one
  3. Add one real endpoint
  4. Add one real UI interaction
  5. Now iterate: add features one at a time, each fully working
```

**The walking skeleton approach:** Build the thinnest possible slice that touches every layer of the system. A real HTTP request, hitting a real endpoint, reading from a real database, rendering a real (ugly) page. Now you have a working system to iterate on instead of separate pieces that might not fit together.

### Step 4: VERIFY — Prove It Works

```
Level 1: Does it compile/run without errors?
Level 2: Does it handle the happy path correctly?
Level 3: Does it handle edge cases? (empty, null, huge, concurrent)
Level 4: Does it perform acceptably under realistic load?
Level 5: Does it fail gracefully when dependencies are down?
```

**Write your verification in code (tests), not in your head.** Manual verification doesn't scale and doesn't persist.

### Step 5: REFLECT — What Did You Learn?

After each significant piece of work:

- What took longer than expected? Why?
- What surprised you?
- What would you do differently?
- Is there a general principle you can extract?

This is how you build engineering judgment. Without reflection, you have 1 year of experience repeated 10 times instead of 10 years of experience.

---

## Systematic Debugging — The Method

### The Scientific Method, Applied to Code

```
1. OBSERVE:    What exactly is happening? (Symptoms, not causes)
2. HYPOTHESIZE: What could cause this? (Generate multiple hypotheses)
3. PREDICT:    If my hypothesis is right, what would I see if I checked X?
4. TEST:       Check X. Does the prediction hold?
5. REPEAT:     If yes → narrow down within that hypothesis.
               If no  → eliminate that hypothesis, try next one.
```

**The key discipline:** Test ONE hypothesis at a time. If you change three things and it starts working, you don't know which fix mattered (and the other two might cause problems later).

### Binary Search Debugging

When you have a long pipeline and something goes wrong at the end, don't trace from the beginning. Check the MIDDLE:

```
Input → [A] → [B] → [C] → [D] → [E] → Wrong Output

Check output of C:
  Correct at C? Bug is in D or E. Check D.
  Wrong at C?   Bug is in A, B, or C. Check B.

3 checks instead of 5. For longer pipelines, the savings compound.
```

This applies to:

- **Debugging functions:** Check intermediate values at the halfway point
- **Debugging commits:** `git bisect` — binary search through commit history
- **Debugging infrastructure:** Check if the request is correct at the load balancer, then the service, then the DB
- **Debugging data pipelines:** Check data shape at the middle transformation

### The "What Changed?" Heuristic

```
It was working. Now it's not. What changed?

1. Your code (most likely) → git diff, recent commits
2. Dependencies (common) → package updates, lockfile changes
3. Configuration (common) → env vars, config files, feature flags
4. Data (sneaky) → new data patterns that hit untested paths
5. Infrastructure (rare) → DNS, certificates, disk space, network
6. Time (sneakiest) → date-dependent bugs, expired tokens, timezone changes
```

80% of the time, the answer is in category 1 or 2. Start there.

### Rubber Duck Debugging (Why It Actually Works)

When you explain a problem out loud (to a person, a rubber duck, or a text file), you're forced to:

1. State your assumptions explicitly (you'll catch wrong ones)
2. Walk through logic sequentially (you'll spot gaps)
3. Articulate what you THINK the code does (vs what it actually does)
4. Say "wait, that can't be right" (the eureka moment)

The act of translating from "how I think about it" to "how I explain it" catches bugs that staring at code doesn't.

---

## Designing APIs and Interfaces

### Think From the Caller's Perspective

Before writing ANY function, class, or API, write the code that CALLS it first:

```python
# STEP 1: Write how you WANT to use it
result = search(
    query="sustainable energy",
    filters={"year": 2024, "type": "research"},
    limit=20
)
for paper in result.papers:
    print(f"{paper.title} ({paper.year}) - {paper.citation_count} citations")

# STEP 2: Now implement search() to make the above work
```

This is test-driven design without the test framework. You're designing the interface from the consumer's perspective, which always produces better APIs than designing from the implementation side.

### The Rule of Three

```
First time: Just write the code inline.
Second time: Note the duplication. Maybe copy-paste (yes, really).
Third time: NOW extract the abstraction. You have 3 examples of usage,
            so you actually know what the general form should be.
```

Abstracting too early creates the wrong abstraction. The wrong abstraction is worse than duplication because it actively fights you when requirements diverge.

### Interface Design Principles

**Principle 1: Easy to use correctly, hard to use incorrectly.**

```python
# Easy to use incorrectly:
def create_user(name, email, age, active, verified, role):
    ...
# What order? What types? Can age be negative? Is active True or "yes"?

# Hard to use incorrectly:
def create_user(
    name: UserName,         # Validated on construction
    email: Email,           # Validated on construction
    age: Age,               # Guaranteed positive
    role: Role = Role.USER  # Enum, can't misspell
) -> User:
    ...
# Active and verified start as False by default (sensible). Types prevent misuse.
```

**Principle 2: Minimize surprise.** Functions should do what their name says and nothing more. No side effects that aren't obvious from the name.

```python
# Surprising:
def get_user(user_id):
    user = db.find(user_id)
    user.last_accessed = datetime.now()  # WRITING during a GET? Surprise!
    db.save(user)
    analytics.track("user_viewed", user_id)  # Side effect? Surprise!
    return user

# Not surprising:
def get_user(user_id):
    return db.find(user_id)

def record_user_access(user_id):  # Separate function with honest name
    db.update_last_accessed(user_id)
    analytics.track("user_viewed", user_id)
```

**Principle 3: Make the common case easy and the advanced case possible.**

```python
# Common case (80% of users):
results = search("python web framework")

# Advanced case (20% of users):
results = search(
    "python web framework",
    filters=SearchFilters(language="en", date_range=(2023, 2025)),
    ranking=RankingStrategy.RECENCY,
    limit=50,
    offset=100
)
```

Default parameters should serve the common case. Don't force every caller to specify everything.

---

## Working With Existing Code

### Reading Code (More Important Than Writing Code)

Most of your time is spent reading code, not writing it. Get good at it.

**The top-down approach:**

1. Read the README / docs → What does this project do?
2. Read the entry point (main, index, app) → What's the top-level flow?
3. Follow ONE request/operation through the code → How does data flow?
4. Now you understand enough to make changes.

**The bottom-up approach (for debugging):**

1. Start at the symptom (error message, wrong output)
2. Find where it's generated → grep for the error string
3. Trace backwards: who calls this? What data arrives here?
4. Find the root cause.

**The grep-and-jump method:**

```bash
# Find the entry point
grep -rn "def main\|func main\|if __name__" .

# Find where a function is defined
grep -rn "def process_order\|func ProcessOrder" .

# Find where it's called
grep -rn "process_order\|ProcessOrder" . | grep -v "def \|func "

# Find what config/env vars are used
grep -rn "os.environ\|env\.\|getenv\|process.env" .
```

### Changing Code You Don't Fully Understand

**The safe approach:**

1. Write a test that captures CURRENT behavior (not what you think it should do — what it actually does)
2. Make your change
3. If the test breaks on something you didn't intend to change → you've learned something about the code
4. If the test passes → your change is safe
5. Add a test for the NEW behavior you want

This is "characterization testing." It protects you from breaking things you don't understand yet.

### Legacy Code Survival

Legacy code isn't bad code. It's code that works, makes money, and was written under different constraints. Respect what it provides while improving it.

```
DON'T: "This is terrible, let's rewrite everything"
  → Rewrites take 3x longer than expected
  → You'll lose edge cases the old code handles
  → The business can't stop while you rewrite

DO: "Strangler Fig Pattern"
  → New features go in new code
  → Gradually route traffic from old to new
  → Old code shrinks over time until it can be deleted

DO: "Sprout Method"
  → Need new functionality? Write it as a new function/class
  → Call the new code from the minimal touch point in old code
  → Old code unchanged except for one call site
```

---

## Architecture Thinking

### Start With the Data

**The data model is the heart of every application.** Get the data model right and the rest follows. Get it wrong and you fight it forever.

```
Questions before writing any code:
1. What are the entities? (User, Order, Product, Payment)
2. What are the relationships? (User has many Orders, Order has many Products)
3. What's the access pattern? (Look up orders by user? By date range? By product?)
4. What needs to be consistent? (Payment + Order must agree)
5. What can be eventually consistent? (Search index can lag behind)
```

The access pattern drives everything. If you need "all orders by user in the last 30 days," your data model must make that query efficient. This might mean denormalization, indexes, or a different storage system entirely.

### Boundaries Are Everything

The most important architectural decision isn't what goes inside a component — it's where you draw the lines between components.

**Good boundaries:**

- Changes inside one component don't affect others
- Each component can be understood independently
- Components can be tested independently
- The interface between components is narrow and well-defined

**Where to draw boundaries:**

- Around concepts that change together (bounded context)
- Around concepts that have different rates of change
- Around different teams' responsibilities
- Around different deployment requirements
- Around different scaling requirements

```
GOOD boundaries for an e-commerce system:
  [Catalog] ── [Cart] ── [Checkout] ── [Shipping]
  Each changes for different reasons. Cart doesn't care about shipping carriers.

BAD boundaries:
  [Frontend Processing] ── [Backend Processing] ── [Data Layer]
  Every feature touches all three. Every change requires changing all three.
```

### Stateless vs Stateful

**Default to stateless.** Stateless components are easy to scale (add more instances), easy to recover (replace a crashed instance), and easy to test (no setup required).

**When you must have state,** put it in a dedicated stateful component (database, cache, message broker) that's designed for it. Stateful components are harder to scale, harder to recover, and harder to test. Make the hard thing as small as possible.

```
STATELESS (easy to scale):
  Web servers, API servers, worker processes, functions

STATEFUL (hard to scale, minimize these):
  Databases, caches, message brokers, user sessions

PATTERN: Make your application servers stateless.
         Store ALL state in purpose-built stateful services.
         Now you can scale app servers horizontally with a load balancer.
```

---

## Working in Teams

### Communication Is a Technical Skill

The best code in the world is useless if your team doesn't understand it, your product manager can't explain it, or your users can't use it.

**Code as communication:**

- Variable names are messages to future readers
- Commit messages are diary entries for future archaeologists
- PR descriptions are teaching documents
- Documentation is a contract with your users

**Commit message discipline:**

```
BAD:  "fix bug"
BAD:  "update stuff"
BAD:  "WIP"

GOOD: "Fix race condition in order processing when concurrent users
       checkout the same item. The inventory check and decrement
       were not atomic, allowing overselling under load."
```

The commit message should say WHY, not WHAT. The diff shows WHAT. Only you know WHY.

### How to Ask for Help Effectively

```
BAD:  "It doesn't work"

GOOD:
  WHAT I'm trying to do: [goal]
  WHAT I expected to happen: [expected behavior]
  WHAT actually happened: [actual behavior, exact error message]
  WHAT I've already tried: [list of approaches]
  WHAT I think the problem might be: [hypothesis]
```

This format helps others help you. It also sometimes solves the problem (see: rubber duck debugging).

### Code Ownership and Collaboration

```
Strong ownership:  One person/team owns each area. Clear accountability.
                   Risk: Knowledge silos, bus factor = 1.

Collective ownership: Anyone can change anything. No bottlenecks.
                      Risk: Nobody feels responsible. Quality drifts.

Balanced: Primary owner + mandatory code review from non-owners.
          Accountability + knowledge sharing.
```

---

## Building for Production

### The Production Checklist

Before going live, every system should have:

```
□ Health check endpoint (/health)
□ Structured logging (JSON, with request IDs)
□ Error alerting (PagerDuty, Slack, email — pick one that wakes you up)
□ Graceful shutdown (finish in-flight requests before dying)
□ Configuration via environment variables (not hardcoded)
□ Secrets in a secret manager (not in code, not in env files committed to git)
□ Database migrations (repeatable, versioned, backwards-compatible)
□ Deployment rollback plan (how do you undo a bad deploy?)
□ Load testing (can it handle 2x your expected traffic?)
□ Backup verification (backups exist AND you've tested restoring from one)
```

### Observability — The Three Questions

When something goes wrong in production, you need to answer three questions fast:

```
1. WHAT is broken?     → Metrics + alerts (error rate spike, latency increase)
2. WHERE is it broken? → Distributed tracing (which service? which endpoint?)
3. WHY is it broken?   → Logs (specific error message, context, stack trace)
```

If you can answer these three questions within 5 minutes of an incident starting, you've built good observability.

### Incident Response — Stay Calm, Be Methodical

```
1. DETECT:    Something is wrong (alert fires, user reports)
2. TRIAGE:    How bad is it? (All users? Some users? One user? Data loss?)
3. MITIGATE:  Stop the bleeding. Rollback, disable feature flag, scale up.
              Don't try to fix the root cause yet. Just stop the damage.
4. FIX:       Now find and fix the root cause (with the system stable).
5. POSTMORTEM: Blameless analysis. What happened? Why? How do we prevent it?
```

**The golden rule:** MITIGATE before you DIAGNOSE. Rolling back a bad deploy takes 2 minutes. Debugging the bug takes 2 hours. Your users shouldn't wait 2 hours.
