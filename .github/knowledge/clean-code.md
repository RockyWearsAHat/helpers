# Clean Code — Robert C. Martin's Rules

Condensed from _Clean Code: A Handbook of Agile Software Craftsmanship_ (2008).

## General Rules

1. Follow standard conventions of the language/project.
2. Keep it simple — simpler is always better. Reduce complexity as much as possible.
3. Boy Scout Rule: leave the campground cleaner than you found it.
4. Always find root cause. Don't treat symptoms.

## Design Rules

1. Keep configurable data at high levels (constants, config files).
2. Prefer polymorphism to if/else or switch/case chains.
3. Separate multi-threading code from business logic.
4. Prevent over-configurability — sensible defaults first.
5. Use dependency injection for loose coupling.
6. Follow the Law of Demeter — a class should know only about its direct dependencies.

## Naming Rules

1. Choose descriptive and unambiguous names.
2. Make meaningful distinctions — `getActiveAccounts()` not `getAccounts2()`.
3. Use pronounceable, searchable names.
4. Replace magic numbers with named constants.
5. Avoid encodings (Hungarian notation, member prefixes).
6. Names should describe side effects: `createOrReturnCachedUser()` not `getUser()`.

## Function Rules

1. Small — functions should be tiny. 20 lines is a lot.
2. Do one thing, and do it well.
3. Use descriptive names — long descriptive names > short cryptic names.
4. Prefer fewer arguments. Zero is best, three is the practical max.
5. Functions should have no side effects (or their names must declare them).
6. Don't use flag arguments — split into two functions instead.

## Comment Rules

1. Always try to explain yourself in code first.
2. Don't be redundant — `i++; // increment i` is noise.
3. Don't add obvious noise or journal comments (that's what version control is for).
4. Use comments to explain **intent** (the _why_), not _what_ the code does.
5. Use comments as warnings of consequences.
6. TODO comments are acceptable but should be cleaned regularly.
7. Don't comment out code — delete it. Version control remembers.

## Source Code Structure

1. Separate concepts vertically with blank lines.
2. Related code should appear vertically dense (close together).
3. Declare variables close to their usage.
4. Dependent functions should be vertically close, caller above callee.
5. Similar functions should be grouped together.
6. Functions should be ordered top-down (newspaper metaphor: high-level overview → details).
7. Keep lines short (< 120 chars).
8. Don't use horizontal alignment — it becomes misleading when types change.
9. Use white space to associate related things and disassociate weakly related things.
10. Don't break indentation for short blocks — consistency matters.

## Objects and Data Structures

1. Hide internal structure.
2. Prefer data structures (no behavior) or objects (behavior, hidden data). Hybrids are the worst of both.
3. Small number of instance variables.
4. Base class should know nothing about its derivatives.
5. Better to have many small functions than to pass behavior-selecting flags.
6. Prefer non-static methods — they can be polymorphic.

## Tests

1. One assert per test (conceptually — one concept per test).
2. Fast, independent, repeatable, self-validating, timely (F.I.R.S.T.).
3. Readable — tests are documentation.
4. Tests should be as clean as production code.
5. Sufficient coverage — test the things that could break. Not just the happy path.

## Code Smells

1. **Rigidity**: one change causes cascade of changes.
2. **Fragility**: code breaks in places unrelated to the change.
3. **Immobility**: code is hard to reuse in other contexts.
4. **Needless complexity**: structures that aren't currently useful.
5. **Needless repetition**: duplicated code that could be unified.
6. **Opacity**: code that is hard to understand.

---

_Source: "Clean Code" by Robert C. Martin, summarized from the community-maintained reference (github.com/ryanmcdermott/clean-code-javascript and wojteklu/clean-code gist)_
