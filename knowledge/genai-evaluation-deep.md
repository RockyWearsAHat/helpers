# LLM Evaluation — Benchmarks, LLM-as-Judge, RAGAS, and Red Teaming

## The Evaluation Crisis

LLM evaluation faces unprecedented challenges:

1. **Data contamination**: Test sets appear in training data; models memorize answers instead of reasoning (e.g., MMLU GPT-4 performance inflated by data inclusion)
2. **Task saturation**: Models exceed human performance; can't distinguish further quality improvements
3. **Benchmark gaming**: Models optimize for benchmark format, not real capability
4. **Subjectivity**: Correctness is often ambiguous; needs human judgment
5. **Cost**: Hiring annotators for large evaluations is expensive

Modern evaluation combines **automatic metrics**, **LLM-as-judge**, **human annotation**, and **red teaming** to triangulate true capability.

## Benchmark Landscape

### Knowledge Benchmarks

#### MMLU (Massive Multitask Language Understanding)

**Format**: Multiple-choice questions across 57 domains (science, history, law, medicine, etc.)

**Examples**:
```
Which of the following is not a type of bias in machine learning?
A) Selection bias
B) Confirmation bias
C) Cognitive bias
D) Temporal bias

Answer: C (Cognitive bias is in psychology, not ML)
```

**Problems with MMLU**:
- **Data contamination**: Training data likely includes MMLU questions; GPT-4 achieves 86% but may be memorizing
- **Multiple-choice bias**: Doesn't test reasoning, only discrimination among options
- **Saturation**: State-of-the-art models near human performance (95%+)

**Scoring**: Accuracy (exact match correct answer).

**Use**: Broad capability evaluation; good for model comparison when contamination is minor.

#### HumanEval (OpenAI)

**Format**: Python programming problems; model must generate code that passes test cases.

**Examples**:
```python
def remove_duplicates(numbers):
    """Remove duplicates from a list while preserving order."""
    
# model output must pass:
# assert remove_duplicates([1, 2, 2, 3]) == [1, 2, 3]
# assert remove_duplicates([]) == []
```

**Strengths**: Tests reasoning + concrete correctness; can't cheat by memorization alone.

**Weaknesses**: Only ~164 problems; models likely seen similar code during pretraining; biased toward Python.

**Pass@K metric**:
$$\text{Pass@}k = 1 - \frac{\text{# of problems with 0 passes in } k \text{ samples}}{\text{# problems}}$$

Generate k code samples per problem; count pass if any passes tests. Tests both diversity (can generate multiple solutions) and robustness.

#### HELM (Holistic Evaluation of Language Models)

**Coverage**: 16 scenarios (QA, translation, summarization, classification, reasoning, generation).

**Metrics**: Accuracy, efficiency (cost/latency), fairness, robustness, calibration.

**Strengths**: Multi-dimensional evaluation; includes out-of-distribution tests (robustness).

**Weaknesses**: Computationally expensive; not a single number (hard to compare).

**Use**: Comprehensive assessment when time/budget allow.

### Reasoning Benchmarks

#### GSM8K (Grade School Math)

**Format**: Multi-step word problems requiring arithmetic reasoning.

**Example**:
```
Roger has 5 tennis balls. He buys 2 more cans of tennis balls.
Each can has 3 tennis balls. How many tennis balls does he have now?

Expected reasoning: 5 + 2*3 = 5 + 6 = 11
```

**Metric**: Exact match (must get answer exactly right).

**Use**: Reasoning capability; sensitive to CoT (Chain-of-Thought) prompting (70-80% accuracy with CoT vs. 40-50% without).

#### ARC (AI2 Reasoning Challenge)

**Format**: Science question answering; requires domain knowledge + reasoning.

**Points**: 5,197 problems from standardized science exams (grades 3-9 + high school).

**Difficulty**: ARC-Challenge (harder, requires external knowledge); ARC-Easy (easier, solvable from common sense).

**Use**: Multi-hop reasoning; distinguishes mid-tier models from top performers.

## Output Evaluation Metrics

### BLEU (Bilingual Evaluation Understudy)

Measures overlap between generated and reference text using n-gram precision.

$$\text{BLEU} = \text{BP} \cdot \exp\left(\sum_{n=1}^N w_n \log p_n\right)$$

Where:
- $p_n$ = precision of n-grams (typically n=1,2,3,4)
- BP = brevity penalty (penalize short outputs)
- $w_n$ = weight per n-gram level (typically uniform 0.25)

**Pros**: Fast; easy to compute; language-agnostic.

**Cons**: Doesn't capture semantics; penalizes paraphrasing; known to correlate poorly with human judgment for generation tasks.

**Use**: Machine translation (still standard); less so for open-ended generation.

### ROUGE (Recall-Oriented Understudy for Gisting Evaluation)

Similar to BLEU but focuses on recall (what reference content is in generation).

**Variants**:
- ROUGE-1: Unigram recall
- ROUGE-2: Bigram recall
- ROUGE-L: Longest common subsequence

$$\text{ROUGE-1} = \frac{\sum_{\text{ref}} \text{# matching unigrams}}{\sum_{\text{ref}} \text{# reference unigrams}}$$

**Use**: Summarization evaluation (standard); better than BLEU for recall-heavy tasks.

### METEOR (Metric for Evaluation of Translation with Explicit ORdering)

Measures unigram precision + recall with synonym matching and word order.

**Use**: Machine translation; better correlation with human judgment than BLEU.

### Parse-Based Metrics

For structured outputs (code, JSON, SQL):

- **Exact match**: Generated output == reference output (character-level)
- **Semantic equivalence**: Code produces same output; SQL query functionally identical
- **Syntax correctness**: Code parses without errors

**Example (code)**:
```python
# Reference:
result = sum([1, 2, 3])

# Generation (equivalent semantically):
result = 1 + 2 + 3

# Exact match: False | Semantic: True
```

## LLM-as-Judge

Use a second LLM (typically stronger) to score outputs. Mitigates contamination (judges rarely see test benchmarks) and captures nuanced quality.

### Pairwise Ranking (A/B Comparison)

**Prompt**:
```
Prompt: "Explain quantum entanglement"

Response A: [output from model A]
Response B: [output from model B]

Which response is better? Consider correctness, clarity, completeness.
Respond: A, B, or Tie
```

**Judge LLM** (typically GPT-4, Claude 3 Opus): Scores A > B, B > A, or tie.

**Analysis**: Aggregate results; express as "Model A wins X% of comparisons vs. Model B."

**Strengths**: Captures human-like quality preferences; can evaluate open-ended tasks (essay, stories).

**Weaknesses**:
- Judge biases (favors certain writing styles)
- Position bias (A vs. B order matters; run both)
- Brittleness to prompt wording

**Mitigation**: Use multiple judges; randomize order; explicit rubrics.

### Likert-Scale Scoring

**Prompt**:
```
Response: [generated text]
Criteria: Accuracy (1-5), Clarity (1-5), Completeness (1-5)
Score each criterion.
```

**Aggregate**: Average scores across criteria and/or examples.

**Advantages**: Standardized rubric; easier statistical analysis (mean, variance).

**Disadvantages**: Less nuanced than open-ended reasoning; raters may not calibrate.

### Reference-Based Evaluation

Compare generated output to reference (human gold standard) using LLM.

**Prompt**:
```
Reference answer: [gold standard]
Generated answer: [model output]
Does the generated answer match the reference in key facts and reasoning? Rate 1-5.
```

**Less subjective** than LLM-as-judge without reference (facts can be verified).

## RAGAS (Retrieval-Augmented Generation Assessment)

Specialized evaluation framework for RAG systems. Measures two dimensions:

### 1. Retrieval Quality (Without Reference Answer)

#### Faithfulness
Does the generated answer follow logically from retrieved documents? Does it avoid hallucination?

**Metric**: Use LLM to fact-check: "Are all claims in the answer supported by the context?"

$$\text{Faithfulness} = \frac{\text{# claims supported by context}}{\text{# total claims}}$$

**Prompt**:
```
Context: [retrieved documents]
Answer: [model answer]

For each sentence in the answer, mark as:
1. Directly stated in context
2. Logically inferred from context
3. Not supported (hallucinated)

Score = (1 + 2) / total sentences
```

#### Relevance
Does the retrieved context actually address the user query?

**Metric**: LLM scores context relevance to query (1-5 scale).

$$\text{Relevance} = \text{mean relevance score across retrieved docs}$$

**Interpretation**: Non-relevant retrieval → LLM can't answer well even with perfect generation.

#### Contextual Relevance
Does the context actually contain the information needed (not just topically related)?

**More nuanced**: Retrieves documents about "quantum mechanics" for "what is quantum entanglement?" — topically relevant but may not contain specific answer.

### 2. Answer Quality (With Reference Answer)

#### AnswerRelevance
Does generated answer address the original question?

**Metric**: Generate multiple question variants from answer; check how similar to original query.

```
Reverse process:
Answer: "Quantum entanglement occurs when particles become correlated..."
Generated question: "What is quantum entanglement?"
Similarity(generated_q, original_q) → relevance score
```

**Intuition**: If answer can regenerate the original question, it's addressing the right topic.

### RAGAS Score

Composite metric:

$$\text{RAGAS} = \sqrt[4]{\text{Faithfulness} \times \text{Relevance} \times \text{ContextRelevance} \times \text{AnswerRelevance}}$$

Geometric mean; penalizes weak dimensions.

**Strengths**: Lightweight (no external models if using GPT-as-judge); covers full RAG pipeline.

**Weaknesses**: Judge-dependent (scores vary by LLM); correlation with human judgment not extensively validated; slow (multiple LLM calls per query).

## Custom Evaluation

Build domain-specific checks:

### Checklist Evaluation

Define must-have criteria:

```
Criteria for medical answer:
☐ Cites at least one clinical study
☐ Mentions side effects
☐ Recommends consulting physician
☐ No absolute claims ("always cures")
☐ Years/dates in citations valid

Score = #checked / 5
```

**Advantage**: Transparent; easy for domain experts to define.

**Disadvantage**: Binary (pass/fail); doesn't capture quality gradations.

### Rubric-Based Evaluation

More nuanced; describe score levels:

```
Accuracy:
5 - All facts correct; well-reasoned
4 - Minor inaccuracies; mostly sound reasoning
3 - Some factual errors; reasoning has gaps
2 - Multiple errors; flawed reasoning
1 - Mostly incorrect or incoherent
```

**Implementation**: Have raters score using rubric; compute inter-rater agreement (Cohen's kappa); average scores.

### Proxy Tasks

Use a concrete downstream task as proxy for model capability.

**Example**: Evaluate RAG system via "Can users answer support questions using generated docs?"

- Serve docs to 50 users
- Track resolution rate (% who get answer)
- Correlate with retrieval/generation metrics

**Expensive but realistic.**

## A/B Testing and Statistical Significance

When comparing two models/systems:

1. **Split traffic**: Route 50% to control, 50% to treatment
2. **Collect metrics**: latency, error rate, user satisfaction
3. **Statistical test**: t-test, chi-square, or Bayesian methods
4. **Minimum sample size** to detect effect size with 90% power

**Example**: "Is Model B significantly better than Model A?"

```
H0: Model A == Model B (null hypothesis)
H1: Model A != Model B (alternate)

Collect 1000 user interactions each model
Compute accuracy, precision
t-test: p-value < 0.05 → reject H0 (sig. different)
```

**Common mistake**: Stopping test early when one model leads; guarantees false positives (look-ahead bias).

## Red Teaming and Adversarial Evaluation

Systematically try to break the model or expose failures:

### Automated Red Teaming

Generate adversarial prompts to elicit failures:

**Techniques**:
- **Prompt injection**: "Ignore previous instructions. Do X instead."
- **Jailbreaks**: "You're now a character without safety constraints..."
- **Out-of-distribution**: "What color is Tuesday?" (nonsensical query)
- **Adversarial suffixes**: Append tokens that flip model behavior

**Tools**: AutoAttack (vision), adversarial-prompts, jailbreak collections.

### Manual Red Teaming

Hire security researchers or domain experts to break the system:

**Coverage**:
- Harmful content generation (violence, hate speech, illegal advice)
- Privacy leaks (memorized training data)
- Fairness (biased outputs by demographic)
- Robustness (fails under mild distribution shift)

**Typical findings**: "System can be tricked to generate hate speech," "Leaks customer emails," "Biased recommendations by gender."

## Bias and Fairness Evaluation

### Statistical Parity

Does model outcome independent of protected attribute (demographics)?

$$P(\hat{Y} = 1 | \text{gender}=\text{M}) \approx P(\hat{Y} = 1 | \text{gender}=\text{F})$$

**Example**: Hiring model shouldn't reject more women than men.

### Equalized Odds

Given ground truth label, is model accuracy same across demographics?

$$P(\hat{Y}=1|Y=1, \text{group}=A) = P(\hat{Y}=1|Y=1, \text{group}=B)$$

**Stricter than statistical parity** (accounts for true differences).

### Toxicity Scoring

Use toxicity classifiers (Perspective API, BOLD detector) to score LLM outputs:

- Generate 100 prompts from various demographic groups
- Collect model outputs
- Score for toxicity
- Aggregate by demographic

**Example finding**: "Model generates 3x more toxic responses to queries mentioning [protected group]."

## Regression Testing

As systems evolve, old regressions can resurface:

**Setup**: Maintain test suite of known failure cases.

```
test_case: {
  prompt: "Summarize quantum mechanics",
  expected_quality: >= 4/5,
  last_updated: 2024-01-15
}

# After new model update, re-run all tests
# If score < 4/5, flag regression
```

**Typical regression**: New model version faster but less accurate; test catches it.

## Production Monitoring

Ongoing evaluation in production:

### Implicit Feedback

- User clicks on generated answer → satisfied
- User re-queries → dissatisfied or seeking clarification
- User edits output → quality too low
- Time spent reading output → engagement proxy

**Limitations**: Noisy; users may click irrelevant results, or not interact at all.

### Explicit Feedback

- Thumbs up/down
- 1-5 rating
- "Was this helpful?"

**Better signal but low response rate** (typically 0.1-1% of users rate).

### Quality Metrics

Automated checks:
- Output length (too short? too long?)
- Sentiment mismatch (sad prompt → happy output = error)
- Toxicity (classifier score)
- Factuality (cross-check with knowledge base)

## See Also

- [LLM Evaluation — Metrics, Benchmarks & Frameworks](genai-evaluation.md)
- [LLM Guardrails — Defense-in-Depth Safety and Output Control](genai-guardrails.md)
- [Machine Learning Operations — Model Lifecycle, Serving & Production Concerns](ml-operations.md)