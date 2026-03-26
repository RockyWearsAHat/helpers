# Agent-Computer Interface (ACI) Design — Tool Formatting, Poka-Yoke & Parameter Excellence

## Overview

The interface between AI agents and tools determines whether agents can productively act on the world or get stuck flailing at misunderstood APIs. **Agent-Computer Interface (ACI) design** is the systematic choice of tool representations, parameter formats, error messaging, and documentation that enable agents to invoke tools reliably.

Poor ACI design manifests as:
- Agents hallucinating tool names
- Incorrect parameter types passed to tools
- Agents using tools in wrong order (tool dependencies)
- Agents invoking tools with impossible parameter combinations
- Agents unable to recover from tool errors

Good ACI design makes correct usage obvious and wrong usage impossible, applying principles from **poka-yoke** (mistake-proofing in manufacturing) to software tools.

## The ACI Design Dimension: Diff vs. Rewrite

When a tool modifies code or state, how should it do it?

### Diff-Based (Partial Change)

Diff tools show the specific lines changing:

```
[Tool Call: EditFile]
File: src/components/Button.tsx
Diff:
  const Button = ({ label, onClick }) => {
-   return <button onClick={onClick}>{label}</button>
+   return <button className="btn-primary" onClick={onClick}>{label}</button>

[Tool Output]
✓ Applied change. File updated. 3 other lines unchanged.
```

**Advantages**:
- Shows exactly what's changing (easy to understand)
- Preserves surrounding code (no accidental loss)
- Easy to review (clear diff)
- Agents can reason about localized changes

**Disadvantages**:
- Requires precise line numbers and context (easy to get wrong)
- Large replacements generate massive diffs (tokens wasted)
- If context shifts (file edited elsewhere), diff breaks

### Rewrite-Based (Full Replacement)

Rewrite tools replace entire file or large sections:

```
[Tool Call: WriteFile]
File: src/components/Button.tsx

const Button = ({ label, onClick, variant = "primary" }) => {
  const styles = {
    primary: "bg-blue-600 text-white px-4 py-2",
    secondary: "bg-gray-200 text-black px-4 py-2",
  };
  return <button className={styles[variant]} onClick={onClick}>{label}</button>;
};
```

**Advantages**:
- Agent writes full valid code (no partial state confusion)
- No line number coupling (file edits elsewhere don't break)
- Easier to reason about correctness (whole file is valid)

**Disadvantages**:
- Can lose surrounding code if agent doesn't preserve it
- Token-heavy for large files
- Reviews are harder (shows entire file, not just change)

### Hybrid: Diff for Small Changes, Rewrite for Large

Best practice:

```
Rule 1: If change is < 50 lines and clearly localized → use diff
Rule 2: If change > 50 lines or spans multiple sections → use rewrite
Rule 3: On any error (diff conflict) → fall back to rewrite
```

Anthropic and SWE-bench experience (a benchmark of AI coding on real GitHub issues) shows agents perform best with **diffs for surgical changes and rewrites for substantial refactoring**.

## Tool Parameter Design

### Absolute vs. Relative Paths

**Bad (ambiguous)**:
```
"path": "components/Button.tsx"
```
Is this workspace-relative? Project-relative? From current working directory? The agent must guess.

**Good (explicit)**:
```
"path": "/Users/alice/project/src/components/Button.tsx"  // Absolute
```

Or include a schema:
```
"path": "/Users/alice/project/src/components/Button.tsx",
"path_format": "absolute_posix",
"workspace_root": "/Users/alice/project"
```

**Principle**: Absolute paths eliminate ambiguity. If the agent writes an incorrect path, the error is immediate and specific ("file not found"), not silent failure.

### Structured Parameters vs. Flags

**Bad (ambiguous options)**:
```
"command": "format_code --style=prettier --line-width=80 --write"
```
Agents often:
- Forget flags
- Use wrong flag names (--lineWidth vs --line-width)
- Pass conflicting flags

**Good (structured schema)**:
```
{
  "action": "format_code",
  "style": "prettier",  // enum: ["prettier", "black", "gofmt"]
  "line_width": 80,      // integer: 40-120
  "write_to_disk": true  // boolean
}
```

With a schema, tools enforce:
- No typos (unknown keys rejected)
- Correct types (line_width must be integer)
- Valid values (style must be one of enum options)

Agents make fewer mistakes with structured input.

### Validation and Clear Error Messages

**Bad error message**:
```
Error: Invalid request
```
Agent doesn't know what went wrong. With compaction trimming history, the agent may forget what it just tried.

**Good error message**:
```
Error: Invalid file path
Reason: Path "/src/Button.tsx" is relative. Tool requires absolute paths with workspace root.
Format: "/workspace_root/relative/path"
Example: "/home/user/project/src/Button.tsx"

Your path: "/src/Button.tsx"
Suggestion: Prepend workspace root. Did you mean "/home/user/project/src/Button.tsx"?
```

Clear errors let agents recover without re-reading documentation.

## Poka-Yoke Principles for ACI

Poka-yoke ("mistake-proofing") comes from Toyota manufacturing: design processes so mistakes are physically impossible.

### Principle 1: Constrain Tool Parameter Space

Make wrong inputs impossible, not merely discouraged.

**Before (agent can do wrong)**:
```
"model": "string"  // Agent might write: "gpt-5", "claude-2", "my-custom-model"
```

**After (wrong inputs impossible)**:
```
"model": {
  "type": "enum",
  "values": ["claude-opus-4.6", "claude-sonnet-4.5", "claude-haiku-3.5"]
}
```

If the agent specifies an unsupported model, the tool rejects it immediately, not after 5 minutes of computation.

### Principle 2: Explicit Required vs. Optional

**Before (ambiguous)**:
```
Tool: search
Params: query, max_results, filters, include_metadata, sort_by
```
Agent doesn't know which are required. It guesses; the tool fails randomly.

**After (crystal clear)**:
```
Tool: search
Required: query (string)
Optional: 
  - max_results (integer, default 10, range 1-100)
  - filters (array of filter objects)
  - include_metadata (boolean, default false)
  - sort_by (enum: "relevance" | "date" | "title", default "relevance")
```

The tool documents exactly what happens if an optional param is omitted (uses default).

### Principle 3: Order Dependencies Explicitly

**Before (confusing)**:
```
Agent calls: WriteFile(path, content)
        then: InvokeFormatter(path, style)

If agent calls in reverse order, file doesn't exist yet and fails mysteriously.
```

**After (explicit dependencies)**:
```
Tool groups:
- File creation group: WriteFile, CreateDirectory
- File modification group: EditFile, DeleteFile (requires files to exist first)
- File analysis group: ReadFile, ListFiles (read-only; no ordering requirement)

Agent prompting includes: "Use File Creation tools before File Modification tools."

Tool error on out-of-order use:
"Error: File does not exist. Must create file with WriteFile before calling EditFile."
```

Agents learn the dependency order quickly and start using tools correctly.

### Principle 4: Atomic vs. Sequential Operations

**Bad (non-atomic)**:
```
Agent calls:
1. ReadFile(config.json)
2. Parse JSON
3. WriteFile(config.json, new_value)

Between steps 2 and 3, another process edits config.json → data corruption.
```

**Good (atomic)**:
```
Tool: UpdateJsonField
Params: file, field_path (dot-notation: "database.host"), new_value
Atomic operation: Read → validate → write (locks to prevent race conditions)
Error handling: If file changed between read and write, tool detects and fails with clear message.
```

For multi-step operations, provide compound tools that handle atomicity.

### Principle 5: Undo/Rollback Capability

**Without**:
```
Agent writes code with a critical bug.
Code is committed. 
Fixing requires manual revert and recommit (expensive tokens).
```

**With**:
```
Tool: CommitWithRollback
Params: message, files
Returns: commit_id (for later rollback)

Later agent can do: RollbackCommit(commit_id)
Tool automatically: Reverts changes, updates branch history, leaves audit trail.
```

Agents take more risks and recover faster when they know failures are reversible.

## Tool Documentation as Prompt Engineering

Tool documentation directly shapes how agents use tools. Specific wording steers behavior.

### Example: File Edit Tool

**Poorly documented**:
```
EditFile: Edit a file
Parameters: file, content
Description: "Modifies file content"
```

Vague description leaves agent guessing. It might:
- Completely overwrite file (losing surrounding code)
- Try to append (wrong for most edits)
- Format the diff incorrectly

**Well documented**:
```
EditFile: Make a targeted change to a file

When to use: Changing a few lines within a larger file. For full rewrites, use WriteFile.

Parameters:
  file (string, required): Absolute path to file
  old_text (string, required): Exact text to replace (must match exactly, 
    including whitespace)
  new_text (string, required): Replacement text
  
Behavior:
  1. Finds old_text in file
  2. Replaces single occurrence with new_text
  3. Returns success + line number of change
  4. If old_text appears multiple times, fails with error message showing all matches
  5. If old_text not found, fails with nearest-match suggestions

Requirements for old_text:
  - Must include surrounding context (>= 3 lines before and after the change)
  - Must be large enough to be unique in file (avoid matching multiple locations)
  
Error cases:
  - "Ambiguous match": old_text appears 3+ times; must be more specific
  - "Not found": old_text doesn't match; suggestions provided
  - "Whitespace mismatch": Tabs vs spaces differ from file; copy-paste old_text from file

Examples:
# Example 1: Add a className to JSX element
old_text:
  return (
    <button onClick={handleClick}>
      Click me
    </button>
  )

new_text:
  return (
    <button className="btn-primary" onClick={handleClick}>
      Click me
    </button>
  )

# Example 2: Fix import path
old_text: import Button from "./Button"
new_text: import Button from "../components/Button"
```

With this documentation, agents:
- Know when to use EditFile vs. WriteFile
- Understand that old_text must be unique
- Know to include surrounding lines
- Recover instantly from ambiguity errors

## Markdown vs. JSON for Tool Output

**Markdown output** (human-readable):
```
Tool Result:
✓ File edited successfully
File: src/Button.tsx
Lines: 42-44 modified
Change: Added className="btn-primary"
```

**JSON output** (machine-readable):
```json
{
  "status": "success",
  "file": "/workspace/src/Button.tsx",
  "lines_modified": [42, 43, 44],
  "old_lines": ["<button onClick={...}>", "  Click me", "</button>"],
  "new_lines": ["<button className=\"btn-primary\" onClick={...}>", "  Click me", "</button>"],
  "token_cost": 145
}
```

**Best practice**: Provide both. Format output as JSON (for agents to parse systematically) with human-readable fields (for debugging).

## SWE-Bench Lessons: What Agents Struggle With

From the SWE-bench benchmark (evaluating AI agents on real GitHub issues):

### Common Tool Usage Mistakes

1. **Path confusion**: Agents write relative paths when absolute are needed; tools silently fail
   - **Fix**: Require absolute paths; error immediately on relative
   
2. **Search scope explosion**: Agents search entire repo when they should search one file
   - **Fix**: Separate SearchFile (single file, fast) from SearchRepo (all files, expensive)
   
3. **Test runner invocation errors**: Agents forget test framework nuances
   - **Fix**: Provide RunTests(test_framework, test_path) not raw bash commands
   
4. **Commit without verification**: Agent commits broken code
   - **Fix**: Require passing test before commit is allowed
   
5. **Parameter type mismatches**: Agent passes list when tool expects string
   - **Fix**: Strict type validation with clear "expected X, got Y" errors

### Agents That Succeed

Best-performing agents (those that solve the most GitHub issues):
- Reread tool errors carefully (don't ignore them)
- Ask clarifying questions when docs are ambiguous (via error messages)
- Test frequently (before committing)
- Use atomic tools (RunTests, CommitWithVerification) rather than raw shell commands

This maps directly to **ACI design quality**: better-designed tools → higher agent success.

## Checklist for Good ACI Design

- [ ] **Paths are absolute** (no ambiguity)
- [ ] **Parameters are strictly typed** with clear enum values and ranges
- [ ] **Required vs. optional is explicit** with defaults documented
- [ ] **Error messages are specific** with examples and recovery suggestions
- [ ] **Tool dependencies are documented** (what tools to call in what order)
- [ ] **Output is JSON + human-readable**
- [ ] **Operations can be undone** (rollback/revert available)
- [ ] **Atomicity is guaranteed** for multi-step operations
- [ ] **Documentation includes examples** of correct usage
- [ ] **Large file operations support diff or rewrite mode**
- [ ] **Tool naming is unambiguous** (no similar names; clear verbs)

## See Also

- [LLM Function Calling](genai-function-calling.md) — Structured output, tool schemas, parameter design
- [Agent Architecture](genai-agent-architecture.md) — How tools fit into agent loops
- [Agent Evaluation Patterns](genai-agent-evaluation-patterns.md) — Testing tools (Playwright, API verification)

---

**Sources**: Anthropic Engineering research (2026); SWE-bench: Software Engineering Benchmark (Princeton, OpenAI), https://www.swebench.com; Poka-yoke (Shigeo Shingo); VS Code Extension Tools API documentation; Anthropic "Prompt Engineering your Tools" research