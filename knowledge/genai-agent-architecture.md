# AI Agent Architecture — Multi-Agent Patterns, Orchestration & Specialization

## Overview

Production AI agents solving complex tasks rarely operate as monoliths. Anthropic's 2026 research on long-running agents reveals that **multi-agent architectures** significantly outperform single-agent systems on tasks requiring domain specialization, iterative refinement, or objective evaluation. A single agent forced to act as planner, code generator, quality assurance, and judge inevitably fails at some roles. Decomposing these concerns into specialized agents with targeted prompts, tool access, and (often) different models yields dramatically better quality and lower cost.

This note covers the primary multi-agent patterns in production use, when to apply each, and the engineering trade-offs involved.

## The Core Problem: Why Single Agents Struggle

A monolithic agent attempting complex work exhibits predictable failure modes:

- **Prompt complexity explosion**: Accumulating requirements (code generation + test writing + security review + performance optimization) creates an unfocused prompt where the model dilutes attention across conflicting objectives
- **Tool overload**: Providing 50 tools in a single call registry causes hallucinated tool names, incorrect parameter selection, and decision paralysis
- **Context bloat**: Fitting all domain knowledge into one prompt window inflates token costs and reduces reasoning coherence
- **Model inefficiency**: Using a reasoning-heavy model (e.g., Opus 4.5) for all tasks—including trivial ones—wastes money on work that a cheaper model (e.g., Haiku) handles equally well

Example: A coding agent that tries to plan architecture (requires reasoning), generate code (requires generative capability), write tests (requires test frameworks knowledge), and verify quality (requires careful evaluation) will produce mediocre results across all four tasks, often running out of context mid-implementation.

## The GAN-Inspired Three-Agent Pattern

Anthropic's most successful long-running harness adopts a **generator-evaluator loop** inspired by Generative Adversarial Networks (GANs). The pattern introduces a third agent—a planner—that decomposes the initial spec into tractable work.

### Pattern: Planner → Generator → Evaluator

**Planner Agent**
- **Input**: High-level user prompt ("Build a 2D retro game maker with editors and playable mode")
- **Task**: Expand into a comprehensive spec across multiple features, sprints, and technical decisions
- **Constraints**: Stay at product/UX level; avoid implementation details that might lock the generator into suboptimal choices
- **Output**: Detailed spec (150–500 features) with clear, testable acceptance criteria
- **Model choice**: Opus 4.5 or 4.6 (reasoning-heavy; run once at start)
- **Cost**: 5–15 minutes, $0.5–$2

The planner intentionally avoids overspecifying implementation. If the spec prescribes "use React with TypeScript," and that choice turns out poorly, downstream agents inherit those constraints. Instead, the planner specifies outcomes: "The user can place tiles on a grid and see updates in real-time." Implementation is left to the generator.

**Generator Agent**
- **Input**: Spec from planner, list of remaining features
- **Task**: Implement one feature at a time; commit to git; update progress tracking
- **Constraints**: Work incrementally; leave the codebase in a production-ready state after each feature (no half-stubs)
- **Output**: Working code, git commits with descriptive messages, updates to progress file
- **Model choice**: Opus 4.5 or 4.6 (the primary workhorse; runs for hours)
- **Cost**: 2–10 hours, $50–$200

The generator's effectiveness depends on **explicit incremental prompting**. Without it, the model one-shots the entire app, running out of context mid-build. With it, the generator picks one feature, implements it fully (including tests), commits, then picks the next. This keeps context window growth manageable and ensures intermediate states are shippable.

**Evaluator Agent**
- **Input**: Running app, previous sprint code, spec requirements
- **Task**: Test the feature end-to-end using browser automation (Playwright MCP); verify against grading criteria; provide detailed feedback
- **Constraints**: Be skeptical; test edge cases; use live UI interaction, not just code review
- **Output**: Pass/fail verdict, specific bugs found, detailed critique for generator to address
- **Model choice**: Opus 4.5 or 4.6 (run per-feature or at end)
- **Cost**: 5–20 minutes per evaluation, $1–$5

The evaluator is non-negotiable because **agents reliably fail to evaluate their own work objectively**. When asked to QA code they just wrote, models tend to praise it—even when it's broken. Separation of concerns enables the evaluator to be calibrated with skepticism (via few-shot examples) without fighting the generator's attachment to its own output.

### How This Pattern Outperforms Solo Agents

**Experimental comparison** (Anthropic, Jan 2026):
- Solo agent building a "retro game maker" app: 20 minutes, $9, broken gameplay
- Three-agent harness on the same spec: 6 hours, $200, fully functional with 200+ features

The 20× cost increase yields dramatically higher quality: The solo run produced an interface that looked reasonable but had non-functional core gameplay. The evaluator repeatedly tested edge cases, found specific bugs (e.g., entity wiring issues), and drove the generator to fix them. The result was a production-quality app.

**Why the three-agent pattern scales**: Each agent specializes. The planner doesn't write code; the generator doesn't judge quality; the evaluator doesn't build architecture. Specialization means each prompt is focused, each tool set is targeted, and failures in one agent don't cascade into wrong decisions for another.

## Orchestrator-Worker Pattern

For tasks with **heterogeneous workloads** (some parts need cheap execution, others deep reasoning), the orchestrator-worker pattern decomposes work across models.

**Orchestrator Agent** (small, fast model: Haiku 3.5)
- Routes incoming requests to specialized workers
- Manages state and progress
- Collects results and aggregates answers

**Worker Agents** (specialized models, per domain)
- Customer service worker: Haiku (fast, cheap, sufficient for standard issues)
- Fraud detection worker: Opus (reasoning-heavy; rare invocation)
- Billing issues worker: Haiku (rule-based, deterministic)

Example: A customer service orchestrator receives a query, classifies it as a billing issue, routes to the billing worker, gets a result, and responds. If classification fails or the worker can't resolve, the orchestrator escalates to a human or to the fraud-detection worker.

**Cost advantage**: Average cost per request $0.05 (mostly Haiku), with Opus engaged only for genuinely complex cases (~5% of traffic).

## Evaluator-Optimizer Pattern

For **iterative refinement tasks** (design, writing, code optimization), the evaluator-optimizer pattern creates a feedback loop similar to reviewer-author collaboration.

**Generator/Optimizer Agent**
- Produces initial output (code, design, text)
- Reads evaluator feedback
- Iterates based on specific critiques

**Evaluator Agent**
- Assesses output against explicit grading criteria
- Provides detailed, actionable feedback (not just "good" or "bad")
- Uses few-shot examples to calibrate judgment

**Feedback loop**: Generator → Evaluator → Feedback → Generator (repeat 5–15 times)

Anthropic's frontend design experiment demonstrated this: After five iterations, designs improved measurably in originality and craft. After ten iterations, some generators made aesthetic leaps (e.g., reimagining a museum website from a 2D layout to a 3D CSS-rendered spatial experience).

## Context Handoff Artifacts

Multi-agent systems fail if agents can't efficiently hand off work across context windows. Three structural patterns handle this:

### 1. JSON Feature Lists

Store work items in machine-readable format. Example:
```json
{
  "features": [
    {
      "category": "gameplay",
      "description": "Player can move entity with arrow keys",
      "id": "movement-1",
      "passes": false,
      "tested": false
    },
    {
      "category": "ui",
      "description": "Tile palette shows all available sprites",
      "id": "palette-1",
      "passes": true,
      "tested": true
    }
  ]
}
```

Agents read this at session start, identify failed tests, and update `passes: true` only after verification. Using JSON prevents agents from misformatting or accidentally deleting requirements (models are more careful with structured data than Markdown).

### 2. Progress Tracking Files (claude-progress.txt)

Agents leave a human-readable summary of their session:
```
=== Session 1 ===
Time: 15 minutes
Features completed: 3/50
- Implemented tile palette UI
- Fixed sprite loading bug
- Added keyboard shortcuts

Known issues to address:
- Entity animation timing is off by 50ms (affects gameplay feel)
- Memory leak in sprite cache (only affects long sessions >30min)

Next agent should:
- Fix timing issue first (high priority)
- Implement level persistence
```

This replaces compaction's ambiguity with explicit context. The next agent doesn't have to infer what happened; it reads a clear summary and inherits a precise to-do list.

### 3. Git Commit History

Each agent commits work with descriptive messages:
```
git log --oneline
commit a3f9b2e: Implement tile palette with drag-to-place
commit 8d4e1c: Fix entity collision detection in level editor
commit 5c7ab3: Add sprite animation keyframe UI
```

Future agents use `git log` to understand what was done, use `git diff HEAD~5..HEAD` to see recent changes, and can revert with `git revert` if needed.

## Cost-Latency Trade-offs

| Pattern | Cost | Latency | Quality | When to use |
|---------|------|---------|---------|------------|
| **Single agent** | Low | Fast | Poor on complex tasks | Narrow tasks (classification, summarization) |
| **Planner-generator-evaluator** | 20-50× higher | Slow (hours) | Excellent (complex coding, design) | Long-running, high-stakes builds |
| **Orchestrator-worker** | Medium (10-50% savings) | Moderate | Good (well-suited to task) | Mixed workloads, cost-sensitive |
| **Evaluator-optimizer** | High (multiple iterations) | Slow | Excellent (refined output) | Quality critical (design, writing) |

**Principle**: Add agents only when the baseline model fails. If a single agent produces acceptable output, the overhead of multi-agent isn't justified.

## When Each Pattern Wins

- **Planner-generator-evaluator**: Complex, multi-phase work (full-stack app dev, research papers, large design systems). Requires 5+ hours of reasoning and many decision points.
- **Orchestrator-worker**: Heterogeneous workloads where different tasks benefit from different models or have misaligned cost-quality curves.
- **Evaluator-optimizer**: Subjective quality (design, branding) or polish (code refactoring, writing). Iteration count matters more than wall-clock time.
- **Sequential pipeline**: Strictly linear tasks where each stage adds to previous (e.g., extract data → validate → format → send).

## Key Insights

1. **Specialization beats generalization.** A prompts designed for code generation is worse at evaluation than a prompt designed for evaluation.
2. **Separation of concerns enables calibration.** An evaluator can be made skeptical; making a generator skeptical of its own work is much harder.
3. **Model selection per agent.** Opus for reasoning-heavy work, Haiku for cheap/fast work. Mix models to optimize cost-quality.
4. **Handoff artifacts are critical.** JSON feature lists, progress files, and git history reduce context overhead and enable agents to resume work intelligently.
5. **Iteration is underrated.** Five to fifteen iterations of evaluation-driven refinement often surpass what a single long run produces.

## See Also

- [Effective Harnesses for Long-Running Agents](genai-agent-harness-design.md) — Initializer, progress tracking, incremental coding
- [Agent Evaluation Patterns](genai-agent-evaluation-patterns.md) — Why agents fail at self-evaluation, calibration strategies
- [Multi-Agent Orchestration](genai-multiagent-orchestration.md) — Sequential pipelines, routing, parallelization
- [LLM Cost Optimization](genai-lm-cost-optimization.md) — Token economics, model selection, context compression

---

**Sources**: Anthropic Engineering research (Prithvi Rajasekaran, Justin Young, 2026); "Harness Design for Long-Running Application Development," https://www.anthropic.com/engineering/harness-design-long-running-apps; "Effective Harnesses for Long-Running Agents," https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents