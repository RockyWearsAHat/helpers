# Anthropic Agent Architecture Patterns — Production Patterns for Long-Running AI Systems

## Overview

Anthropic's 2026 engineering research synthesizes patterns that emerged from building production AI agents with Claude across multiple domains. The key insight: **simplicity and composability win over framework complexity**. These patterns are production-grade, battle-tested approaches grounded in actual deployments rather than theoretical ideals.

The research distinguishes between **workflows** (predefined code paths where LLMs orchestrate via tools) and **agents** (systems where LLMs dynamically direct their own process). Most production systems use hybrid combinations, not pure agents.

## Anthropic's Six Composable Agent Patterns

### Pattern 1: Prompt Chaining

**Definition**: Break a complex task into a sequence of steps, each with a dedicated AI call and its own specialized prompt.

**When to use**: When you can decompose a problem into strictly sequential stages where later steps depend cleanly on earlier output. Common in: content generation pipelines, data classification, report generation.

**Key advantage**: Each step can be optimized independently. Models stay focused on one task. Easy to add branching logic between steps.

**Example usage**: Extract data from a document → check for missing fields → enrich with external context → format for presentation. Each LLM call runs with minimal context, reducing hallucination and cost.

**Anthropic's observation**: This is the most reliable pattern when you know the exact sequence ahead of time. Framework overhead is minimal—just function calls and state passing.

### Pattern 2: Routing (Dynamic Branching)

**Definition**: Use an LLM to classify an input and route it to specialized handlers or prompts based on classification.

**When to use**: When input categories map to fundamentally different solution approaches. Examples: customer service (complaints vs. questions vs. feedback), code tasks (refactoring vs. testing vs. optimization), content moderation (spam vs. policy violations vs. edge cases).

**Implementation**: Router agent examines input, produces a classification token, downstream system branches. Router prompt must be crisp and fast—overhead compounds per request.

**Anthropic's caveat**: The router itself is an LLM call. If classification is nondeterministic, cascading errors compound downstream. Use structured extraction (JSON schemas) from the router to force clean classification.

**Cost tradeoff**: One extra LLM call per input, but later calls are cheaper and faster because they're targeted.

### Pattern 3: Parallelization

**Definition**: Decompose a task into independent subtasks, run multiple LLM calls in parallel, aggregate results.

**When to use**: When work is parallelizable (independent analyses, multi-perspective evaluation, generating multiple candidates for later selection).

**Examples**:
- Generate 5 independent user personas, select the best 3
- Evaluate code from multiple reviewers in parallel, aggregate findings
- Answer the same customer query from 5 different business perspectives, merge responses

**Key limitation**: Only aggregate if combining perspectives makes sense. Naive or contradictory merging of parallel outputs produces confusion, not improvement.

**Anthropic's finding**: Parallelization works best when you have a clear aggregation strategy defined in advance, not ad-hoc merging.

### Pattern 4: Orchestrator-Worker

**Definition**: One orchestrating agent (the "coordinator") manages multiple specialized worker agents, dispatching tasks and synthesizing their outputs.

**When to use**: Complex workflows where different specialists (researcher, analyst, writer) need to collaborate. The coordinator breaks down the task, assigns to workers, gathers results.

**Distinction from workflows**: The coordination logic is in an LLM (flexible, can adapt), not hardcoded predefined paths. Workers are typically prompted differently (research agent vs. analysis agent vs. writer agent).

**Scaling risk**: Orchestrator becomes the bottleneck. Token costs scale with coordination overhead. Breaks down for very large numbers of workers (5-7 is typical sweet spot).

**Anthropic's guidance**: Don't use this just because you have multiple agents. Use it when coordination requires judgment calls and context-awareness that hardcoded logic can't handle.

### Pattern 5: Evaluator-Optimizer (Generator-Evaluator)

**Definition**: Separate an agent that generates outputs from an agent that evaluates them. Evaluator feedback drives iterative improvement.

**This is Anthropic's most significant 2026 contribution.** Tested extensively on both subjective tasks (frontend design) and objective tasks (software engineering).

**How it works**:
- Generator agent produces output (code, design, content)
- Evaluator agent grades against explicit criteria—does NOT just praise
- Evaluator must be skeptical by nature; generator must be responsive to feedback
- Loop: generator refines, evaluator re-grades, continues until plateau or threshold hit

**Why separate agents matter**: When asked to evaluate their own work, LLMs exhibit "self-praise bias"—they confidently approve mediocre output. **Separating generator from evaluator dramatically improves critique quality.** This was Anthropic's key finding from frontend design experiments.

**Tuning the evaluator**: Provide concrete grading criteria (design quality, originality, craft, functionality rather than abstract "is this good?"). Calibrate with few-shot examples showing what high/medium/low scores look like. Give evaluator tools to *interact* with output (Playwright for web apps, browser automation for interactive features).

**Cost-quality tradeoff**: 5-15 iterations per generation common. Frontend design example: 4-15 iteration cycles took 4 hours and $200-300 in tokens vs. single-shot baseline at 20 minutes and $9. **20x more expensive, but visibly superior output.**

**Key insight from Anthropic's experiments**: The wording of grading criteria directly shapes output character. Criteria like "museum quality design" subtly steer aesthetic direction. Criteria act as implicit constraints.

**When NOT to use**: If the task is already at the edge of model capability and evaluator feedback won't help (e.g., asking an LLM to evaluate musical taste when it cannot hear). Evaluator primarily helps when generator is capable but undirected.

### Pattern 6: Multi-Agent with Context Resets

**Definition**: For tasks spanning many hours or multiple context windows, design a handoff protocol between sequential agents.

**The core problem**: LLMs lose coherence as context window fills. Compaction (summarizing earlier parts in place) preserves continuity but doesn't solve "context anxiety"—models begin wrapping up prematurely as they believe they're approaching context limits.

**Anthropic's solution**: **Hard context reset between agent sessions.** Agent N exhausts its work, writes complete handoff artifacts, Agent N+1 starts fresh with explicit state. Costs latency and handoff token overhead but provides clean slate.

**Handoff artifacts critical to success**:
- `claude-progress.txt`: log of what each agent did, current state
- `feature-list.json`: structured list of remaining tasks (all marked with `passes: false` initially)
- Git history: code commits showing what changed
- `init.sh`: script to restart dev environment

**Anthropic's finding**: This pattern was essential for Claude Sonnet 4.5, which exhibited strong context anxiety. Claude Opus 4.5-4.6 largely eliminated context anxiety natively, reducing (but not eliminating) the need for resets for some tasks.

**Session startup ritual**: Each agent reads progress file, git logs, feature list. Runs health check on environment. Only then begins new work. This takes minutes but prevents agents from starting with broken state.

## The Harness Design Methodology

### From Frontend Design to Full-Stack Coding

Anthropic applied generator-evaluator pattern to two domains simultaneously:

**Frontend Design Task**:
- Generator: Creates HTML/CSS/JS frontend from prompts
- Evaluator: Uses Playwright MCP to interact with live page, scores on design quality (coherence, distinctiveness), originality (avoids AI clichés), craft (typography, spacing, color harmony), functionality
- Loop: 5-15 iterations, each iteration taking real wall-clock time (pages render, Playwright interacts)
- Result: Designs shifted from safe/generic to distinctive/museum-quality aesthetic
- Tuning required: Evaluator prompt went through multiple rounds to calibrate skepticism; early versions praiseworthy toward AI-generated mediocrity

**Full-Stack Coding Task** (Retro Game Maker + DAW examples):
- Planner agent: Takes single-sentence prompt ("Build a 2D retro game maker"), expands to 16-feature full product spec
- Generator agent: Implements features in sprints, uses git for version control, builds against explicit "sprint contract" agreed with evaluator upfront
- Evaluator agent: Uses Playwright to click through running app like user would, tests UI, API endpoints, database state, grades against bug-free + design + functionality criteria
- Failure modes caught by evaluator:
  - Rectangle fill tool only places tiles at drag endpoints, not filling region
  - Delete key handler logic missing conditional check
  - Route matching issues with FastAPI routing order

**Key lesson**: Evaluator identified system-level bugs (incorrect route matching) that static code review alone wouldn't catch, because evaluator ran the app.

### Model Progression and Harness Simplification

Anthropic tested across model generations and simplified harness as models improved:

**With Opus 4.5**:
- Required sprint decomposition (feature-by-feature)
- Required context resets between agent sessions
- Evaluator was load-bearing; caught many bugs
- Typical app: 6 hours, $200 token cost

**With Opus 4.6** (released mid-research):
- Removed sprint decomposition; model could sustain 2+ hour coding sessions
- Removed context resets; native improvements to planning and long-horizon coherence
- Evaluator still added value but less critical
- Single-pass generation possible for simpler tasks
- Typical app: 3-4 hours, $100-120 token cost

**Principle**: Every component of a harness encodes an assumption about what the model can't do alone. As models improve, those assumptions go stale. Regularly stress-test harness assumptions and strip away load-bearing components that are no longer needed.

## Multi-Agent Architecture for Complex Tasks

### Three-Agent Pattern (Planner-Generator-Evaluator)

Anthropic's production pattern for full-stack app generation:

1. **Planner**: Takes vague user prompt, produces structured spec
   - Ambitious on scope
   - Stays high-level (avoids over-specifying implementation)
   - Identifies opportunities to weave AI features into app
   - Output: JSON spec with feature categories, user stories, data models

2. **Generator**: Implements apps feature-by-feature
   - Takes one feature from spec
   - Negotiates "sprint contract" with evaluator (what "done" means)
   - Implements and self-verifies
   - Commits to git with descriptive messages
   - Output: Running code + documentation

3. **Evaluator**: QA/Product role
   - Reads sprint contract upfront
   - Interacts with running app (Playwright MCP)
   - Tests against contract criteria
   - Writes detailed bug/gap reports
   - Grades against hard thresholds (if any criterion fails, sprint fails)

**Communication protocol**: Agents communicate via files, not direct function calls. Generator writes `sprint-contract.md`, Evaluator reads and responds, Generator reads Evaluator feedback, etc. This async pattern scales better than synchronous back-and-forth.

## Long-Running Agent Failure Modes and Solutions

### Four Common Failure Patterns

| Problem | Cause | Solution |
|---------|-------|----------|
| Agent declares victory prematurely | No explicit task list; after partial work, agent thinks done | Structured feature list (JSON) with all tasks marked `passes: false`. Read list at session start. |
| Half-implemented features break next session | No clean handoff state | Progress file + git commits. Each session: read progress, run health check, end with clean commit. |
| Features marked done without testing | Prompt asks for implementation but not verification | Explicit testing tools (Puppeteer/Playwright MCP). Require agent to test end-to-end like a user would. |
| Agent wastes time figuring out environment | No setup documented | Write `init.sh` script. Archive as git commit. Read it at session start. |

### Session Startup Checklist

Every agent session should:
1. Run `pwd` to confirm working directory
2. Read git logs to understand recent work
3. Read progress file to see current state
4. Read feature list and identify next task
5. Run `init.sh` to start development environment
6. Run health check (basic end-to-end test of core features)
7. Only then begin new work

Anthropic's logs show this takes 2-5 minutes per session but prevents cascading errors.

## Key Tradeoffs and Limitations

### Throughput vs. Quality

- Single-pass generation: Fast, cheap, frequent errors
- Generator-evaluator loops: Slow, expensive, higher quality
- Boundary depends on task complexity and model capability
- For frontier tasks (complex coding, subjective design), evaluator ROI is positive despite cost

### Agent Autonomy vs. Reliability

- Fully autonomous agents (Pattern 6+ multi-agent) can hallucinate, go off-rails
- Constrained workflows (Pattern 1: chaining) are boring but reliable
- Most production systems: 70-80% workflow + 20-30% agent autonomy for dynamic parts

### Context Window Management

- Compaction preserves continuity but doesn't prevent context anxiety
- Hard resets provide clean slate but add latency + token overhead
- Latest models (Opus 4.5+) need less scaffolding than 4.0-4.5
- No single best approach; depends on task and model

## When to Use Agents vs. Workflows

**Use workflows (not agents) when**:
- Task sequence is known upfront
- Determinism matters (auditing, compliance, reliability)
- Context is small
- Cost must be minimized

**Use agents when**:
- Task is too complex to pre-specify all paths
- You need dynamic decision-making mid-flow
- Context extraction and planning add value
- Quality matters more than cost

**Hybrid is most common**: Use workflows for understood parts, agents for adaptive/complex parts.

## State-of-the-Art Challenges Still Open

- **Multi-agent coordination at scale**: Orchestrator patterns work for 5-7 workers; scaling to 20+ agents unclear
- **Effective long-horizon planning**: Models still sometimes "declare victory" even with explicit scaffolding
- **Generalization across domains**: Patterns here tested on frontend design + full-stack coding; applicability to scientific research, financial modeling unclear
- **Evaluator bias**: Evaluators still exhibit some leniency toward LLM output even when separated from generator
- **Cross-domain knowledge transfer**: Learning about agent patterns from code tasks doesn't immediately transfer to design or writing domains

## Implementation Priorities

Start simple: prompt chaining or routing. Add evaluator pattern only when single-pass output isn't good enough. Multi-agent harnesses should be last resort, not starting point.

Anthropic's principle: "Find the simplest solution possible, and only increase complexity when needed."

## See Also

- `genai-agents.md` — Foundational agent concepts and tool use
- `genai-agent-frameworks.md` — LangChain, AutoGen, orchestration libraries
- `genai-coding-agent-state-of-art.md` — Specific patterns in Claude Agent SDK vs. competing agents
- `llm-prompt-engineering.md` — Detailed prompt tuning and criterion design