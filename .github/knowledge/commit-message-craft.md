# Commit Message Craft — Behavior Over Code

Synthesized from Linux kernel guidelines, Go/Tailscale conventions, Simon Tatham's
guide, Chris Beams' "How to Write a Git Commit Message", Tim Pope's original 50/72
note, Thoughtbot's analysis, and Michael Lynch's "Refactoring English" material.

## The Core Principle

The diff tells you WHAT code changed. Only the commit message can tell you WHY,
and what the system does differently now. A commit message earns its existence
by communicating something the patch alone cannot.

## The Behavior Test

Every subject line should pass this test: **Could a user, tester, or product
manager understand this without reading code?**

Bad (code-focused):
- "Update handleCheckpoint() to call generateAiCommitMessage()"
- "Refactor prompt construction in git-shell-helpers-mcp"
- "Add context parameter to CHECKPOINT_TOOL inputSchema"
- "Modify extension.js chip click handler"

Good (behavior-focused):
- "Checkpoint commits now generate their own message from the diff"
- "Stop MCP status chip from showing stale state after restart"
- "Let callers pass context hints to improve commit messages"
- "Clicking the MCP chip now opens the right panel for its state"

## What The Best Projects Do

### Linux Kernel (the gold standard)
- "Describe user-visible impact" — first thing in the guidelines
- "Quantify optimizations and trade-offs" — if you claim improvement, show numbers
- "Describe your changes in imperative mood, as if giving orders to the codebase
  to change its behaviour" — not reporting what you did, commanding what happens
- Side effects mentioned explicitly: "This also means X will now Y"
- The body reads like a mini design document, not a changelog

### Go Project
- Subject completes: "this change modifies Go to ___"
- Body explains the situation that motivated the change
- Links to issues with `Fixes #N` or `Updates #N`

### Tailscale
- Same Go/Linux hybrid style
- Imperative verb right after the prefix colon
- Every non-cleanup commit links to a tracking issue
- "Does it change behavior? Not a cleanup. File a bug to track why."

### Bitcoin Core (Pieter Wuille's exemplary commit)
```
Simplify serialize.h's exception handling

Remove the 'state' and 'exceptmask' from serialize.h's stream
implementations, as well as related methods.

As exceptmask always included 'failbit', and setstate was always
called with bits = failbit, all it did was immediately raise an
exception. Get rid of those variables, and replace the setstate
with direct exception throwing (which also removes some dead code).

As a result, good() is never reached after a failure (there are
only 2 calls, one of which is in tests), and can just be replaced
by !eof().

fail(), clear(n) and exceptions() are just never called. Delete them.
```
Note: the subject says WHAT behavior changes. The body explains the reasoning
chain — why things were the way they were, why the change is safe, what falls out
as a consequence. Zero mention of file names in the subject.

## The Hierarchy of Information

From Simon Tatham's guide, a commit message should include (in descending priority):

1. **User-visible behavior change** — what the program does differently now
2. **Why** — the motivation, the problem, the situation before
3. **Side effects** — what else changed as a consequence
4. **What isn't in the patch** — why you didn't do other obvious things
5. **Call out the interesting part** — if 90% of the diff is mechanical, point to
   the 10% that matters
6. **Describe the structure** — for large patches, give a reading guide
7. **Links to external context** — issues, discussions, related commits

Not all are needed every time. A one-line typo fix needs only one sentence.
A structural refactor needs items 1, 2, 3, 5.

## Subject Line Rules (universal consensus)

- Imperative mood: "Fix", "Add", "Stop", "Let", not "Fixed", "Adding", "Stops"
- Complete the sentence: "If applied, this commit will ___"
- ≤ 50 chars ideal, 72 hard max
- Capitalize first word, no trailing period
- Describe the EFFECT, not the mechanism

## Body Writing Principles

- First sentence: the situation / problem that existed before
- Middle: what you did and why (not how — the code shows how)
- End: consequences, side effects, what someone should know going forward
- No corporate filler, no section headers like "What changed:" / "Why:"
- Wrap at 72 characters
- If quoting a number (speed, count, size), include it — don't say "faster"
- If referencing another commit, include its short hash AND subject

## The Anti-Patterns (what AI-generated messages get wrong)

1. **Narrating the diff** — "Updated X, modified Y, changed Z" is just a worse
   version of `git diff --stat`. Worthless.
2. **Listing file names** — "Changes to extension.js and mcp-server.js" — the
   diff already shows this.
3. **Restating the subject in different words** — subject says "Fix crash on
   startup", body says "Fixed the crash that happened on startup". Dead weight.
4. **Pseudo-structure** — "What changed: / Why: / Impact:" headers make it look
   organized but add no information. Write prose.
5. **Anthropomorphizing** — "The system now gracefully handles..." — no. Code
   doesn't have grace. Say what it does.
6. **Vague verbs** — "Improve", "Enhance", "Update", "Refactor" — these are
   empty without specifics. Improve HOW? Refactor to WHAT END?
7. **Missing the why** — the hardest one. If the message doesn't answer "why
   was this change worth making?", it failed.

## Writing Like A Person, Not A Machine

The best commit messages sound like someone explaining the change to a colleague
at a whiteboard:

"So the MCP status chip was showing 'Initializing' even after the server was
already running. Turns out the state was derived from the binary check, but we
never updated it after auto-start. Now the chip reads the actual server state
on each panel render."

That's the voice. Direct, specific about the problem, clear about the fix,
grounded in observable behavior. No "Refactored state management in status chip
component to derive from server lifecycle."

## Scale The Message To The Change

- **Typo/trivial**: Subject only. "Fix typo in installer help text"
- **Small fix**: Subject + one sentence. "Stop spinner from persisting after Ctrl-C"
- **Feature/rework**: Subject + paragraph explaining the situation and approach
- **Large change**: Subject + narrative with the reasoning chain, possibly bullets
  for distinct sub-changes grouped by BEHAVIOR not by FILE