# Prompt Testing and Evaluation — Frameworks, Rubrics, A/B Testing, and Regression Detection

## From Trial-and-Error to Structured Evaluation

Early LLM work was trial-and-error prompting: write a prompt, run it, check if it worked. Iterate by feel. Modern production systems treat prompt evaluation like software testing—with benchmarks, rubrics, statistical rigor, and regression detection. You don't ship code without tests; you shouldn't ship prompts without evals.

The shift is profound: **evals are rising because prompt engineering alone is falling.** A prompt that works on 5 examples doesn't mean it works on production traffic. The industry converged on a principle: measure what "good" means first, then iterate toward it.

## Defining Success: The Rubric

A **rubric** is a structured scoring framework that defines what "good" looks like for a task. It's the specification for evaluation.

### Rubric Structure

Single-criterion rubric:
```
Task: Generate a customer support response

Criterion: Accuracy
  Score 1: Response contains factual errors or wrong information
  Score 2: Response is partially accurate but missing key details
  Score 3: Response is accurate and complete
```

Multi-criterion rubric (for complex tasks):
```
Task: Writing a feature specification for an API

Criteria:

1. Clarity (0-10)
   0: Incomprehensible
   3: Some confusion; key concepts unclear
   7: Clear with minor ambiguities
   10: Crystal clear; unambiguous

2. Completeness (0-10)
   0: Missing most elements (request, response, error cases)
   5: Most elements present, edge cases partially covered
   10: All elements including edge cases, rate limits, auth

3. Implementability (0-10)
   0: Contradictory or infeasible
   5: Implementable but requires clarification
   10: Developer can implement without additional questions

Overall score: average of three criteria (adjusted by weight if needed)
```

### Rubric Design Principles

**Be specific.** "Good quality" is useless. "Avoids hallucinated API endpoints" is evaluable.

**Include negative examples.** Show what scores 1, 3, and 5 look like. Humans and LLM-based evaluators both need anchors.

```Bad rubric:
"How helpful is this response?" (1-5)

Good rubric:
Score 1: Refuses to engage or provides unrelated information
Score 3: Addresses the question but misses key context
Score 5: Directly answers the question with evidence
```

**Weight by business consequence.** Dangerous errors (security, financial) weight higher than style issues.

```
Rubric for AWS bill analyzer:

Cost accuracy (weight: 5x): Error > $100 → Score 0
Recommendation quality (weight: 2x): Suggestions not grounded in data → Score 1
Clarity (weight: 1x): Jargon-heavy explanation → Score 2
```

## Evaluation Frameworks: Tools and Patterns

### Human Evaluation (The Gold Standard, When Affordable)

**What**: A domain expert reads responses and scores them against the rubric.

**Pros**:
- Catches nuances automata miss (subtle bias, tone, context misalignment)
- Ground truth for training automated evaluators
- Catches entire rubric dimensions (e.g., "is this safe?") that are hard to encode algorithmically

**Cons**:
- Expensive (~$1-5 per example at scale)
- Slow (human throughput limits iteration speed)
- Inconsistent (inter-rater disagreement 15-30%, even among experts)

**Best for**: High-stakes decisions (security recommendations), novel tasks, establishing ground truth for training automated evaluators.

### Automated Rubric Evaluation (LLM-as-Judge)

**What**: An LLM reads a prompt, generated output, and rubric, then scores the output.

```
System prompt for evaluator:
"You are an expert evaluator. Score this response against the rubric.
Be strict but fair. Explain your reasoning before giving a score."

Input:
---
Task: "Write a marketing email for a tech conference"
Response: [output from main model]
Rubric:
  1. Clarity (0-10)
  2. Engagement (0-10)
  3. Call-to-action strength (0-10)
---

Output:
"Clarity (8/10): The writing is professional and easy to follow.
Engagement (6/10): The opening is generic; could be more compelling.
Call-to-action (7/10): Clear but lacks urgency.
Overall: 7/10"
```

**Pros**:
- 1000x cheaper than human eval (~$0.01 per example)
- Fast (run instantly, not weeks)
- Consistent across runs
- Works with any rubric

**Cons**:
- LLM-based judges often miss subtle failures or have their own biases
- Can be gamed (model learns what the judge rewards, not what humans want)
- Requires validation against human gold standard (judge calibration)

**Production pattern**: Use LLM-as-judge for rapid iteration (optimize toward rubric), then spot-check with human eval to ensure the rubric aligns with reality.

### Unit Test Style Evaluation (Automated Signals)

**What**: Specific, testable assertions about output, independent of subjective rubrics.

```
Task: Code generation
LLM generates Python function

Automated checks:
  ✓ Syntax is valid Python (compile)
  ✓ Passes on provided test cases (functional)
  ✓ No hallucinated library imports (grep against Python stdlib)
  ✓ No hardcoded secrets (check for API keys, passwords)
  ✓ Performance: < 100ms on benchmark dataset

Pass/fail result: 4/5 checks passed
```

**Pros**:
- Completely deterministic and automated
- Catches entire classes of errors (syntax, import hallucinations, security issues)
- Cheap and fast

**Cons**:
- Only works for tasks with clear correctness criteria
- Doesn't measure subjective quality (is the generated code idiomatic?)

**Best for**: Code generation, data extraction, structured formatting where right-answer tests are possible.

## A/B Testing Prompts: Comparing Variants

Once you have a rubric and eval framework, comparing two prompts is systematic hypothesis testing.

### Design: Hypothesis, Sample Size, Significance

**Hypothesis**: "Adding a few-shot example to the prompt improves accuracy by 10%+"

**Null hypothesis**: "No effect (or <5% improvement)"

**Setup**:
```
Control: Current prompt (no examples)
Treatment: New prompt (+ 3 examples)

Test set: 100 real customer queries (representative of traffic)

Eval rubric: Accuracy (0-10)
```

### Significance Requires Sample Size

LLM outputs are noisy. A 2-3% improvement on 10 examples is noise. On 1000 examples, it's likely real.

**Rule of thumb**: N > 30 for initial screening, N > 100 for confidence, N > 500 for production gating.

```
Example: Prompt A vs. Prompt B

Run 1 (N=10):
  A: 7.2 avg
  B: 7.9 avg
  Difference: +0.7 (looks good, but is it real?)
  
Run 2 (N=100):
  A: 7.15 avg
  B: 7.18 avg
  Difference: +0.03 (is this statistical noise?)

Run 3 (N=500):
  A: 7.14 avg
  B: 7.19 avg
  Difference: +0.05 (tiny but consistent)
  
Conclusion: Prompt B is slightly better, but improvement is marginal.
Cost tradeoff: Worth switching if cost is same; not if B is more expensive.
```

### Statistical Difference vs. Practical Difference

Two distributions can be statistically different but practically negligible.

```
Control: Mean 7.1, StdDev 1.2 (excellent consistency)
Treatment: Mean 7.5, StdDev 0.5 (tighter but modest improvement)

Statistical test: p-value < 0.05 (statistically significant)

Practical consideration: Users care about tail behavior.
  7.5% of Control scores < 3 (bad)
  1% of Treatment scores < 3

Practical significance: Treatment dramatically reduces failures. Worth switching even for small mean improvement.
```

## Regression Testing: Preventing Quality Decay

As you iterate on prompts, ensure you don't break what was already working.

### Regression Test Suite

Build a set of test cases representing:
- Core functionality (high-stakes, must-pass)
- Edge cases (corner cases that broke before)
- Known difficult examples (challenging but important)

```
Regression tests for code generation:

Core functionality:
  - Generate a simple function (Fibonacci) ✓
  - Add error handling ✓
  - Write matching unit tests ✓

Edge cases:
  - Generate code with unusual var names ✓
  - Synthesize code mixing 2+ paradigms ✓
  - Respond to "add this feature to existing code" ✓

Known difficult:
  - Off-by-one errors in loops (historically 20% failure) ✓
  - Confusing specification with grammar bugs ✓
  - Generate code using deprecated libraries ✓

Threshold: 100% pass on core + edge, 90%+ on difficult
If new prompt fails > 5% of core tests, reject it.
```

### Automated Regression Checks (CI/CD for Prompts)

```
On every prompt change:

1. Generate outputs on regression suite
2. Compare to baseline (previous known-good version)
3. Calculate:
   - Regression rate: % of baseline cases that now fail
   - Quality delta: Change in rubric score vs. baseline
4. Gate policy:
   If regression > 5% AND quality delta < +20%: Reject
   If regression > 10%: Reject regardless
   If quality delta > +30%: Allow even with 10% regression (intentional tradeoff)
```

## Practical Frameworks: What Teams Use

### Anthropic's Evals Framework

```
Define evaluation tasks: Code generation, reasoning, summarization, coding
  
Provide ground truth (reference answers) or rubric
  
Run model against benchmark
  
Compare accuracy/quality/cost across:
  - Models (Haiku vs. Sonnet)
  - Prompts (few-shot vs. chain-of-thought)
  - Hyperparameters (temperature 0.7 vs. 1.0)
  
Result: Structured comparison enabling routing and prompt selection
```

### Research-Grade: RESEARCHRUBRICS Benchmark

A 2,500+ example benchmark of deep research tasks with expert-written rubrics.

```
Example task: Find evidence that contradicts a scientific claim

Ground truth rubric:
  - Finding validity (does the evidence actually contradict?)
  - Source reliability (is the source credible?)
  - Explanation clarity (is the contradiction explained?)
  
Evaluation: Researchers score candidate answers
Result: Compare models/prompts on expert-curated criteria
```

## Benchmark Design: Building Your Own

Not every task has a public benchmark. For proprietary or niche tasks, you'll build your own.

### Steps

1. **Sample real traffic**: Take 100-300 representative queries from your production system
2. **Generate answers** with your current best prompt/model
3. **Have humans score** on your rubric (50-100 examples for calibration)
4. **Compute baseline**: Calculate mean score (e.g., 7.2/10)
5. **Version control the benchmark**: Save queries + ground-truth scores. Reruns must match (deterministically).
6. **Use as gating criteria**: New prompts must beat baseline by >5% to ship.

```
Benchmark for customer support quality:

100 real support tickets (sampled last quarter)
  
Rubric: Accuracy (0-5), Tone (0-5), Completeness (0-5)
  
Human eval: 10 expert customer support reps scored all 100
  
Baseline: 13.2/15 average (88%)
  
New prompt test (50 tickets):
  Scores: 13.5 avg (90%)
  
Improvement: +2% absolute (1.8% relative)
  
Decision: Barely worth switching (marginal benefit, risk of regression)
```

## The Caveat: Evals Don't Replace Monitoring

A strong eval framework tells you if a change is good in controlled conditions. **Production monitoring tells you if it's good in the wild.**

```
Offline eval: New prompt scores 91% on benchmark
-> Ship it

Production metrics (week 1):
  User satisfaction: Down from 8.2 to 7.9
  Error rate: Up from 2% to 4%
  
Problem: Benchmark didn't include an edge case that appears in real traffic (10% of queries)
  
Action: Add failing cases to benchmark, retrain, redeploy
```

Always pair strong offline eval with production monitoring (user feedback, error signals, explicit complaints). Neither alone is sufficient.

## See Also

- **genai-agentic-coding.md** — test-driven agent evaluation
- **genai-model-routing.md** — using eval results to route between models
- **genai-lm-cost-optimization.md** — cost-quality tradeoffs in prompt selection
- **ml-experiment-tracking.md** — experiment infrastructure for prompt variants