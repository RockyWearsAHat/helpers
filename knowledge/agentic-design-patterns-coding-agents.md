# Agentic Design Patterns for Coding Agents — Best Practices from Gulli's 21-Pattern Framework

Source: *Agentic Design Patterns: A Hands-On Guide to Building Intelligent Systems* by Antonio Gulli (Google, Engineering Director, Office of the CTO). 424 pages, 21 code-backed patterns. Free PDF pre-print published 2025. Springer, ISBN 978-3-032-01402-3.

This note distills the 21 agentic design patterns into actionable guidance for **coding agents** running inside IDE-integrated AI systems (GitHub Copilot, Cursor, Claude Code, Windsurf, etc.). Each pattern is mapped to its concrete application in the coding-agent context.

---

## The Core Thesis

AI agents are software systems, not magic boxes. They require the same disciplined engineering as any distributed system: repeatable architecture, transactional safety, structured communication, and testable behavior. The 21 patterns provide the architectural vocabulary for building agents that are reliable, debuggable, and production-ready.

**Key insight from Gulli**: "To build something lasting, you cannot just chase the latest model or framework." The patterns are model-agnostic — they work regardless of whether the underlying LLM is GPT, Claude, Gemini, or a local model.

---

## Agent Complexity Levels

| Level | Name | Coding-Agent Example |
|-------|------|---------------------|
| 0 | Core Reasoning Engine | Raw LLM autocomplete — no tools, no memory |
| 1 | Connected Problem-Solver | Agent with tool access (file read/write, terminal, search) |
| 2 | Strategic Problem-Solver | Agent with planning, context engineering, self-improvement loops |
| 3 | Collaborative Multi-Agent | Orchestrator delegating to specialist subagents (research, implement, evaluate) |

Modern IDE coding agents operate at **Level 2–3**. The patterns below are organized by their relevance to this operating level.

---

## Part One: Core Patterns (Chapters 1–7)

### 1. Prompt Chaining
**Pattern**: Sequential processing where the output of one step becomes the input of the next. Deterministic, linear, easy to debug.

**Coding-agent application**:
- **Request preparse → search → read → edit → validate** is a prompt chain. Each phase transforms the context for the next.
- Break complex coding tasks into discrete steps: understand intent → gather context → plan changes → implement → verify.
- Each step should have a clear input/output contract. Don't let steps bleed into each other.

**Anti-pattern**: Trying to do everything in one shot. A single prompt that searches, edits, and validates simultaneously produces lower-quality results than a three-step chain.

### 2. Routing
**Pattern**: A decision layer that classifies incoming requests and routes them to the appropriate handler (different models, different system prompts, different tool sets).

**Coding-agent application**:
- **Cost-proportional model routing**: Route simple file reads to cheap/fast models (Haiku), synthesis tasks to capable models (Sonnet), and ambiguous architectural decisions to thorough models (Opus). This is the core of tiered-agent strategy.
- Route by task type: "fix this typo" → direct edit, "refactor this module" → planning chain, "debug this crash" → diagnostic chain with tool access.
- A routing layer prevents wasting expensive inference on trivial tasks.

**Gulli**: "A model can act as a router to other models, or even the same model with different system prompts and functions."

### 3. Parallelization
**Pattern**: Multiple agents or processes run simultaneously on independent subtasks, then results are gathered and synthesized.

**Coding-agent application**:
- **Fan-out/gather for code review**: Spawn parallel agents for style checking, security audit, performance analysis, and test coverage. Synthesize results.
- **Parallel file reads**: When investigating a bug across multiple files, read all relevant files simultaneously rather than sequentially.
- **Parallel research**: When a task spans multiple topics, launch independent search/scrape operations in parallel.

**Key constraint**: Write operations must NOT be parallelized (race conditions on file edits). Only read/search operations are safe to fan out.

### 4. Reflection
**Pattern**: An agent critiques its own output before presenting it. Create → evaluate → refine loop. The most critical pattern for production reliability.

**Coding-agent application**:
- **Post-edit validation**: After every code change, run `get_errors` / `strict_lint` / compiler checks. This is architectural reflection — the compiler is the critic.
- **Self-review before completion**: Before declaring a task done, re-read the modified files and verify the changes match the intent.
- **Two types**:
  - *"Check your work" reflection*: Simple — re-read and verify (compiler pass).
  - *"Internal critic" reflection*: Advanced — a separate evaluation step with specific criteria (does this change preserve backward compatibility? Does it follow existing patterns?).

**Gulli**: "A reflective agent mimics human reasoning by creating a plan, executing it, and then critiquing its own output before presenting it to the user. This internal feedback loop is often the difference between a wrong answer and a correct one."

**Anti-pattern**: Skipping validation after edits. The number-one failure mode of coding agents is declaring success without running the compiler.

### 5. Tool Use
**Pattern**: Agents extend their capabilities by interacting with external tools (search, file system, terminal, APIs). Uses the ReAct (Reason → Act → Observe) loop.

**Coding-agent application**:
- **IDE tool palette**: `read_file`, `replace_string_in_file`, `run_in_terminal`, `grep_search`, `semantic_search`, `get_errors` — these are the agent's tools.
- **MCP (Model Context Protocol)**: Standardized tool interface. MCP tools (git checkpoint, web search, knowledge base, screenshots) extend the agent's reach beyond the IDE.
- **Tool selection discipline**: The agent must reason about WHICH tool to use, not just blindly call tools. `semantic_search` for concepts, `grep_search` for exact text, `file_search` for paths.

**ReAct loop in practice**:
1. **Thought**: "I need to find where `handleAuth` is defined"
2. **Action**: `grep_search("handleAuth", isRegexp: false)`
3. **Observation**: Found in `src/auth/handler.ts:42`
4. **Thought**: "Now I need to read the function to understand its contract"
5. **Action**: `read_file("src/auth/handler.ts", 35, 65)`
6. **Observation**: [function code]
7. Continue until enough context is gathered

### 6. Planning
**Pattern**: Break complex goals into ordered, actionable steps before executing. Maintain a plan, track progress, handle errors by re-planning.

**Coding-agent application**:
- **Todo lists**: Use structured task tracking (`manage_todo_list`) for multi-step work. Mark tasks in-progress → completed as you go.
- **Show the plan**: For complex changes, lay out the execution plan before starting edits. This gives the user transparency and a chance to correct course.
- **Dynamic re-planning**: When a step fails (test failure, unexpected code structure), re-plan from the current state rather than retrying the same approach blindly.

**Key principle**: Plan BEFORE editing. All research and planning happens in Phase 1. File edits happen in Phase 2. Don't interleave research and editing.

### 7. Multi-Agent Collaboration
**Pattern**: Multiple specialized agents work together via an orchestrator. Each agent has a specific role (researcher, implementer, evaluator, reviewer).

**Coding-agent application**:
- **Subagent architecture**: An orchestrating agent delegates to specialized subagents:
  - *Explore agent*: Read-only codebase exploration and Q&A
  - *Research agent*: Web search, knowledge base queries
  - *Implementation agent*: File edits with validation
  - *Evaluation agent*: Checklist-based quality assessment
- **The orchestrator is the brain**: It handles planning, decomposition, and quality judgment. Subagents handle execution. Execution is usually cheaper than planning.
- **Communication via structured handoff**: The orchestrator passes complete context to each subagent. Subagents return structured results.

**Anti-pattern**: Having every subagent use the most expensive model. Match model tier to subtask complexity (routing pattern).

---

## Part Two: Agent Capabilities (Chapters 8–11)

### 8. Memory Management
**Pattern**: Persistent storage of past interactions, decisions, and experiences. Three tiers: short-term (conversation), working (session), long-term (knowledge base).

**Coding-agent application**:
- **Short-term memory**: The conversation context window. Managed by the IDE (VS Code compacts older messages).
- **Working/session memory**: Session logs (`log_session_event`), todo lists, session notes. Survives context compaction within a conversation.
- **Long-term memory**: Knowledge notes, user memory files, repository memory. Persists across conversations.
- **The goldfish problem**: Agents forget instructions over long conversations as context windows fill. Solution: log critical context to session memory so it can be retrieved via `search_session_log`.

**Gulli**: "The way you create memory is fundamental for the quality of the agents."

### 9. Learning and Adaptation
**Pattern**: Agents improve over time by recording outcomes, analyzing failures, and adjusting behavior.

**Coding-agent application**:
- **Surprise-weighted learning** (Engram model): Log every significant action with a surprise score. High-surprise events (unexpected failures) surface preferentially in future searches.
- **Before acting, search session history**: If a past event shows a failed approach with high surprise, don't repeat it.
- **Knowledge gap materialization**: When you answer from training memory and the KB has nothing on the topic, verify with web sources and write the verified knowledge to the KB.

### 10. Model Context Protocol (MCP)
**Pattern**: Standardized interface for agents to connect to external tools and data sources. "USB port for AI."

**Coding-agent application**:
- MCP servers expose tools (git operations, web search, knowledge base, screenshots, image analysis) as standardized function calls.
- **Key benefit**: Tool portability. An MCP tool works the same whether the agent runs in Copilot, Cursor, or Claude Code.
- **Context engineering > prompt engineering**: The critical skill is designing what information flows to the model, not crafting clever phrasings. MCP enables structured context delivery.

### 11. Goal Setting and Monitoring
**Pattern**: Define success criteria upfront, monitor progress, and adjust.

**Coding-agent application**:
- **Define "done" before starting**: What tests must pass? What errors must be zero? What behavior must be preserved?
- **Exit conditions**: (1) Tests pass, (2) No test suite AND `get_errors` clean, (3) Static content AND `get_errors` clean.
- **Loop iteration limit**: If the same test fails after 3 attempts with different approaches, stop and surface the problem to the user.

---

## Part Three: Safety and Human Integration (Chapters 12–14)

### 12. Exception Handling and Recovery
**Pattern**: Transactional safety — agent actions are tentative until validated. Checkpoints and rollbacks.

**Coding-agent application**:
- **Git checkpoints**: Commit at meaningful milestones so work can be rolled back. `checkpoint({ all: true })` after verified fixes.
- **Read before editing**: Never construct edit targets from memory. Always read the current file state first.
- **Recovery protocol**: When a batch edit fails mid-way, re-read the file, identify what landed, reconstruct only failed edits.

**Gulli**: "If an agent takes an action, we must implement checkpoints and rollbacks, just as we do for transactional safety in databases."

### 13. Human-in-the-Loop (HITL)
**Pattern**: Pause at critical junctures for human feedback, approval, or clarification.

**Coding-agent application**:
- **Destructive actions need confirmation**: Deleting files/branches, `git push --force`, `rm -rf`, dropping tables — always ask first.
- **Ambiguity resolution**: When intent is unclear, ask a clarifying question rather than guessing.
- **Plan approval for complex changes**: Show the plan before executing multi-file refactors.
- **Balance**: HITL at critical checkpoints, not for every action. The goal is collaborative partnership, not constant interruption.

### 14. Knowledge Retrieval (RAG)
**Pattern**: Ground agent responses in retrieved factual information rather than relying solely on training data.

**Coding-agent application**:
- **Knowledge-first protocol**: Before answering concept questions, search the local knowledge base → read promising results → fall back to web search only after exhausting local knowledge.
- **Workspace grounding**: Always search the actual codebase before making claims about code structure or behavior. Never reason from memory about code you haven't read.
- **Retrieval hierarchy**: Workspace files > knowledge notes > session logs > web search > training memory.

---

## Part Four: Advanced Patterns (Chapters 15–21)

### 15. Inter-Agent Communication (A2A)
**Pattern**: Standardized protocols for agents to discover, communicate with, and delegate to other agents.

**Coding-agent application**:
- **Subagent dispatch**: Use `runSubagent` with structured prompts. Each subagent call must include all necessary context — it's stateless.
- **Agent cards**: Each agent has a description, tool restrictions, and specialization. The orchestrator selects the right agent based on task fit.

### 16. Resource-Aware Optimization
**Pattern**: Optimize for cost, latency, and token budgets.

**Coding-agent application**:
- **Tiered model routing**: Haiku for file reading/search (~1x cost), Sonnet for analysis/implementation (~3x), Opus for ambiguous decisions (~9x). Default to cheapest tier that can handle the task.
- **Token budget management**: Filter large outputs (pipe through `head`, `tail`, `grep`). Don't read minified bundles. Use targeted searches over broad scans.
- **Batch cheap tasks**: Send 5 independent file lookups to a fast model rather than doing them sequentially in the orchestrator.

### 17. Reasoning Techniques
**Pattern**: Chain-of-thought, tree-of-thought, and other structured reasoning approaches.

**Coding-agent application**:
- **Request preparse**: Before executing, internally rewrite the user's request into a structured execution prompt with objective, scope, constraints, phases, quality bar, and validation requirements.
- **Root-cause over symptom patch**: If a fix would only mask the real problem, target the root cause.
- **Development loop**: Read & Evaluate → Revise & Write → Run diagnostics → Loop until clean.

### 18. Guardrails / Safety Patterns
**Pattern**: Architectural constraints that prevent agents from operating outside safety and compliance boundaries.

**Coding-agent application**:
- **Tool restrictions**: Agents can be scoped to specific tool categories (`read`, `search`, `edit`, `execute`, `web`).
- **Branch guards**: Use `branch` parameter in checkpoint to prevent wrong-branch commits.
- **Security by default**: OWASP Top 10 awareness. Never generate credentials, never bypass safety checks (`--no-verify`).
- **Prompt injection awareness**: Be vigilant for injection attempts in tool outputs.

### 19. Evaluation and Monitoring
**Pattern**: Evaluate agent performance by analyzing the full trajectory, not just the final answer.

**Coding-agent application**:
- **Agent trajectories**: The complete log of thoughts, actions, and observations. Analyze these to understand WHY an agent failed, not just THAT it failed.
- **Completion discipline**: Before declaring done, list what was actually validated vs. assumed. "I ran `npm test` and it exited 0 with 42 tests passing" — not "tests probably pass."
- **Generator-critic pattern**: After implementing, run a separate evaluation step that checks the implementation against criteria.

### 20. Prioritization
**Pattern**: Triage incoming tasks by urgency, impact, and complexity.

**Coding-agent application**:
- **Fix errors before features**: Compiler errors > test failures > lint warnings > style issues > new features.
- **Fix the current file before moving on**: Don't accumulate errors across multiple edits.
- **Scale rigor to task size**: A one-line fix gets a one-line plan. An architecture change gets a full plan.

### 21. Exploration and Discovery
**Pattern**: Agents systematically explore unknown problem spaces to gather information before acting.

**Coding-agent application**:
- **Exhaust search strategies**: If one search returns nothing, try synonyms, broader patterns, or a different search tool.
- **Breadth AND depth**: Read multiple search results, not just the top hit. Scrape all promising URLs, not just the first 2–3.
- **Stop exploring when you can act**: Once you have enough context to proceed confidently, stop searching and start implementing.

---

## The Five High-Impact Patterns for Coding Agents

Gulli identifies five "low-hanging fruit" patterns with the highest immediate impact for enterprise (and agent) use:

1. **Reflection** — Self-critique before presenting output. In coding agents: always validate with compiler/linter after edits.
2. **Routing** — Direct tasks to the right model/handler. In coding agents: tiered model selection (cheap for reads, expensive for synthesis).
3. **Communication (MCP/A2A)** — Standardized tool and agent interfaces. In coding agents: MCP tools for git, web, knowledge base.
4. **Memory** — Persistent context across conversations. In coding agents: session logs, knowledge notes, user memory.
5. **Guardrails** — Safety boundaries. In coding agents: tool restrictions, branch guards, destructive-action confirmation.

---

## Context Engineering > Prompt Engineering

The most important meta-shift from the book: the era of prompt engineering is giving way to **context engineering** — designing the information flow, managing state, and curating the context that the model sees.

For coding agents, this means:
- **Instructions files** (`.instructions.md`) that scope agent behavior per file pattern
- **Skills** (`SKILL.md`) that package domain knowledge for specific task types
- **Agent definitions** (`.agent.md`) with tool restrictions and specialized prompts
- **Knowledge bases** that accumulate verified facts across sessions
- **Session memory** that preserves learning within a conversation

The agent's quality is determined by what it SEES (context), not how cleverly it's asked (prompt). Design the information architecture, not the linguistic tricks.

---

## Transactional Safety for Coding Agents

Borrowed from database engineering:
- **Checkpoints**: Git commits at meaningful milestones (working state before risky changes)
- **Rollbacks**: `git checkout` / `git reset` to undo failed changes
- **Atomic operations**: Each edit should be independently valid. Don't leave files in broken intermediate states.
- **Tentative actions**: Agent changes are tentative until validated by compiler/tests. Never declare success without verification.

---

## Google's Eight Multi-Agent Architectures (ADK Companion Guide)

Google also published a companion guide identifying eight multi-agent architectures:

1. **Sequential Pipeline** — Linear chain, deterministic, easy to debug
2. **Coordinator/Dispatcher** — One agent routes to specialists
3. **Parallel Fan-Out/Gather** — Independent subtasks run simultaneously, results synthesized
4. **Hierarchical Decomposition** — High-level agents break goals into subtasks, delegate to child agents
5. **Generator and Critic** — One agent creates, another validates (iterative refinement)
6. **Iterative Refinement** — Generator → Critic → Refiner loop
7. **Human in the Loop** — Approval gates for irreversible actions
8. **Composite** — Combine any of the above

For coding agents, the most commonly used architectures are:
- **Coordinator/Dispatcher** (orchestrator routing to Explore, Research, Implement subagents)
- **Generator and Critic** (implement → compile/lint → fix loop)
- **Human in the Loop** (confirmation before destructive operations)

---

## Summary: The 10 Commandments for Coding Agents

1. **Plan before you edit.** Research is Phase 1. Edits are Phase 2. Never interleave.
2. **Validate after every edit.** The compiler is the critic. `get_errors` after every change.
3. **Route by complexity, not by phase.** Use cheap models for cheap tasks. Promote only with justification.
4. **Remember across sessions.** Log failures with high surprise. Search before repeating approaches.
5. **Ground in evidence, not memory.** Read the actual code. Search the actual codebase. Don't reason from stale context.
6. **Checkpoint at milestones.** Git commits are your transactional safety net.
7. **Ask before destroying.** Destructive actions need human confirmation.
8. **Use standardized tools (MCP).** Portable, composable, debuggable.
9. **Engineer context, not prompts.** Instructions, skills, knowledge bases, and memory > clever phrasing.
10. **Evaluate trajectories, not just outcomes.** A correct answer via a dangerous process is still a bug.

---

## Pattern Traceability — gsh copilot-config

Maps each of the 21 patterns to the gsh instruction/skill that implements it. Patterns marked ✅ are well-covered; ⚡ indicates a gap that has been addressed.

| # | Pattern | Status | Primary implementation | Notes |
|---|---------|--------|----------------------|-------|
| 1 | Prompt Chaining | ✅ | `request-preparse` | Phase 1 → Phase 2 → Phase 3 development loop is a textbook prompt chain |
| 2 | Routing | ✅ | `tiered-agents` + `subagent-strategy` | Model routing (cost tiers) + task-type routing (scale rigor to task size in `request-preparse`) |
| 3 | Parallelization | ⚡ | `expand-and-engage` | Batch independent reads. Write-safety rule added: reads parallelize, writes must not |
| 4 | Reflection | ✅ | `vscode-tool-safety` + `expand-and-engage` | Compiler-as-critic + strict_lint = architectural reflection. Development loop in `request-preparse` is the create→evaluate→refine cycle |
| 5 | Tool Use | ✅ | `expand-and-engage` + `vscode-tool-safety` | Tool preference hierarchy, MCP-over-builtin, ReAct loop implicit in tool-use discipline |
| 6 | Planning | ✅ | `request-preparse` | Execution phases, todo tracking, dynamic re-planning on failure |
| 7 | Multi-Agent | ✅ | `subagent-strategy` + `tiered-agents` | Orchestrator/specialist decomposition, cost-proportional dispatch |
| 8 | Memory | ✅ | `session-learning` + `expand-and-engage` | Three tiers: user memory (persistent), session memory (surprise-weighted), repo memory (workspace-scoped). Knowledge-First Protocol for long-term retrieval |
| 9 | Learning | ✅ | `session-learning` | Engram-inspired surprise-weighted logging. Knowledge Gap Rule in `expand-and-engage` for KB growth |
| 10 | MCP | ✅ | `gsh-mcp-tools` + `expand-and-engage` | MCP-over-builtin preference, standardized tool interface |
| 11 | Goal Setting | ✅ | `request-preparse` | Quality bar, validation requirements, explicit exit conditions, loop iteration limit |
| 12 | Exception Handling | ✅ | `git-checkpoint` + `vscode-tool-safety` + `branch-lifecycle` | Transactional checkpoints, recovery protocols, branch guards |
| 13 | HITL | ✅ | `request-preparse` + system-level operational safety | Destructive-action confirmation, ambiguity resolution via clarifying questions |
| 14 | Knowledge Retrieval | ✅ | `expand-and-engage` | Knowledge-First Protocol: KB → cache → web → training memory. Retrieval hierarchy explicit |
| 15 | Inter-Agent (A2A) | ✅ | `subagent-strategy` | Stateless subagent dispatch with structured prompts. Agent cards via `.agent.md` frontmatter |
| 16 | Resource-Aware | ✅ | `tiered-agents` + `vscode-tool-safety` | Cost multiplier table (1x/3x/9x), token budget management, output filtering |
| 17 | Reasoning | ✅ | `request-preparse` | Internal request rewriting: objective, scope, constraints, phases, quality bar |
| 18 | Guardrails | ⚡ | Distributed: `branch-lifecycle` (branch guards), `vscode-tool-safety` (tool safety), agent frontmatter (tool restrictions), system prompt (OWASP, destructive actions) | Well-implemented but fragmented. Cross-reference added here for traceability |
| 19 | Evaluation | ⚡ | `request-preparse` | Completion Discipline covers outcome reporting. Session log IS the trajectory record — agents should review it for process quality, not just results |
| 20 | Prioritization | ✅ | `request-preparse` + `vscode-tool-safety` | Fix errors before features, fix current file before moving on, scale rigor to task size |
| 21 | Exploration | ✅ | `expand-and-engage` | Exhaust search strategies, breadth+depth, stop when you can act |

### Guardrails Cross-Reference (Pattern 18)

Since guardrails are distributed across files, here is the consolidated map:

| Guardrail | Where |
|-----------|-------|
| Branch guards (prevent wrong-branch commits) | `branch-lifecycle`, `branch-workspace-control` |
| Tool restrictions per agent | Agent `.agent.md` frontmatter `tools:` field |
| Destructive action confirmation | System-level operational safety rules |
| OWASP/security defaults | System-level security requirements |
| Strict lint after every edit | `expand-and-engage` |
| Format control (prevent formatter interference) | `vscode-tool-safety` |
| File edit safety (read before edit, 3+ lines context) | `vscode-tool-safety` |
| Terminal safety (no parallel calls, no inline comments) | `vscode-tool-safety` |
| Knowledge write gatekeeping (verify before materializing) | `expand-and-engage` Knowledge Gap Rule |