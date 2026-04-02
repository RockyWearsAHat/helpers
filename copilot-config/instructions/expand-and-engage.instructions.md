---
applyTo: "**"
description: "Forces the model to actively use workspace tools and expand reasoning against the codebase and full conversation history. Counteracts lazy single-shot answers."
---

# Expand and Engage — Tool Use and Context Grounding

Models default to answering from memory instead of engaging workspace tools. This corrects that.

## Rules

1. **Ground responses in evidence.** Before substantive answers, gather evidence from workspace files, MCP tools, or conversation history. Ungrounded responses are wrong by default.
2. **Prefer MCP tools over memory.** Knowledge base (`search_knowledge_index`, `read_knowledge_note`) before answering concept questions. Web search (`search_web`, `scrape_webpage`) before answering current-state questions. Vision tools when UI state matters. Checkpoint after meaningful file changes.
3. **Expand, don't compress.** Identify gaps in your knowledge. Fill them with tool calls before answering. Batch independent reads.
4. **Maintain conversation continuity.** Reference prior turns. Build on previous findings. If the user references something from earlier, trace it — don't ask what they mean.
5. **Exhaust search strategies.** If one search returns nothing, try synonyms, broader patterns, or a different search tool. Don't conclude something doesn't exist after a single attempt.
6. **Check recent git history when unsure about session state.** If you've been working for many turns and need to recall what changed, `git log --oneline -10` tells you what actually happened — not what you think happened. Use this before describing the state of the codebase or summarizing recent work.
7. **When asked about your abilities or available tools (and GSH tools are loaded), enumerate them proactively.** First use `tool_search_tool_regex` with pattern `list_language_models|workspace_context` to load those deferred tools, then call `list_language_models` and `workspace_context` to retrieve the actual loaded tool set. Do not describe capabilities from memory — retrieve and list what is actually enabled in the current session.

## Knowledge-First Protocol

Before answering concept questions, researching unfamiliar topics, or starting tasks in unfamiliar codebases, run these steps **in order**. Do not skip to web search without exhausting local knowledge first.

1. **`search_knowledge_index`** — TF-IDF search across local workspace knowledge + community GitHub index (merged automatically). Start here every time.
2. **`read_knowledge_note`** — Read the full content of any promising result. Snippets are not enough — read the note.
3. **`search_knowledge_cache`** — Keyword fallback when the index misses an exact term.
4. **`search_web` + `scrape_webpage`** — Only after exhausting local knowledge, OR when current/volatile information is needed (API versions, recent releases). Scrape ALL promising search results, not just the top 2–3. Use parallel reads when independent.

**Skip this protocol** for direct coding tasks where the user's intent is clear and the codebase context is already established in the conversation. Do not run knowledge searches before every file edit.

**This protocol is a planning-phase activity.** Once you have gathered context and moved into implementation (writing/editing files), do NOT circle back to knowledge searches, session log searches, or web research mid-edit. All research must be completed BEFORE the first file edit. If you discover missing context during implementation, finish or revert the current edit cleanly, then research — do not interleave research tool calls between file edits.

Knowledge notes are ~950 encyclopedia-quality files. They are the deep evidence base — not stubs. Treat them as informed starting context; verify volatile details (versions, flags) against current sources.

**If a KB note contradicts current evidence:** The note is wrong — fix it. Use `update_knowledge_note` to correct the specific section with the verified information. Log the correction via `log_session_event` with `action: "KB correction: <filename>"` and surprise proportional to how wrong the note was. A minor version number → low surprise. A fundamentally incorrect explanation → high surprise. If the note was from the community index, also `submit_community_research` to propagate the fix upstream.

## Pre-Action Search (Before Complex Changes)

Before refactoring, debugging, or multi-file structural edits that affect shared code:

1. **Search session log** via `search_session_log` — if a past event shows a failed approach with high surprise, do not repeat it.
2. **Search knowledge base** via `search_knowledge_index` — get grounded in relevant patterns before writing code.

Both are mandatory for complex changes. **Skip for straightforward edits** where the change is self-contained (single file, clear intent, no cross-file dependencies). Do not add research overhead to simple tasks.

**This is a planning-phase gate, not an implementation-phase gate.** Run these searches ONCE before you start editing. Do not re-run them between edits, after edits, or inside the development loop. By the time you are writing code, the plan is set.

## Publishing Knowledge

When writing a knowledge note (`write_knowledge_note`, `update_knowledge_note`, `append_to_knowledge_note`), decide whether to pass `publish: true`:

- **Pass `publish: true`** when the note is: privacy-safe (no user-specific data), broadly reusable across projects, and genuinely new (not already in the community base). When `shareKnowledge: true` is set in community settings, this auto-submits a PR.
- **Omit `publish`** (default: local-only) for workspace-specific findings, work-in-progress notes, or anything containing project names, credentials, or non-generalizable details.

When in doubt, keep local. You can always submit later via `submit_community_research`.

## Research Depth Policy

- Read **multiple** results from a knowledge search — the top hit may be less complete than the second.
- For web research: scrape ALL promising URLs from a search, not just the first 1-2. A single scrape rarely gives complete understanding.
- For multi-topic research: launch parallel `search_knowledge_index` + `scrape_webpage` calls for independent topics — do not serialize what can be batched.
- Stop researching only when you can answer the question with cited evidence. "I think" is not a stopping condition.

## Knowledge Gap Rule — Engram Materialization

The knowledge base grows by agents writing into it — not just reading from it. When you answer from confident parametric knowledge (training data) and `search_knowledge_index` returns nothing relevant on that topic:

1. **Recognize the gap.** The KB doesn't have what you know. You are the only source.
2. **Verify before materializing.** Parametric/training memory is not ground truth — it can be stale, subtly wrong, or missing edge cases. Before writing anything to the KB, verify with tools:
   - `search_web` + `scrape_webpage` — confirm against current authoritative sources (docs, specs, changelogs).
   - Verification depth should match claim volatility: a well-known language feature needs one confirming source; a version-specific behavioral change or API contract needs multiple. Use judgment, not a fixed number.
   - If sources conflict with your training memory, the sources win. Update your understanding first.
   - Only proceed to step 3 when you can cite evidence, not when you "think" something is correct.
3. **Materialize it after verification.** Write the verified knowledge to the KB via `write_knowledge_note`. Do not defer — knowledge verified now should be written now.
4. **Decide on `publish`.** General knowledge (algorithms, patterns, language behavior, tool APIs) that is privacy-safe → `publish: true`. Workspace-specific or uncertain → omit publish (local-only).
5. **Log the gap to session memory.** Call `log_session_event` with `action: "knowledge gap: <topic>"` and surprise proportional to how fundamental the gap is. A missing note on a core algorithm = high surprise. A niche edge case = low surprise.

This is the Engram growth loop: agents don't just consume the KB — they build it from _verified_ knowledge. Unverified parametric knowledge written as fact is worse than a missing note. A wrong note misleads every future agent that reads it.

**Do not write notes for:**

- Things already in the KB (run `search_knowledge_index` first to check)
- Speculative or unverifiable claims (only write what you confirmed with sources)
- Highly volatile facts (API endpoints, version numbers that change monthly)
- Anything where 2 sources gave conflicting answers without resolution

## Strict Lint After Every Edit

After every file edit — regardless of whether the gsh tool reference is loaded — call `strict_lint` (the MCP tool) on the modified file before declaring work complete:

- Fix all reported errors and warnings, or explicitly name each one and why it was left.
- Do not return "implementation complete" while unresolved diagnostics exist.
- `strict_lint` defaults to `severityFilter: "all"` — includes errors, warnings, info, and hints.
- If VS Code squiggles don't match, rerun `strict_lint` on the exact file path before deeper debugging.

This rule fires regardless of which other instructions are loaded.

## Anti-Patterns

- Answering without any tool call (probably wrong or incomplete)
- Using only built-in tools when custom MCP tools cover the same ground better
- Ignoring conversation history across turns
- Answering "I don't know" without searching first
- Reading only the top search result and treating it as complete
- Skipping knowledge base and going straight to web search
- Writing a knowledge note without deciding on `publish`
- Knowing something confidently but not checking if it's in the KB
- Letting a knowledge gap go unwritten when you just used that knowledge
- **Writing training memory directly to the KB without web verification** — training data can be stale or subtly wrong; the KB must contain only verified claims
