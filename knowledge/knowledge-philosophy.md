# Knowledge Philosophy — How to Use (and Distrust) Everything in This Directory

**Read this first. It governs how every other file here should be interpreted.**

---

## These Files Are Starting Points, Not Scripture

Every knowledge file in this directory is a **mental model** — a simplification of reality that's useful in some contexts and misleading in others. None of it is absolute truth. Software development is a living discipline where:

- "Best practices" are best under specific assumptions that may not hold for your project
- Language features, library APIs, and ecosystem tools change with every release
- What the community considers idiomatic shifts over years (callbacks → promises → async/await)
- Performance characteristics depend on hardware, OS, runtime version, and workload
- Security advice has a shelf life — new vulnerabilities, new attack surfaces, new mitigations appear constantly

**The right mental posture:** Treat knowledge files as _an experienced colleague's opinion from a recent-ish conversation_ — informed, worth hearing, but not authoritative. Verify against the current state of the ecosystem before committing to any approach.

---

## The Uncertainty Gradient

Not all knowledge ages at the same rate. Use this gradient to calibrate confidence:

```
STABLE (trust for years):
  ├── Mathematical fundamentals (complexity theory, graph theory, logic)
  ├── Core data structures (hash tables, trees, graphs, heaps)
  ├── Foundational algorithms (sorting, searching, dynamic programming)
  ├── Operating system concepts (processes, memory, I/O)
  ├── Network protocols (TCP/IP, DNS, HTTP semantics)
  └── Reasoning patterns (decomposition, abstraction, invariants)

DURABLE (trust for 1-3 years, verify specifics):
  ├── Language core syntax and semantics
  ├── Database fundamentals (B-trees, ACID, indexing)
  ├── Design pattern concepts (not specific implementations)
  ├── Security principles (defense in depth, least privilege)
  └── Architecture patterns (the concept of microservices, not specific frameworks)

VOLATILE (verify before every use):
  ├── Specific library APIs and their current recommended usage
  ├── Framework conventions and configuration patterns
  ├── CLI tool flags and options
  ├── Package manager commands and behaviors
  ├── Cloud service offerings and pricing
  ├── "Recommended" linter/formatter/toolchain configurations
  └── Version-specific language features
```

**When in doubt, verify.** Read the official docs. Check the changelog. Look at the repo's recent activity. A knowledge file written 6 months ago may reference a library that was deprecated last week.

---

## How to Actually Use These Files

### For Writing Code

1. **Start with the PROBLEM, not the knowledge file.** Understand what you're building and why.
2. **Check the relevant knowledge file** for mental models, patterns, and tradeoffs.
3. **Then research the CURRENT ecosystem** — official docs, recent release notes, community discussions.
4. **Synthesize.** The knowledge file gives you vocabulary and framing. Current docs give you accuracy. Your judgment connects them.

### For Choosing Technologies

Knowledge files describe what tools _conceptually do_ and where they fit. They should NOT be the basis for technology selection. Instead:

1. Use the knowledge file to understand the _category_ of tool you need
2. Research the _current_ options in that category
3. Evaluate based on your actual constraints (team skills, existing infrastructure, scale, timeline)
4. Prototype before committing — a half-day spike reveals more than any knowledge file

### For Understanding Concepts

This is where knowledge files are most valuable. Core CS concepts — how hash tables work, why B-trees are used in databases, what a deadlock is, how TCP handles congestion — are stable enough to trust. But even here:

- Don't trust implementation-specific details without verification
- Don't assume a concept example from one language transfers to another syntactically
- Don't assume performance characteristics without profiling your actual workload

---

## The Research-First Principle

**When starting any non-trivial project or working in an unfamiliar ecosystem, research comes before building.**

This isn't optional. It's the difference between building something solid and building something that fights its own tools.

### What "Research" Actually Means

**NOT:** Reading one blog post and assuming it's current.
**NOT:** Using whatever a knowledge file says without checking.
**NOT:** Going with the first Stack Overflow answer.

**ACTUALLY:**

1. **Official documentation** — The canonical source. Read the "getting started" AND the "concepts" section. Skim the API reference.
2. **Recent release notes / changelog** — What changed in the last 2-3 versions? What's deprecated? What's new?
3. **GitHub issues and discussions** — What are people running into? What's the maintainer's stance on common problems?
4. **Multiple perspectives** — Read 3 opinions on a technical choice, not 1. Look for the dissenting view.
5. **Working examples** — Clone a well-maintained example project. Read how they actually structure things, not how a tutorial says to.

### Ecosystem Investigation Protocol

When you encounter a project in any technology stack:

```
1. IDENTIFY THE STACK
   What language? What runtime version? What framework?
   What package manager? What build tool? What test runner?
   Read: package.json / Cargo.toml / go.mod / pyproject.toml / *.csproj

2. UNDERSTAND THE CONVENTIONS
   How does THIS ecosystem do things? Not how another ecosystem does it.
   → Go doesn't use exceptions. Don't suggest try/catch patterns.
   → Rust uses Result types. Don't suggest returning null.
   → Python values readability. Don't write clever one-liners.
   → Shell scripts need defensive patterns. Don't assume set -e covers everything.

3. CHECK WHAT'S CURRENT
   What's the latest stable version of the core framework/language?
   What recently changed that affects how you'd build this?
   Are there new recommended patterns that replace old ones?

4. MAP THE ECOSYSTEM
   What are the standard companion libraries?
   What's the testing story? Linting? Formatting? CI?
   What does a well-structured project in this ecosystem look like?

5. IDENTIFY THE COMMUNITY'S ACTUAL PRACTICES
   GitHub trending repos in this ecosystem
   Official "awesome-X" lists
   Framework-specific style guides and RFCs
```

---

## When Knowledge Files Contradict Reality

It will happen. A knowledge file says "use library X" but library X was deprecated. Or it recommends a pattern that the framework now has a better built-in solution for. Or it describes a security practice that's been superseded.

**Resolution order:**

1. **Official documentation** wins over knowledge files
2. **Current community consensus** wins over outdated patterns
3. **The actual codebase you're working in** wins over theoretical ideals
4. **Working code that solves the problem** wins over architectural purity

**Don't "fix" working code to match a knowledge file.** If the codebase uses a pattern that contradicts the knowledge file but works well in context, respect the codebase. The knowledge file doesn't know your constraints. The codebase lives with them.

---

## How to Think About "Best Practices"

Every best practice is actually: **"This worked well in [context] under [constraints] for [goals]."**

Before adopting any practice, extract the hidden context:

```
"Always write unit tests"
  Context: Long-lived production code with multiple contributors
  Constraint: Code must remain correct through changes
  Goal: Prevent regressions
  → Does this apply to a throwaway prototype? No.
  → Does this apply to a 50-line shell script? Probably not.
  → Does this apply to a web API with 20 endpoints? Absolutely.

"Use TypeScript instead of JavaScript"
  Context: Teams larger than 2, codebases that will live for years
  Constraint: Need to catch type errors before runtime
  Goal: Maintainability and confidence during refactoring
  → Does this apply to a quick script? No.
  → Does this apply to a library others will consume? Strongly yes.

"Microservices over monolith"
  Context: Large org with many independent teams
  Constraint: Teams need to deploy independently
  Goal: Organizational scaling
  → Does this apply to a 3-person startup? Almost never.
  → Does this apply to a single-team project? Rarely.
```

**The skill isn't knowing best practices. It's knowing WHEN they apply and WHEN they don't.**

---

## Evolving This Knowledge Base

These files should change. They should be challenged, updated, extended, and occasionally deleted.

**Signs a file needs updating:**

- References specific library versions older than 1-2 years
- Recommends tools that have been superseded (e.g., TSLint → ESLint)
- Describes patterns without tradeoffs (everything is presented as "the right way")
- Contradicts current official documentation
- Doesn't acknowledge the ecosystem has evolved since it was written

**What makes a good knowledge file:**

- Focuses on **why** and **tradeoffs**, not just **what**
- Acknowledges when advice is context-dependent
- Teaches mental models that transfer, not just specific tools
- Includes "when NOT to use this" alongside "when to use this"
- Cites the reasoning, not just the conclusion

**What makes a bad knowledge file:**

- Lists facts without context ("Use bcrypt for passwords")
- Treats opinions as universal truths ("Always use a factory pattern")
- Describes the current ecosystem as if it's permanent
- Gives specific commands without explaining what they do
- Presents a single approach without alternatives

---

## The Goal

The purpose of this knowledge directory is NOT to make Copilot a walking encyclopedia that recites facts.

The purpose is to give Copilot:

- **Vocabulary** — knowing what concepts exist and what they're called
- **Mental models** — frameworks for thinking about problems
- **Pattern recognition** — "I've seen problems shaped like this before"
- **Judgment** — knowing when to apply a pattern and when it's wrong
- **Research instincts** — knowing when to stop guessing and start investigating

An engineer with these five things and access to official documentation will outperform an engineer who has memorized every API but can't reason about when to use what.

**Knowledge files + critical thinking + active research = great engineering.**
**Knowledge files alone = a textbook that's already slightly out of date.**
