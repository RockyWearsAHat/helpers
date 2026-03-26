# Model Routing — Cost vs. Capability Selection and Dynamic Dispatch

## The Economics of Model Selection

LLM costs don't scale linearly with capability. A 40% cheaper model might be 70% as capable for some tasks, making it the wrong choice to save money. Conversely, using the most expensive model for every task wastes resources on queries that don't need that capability.

**Model routing**—selecting the right model for each task—is the primary lever for controlling LLM costs in production. It subsumes hardware selection, prompt optimization, and token caching in impact.

### The Claude Model Hierarchy (2026)

Current Anthropic offerings represent three distinct capability tiers:

| Model | Input $/MTok | Output $/MTok | Speed | Capability | Context | Use Case |
|-------|------------|--------------|-------|---------|---------|----------|
| Haiku 4.5 | $1 | $5 | ~600 tok/s | Moderate | 200K | High-volume, latency-sensitive, routine tasks |
| Sonnet 4.6 | $3 | $15 | ~150 tok/s | Strong | 1M | General production, balanced default |
| Opus 4.6 | $5 | $25 | ~60 tok/s | Highest | 1M | Complex reasoning, multi-stage planning |

**Cost implications:** Using Opus for a simple classification task ($0.10-0.50 per request) when Haiku would suffice ($0.01-0.05) is a **5-10x waste**. But using Haiku for a complex architecture decision where Opus would get it right might mean regenerating 5x times (Haiku accuracy 40%, Opus accuracy 85%), which costs more total.

## Task Complexity as the Routing Signal

The fundamental insight: route based on **task complexity and consequence**, not budget alone.

### Complexity Tiers

**Tier 1: Routine, Low-Complexity Tasks (5-15% reasoning depth)**
- Classification, retrieval, formatting
- Entity extraction, templated generation
- Straightforward Q&A from context
- **Router decision**: Use Haiku 4.5

Examples:
```
Task: "Extract customer name from this support ticket"
Expected: Haiku catches all straightforward names (96%+ accuracy)

Task: "Summarize this email in 2 sentences"
Expected: Haiku handles factual summaries (95% quality)

Task: "Is this review positive or negative?"
Expected: Haiku achieves 92% accuracy on sentiment
```

**Tier 2: Moderate Complexity (30-60% reasoning depth)**
- Code generation for straightforward logic
- Multi-step retrieval and synthesis
- Debugging with provided error messages
- Quality writing and rewriting
- **Router decision**: Use Sonnet 4.6

Examples:
```
Task: "Write a function to calculate compound interest"
Expected: Sonnet generates correct, tested code (90% first-try)
Expected: Haiku might miss edge cases around rounding (70%)

Task: "Rewrite this paragraph for clarity"
Expected: Sonnet's suggestions improve readability

Task: "Debug this type error in TypeScript"
Expected: Sonnet provides correct diagnosis with fix
```

**Tier 3: High Complexity (70-100% reasoning depth)**
- Novel architecture decisions
- Multi-file code refactors with cross-cutting concerns
- Synthesis across contradictory information
- Root cause analysis of systemic failures
- **Router decision**: Use Opus 4.6

Examples:
```
Task: "Design a distributed caching strategy for high-traffic API"
Expected: Opus weighs trade-offs (consistency, availability, cost)
Expected: Sonnet proposes reasonable but suboptimal approaches
Expected: Haiku generates generic boilerplate

Task: "Why is this agent getting stuck in infinite loops?"
Expected: Opus identifies root cause (memory structure issue) in 1 turn
Expected: Sonnet might need multiple clarifying questions
Expected: Haiku generates surface-level suggestions
```

### The Fail-Fast Pattern: Cheapest-First Validation

Don't route based on task complexity alone. **Route based on validation checkpoint cost**.

```
Workflow: Generate a script, validate it passes tests

Naive approach:
  Opus generates script → Run tests (30s) → Pass? → Cost: $0.20 per run

Better approach:
  Haiku generates script → Run tests (30s) → On failure, escalate to Sonnet
  → Cost: $0.02 + $0.10(failures) = ~$0.05 per run (if 50% fail)

Best approach:
  1. Haiku generates script
  2. Haiku writes unit tests (cheap, format validation)
  3. Run tests locally → Fail? Return to Haiku for refinement
  4. Only send to Sonnet if Haiku runs out of refining ideas
  → Cost per run: ~$0.03 (most fixed on first try with cheap loops)
```

The key: **fail fast at cheap stages** before sinking cost into expensive stages.

## Dynamic Routing Architecture

Production systems don't hardcode model choices. They route dynamically based on:

1. **Task metadata**: Is this a known routine task (classification) or unknown (novel architecture)?
2. **Recent performance**: Did this query class (e.g., "customer service replies") perform well with Haiku? Keep using it.
3. **User preference**: Is the user paying for "always-best" or "optimal cost"?
4. **Fallback chains**: Route to Haiku, monitor error signals, escalate to Sonnet if needed.

### Example: Complexity Classifier

```
Input: Raw user query "Write a function to..."

Classifier (small Sonnet call, ~$0.001):
  Analyzes query, outputs complexity score 1-10
  
Decision logic:
  Score 1-3: Route to Haiku
  Score 4-7: Route to Sonnet
  Score 8-10: Route to Opus
  
  Expected accuracy of classifier itself: 80-90%
  Break-even point: Classifier cost ($0.001) < Wrong model waste (>$0.005)
```

### Parallel Candidate Generation and Ranking

For high-stakes decisions, generate with two models in parallel, rank outputs:

```
Architecture decision task:
  
  Haiku generates approach A (50ms, $0.02)
  Sonnet generates approach B (200ms, $0.08)
  [Parallel: no wait-time cost]
  
  Both approaches scored by criteria:
    - Feasibility (1-10)
    - Cost ($)
    - Maintainability (engineering judgment)
  
  Present both to human; let them choose
  
  Cost: $0.10 per decision (vs. $0.25 for Opus alone + $0.05 for Haiku)
  Benefit: Wider option space for same cost
```

## Cost Composition Estimates (2026)

Typical production system with model routing:

### Customer Support Agent (High Volume)

Tier 1 (routine issue categorization): 60% of requests → Haiku @$0.01
Tier 2 (specific troubleshooting): 35% of requests → Sonnet @$0.05
Tier 3 (escalation analysis): 5% of requests → Opus @$0.15

**Average cost per support interaction**: (0.60 × $0.01) + (0.35 × $0.05) + (0.05 × $0.15) = **$0.025**

**Without routing (all Sonnet)**: $0.05 per interaction (2x cost)
**Without routing (all Opus)**: $0.15 per interaction (6x cost)

### Code Generation with Validation

Tier 1 (format/lint checks): Haiku @$0.01
Tier 2 (implementation): Sonnet @$0.08
Tier 3 (architecture review): Opus @$0.10

One complete feature implementation:
- Success on Haiku: $0.01
- Haiku fails, escalate to Sonnet: $0.01 + $0.08 = $0.09
- Sonnet fails, need architecture review: $0.09 + $0.10 = $0.19

**Expected cost** (70% Haiku success, 20% Sonnet success, 10% need Opus):
(0.70 × $0.01) + (0.20 × $0.09) + (0.10 × $0.19) = **$0.035**

## When NOT to Route: The Unified Model Case

Routing adds complexity: separate prompts, fallback chains, orchestration logic. **In early/small deployments**, a single model is often better:

✓ Route when:
- High volume (>100 requests/day)
- Cost is driver (tight margins)
- Requests fall into clear, repeatable categories
- Performance/cost trade-offs are acceptable

✗ Don't route when:
- Low volume (<100 requests/day)
- Latency is critical (routing adds ~50-100ms decision time)
- All requests are unpredictable "novel" queries
- Simplicity > cost savings

## Practical Routing Patterns

### Pattern 1: Explicit Routing (Rule-Based)

```
if task_type == "classification":
  use Haiku
elif task_type == "generation":
  use Sonnet
else:
  use Opus
```

**Pros**: Predictable, debuggable, deterministic
**Cons**: Doesn't adapt to reality; requires manual category definition

### Pattern 2: Learned Routing (Observational)

```
Track accuracy by query class and model:
  "bug diagnosis with Sonnet" → 78% high-quality fixes
  "bug diagnosis with Haiku" → 42% high-quality fixes
  
Route future bug diagnostics to Sonnet
```

**Pros**: Adapts to real-world performance
**Cons**: Requires historical data; needs retraining as models update

### Pattern 3: Confidence-Based Escalation

```
Haiku generates response
  Extracts confidence score (LLM-generated or pattern-based)
  
If confidence < 0.6:
  Escalate to Sonnet
  If Sonnet confidence < 0.7:
    Escalate to Opus
```

**Pros**: Catches likely-bad outputs before they reach users
**Cons**: Confidence calibration is hard; models often miscalibrated

### Pattern 4: Human-in-the-Loop with Appeal

```
Route to cheap model
Monitor cost vs. error rate
If error rate exceeds threshold:
  Show user: "I'm less confident. Would you like a second opinion?"
  If user clicks "yes" → Escalate to Sonnet
  
Cost: Only pay for escalation when user explicitly wants it
Benefit: User gets fast response by default, expensive second-opinion on demand
```

## Cold-Start Problem: Routing Without Data

When launching a new system, you don't have performance data. Start conservative:

**Week 1-2**: Operate entirely on Sonnet
- Gather performance baseline (accuracy, latency, cost)
- Identify which query classes underperform (those are Tier 3 candidates)
- Identify which consistently succeed (Tier 1 candidates)

**Week 3-4**: Introduce Haiku for Tier 1
- Monitor substitution on clear wins (classification, formatting)
- Track failure rate; if >10%, revert to Sonnet

**Week 5-6**: Introduce Opus for Tier 3
- Flag hard failures from Sonnet; route to Opus
- Track cost reduction vs. quality improvement

This staged rollout minimizes risk of silent performance degradation.

## Monitoring: Key Metrics for Routing Systems

- **Accuracy by model by task**: Are cheaper models performing as expected per task class?
- **Cost per task**: Is routing actually reducing cost, or is escalation too frequent?
- **Latency per path**: Does the routing decision overhead (time to classify) save time vs. just using a fast model?
- **User satisfaction**: Do users notice quality differences between paths?
- **Regression detection**: Did a new model version break assumptions about a tier?

## See Also

- **genai-lm-cost-optimization.md** — token caching, prompt compression, output budgeting
- **genai-agentic-coding.md** — routing strategies within multi-stage agent pipelines
- **genai-prompt-testing.md** — evaluating output quality across models before routing
- **ml-operations.md** — monitoring and observability for production ML systems