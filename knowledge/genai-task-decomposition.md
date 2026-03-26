# Task Decomposition for AI Agents — Breaking Complex Problems Into Atomic Sub-tasks

## Overview

Task decomposition is the practice of breaking a complex problem into smaller, verifiable, sequential sub-tasks that an AI agent can tackle one at a time. Rather than asking an agent to build an entire feature, system, or project in a single generation pass, decomposition surfaces explicit intermediate states, progress checkpoints, and dependencies that allow the agent to reason locally about each piece before moving forward.

This is not a novel concept — software engineering has always used requirements decomposition, sprint planning, and modular design. What distinguishes task decomposition for AI agents is that it directly addresses the fundamental constraints of how large language models process instructions and manage state: limited context windows, sequential token generation, and attention fatigue on long-running tasks without explicit progress tracking.

## Why One-Shotting Fails

Without decomposition, agents face a reliability cliff when asked to handle non-trivial projects. Anthropic's harness research (2025) demonstrated that when coding agents are given an entire specification and asked to implement it end-to-end, they exhibit several failure modes:

- **Context dilution**: As the specification grows, the model's attention is spread across increasingly distant requirements. Early decisions made without full context often conflict with later implementation details. By the time the model reaches the end of a large task, it has "forgotten" constraints from the beginning.
- **One-shot exhaustion**: Models attempt to generate all code at once, without incremental verification. Errors introduced early propagate unchecked, and the agent has no feedback signal until the entire pass is complete.
- **Plan–implementation mismatch**: When agents operate without explicit planning, they oscillate between reasoning about what to do and doing it, leading to inefficient exploration and backtracking.

Anthropic's research specifically showed that forcing agents to work **one feature at a time** instead of whole-system generation improved success rates dramatically. Without that constraint, Claude would attempt to one-shot the entire problem, nearly always failing due to complexity overload.

## The Role of Context Windows

Context window size is the hard constraint that motivates decomposition. A context window is the maximum number of tokens an LLM can ingest in a single request. Modern models support 400k–1M tokens, but even large windows are finite. Additionally, not all tokens are equally useful: **context rot** (the phenomenon where quality of reasoning degrades as context length increases) means that the model's ability to attend to distant information decreases.

For code generation specifically, a large feature might require:
- Specification and requirements (10k tokens)
- Existing codebase context (100k+ tokens)
- API/framework documentation (20k+ tokens)
- Generated code (50k+ tokens)
- Feedback and error messages (10k+ tokens)

This easily exhausts typical working context before the feature is complete. Decomposition ensures each sub-task fits comfortably within the available window, with room for both input specification and output generation.

## Structured Progress Tracking

The key practical insight from Anthropic's harness research is that explicit **progress files** (a canonical record of what has been done and what remains) form the backbone of sustained agent execution. Rather than relying on conversation history or implicit state, agents maintain a structured todo list or feature checklist that they update after each completed task.

This serves several functions:

1. **Checkpoint creation**: After each sub-task, the agent has a concrete deliverable (a feature, module, test, or fix) that can be verified and committed.
2. **Dependency ordering**: Progress files encode which sub-tasks must complete before others. The agent can see that "fix Auth module" blocks "integrate Auth into API," preventing premature work.
3. **State recovery**: If an agent session dies or needs to resume, the progress file is the source of truth about what was completed and where to continue.
4. **Distraction prevention**: A structured todo prevents agents from context-switching arbitrarily. The next task is always explicit.

In practice, progress files are light structures: a checklist, a feature queue, or a DAG of task dependencies. Anthropic's harness uses a progress file that the agent reads at the start of each iteration, marks items complete, and writes back. This creates a visible feedback loop: each task's completion is recorded and acknowledged before moving to the next one.

## Verify-After-Each-Step Pattern

Decomposition enables tight verification loops. After completing each sub-task, the agent:

1. **Compiles or runs tests** for that piece in isolation
2. **Fixes errors** found in the verification step
3. **Integrates** the piece into the larger system (if applicable)
4. **Marks the task complete** in the progress file

This is fundamentally different from stream-of-consciousness generation, where code is produced without intermediate proof that it works. Each sub-task is treated as a mini-project with its own verification lifecycle. When the next agent iteration begins, it can assume all prior tasks have been validated.

## Dependency Ordering and Parallelization

Task decomposition also exposes the true dependencies between sub-tasks. Some tasks are truly sequential (feature A must exist before feature B can use it), while others are independent and could theoretically run in parallel.

Anthropic's approach with multiple agents (e.g., building a C compiler with parallel Claudes) applies this principle: one orchestrator agent plans the decomposition and dependency DAG, and multiple worker agents execute independent sub-tasks concurrently. Each worker updates a shared progress file, and the orchestrator ensures dependencies are respected.

For single-agent harnesses, explicit dependencies still help: the agent can see which completed tasks unblock which pending ones, improving scheduling decisions and reducing wasted motion.

## Cognitive Load and Token Efficiency

From a token-usage perspective, decomposition is efficient:

- A single large request for a complex feature might generate 100k tokens of response, much of which is intermediate reasoning or discarded dead-ends.
- Ten decomposed sub-tasks each generating 5k–10k tokens of response total 50k–100k tokens, but most of that output is retained and used.
- Smaller requests are faster and cheaper to execute; errors can be fixed locally without regenerating the entire response.

From a reasoning perspective, smaller tasks let agents use tokens more effectively. Attention is concentrated on the problem at hand rather than diluted across distractions. Tests and errors in a small sub-task are easier for the agent to reason about and fix.

## Practical Decomposition Patterns

### Feature-Based Decomposition
Break work by feature or capability. Each task is a single user-facing or architectural feature:
- Task 1: Implement basic CRUD API endpoints
- Task 2: Add authentication layer
- Task 3: Integrate caching
- Task 4: Write integration tests

### Layer-Based Decomposition
Decompose by layer or abstraction level, building bottom-up:
- Task 1: Define data models and types
- Task 2: Implement database schema and migrations
- Task 3: Build repository/ORM layer
- Task 4: Build API or service layer
- Task 5: Build controller/handler layer

### Test-Driven Decomposition
Start by writing failing tests, then implement features to pass them:
- Task 1: Write test suite for feature
- Task 2: Implement feature logic
- Task 3: Verify all tests pass
- Task 4: Refactor and optimize

### Refactoring Increments
When improving existing code:
- Task 1: Extract one function/module
- Task 2: Add type annotations
- Task 3: Optimize a specific hot path
- Task 4: Add error handling

## Related Concepts

Large-scale decomposition connects to broader software engineering practices: **agile sprint planning** (time-boxing tasks), **continuous integration** (verifying each change independently), **monorepo tooling** (explicit build task graphs), and **microservices decomposition** (bounded contexts and interfaces). The difference is that AI agents need **explicit, readable progress and dependency encoding** because they cannot rely on developer intuition about what to do next.

LLM research on **chain-of-thought prompting** and **structured reasoning** shows similar patterns: forcing models to break reasoning into steps, rather than jumping to conclusions, improves accuracy. Task decomposition is the execution-time equivalent of this principle applied to code generation: thinking through the architecture step-by-step, not trying to generate it all at once.

## Trade-offs and Limitations

Decomposition is not always optimal:

- **Over-decomposition** can fragment work into so many tiny tasks that coordination overhead (reading the progress file, context-switching) dominates execution time.
- **Async coordination** becomes necessary when decomposing across multiple agents, introducing complexity in dependency resolution and shared state management.
- **Hidden dependencies** sometimes only become visible after a feature is partially implemented; rigid decomposition upfront can fail to account for emergent constraints.
- **Business requirements volatility** means initially decomposed plans may need reworking if specs change mid-execution.

The sweet spot appears to be **a few moderately-sized tasks** (4–10 sub-tasks for a typical feature-length project) with **explicit dependency tracking** and **regular re-planning**. Progress files should be read and updated at agent iteration boundaries, allowing for adaptive re-decomposition if blocked or new information emerges.

## Current State of Practice

As of 2026, Anthropic's harness design and Claude Code are the reference implementations of decomposition in production AI agents. OpenAI's (pre-integrated) approaches to code generation emphasize structured prompting and planning, but less explicit progress tracking. Google's agent frameworks and open-source tools like LangChain have incorporated decomposition concepts, though with varying degrees of support for progress files and dependency tracking.

The trend across the industry is toward **more structured decomposition** as a best practice, not an optional optimization. Agents that attempt large tasks without decomposition are increasingly seen as unreliable for sustained execution, especially in software engineering contexts where code must compile and tests must pass.