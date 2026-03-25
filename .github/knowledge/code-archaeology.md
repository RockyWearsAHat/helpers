# Code Archaeology — Reading Unfamiliar Codebases

## The Mindset
You don't need to understand every line. You need to build a mental map: where things happen, how data flows, what the key abstractions are. Understanding comes in layers.

## Phase 1: Reconnaissance (Don't Read Code Yet)

### 1. Read the README and docs
- What does this project do?
- How do you build and run it?
- What are the main concepts/domain terms?

### 2. Look at the project structure
```bash
# Get the lay of the land
find . -type f -name "*.py" | head -30     # What language?
wc -l $(find . -name "*.py") | tail -1     # How big?
ls -la                                      # Top-level files
tree -L 2 -I 'node_modules|__pycache__|.git|vendor|target'
```

### 3. Read the dependency file
```
package.json → what frameworks/libraries    Python: requirements.txt, pyproject.toml
Cargo.toml → Rust crate ecosystem          Go: go.mod
build.gradle / pom.xml → Java ecosystem    Gemfile → Ruby gems
```
Dependencies tell you what the project relies on. If you see `express`, it's a web server. If you see `sqlalchemy`, there's a database. If you see `pytest`, there are tests.

### 4. Check the entry points
```bash
# Find main/entry files
grep -rl "if __name__" .                   # Python
grep -rl "func main()" .                   # Go
grep -rl "public static void main" .       # Java
cat package.json | jq '.main, .scripts'    # Node.js
```

### 5. Read the test names (not bodies)
```bash
grep -rn "def test_\|it(\|describe(\|#\[test\]" tests/ --include="*.py" --include="*.js" --include="*.rs" | head -30
```
Test names are a specification written by someone who understood the code. They tell you what the code is supposed to do.

## Phase 2: Mapping the Architecture

### Follow the Request Path
For web services, trace a single HTTP request:
```
1. Where does the router/handler live?
2. What middleware runs (auth, logging, validation)?
3. Where's the business logic?
4. How does data reach the database?
5. How is the response formatted?
```

### Find the Abstractions
Every codebase has a few key abstractions. Find them:
```bash
# Most-imported files are architectural pillars
grep -rh "^import\|^from\|require(" src/ | sort | uniq -c | sort -rn | head -20

# Largest files often contain core logic (or need refactoring)
find . -name "*.py" -exec wc -l {} + | sort -rn | head -10

# Classes/interfaces with many implementations
grep -rn "class.*:\|interface " src/ | head -20
```

### Build a Dependency Graph (Mental or Visual)
```bash
# Python: pydeps, import-linter
# JavaScript: madge
npx madge --image graph.svg src/

# Any language: grep for imports
grep -rn "from.*import\|import " src/ | awk -F: '{print $1, $NF}' | sort
```

### Identify the Data Model
```bash
# Find database models/schemas
grep -rn "class.*Model\|CREATE TABLE\|Schema(\|@Entity\|#\[derive.*Serialize\]" . --include="*.py" --include="*.java" --include="*.rs" --include="*.sql"

# Find API types/DTOs
grep -rn "interface.*Response\|type.*Request\|dataclass\|@dataclass" . 
```

## Phase 3: Understanding Specific Code

### The "Five Whys" of Code Reading
1. **What** does this function do? (Read the signature, name, return type)
2. **Who** calls it? (Search for references)
3. **What** does it call? (Follow the function chain)
4. **What** data does it touch? (Parameters, globals, database)
5. **Why** does it exist? (Check git blame for the commit that introduced it)

### Git as an Archaeology Tool
```bash
# Who wrote this and when?
git blame file.py

# When was this function introduced?
git log -S "def problematic_function" --oneline

# What changed in this file recently?
git log --oneline -20 -- path/to/file.py

# What did the codebase look like when this was added?
git show abc1234:path/to/file.py

# What was the commit message / PR context?
git log --format="%h %s%n%b" abc1234 -1

# Find all changes related to a feature
git log --all --grep="feature-name" --oneline
```

### Using the Debugger as a Reader
Instead of reading code linearly, run it and observe:
```
1. Set a breakpoint at the entry point
2. Step through execution
3. Watch variables change
4. Note the actual call sequence (not what you assumed)
```

### Rubber Duck Direction
Explain what you think the code does to someone (or to yourself). Where you stumble is where your understanding breaks down. Investigate those spots.

## Phase 4: Working With Legacy Code

### Michael Feathers' Legacy Code Definition
> "Legacy code is simply code without tests."

### The Legacy Code Change Algorithm
1. **Identify change points**: Where in the code do you need to make changes?
2. **Find test points**: Where can you observe behavior?
3. **Break dependencies**: Make the code testable (extract interface, inject dependency)
4. **Write characterization tests**: Tests that document current behavior (even if "wrong")
5. **Make changes**: Now you have a safety net
6. **Refactor**: Improve structure while tests keep passing

### Characterization Tests (Tests That Document Reality)
```python
def test_what_calculate_price_actually_does():
    """I don't know if this is correct, but it's what the code does NOW."""
    result = calculate_price(item={"qty": 3, "unit_price": 10.0}, discount_code="SAVE20")
    assert result == 25.5  # Discovered by running the code
    # Now if we change calculate_price and this breaks, we'll know we changed behavior
```

### Strangler Fig Pattern (Incremental Replacement)
```
1. Build new code alongside old code
2. Route some traffic/calls to new code
3. Gradually move more functionality
4. Eventually, old code has no callers → remove it

NOT: Big-bang rewrite (the Second System Effect — historically fails)
```

## Code Smells That Reveal Architecture

| What You See | What It Tells You |
|-------------|-------------------|
| Huge files (1000+ lines) | Likely God Objects or accumulated features without refactoring |
| Many tiny files with one function each | Over-abstracted, possibly Java-brain |
| Deep directory nesting | Either well-organized domain or over-engineered hierarchy |
| Files named `utils.py`, `helpers.js`, `misc.go` | Dumping ground — look here for core logic that outgrew its home |
| Lots of `TODO`/`FIXME`/`HACK` comments | Technical debt map — read these first |
| Commented-out code blocks | Fear of deleting (version control handles this) |
| Inconsistent naming conventions | Multiple authors over time, or no style guide |
| `config.py` that's 500 lines | Configuration creep — business logic disguised as config |

## Tools for Code Exploration

### Static Analysis / Navigation
```bash
# ctags — jump-to-definition for any editor
ctags -R --exclude=.git --exclude=node_modules .

# Sourcegraph — web-based code search across repos
# https://sourcegraph.com

# Understand (SciTools) — architecture visualization
# CodeScene — behavioral code analysis (hotspots, coupling)
```

### Quick Metrics
```bash
# Lines of code by language
cloc .

# Most-changed files (hotspots for bugs)
git log --format=format: --name-only | sort | uniq -c | sort -rn | head -20

# Files that change together (coupling)
# If A and B always change in the same commit, they're coupled
git log --format=format: --name-only | awk 'NF' | sort | uniq -c | sort -rn

# Complexity metrics
radon cc src/ -a -s  # Python: cyclomatic complexity
```

## The Art of Asking Good Questions

When you're stuck on unfamiliar code, ask:
1. **"What's the simplest path through this?"** — Ignore error handling, caching, metrics. Follow the happy path.
2. **"What would I name this?"** — If your name differs from the actual name, one of you misunderstands the responsibility.
3. **"What breaks if I delete this?"** — (Mentally or with tests.) Reveals what the code actually does.
4. **"When was this last changed and why?"** — Recent changes are more likely to be relevant.
5. **"Who owns this?"** — `git shortlog -sn -- path/to/dir` → find the expert to ask.

---

*"Programs must be written for people to read, and only incidentally for machines to execute." — Harold Abelson. This is aspirational. Most code is written for yesterday's deadline.*
