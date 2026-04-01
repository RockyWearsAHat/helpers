# LLM Fine-Tuning — From Full Updates to Efficient Adaptation

## Full Fine-Tuning vs Parameter-Efficient Methods

**Full fine-tuning** retrains all model weights on a target task, typically using supervised examples. For a 13B parameter model, this requires storing gradients (~4× memory of weights) plus optimizer states (Adam needs momentum and variance), totaling ~1TB GPU memory. It yields maximum accuracy but is computationally expensive and risks overfitting on small datasets.

**Parameter-efficient fine-tuning (PEFT)** adapts models by training only a small subset of parameters while freezing the base model. This approach reduces memory 10-50x and enables fine-tuning on consumer hardware.

| Method | Trainable Params | Memory | Accuracy | Use Case |
|--------|------------------|--------|----------|----------|
| Full fine-tuning | 100% of model | ~1TB (13B model) | Highest | Large datasets, maximum performance required |
| LoRA | ~0.1% | ~50GB | 95%+ of full | Most tasks; good speed/accuracy tradeoff |
| QLoRA | ~0.1% | ~20GB | 95%+ of full | Consumer GPU (24GB VRAM) |
| Adapter layers | ~0.5-5% | ~100-200GB | 92-98% | Modular, mix-and-match architectures |
| Prompt tuning | 0% of model + prompt vectors | ~20GB | 80-90% | Extreme resource constraints |

## Low-Rank Adaptation (LoRA)

LoRA (Hu et al., 2021) injects trainable low-rank decompositions into weight matrices. Instead of updating $W$ directly, the model learns $W + ΔW$ where $ΔW = AB^T$ with $A ∈ ℝ^{d×r}$ and $B ∈ ℝ^{k×r}$ (rank $r << d, k$).

- **Rank $r$ = 8-64**: typical sweet spot. Larger $r$ approaches full fine-tuning accuracy but uses more memory.
- **Scaling factor $α$**: controls adaptation strength; often set to $2r$ for stability.
- Applied selectively to query/value projections in attention; FFN layers can be included but with diminishing returns.

LoRA is modular: multiple task-specific LoRA weights can share a frozen base model, enabling rapid switching between domains. The per-task overhead is ~0.1% of original parameters—a 7B model needs only ~7M LoRA parameters per task.

## QLoRA: Quantized Low-Rank Adaptation

QLoRA (Dettmers et al., 2023) combines LoRA with 4-bit quantization of the base model, further reducing memory. The base model is quantized to 4-bit; only the LoRA parameters (which are small) remain in high precision. During forward pass, weights are dequantized on-the-fly in blocks—a <5% speed penalty but 4x memory savings over standard LoRA.

QLoRA enables fine-tuning on a single 24GB GPU (e.g., RTX 4090) what previously required 8× A100s. The tradeoff: training is slower (more dequantization overhead) but feasible on consumer hardware.

## Instruction Tuning and Supervised Fine-Tuning (SFT)

Base foundation models (pretrained on next-token prediction) don't follow instructions well. **Instruction tuning** adapts them to respond to diverse task prompts by training on curated instruction-response pairs:

```
Input: "Summarize in 3 bullet points: [article text]"
Output: "- Point 1\n- Point 2\n- Point 3"
```

Data quality matters more than quantity. A dataset of 1,000 high-quality, diverse examples often outperforms 100,000 repetitive examples. Strategies:

- **Diversity**: Mix domains, task types, reasoning styles
- **Clarity**: Remove ambiguous or incorrect examples
- **Coverage**: Ensure breadth across intended use cases (writing, coding, analysis, creative tasks)

After instruction tuning, models become more generalist and instruction-following, though they may lose some raw knowledge. This is the starting point for RLHF-based alignment.

## Reinforcement Learning from Human Feedback (RLHF)

RLHF (Christiano et al., 2017; scaled by OpenAI, DeepMind) aligns model outputs with human preferences. The pipeline has 3-4 stages:

1. **Supervised Fine-Tuning (SFT)**: Train on human-written examples (instruction tuning)
2. **Reward Model (RM)** training: Collect human preference labels comparing two model outputs. Train a separate model to predict human preference scores. This RM becomes a proxy for human judgment during training.
3. **Reinforcement Learning**: Optimize the SFT model using the RM as reward via PPO (Proximal Policy Optimization). The model learns to generate outputs the RM ranks highly.
4. **RLHF rounds** (optional): Collect new human feedback on RM-optimized outputs and retrain

RL optimization can cause models to exploit reward function weaknesses (reward hacking). A common issue: models become verbose (humans rate longer outputs as better) or refuse harmless requests (overly conservative). Modern implementations use KL divergence penalties to prevent divergence from the SFT model.

## Direct Preference Optimization (DPO)

DPO (Rafailov et al., 2023) simplifies RLHF by removing the separate reward model. Instead, the preference data directly optimizes the policy model via a contrastive loss that encourages preferred outputs and discourages dispreferred ones:

$$\log σ(\beta \log \frac{π_θ(y_w | x)}{π_{ref}(y_w | x)} − \beta \log \frac{π_θ(y_l | x)}{π_{ref}(y_l | x)})$$

with $y_w$ (preferred) and $y_l$ (dispreferred) outputs. DPO is simpler to implement, more stable, and requires less computational overhead than RLHF—no separate reward model means fewer moving parts and faster iteration.

## Catastrophic Forgetting

Fine-tuning on a narrow task risks degrading performance on original pretraining objectives. A model fine-tuned on coding tasks might lose general knowledge; one fine-tuned on legal documents might become overly verbose and formal on creative writing.

Mitigation strategies:
- **Mixed batches**: Interleave target-task and general-task examples during training
- **Knowledge distillation**: Use base model predictions as soft labels; regularize toward original model behavior
- **Elastic Weight Consolidation (EWC)**: Penalize parameter changes that contributed heavily to pretraining loss
- **Continual fine-tuning**: Careful learning rate scheduling (learning rate ≤ 1e-5) and regularization

The degree of forgetting depends on task similarity, dataset size, and fine-tuning duration. A small fine-tuning on in-distribution data (e.g., refining instructions) causes minimal harm; large-scale domain shift fine-tuning incurs larger degradation.

## When Fine-Tuning vs Alternatives

- **Fine-tuning chosen when**: Domain-specific knowledge transfer is critical, response format must be consistent, cost per inference doesn't matter
- **Prompting/few-shot chosen when**: Task is simple, model already performs adequately, latency matters
- **RAG chosen when**: Knowledge cutoff is an issue, need cited sources, retrieval corpus is available and updatable

Modern trend: RAG + modest fine-tuning on task-specific examples often beats pure fine-tuning, since retrieval adds grounding and fine-tuning provides format/stylistic adaptation.

See also: **LLM prompt engineering**, **Retrieval-augmented generation**, **Model evaluation**