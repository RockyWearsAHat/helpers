---
applyTo: "**"
description: "Universal software design principles. Language-specific rules live in the linter — run lint after edits."
---

# Software Design

Language-specific rules (idioms, unsafe patterns, style) are enforced by the AI linter. Run `lint` after edits; treat its output like compiler errors.

These principles apply everywhere, regardless of language or framework:

1. **Prefer clear, traceable logic over clever compactness.** Code is read far more than written. If the next person has to trace three levels of indirection to understand what a line does, it's wrong.

2. **Keep modules and functions focused; extract cohesive helpers when needed.** A unit that does one thing is testable, nameable, and replaceable. "And" in the description is a split signal.

3. **Validate at boundaries; trust internals.** User input, external APIs, and file I/O need explicit validation and error handling. Internal calls between units you own do not. Never swallow errors — propagate with context or fail explicitly.

4. **Preserve behavior unless change is intentional.** Refactoring and feature work are separate commits. A change that both fixes a bug and renames things is two changes.

5. **Validate before claiming done.** Run diagnostics and tests. A clean lint and a passing test suite are the minimum bar, not a bonus.
