# Ecosystem Research — How to Understand Any Technology Stack From Scratch

This is the methodology for becoming effective in ANY ecosystem quickly. Not by memorizing — by investigating intelligently and building a working mental model that updates as you learn more.

---

## Phase 1: Situational Awareness (Minutes)

Before writing a single line of code, understand what you're working with.

### Read the Project's Manifest

Every ecosystem has a file that tells you what's in play:

```
JavaScript/TypeScript:  package.json        → dependencies, scripts, engines
Python:                 pyproject.toml       → dependencies, build system, tool config
                        requirements.txt     → older style, just dependency list
                        setup.py / setup.cfg → legacy, but still common
Rust:                   Cargo.toml           → dependencies, features, edition
Go:                     go.mod               → module path, Go version, dependencies
Java:                   pom.xml (Maven)      → dependencies, plugins, profiles
                        build.gradle         → dependencies, tasks, plugins
C#:                     *.csproj             → target framework, packages, settings
Ruby:                   Gemfile              → dependencies, source, groups
PHP:                    composer.json        → dependencies, autoload, scripts
Swift:                  Package.swift        → targets, dependencies, platforms
Elixir:                 mix.exs              → dependencies, application config
```

**What to extract:**

- Language/runtime version constraints
- Core framework (if any) and its version
- Key dependencies and what they're for
- Build/test/lint scripts defined in the project
- Configuration files referenced (eslint, prettier, jest, etc.)

### Map the Directory Structure

```
Every ecosystem has conventions about where things go:

src/ or lib/        → Application code
test/ or tests/     → Test code
config/             → Configuration
public/ or static/  → Static assets (web)
migrations/         → Database schema changes
scripts/            → Automation / build scripts
docs/               → Documentation
```

Structure reveals architecture. A flat directory with 50 files is different from a layered structure with clear boundaries. Neither is inherently wrong — but each tells you something about the project's complexity and organization philosophy.

### Check the README and Contributing Guide

```
README.md           → What is this? How do I run it? What's the architecture?
CONTRIBUTING.md     → How does the team work? What are the conventions?
.editorconfig       → Indentation, line endings, charset
.eslintrc / ruff.toml / .rubocop.yml → Code style rules
Makefile / justfile → Available commands and workflows
```

---

## Phase 2: Ecosystem Context (30 Minutes)

Now understand the ecosystem this project lives in.

### The Core Questions

**For any technology, answer these:**

1. **What problem does it solve?** Not "what is it" but "what would I have to do without it?"
2. **What's the mental model?** Every framework has a core idea — components, middleware pipeline, actor model, reactive streams. What's this one?
3. **What's the convention vs configuration balance?** Rails is convention-heavy (magic). Express is configuration-heavy (explicit). Where does this tool fall?
4. **What's the error model?** Exceptions? Result types? Error codes? Panic? This shapes how you write every function.
5. **What's the concurrency model?** Single-threaded event loop? Green threads? OS threads? Actor model? This shapes every I/O decision.
6. **What's the dependency management story?** How are libraries installed, versioned, locked, and updated?
7. **What's the deployment story?** Container? Serverless? Binary? Package? This affects how you structure the build.

### Framework-Specific Investigation

When working with a framework you're not deeply familiar with:

```
1. Read the "Core Concepts" or "Architecture" docs section
   (Not the tutorial — the CONCEPTS. This is the mental model.)

2. Understand the request lifecycle / component lifecycle
   → In web frameworks: How does a request flow through middleware to handler to response?
   → In UI frameworks: How do components mount, update, and unmount?
   → In data frameworks: How does data flow from source to transformation to output?

3. Learn the extension points
   → Where is the framework designed to be customized?
   → What's the plugin/middleware/hook system?
   → What are you NOT supposed to override?

4. Read the migration guide for the latest version
   → This tells you what changed and why — which reveals design philosophy.
   → Deprecated features reveal what the community learned was a bad idea.

5. Find one well-maintained open-source project using this framework
   → Read their directory structure, config, and a few core modules.
   → Real projects reveal patterns that tutorials omit.
```

### Evaluating a Library Before Adopting It

Not every npm package / crate / gem / pip package deserves to be a dependency:

```
CHECK                           WHY
─────────────────────────────── ──────────────────────────────────
Last commit date                Abandoned? (> 1 year without commits = caution)
Open issues vs closed           Maintainer responsive? (100 open, 3 closed = red flag)
Download trends                 Growing, stable, or declining adoption?
License                         Compatible with your project? (GPL in a commercial app?)
Transitive dependencies         What else gets pulled in? (1 dep or 200?)
Bundle size / binary impact     Acceptable overhead for what it provides?
Security advisories             Any known vulnerabilities? (check npm audit, Snyk, etc.)
TypeScript/type support         First-class types, DefinitelyTyped, or nothing?
Maintenance health              Bus factor — 1 person or a team/org?
```

**The "should I use this library?" decision tree:**

```
Can I solve this in < 50 lines of my own code?
  YES → Write it yourself. Less dependency risk.
  NO  → Is there a standard library solution?
    YES → Use the standard library.
    NO  → Is this library well-maintained and widely used?
      YES → Adopt it.
      NO  → Can I vendor/fork a minimal implementation?
        YES → Do that.
        NO  → Accept the risk, document it, and set a reminder to revisit.
```

---

## Phase 3: Deep Understanding (Hours to Days)

This is ongoing. You build deep understanding by _working_ in the ecosystem, not by reading about it.

### The Build-to-Learn Approach

```
1. Get the project running locally (or fail trying — the failures teach you the most)
2. Make one small, real change (not a tutorial exercise — a real improvement)
3. Run the tests. Read the test output. Understand the test framework.
4. Read one module deeply — not skimming, but understanding every line.
5. Trace one request/operation from entry to exit through the full stack.
```

### Understanding Through Modification

The fastest way to understand a system is to change it:

```
"What happens if I remove this middleware?" → Run it. See what breaks.
"What does this config option actually do?" → Change it. Observe the difference.
"Is this library actually needed?" → Remove it. See if tests still pass.
"What's this error handling for?" → Let it throw. See the failure mode.
```

This is active learning. It produces understanding 10x faster than reading because you're forcing the system to show you its behavior, not trusting a description of it.

### Reading Source Code of Dependencies

When a library behaves unexpectedly, don't just read the docs — read the source:

```
"Why does this ORM generate that SQL?"
  → Read the query builder. It's usually < 500 lines.

"Why does this HTTP client timeout differently than expected?"
  → Read the timeout handling. Default configs are in the source.

"Why does this framework call my middleware twice?"
  → Read the middleware pipeline. The answer is in the route matching.
```

Most libraries are much simpler than they seem from the outside. The source code is often the best documentation.

---

## Phase 4: Staying Current (Ongoing)

Ecosystems don't stand still. What you learned last month may be outdated.

### What to Monitor

```
FOR ANY ECOSYSTEM YOU ACTIVELY USE:

1. Major version releases → Read the release notes. Breaking changes? New patterns?
2. RFC / proposal process → Where is the ecosystem heading? (TC39 for JS, PEPs for Python, RFCs for Rust)
3. Security advisories     → Is a dependency compromised? New vulnerability class?
4. Toolchain changes       → New build tool that's 10x faster? New linter that catches more bugs?
5. Community shifts         → Is the community moving away from this framework? Toward what?
```

### The Deprecation Detection Mindset

Watch for these signals that a practice or tool is on its way out:

```
SIGNAL                                    MEANING
─────────────────────────────────────── ──────────────────────────
Official docs add "legacy" or "compat"  New approach is preferred
Maintainer recommends a different tool  The tool has been superseded
Framework removes feature in next major  Migrate before you're forced to
New projects in the ecosystem don't use it  Community has moved on
Eslint/clippy rules flag the pattern     The ecosystem considers it a mistake
```

---

## Researching for a Specific Project

When you need to make Copilot (or any AI assistant) deeply effective for a specific project, this is the investigation sequence:

### 1. Stack Identification

```
What language and version?
What framework and version?
What are the KEY dependencies (not the full list — the 5-10 that define the architecture)?
What's the test framework?
What's the build/deploy pipeline?
```

### 2. Convention Discovery

```
Does this project follow the framework's conventions, or has it customized?
What patterns does the team use? (Repository pattern? Service layer? Functional core?)
How is error handling done? (Exceptions? Result types? Error codes?)
How are config and secrets managed?
What's the naming convention? (camelCase? snake_case? Is it consistent?)
```

### 3. Architecture Mapping

```
What are the major modules/packages and their responsibilities?
How do they depend on each other? (dependency graph)
Where are the system boundaries? (API layer, domain layer, infrastructure layer)
What's the data flow for a typical operation?
Where are the pain points? (Mentioned in TODOs, issue tracker, or team discussions)
```

### 4. Ecosystem Intelligence

```
What does a well-structured project in this ecosystem look like?
What are the current recommended tools for testing, linting, formatting?
What patterns has the community moved away from?
What new capabilities are available in the current version that the project might not be using?
Are there better alternatives for any of the current dependencies?
```

### 5. Customization Design

When designing project-specific instructions (copilot-instructions.md, instruction files, etc.):

```
Avoid: Restating generic language conventions (Copilot already knows those)
Avoid: Including information that changes frequently (version numbers, API specifics)
Avoid: Stating preferences as facts ("always use X" without explaining when and why)

Include: The project's architecture and conventions
Include: Domain-specific terminology and business logic
Include: Non-obvious decisions and their rationale
Include: Where the project diverges from ecosystem defaults
Include: References to the project's actual patterns (specific files, specific conventions)
Include: The "why" behind conventions, so they can be intelligently adapted
```

---

## The Research Tools at Your Disposal

You're not limited to static knowledge files. Use every tool available:

```
TOOL                    WHEN TO USE IT
──────────────────────  ──────────────────────────────────────────────
Official documentation  First priority. The primary source. Read before coding.
Package repository      Evaluate libraries (npm, crates.io, PyPI, Maven Central)
GitHub repo             Read source, issues, PRs, release notes, stars, activity
Changelog / RELEASES    What changed between versions? Migration paths?
Web search              Current community discussions, tutorials, blog posts
Fetch web pages         Pull down specific docs pages, API references, examples
MCP tools               Specialized research capabilities (if available)
The codebase itself     grep, find, read the actual code — it's the ground truth
Test suite              Run tests to understand behavior. Tests don't lie.
Error messages          Read them carefully. They often tell you exactly what's wrong.
```

### Research Quality Hierarchy

Not all sources are equal:

```
MOST RELIABLE:
  1. Official documentation (maintained by the project authors)
  2. Source code (the actual implementation)
  3. Test suite (reveals intended behavior and edge cases)
  4. Release notes / changelog (what the authors deemed important to communicate)

MODERATELY RELIABLE:
  5. GitHub issues with maintainer responses
  6. Conference talks / blog posts by core contributors
  7. Well-maintained "awesome" lists

LEAST RELIABLE (verify before trusting):
  8. Tutorial blog posts (may be outdated, simplified, or wrong)
  9. Stack Overflow answers (may solve the wrong problem or use deprecated APIs)
  10. AI-generated content including these knowledge files (may hallucinate or be stale)
```

---

## The Fundamental Insight

**Expertise isn't knowing the answer. It's knowing how to find the right answer, evaluate it, and apply it to the specific situation in front of you.**

A knowledge base full of facts is a crutch. A knowledge base that teaches you how to investigate, evaluate, and reason is a superpower. The goal is the second one.

When you encounter something you don't know:

1. Don't guess — research
2. Don't trust one source — triangulate
3. Don't apply generically — adapt to context
4. Don't stop learning — the ecosystem evolved while you were reading this
