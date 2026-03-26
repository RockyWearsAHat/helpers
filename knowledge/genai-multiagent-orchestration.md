# Multi-Agent Orchestration — Patterns for Coordination, Communication, and Handoffs

## Beyond Single Agents: When and Why to Decompose

A monolithic single agent forced to reason about everything struggles:

- **Prompt complexity explosion**: "You are an expert in customer service, fraud detection, billing, API design, and code generation" → unfocused prompt, confused agent
- **Tool overload**: 50 tools in one call registry causes the agent to hallucinate tool names or pick wrong ones
- **Context bloat**: Fitting all knowledge into one context window costs tokens and reduces reasoning quality
- **Inefficiency**: One agent reasoning about a database schema design for an hour (Opus-level reasoning) to later hand off to a tier-1 agent for "apply this schema change" (Haiku-level work)

**Multi-agent orchestration** decomposes large tasks into specialized agents, each with:
- Specific domain (code gen, test gen, review)
- Targeted tool access (code gen doesn't need a security scanner)
- Chosen model (cheap models on cheap, fast work; expensive on deep reasoning)
- Clear input/output contracts

Result: better quality, lower cost, easier debugging.

## The Orchestration Hierarchy

Not all multi-agent systems are equal. They range in complexity:

### Level 1: Sequential Pipelines (Simplest)

Agents work in predefined order: A → B → C

```
Feature requirements (input)
  ↓ [Agent 1: Code gen]
  Generate initial code
  ↓ [Agent 2: Test gen]
  Write tests for code
  ↓ [Agent 3: Security audit]
  Check for vulnerabilities
  ↓ [Agent 4: Refactor]
  Improve code quality
  [Output: Production-ready feature]
```

Characteristics:
- No branching: always follows the same pipeline
- Each agent reads previous agent's output
- State flows linearly through the pipeline
- Failures: If any agent fails, pipeline stops (no recovery)

**When to use**: Clear linear dependencies, each stage improves on previous, predictable workflow.

**When to avoid**: Stages are parallelizable, need dynamic routing, need backtracking.

### Level 2: Conditional Routing (Moderate Complexity)

Agents fork conditionally based on intermediate results.

```
Feature requirements (input)
  ↓ [Router: Estimate complexity]
  Is this complex (>500 LOC)?
  ├─ YES → [Opus agent: Full feature design]
  └─ NO → [Sonnet agent: Quick implementation]
  ↓ [Code gen]
  Generate code (model chosen by previous step)
  ↓ [Decision point]
  Tests pass?
  ├─ YES → Success, done
  └─ NO → Re-route to different agent or add debug step
```

Characteristics:
- Branching logic based on state (complexity, success, risk level)
- Different paths converge at common end stages
- Enables fail-fast (cheap validation before expensive stages)
- Allows backtracking (retry with different approach)

**When to use**: Tasks with variable difficulty, want cost optimization, need fallback paths.

### Level 3: Concurrent/Parallel Agents (Higher Complexity)

Agents work simultaneously on the same problem from different angles.

```
Request (input)
  ┌─ [Agent 1: Technical reviewer] ────┐
  ├─ [Agent 2: Business reviewer] ──── Aggregator
  └─ [Agent 3: Edge case finder] ──────┘
  ↓
[Synthesize: Combine all 3 reviews into one recommendation]
```

Characteristics:
- Multiple agents work in parallel, reducing latency
- Aggregator combines results (voting, weighted merge, LLM synthesis)
- Enables ensemble reasoning (better quality than any single agent)
- Higher latency and cost than sequential

**When to use**: Time-sensitive decisions, quality is critical, diverse perspectives improve answers.

### Level 4: Dynamic Handoffs (Highest Complexity)

Agents transfer context and control based on conversation state, not predefined choreography.

```
User: "I need an API to store photos"

Agent 1 (API Designer): 
  Designs REST schema
  → Detects: "This will need authentication"
  → Hands off to Agent 2

Agent 2 (Security Specialist):
  Reviews auth design
  → Detects: "Need rate limiting"
  → Hands off to Agent 3

Agent 3 (Infrastructure):
  Designs rate-limiting backend
  → Ready, hands off to Agent 1 for refinement

Agent 1 (API Designer):
  Reviews complete design
  → Done
```

Characteristics:
- Agents decide when to hand off, not orchestrator
- State flows opportunistically (non-linear)
- Requires explicit context passing between agents
- Highest complexity but most flexible

**When to use**: Multi-domain problems, agents interact in unpredictable order, human-like collaboration needed.

## Orchestration Patterns (Practical Designs)

### Pattern: Sequential with Retry (Most Common)

```python
# Pseudocode
agents = [CodeGen, TestGen, SecurityAudit, Refactor]
state = initial_task
max_retries = 3

for agent in agents:
    retries = 0
    while retries < max_retries:
        output = agent.run(state)
        if output.success:
            state = output  # Pass to next agent
            break
        else:
            retries += 1
            if agent.can_self_correct(output.error):
                # Agent reads error and regenerates
                state.add_feedback(output.error)
            else:
                # Hand to human or escalate
                raise HandoffRequired(agent, output.error)
```

**Flow**:
1. Agent runs
2. Check if output is good (pass test, no syntax errors, etc.)
3. If yes: pass to next agent
4. If no: agent reads error and regenerates
5. After N failures: stop, require human review

**Cost**: Failures cost extra agent calls. Invest in good error signals (tests, linters, type checkers) to make failures clear.

### Pattern: Fail-Fast Filtering (Cost Optimization)

Route expensive validators only after cheap validation passes.

```python
# Expensive validation: Security audit
# Cheap validation: Linting, syntax check, basic tests

pipeline = [
    (Linter, "cheap", required=True),
    (Tests, "cheap", required=True),
    (Haiku, "cheap", estimated_quality=low),
    (Sonnet, "moderate", estimated_quality=medium),  # Gate for Sonnet
    (SecurityAudit, "expensive", required_on_high_risk),
    (Opus, "expensive", required_for_design_review),
]

for agent, cost_tier, _ in pipeline:
    output = agent.run(state)
    if output.fails_gating():
        return Failed(agent, output)  # Stop here, fail fast
    state = output
```

Benefit: 80% of outputs fail linting; don't waste Opus time on them.

### Pattern: Supervisor + Specialized Workers (Hierarchical)

```
Supervisor (Opus acting as orchestrator):
  Receives task
  Decides task type (API design, code gen, refactor, etc.)
  Routes to specialized worker
  
Workers (specialized agents):
  CodeGen (Sonnet + code tools)
  TestGen (Sonnet + test tools)
  SecurityAudit (Sonnet + security database)
  
Supervisor receives results:
  Validates outputs
  Routes for additional processing if needed
  Synthesizes final result
```

The supervisor doesn't do the work; it orchestrates. This avoids the "too many tools and domains" problem by narrowing each worker's scope.

### Pattern: Chat-Based Group Discussion (Agentic Consensus)

```
MultiAgentDebate:
  Agent 1 (Frontend): "We should use React"
  Agent 2 (Backend): "Use Python FastAPI"
  Agent 3 (DevOps): "Docker + Kubernetes orchestration"
  
  Moderator (LLM):
    "Group, compile a unified architecture document integrating all perspectives"
  
  Result: Agreed-upon tech stack with rationales
```

Use when:
- Multiple valid perspectives exist
- You want documented discussion (not just one agent's opinion)
- Human stakeholders want to see reasoning

## Communication Between Agents: File-Based, Message Queues, and APIs

Agents need a way to pass context. Three patterns:

### Pattern 1: File-Based (Simple, For Same-Machine Agents)

```
Agent 1 writes: /tmp/agent-state/spec.md
Agent 2 reads: /tmp/agent-state/spec.md
Agent 2 writes: /tmp/agent-state/implementation.py
Agent 3 reads: /tmp/agent-state/implementation.py
```

**Pros**: Simple, no infrastructure, easy to debug (files are visible)
**Cons**: Not distributed, not durable, race conditions if agents concurrent

### Pattern 2: Message Queues (Distributed, Async)

```
Agent 1 emits: { type: "code_generated", code: "...", timestamp: "..." } → Queue
Agent 2 subscribes: Reads from queue, processes, emits: { type: "tests_written", ... }
Agent 3 subscribes: Reads from queue, processes, emits: { type: "audit_complete", ... }
```

**Pros**: Distributed, durable, async (agents don't wait for each other), scalable
**Cons**: Infrastructure complexity, eventual consistency (ordering can be fuzzy)

### Pattern 3: Request-Response API

```
Orchestrator: POST /api/agents/code-gen with { spec: "..." }
CodeGenAgent: Responds with { code: "...", status: "success" }

Orchestrator: POST /api/agents/test-gen with { code: "..." }
TestGenAgent: Responds with { tests: "...", status: "success" }
```

**Pros**: Simple orchestration, sync (orchestrator waits for response, simpler control flow)
**Cons**: Tight coupling, blocking calls (slow if one agent is slow)

## Context Passing and Sprint Contracts

Agents need context from previous stages. What should be passed?

### Minimal Context (Just the Result)

```
CodeGen → TestGen:
  { code: "function foo() {...}" }
  
TestGen generates tests.
Problem: TestGen doesn't know the intent. Tests might be technically correct but miss the spec.
```

### Fat Context (Everything)

```
CodeGen → TestGen:
  {
    original_spec: "...",
    code: "...",
    design_rationale: "...",
    error_history: [...],
    intent: "..."
  }
  
TestGen has full context but reads massive input (costs tokens).
```

### Sprint Contracts (Structured Handoff)

A middle ground: standardized context schema that each agent populates.

```
{
  "task_id": "feature-123",
  "original_spec": "User story: ...",
  "stage": "test-generation",
  
  "code_generation": {
    "output": "...",
    "rationale": "Chose this approach because...",
    "edge_cases_considered": ["...", "..."],
    "known_issues": ["..."],
  },
  
  "test_generation": {
    "output": "...",
    "coverage_report": "...",
    "failed_cases": []
  }
}
```

Each agent:
1. Reads the full contract
2. Adds its own stage section
3. Passes to next agent

Benefits:
- Bounded context (each section is finite)
- Explicit handoff (each agent knows what to expect from previous)
- Audit trail (who did what, in what order)
- Resumability (can restart from any stage)

## Error Handling and Recovery

What happens when an agent fails?

### Strategy 1: Immediate Escalation (Fail-Fast)

```
Agent fails → escalate to human
Cost: Blocks workflow
Benefit: No bad output enters system
Use for: High-stakes decisions (security, finance)
```

### Strategy 2: Retry with Feedback

```
Agent fails → Read error signal → Agent regenerates
Repeat up to N times
If still failing → escalate
Cost: Extra agent calls
Benefit: Often recovers automatically
Use for: Errors that are addressable (syntax, logic)
```

### Strategy 3: Alternative Agent

```
Sonnet fails (low confidence) → Route to Opus
Opus fails → Escalate to human
Cost: More expensive, slower
Benefit: Higher chance of success before escalation
Use for: Task-critical but moderate frequency
```

### Strategy 4: Partial Success + Qualification

```
CodeGen partially succeeds:
  - 60% of API endpoints generated
  - 40% skipped (too complex)
  
Output: "Successfully generated X, manual work needed for Y"

User can:
  - Accept partial solution
  - Provide more spec for skipped parts
  - Re-run agent with guidance
```

## Monitoring Multi-Agent Systems

Single agents are hard to debug. Multi-agent workflows are harder. Key observables:

- **Per-agent latency**: How long does each stage take?
- **Per-agent failure rate**: Which stages fail most often?
- **Context size by stage**: Are we bloating state as it flows?
- **End-to-end success rate**: % of tasks that complete without escalation
- **Cost breakdown**: Which stage costs most?
- **Retry loops**: How many times does average task retry?

Example dashboard:
```
CodeGen:
  Latency: 45s avg (P99: 120s)
  Success rate: 85%
  Avg retries: 1.2
  Cost: $0.08 per call
  
TestGen:
  Latency: 30s avg (P99: 60s)
  Success rate: 92%
  Avg retries: 0.5
  Cost: $0.05 per call
  
[Bottleneck: CodeGen (longest latency, lowest success rate)]
→ Consider routing simple code to Haiku, complex to Opus
```

## See Also

- **genai-agentic-coding.md** — multi-stage code generation pipelines
- **genai-model-routing.md** — choosing models for specialized agents
- **genai-agent-memory.md** — context passing and persistent state across stages
- **api-design.md** — designing APIs for agent-to-agent communication
- **architecture-api-gateway.md** — orchestration infrastructure patterns