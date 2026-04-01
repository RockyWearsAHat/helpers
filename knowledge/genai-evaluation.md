# LLM Evaluation — Metrics, Benchmarks & Frameworks

## The Evaluation Crisis

LLM benchmarks are plagued by **data contamination** (test sets present in training data), **saturation** (scores plateau near 100% across models, losing discriminative power), and **misalignment with production performance**. High benchmark scores do not predict real-world utility. Evaluation must combine multiple approaches: surface metrics, standardized benchmarks, task-specific harnesses, human judgment, and red-teaming.

## Surface-Level Reference Metrics

These measure n-gram or token-level overlap with reference text. Fast to compute but insensitive to semantic equivalence.

### BLEU (Bilingual Evaluation Understudy)
Original machine translation metric. Counts n-gram precision (1-4 grams) with brevity penalty.

```
Score = precision * sqrt(recall/reference_length)
```

**Limitations**: Rewards exact token matches; penalizes synonyms and paraphrase. A correct answer "Paris" vs. reference "the capital of France" scores zero. Best for translation where word order matters.

### ROUGE (Recall-Oriented Understudy for Gisting Evaluation)
Measures recall of n-grams and longest common substrings (LCS).

- **ROUGE-N**: Counts n-gram overlap
- **ROUGE-L**: Longest common subsequence (captures word order without exact position match)
- **ROUGE-W**: Weighted LCS (closer tokens weighted higher)

Better than BLEU for summarization where recall dominates precision. Still fails on semantic correctness.

### METEOR (Metric for Evaluation of Translation with Explicit Ordering)
Aligns words using stemming and synonymy, then computes order-aware match score.

Better at handling paraphrase than BLEU, but slower. Moderate correlation with human judgment for translation.

### Perplexity
Intrinsic metric: average -log(P(next_token | context)) across test set.

```
Perplexity = exp(mean(-log P(token_i | context_i)))
```

**Role**: Diagnostic, not calibrated to task performance. A 10-point perplexity improvement doesn't translate to task improvement. Useful for comparing models on same domain but weak for cross-domain prediction.

## Standardized Benchmarks

Benchmarks provide fixed test sets to enable reproducible comparisons. However:
- Scores inflate over time as models memorize test sets
- Saturation makes new models indistinguishable
- Cultural and linguistic bias in dataset creation

### MMLU (Massive Multitask Language Understanding)
Covers 57 domains (STEM, humanities, social science, professional). 14K multiple-choice questions requiring knowledge and reasoning.

**Interpretation**: Useful for general capability measurement but outdated for discriminating frontier models (most score >80%). Contamination suspected for models trained post-2023.

### HellaSwag
Commonsense reasoning task: pick the correct ending for a video description. 10K examples.

Less contaminated than MMLU historically, but subject to saturation (frontier models near 95%).

### HumanEval
Code generation: 164 programming problems requiring function implementation. Pass@k: fraction of k samples containing correct solution.

**Strengths**: Objective correctness (code runs or fails). **Weaknesses**: Small set; bias toward Python; weak correlation with real-world code quality (lacks non-functional properties: efficiency, maintainability, security).

### Additional Benchmarks
- **GSM8K**: Grade school math, 8.5K problems. Sensitive to reasoning capability
- **ARC**: Causal reasoning, 5.6K questions
- **TruthfulQA**: Factual accuracy against adversarially written questions
- **BigBench**: Meta-benchmark combining 200+ tasks

## Human Evaluation Frameworks

Scalable human judgment using structured protocols.

### Likert Scale Evaluation
Annotators rate outputs on 1-5 scale for criteria (helpfulness, correctness, safety). Allows statistical aggregation.

**Challenge**: Inter-rater disagreement (Cohen's kappa ~0.6-0.7); expensive at scale.

### Pairwise Ranking
Show two model outputs; ask which is better. More reliable than Likert (humans prefer relative comparisons).

Used by:
- **Chatbot Arena**: Crowd-ranked LLM outputs; produces ELO scores. Captures production-relevant preferences (user satisfaction)
- **AlpacaEval**: Lightweight version with fewer annotators, faster turnaround

### Taxonomic Evaluation
Define rubric (e.g., "Does output contain factually correct entities?", "Is reasoning sound?"). Annotators assign categories.

More objective than Likert, but rubric design requires domain expertise.

## LLM-as-Judge

Use a strong LLM (GPT-4, Claude) to evaluate outputs from weaker models. Dramatically reduces cost compared to human annotation.

**Protocol**:
```
You are an expert evaluator. Compare model output to reference on criteria X, Y, Z.
Output: [model response]
Reference: [ground truth or none]
Task context: [instruction]
Your judgment:
```

### Strengths
- Customizable to any task via prompt engineering
- Scales cheaply; enables frequent evaluation
- Aligns with human preferences better than surface metrics (studies: r=0.7-0.8 with human judgment)

### Weaknesses
- **Position bias**: Prefers first output when given two options (50%+ false positive rate if not randomized)
- **Length bias**: Favors longer, verbose outputs
- **Model collapse**: Multiple models evaluated by GPT-4 tend to converge in score

### Calibration
Mitigate position bias via:
- Swap order and repeat; average across runs
- Include reference context to anchor scoring
- Validate against human annotations on sample (~100 examples)

## Contamination: The Core Problem

Test data leaked into training corpora biases benchmarks upward. Detection methods:

### Exact Match Detection
Hash test examples; search training data. Limited (reformatting breaks detection).

### Perplexity-Based Detection
Test set perplexity lower than expected for held-out data suggests contamination (statistical anomaly).

### N-gram Overlap
High textual overlap between test and training suggests contamination risk.

**Estimate**: ~5-10% of frontier LLM benchmark performance explainable by contamination (Openai research, 2024).

## Red-Teaming and Safety Evaluation

Adversarial testing to identify failure modes (hallucination, manipulation, reasoning collapse).

### Techniques
- **Prompt injection**: Try to override system context with adversarial input
- **Jailbreaks**: Roleplay, hypotheticals to bypass safety guidelines
- **Distribution shift**: Out-of-domain inputs (slang, dialects, specialized jargon)
- **Adversarial examples**: Carefully crafted inputs exploiting known weaknesses

### Evaluation
- **HELM (Holistic Evaluation of Language Models)**: Comprehensive framework combining robustness, bias, and fairness evaluation
- **TruthfulQA**: Targets hallucination on open-ended questions
- **StereoSet**: Measures gender/race/religion bias in completions

## Task-Specific Evaluation Harnesses

General benchmarks miss domain specifics. Task-specific evaluation captures real constraints.

### lm-eval-harness (EleutherAI)
Extensible framework for writing evaluators:
```python
# Define task: examples, metrics, metrics computing function
class MyTask(Task):
    DATASET_PATH = "dataset/subset"
    METRIC = ["micro_f1"]
    def process_results(self, doc, results):
        # Custom metric: domain-specific scoring
```

Supports multiple metrics per task, few-shot examples, caching. Industry-wide standard.

### Domain-Specific Harnesses
- **Legal**: Contracts QA, statute interpretation
- **Medical**: Diagnosis reasoning (MedQA, MMLU-Prof Medical)
- **Finance**: Temporal reasoning over financial statements
- **Code**: Runtime correctness, security vulnerability detection

## Trade-Offs and Integration Strategy

| Metric | Cost | Reliability | Speed | Best For |
|--------|------|-------------|-------|----------|
| BLEU/ROUGE | Low | Low | High | Baseline; not recommended alone |
| Perplexity | Low | Medium | High | Diagnostic; domain comparison |
| Standardized benchmark | Low | Low (contaminated) | High | Initial filtering; not ranking |
| Human evaluation | High | High | Low | Golden standard; small sample |
| LLM-as-judge | Medium | Medium | High | Scale-up after validating on humans |
| Red-teaming | High | High | Medium | Safety-critical systems |

**Recommended pipeline**:
1. Filter model candidates via suites of LLM-as-judge on representative tasks
2. Validate top 2-3 on human evaluation (100-200 examples) to calibrate LLM judge
3. Domain-specific harness testing on production distribution (if available)
4. Red-team top finalist if safety-critical
5. Continuous monitoring post-deployment: track distribution shift, user satisfaction

## See Also

- **genai-training-infrastructure** — how evaluation shapes training methodology (RLHF, DPO)
- **ml-operations** — production monitoring and evaluation pipeline design
- **data-engineering-quality** — data quality constraints for evaluation datasets