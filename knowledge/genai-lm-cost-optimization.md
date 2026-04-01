# LLM Cost Optimization — Token Economics, Caching, and Strategic Selection

## Overview

LLM costs are driven by **tokens**: each model charges independently for input (or "prompt") tokens and output (or "completion") tokens. At scale, LLM costs can exceed compute costs for other workloads. Cost optimization requires understanding token economics, model selection trade-offs, caching strategies, and batching.

Cost structure:
- **Input tokens**: 10–100× cheaper per token than output tokens
- **Output tokens**: The expensive part; more generation = higher cost
- **Model selection**: Smaller/cheaper models output more tokens (slower inference); expensive models are fast
- **Context window**: Using full 100K token context is expensive; pruning is cheap

## Token Management: The Core Lever

### Prompt Compression

Raw prompts are often verbose. Effective compression reduces input token count without losing semantic meaning.

**Example: RAG context pruning**
```
Raw retrieved context (1500 tokens):
"The company was founded in 1995 in Silicon Valley. 
John Smith was the CEO. The early days were challenging... 
[16 paragraphs of history]
The company pivoted to AI in 2020. Their first AI product..."

Compressed (150 tokens):
"Company: Founded 1995, pivoted to AI in 2020. 
CEO: John Smith. Key achievement: First AI product 2020."
```

Savings: 90% token reduction, question still answerable.

**Effective compression techniques**:
- **Information extraction**: Summarize context to only facts relevant to upcoming query
- **Deduplication**: Remove repeated information across multiple retrieved documents
- **Lossy merging**: When multiple sources say the same thing, keep one
- **Structural simplification**: Convert narrative text to tables/outlines

Cost-benefit: Compression costs tokens (another model call, or post-processing), but breaks even if you save >2x input tokens in downstream requests.

### Context Window Management

Modern models have huge context windows (200K tokens for Claude, GPT-4). Using all of it is expensive.

**Problem**: Loading full email histories, document archives, and conversation history into context drives costs up 10–100x.

**Strategy: Progressive context loading**
```
START: user asks question Q
  ↓
[1. Retrieve minimum context: last 5 messages, 1 document]
  ↓
[2. Model answers Q if sufficient]
  ↓
[Otherwise, retrieve more: 20 messages + 3 documents]
  ↓
[3. Retry with expanded context]
```

Trade-off: Extra latency on cache misses vs. token savings on most queries.

### Output Token Prediction and Budgeting

Output tokens are expensive. Predicting how many you'll need allows budgeting.

**Budget-based generation**:
```
Query: "Summarize this paper in 100 words"
Output budget: 100 tokens
Model setting: max_tokens=100

(Even if model wants to generate 500 words, it's truncated at 100)
```

Cost: ~$0.003 per 100 output tokens (Claude 3 Opus rates).

**Sampling vs. generation**: Models like Claude support sampling methods (top-p, temperature) that control output variability but not length. For cost control, explicit `max_tokens` is necessary.

## Model Selection: Cost vs. Quality Trade-offs

### Model Tiers and Performance

Rough spectrum (as of 2026):

| Model Tier | Example | Speed | Quality | Cost/token |
|------------|---------|-------|---------|-----------|
| Ultra-small | Llama 2 7B (quantized) | Very fast | Good for simple tasks | $0.00001–0.0001 |
| Small | Llama 3 8B, Phi 3 | Fast | Good, some gaps | $0.0001–0.001 |
| Medium | GPT-4o mini, Claude Haiku | Very fast | Very good | $0.001–0.005 |
| Large | Claude 3 Sonnet, GPT-4 Turbo | Moderate | Excellent | $0.01–0.02 |
| Frontier | Claude 3.5 Opus, o1 | Slow (o1 is 10s latency) | SOTA | $0.05–0.2 |

**Decision framework**:
- **Latency-sensitive** (sub-second SLA): Use small/medium model
- **Quality-critical** (legal, medical): Use large/frontier model
- **Bulk processing** (millions of documents): Use small model + fallback to large for failures
- **Users with budget**: Let them tradeoff quality vs. cost explicitly

### Routing and Adaptive Selection

For mixed workloads, route different request types to different models.

```python
def select_model(request, budget=1.0):
  if request.type == "simple_faq":
    return "gpt-4o-mini"  # Fast, cheap
  elif request.type == "reasoning":
    return "o1"  # Expensive, coherent
  elif estimated_tokens(request) < 500:
    return "claude-3-haiku"  # Cheapest option
  else:
    return "claude-3.5-sonnet"  # Fallback
```

Benefit: Optimize for throughput and cost by matching problem complexity to model capability.

## Caching Strategies

### Semantic Caching

Most caching approaches (key-value) require exact input match. Semantic caching recognizes that similar inputs produce similar outputs.

**Approach**: For each user request, compute an embedding. Check if a similar embedding exists in cache; if yes, return cached result.

```
Request 1: "What's the capital of France?"
Embedding: [0.012, -0.045, 0.234, ...]
Cache miss → Call model → Result: "Paris"
Store: (embedding, "Paris")

Request 2: "What is the capital city of France?"
Embedding: [0.011, -0.043, 0.231, ...]  (very similar to Request 1)
Cosine similarity > 0.95 → Cache hit → Return "Paris"
```

Cost savings: 1 embedding call + cache lookup << 1 full LLM call.

**Implementation considerations**:
- Embedding model must be fast (<10ms per query)
- Threshold tuning: too low = wrong results, too high = cache misses
- Stale results: Cache doesn't age; old answers may be incorrect (e.g., "who won the World Cup?")

### Exact-Match Caching (Prompt Caching)

Some platforms (Claude, GPT-4) support native prompt caching: repeated prefixes in the context window are cached and reused at lower cost.

```
Request 1:
  System prompt: [1000 tokens]
  User request: "Summarize document A"
  Document A: [5000 tokens]
  → Total: 6000 tokens, cost = cost_6000_tokens

Request 2 (same system prompt + doc, different question):
  System prompt: [1000 tokens]  (cached)
  User request: "Extract facts from document A"
  Document A: [5000 tokens]  (cached)
  → Total: 2000 new tokens charged (cost ≈ 10% of Request 1)
```

Benefit: Massive savings for repeated context (RAG documents, system instructions).

Constraint: Only contiguous **prefix** matching (first N tokens must be identical).

### Cache Invalidation

Caches become stale. Strategies:
- **Time-based expiry**: Invalidate after N hours (for current events, use 1–24h)
- **Versioning**: Tag cache entries with content version; invalidate on update
- **Hybrid**: Keep cache but mark uncertain with confidence score (0–1); if <0.8, invalidate

## Batching and Request Aggregation

Batch processing reduces per-request overhead and enables bulk discounts.

### Synchronous Batching

Collect multiple independent requests, process together.

```
Collect requests for 10 minutes or 1000 requests (whichever first)
Batch 1000 requests into 50 calls (20 per call)
Process in parallel
Return results to individual requestors
```

Cost savings: 50 calls >> 1000 calls if you save 5x overhead per call.

Latency trade-off: Users wait up to 10 minutes for response.

Appropriate for: Batch reporting, data processing, non-interactive.

### Asynchronous Batching with Webhooks

User submits request → immediately returns batch ID → background job processes → webhook callback to user.

```
POST /analyze {"documents": [...]} 
← Immediate: {"batch_id": "b123"}

Background processes batch_id=b123
→ Webhook: POST /callback?batch_id=b123&result=[...]
```

Latency: 5–60s vs. user-synchronous wait.

Benefit: Clients don't block; requests can be batched arbitrarily.

## Fine-tuning vs. Few-shot vs. RAG Cost Comparison

Three strategies for improving model accuracy on specific domains:

### Fine-tuning (Upfront Cost)

Train model on domain examples. Cost: high upfront (100–1000s of examples + compute), low per-inference.

```
Cost: ($100–10,000 training) + ($0.0001/token inference × 1M tokens/day) = ~$1,000/day
Benefit: Model is domain-optimized, fewer tokens needed to prompt
Break-even: ~6 months at medium volume
```

**When it works**:
- Stable domain (legal contracts, medical records)
- High volume (1M+ tokens/day)
- Proprietary data (don't want it in prompts to third-party services)

### Few-shot Prompting (Medium Cost)

Include 2–5 examples in the prompt.

```
System: "You are a contract analyzer. Here are examples:

Example 1:
  Input: [contract text]
  Output: [analysis]
  
Example 2: ...

Now analyze this contract:"
```

Cost: Examples inflate input tokens (~500–2000 each).

```
Cost: ($0.00001/token × 1500 examples × 1M queries/day) = ~$15/day
```

**When it works**:
- Examples are short (not 50K token documents)
- Task is simple to demonstrate (classification, not reasoning)
- Don't have thousands of training examples

### RAG (Retrieval-Augmented Generation)

Fetch relevant documents, inject into context.

```
Cost: (retrieval overhead $1–5/1M queries) + (expanded context $0.00001/token × avg 5K context × 1M queries) = ~$50/day
```

**When it works**:
- Knowledge is dynamic (updates frequently)
- Queries vary widely (not fine-tuned to specific distribution)
- Documents are already stored (e.g., in a database)

### Rough Cost Ordering

Per 1M tokens/day, assuming $0.01/1K input tokens):

| Strategy | Monthly Cost | Accuracy | Latency | Privacy |
|----------|--------------|----------|---------|---------|
| Fine-tuning | $1–30K (amortized) | Excellent | Fast | Good (data stays local) |
| Few-shot (short) | $100–500 | Good | Fast | Medium (examples in prompt) |
| RAG | $500–5K | Good–Excellent | Moderate (retrieval latency) | Good (external data source) |
| Zero-shot | $300–1K | Fair | Fast | Excellent (no extra data) |

## Monitoring and Budgeting

### Token metrics to track

- **Avg input tokens/request**: Should decrease over time (better compression)
- **Avg output tokens/request**: Check for runaway generation
- **Cost per request**: Track trend; flag outliers
- **Model tier distribution**: % requests using cheap vs. expensive models
- **Cache hit rate**: If <50%, improve cache hit rate

### Alerting and Caps

```
if cost_today > budget * 1.2:
  alert("Cost overage detected")
  
if requests_queued > 10000:
  downgrade_to_cheaper_model()
  
if model_error_rate > 5%:
  escalate_to_better_model()
```

## See Also

- [LLM Inference Optimization](genai-inference-optimization.md) — Serving optimization and throughput
- [Information Retrieval](cs-information-retrieval.md) — Retrieval quality for RAG
- [FinOps](cloud-finops.md) — General cost governance principles