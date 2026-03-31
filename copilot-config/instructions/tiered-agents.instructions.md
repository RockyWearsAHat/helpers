---
description: "Cost-proportional model routing for orchestrators. Use when dispatching subagents and you want to match model capability to task complexity."
applyTo: "**"
---

# Cost-Aware Model Routing

**Read this entire section before your first subagent call.**

Model routing is not about quality preferences. It is about cost. The difference between tiers is not incremental — it is multiplicative.

| Tier         | Cost multiplier        | Token pricing (approx)      | Role                                                                |
| ------------ | ---------------------- | --------------------------- | ------------------------------------------------------------------- |
| **quick**    | 1x (baseline)          | ~$0.80/M input, $4/M output | Gather, read, format, inventory                                     |
| **capable**  | ~3x quick              | ~$3/M input, $15/M output   | Analyze, research, evaluate, implement                              |
| **thorough** | ~9x quick, ~3x capable | ~$15/M input, $75/M output  | Resolve ambiguity, make final calls, handle the hardest single step |

A **single thorough call costs as much as nine quick calls**. Three capable calls. This is not a rounding error — it is the difference between a pipeline that costs $0.50 and one that costs $4.50.

## The Routing Strategy

**Quick and capable are the workhorses. Thorough is the closer.**

The entire point of routing is to ensure that when you make a thorough call — and you may only make one — that call has everything it needs to finish the job completely. No gaps, no "I need more context," no wasted tokens re-reading files that a quick call could have summarized.

### How to think about it:

1. **Quick calls do the legwork.** File reading, directory listing, structured extraction, formatting, submitting data. These tasks don't need reasoning — they need speed and coverage. Use quick calls generously. Run several in parallel if needed. They're cheap.

2. **Capable calls do the thinking.** Web research, code analysis, evaluation against criteria, writing instructions, making file edits. This is the tier for work that requires judgment but not genius. Most of your pipeline runs here.

3. **Thorough calls resolve the hardest problem.** One call, fully loaded with context from the quick and capable steps, aimed at the single highest-uncertainty task. Architecture decisions. Ambiguous research where multiple sources conflict. Complex multi-file refactors where a mistake means significant rework. If the thorough call doesn't need to ask for more information, you routed correctly.

### The thorough call contract:

Before making a thorough call, verify:

- [ ] All relevant files have been read (by quick calls)
- [ ] All research has been gathered (by capable calls)
- [ ] All evaluation criteria have been established (by capable calls)
- [ ] The prompt to the thorough call includes EVERYTHING it needs — no lazy "see above"
- [ ] The task genuinely requires this tier — could a capable call handle it?

If you cannot check every box, you are not ready for a thorough call. Do more prep work at cheaper tiers first.

## Routing Rules

**Use quick (Haiku, GPT-4o-mini) for:**

- Listing files, reading directories, inventory work
- Extracting structured data from files with clear schemas
- Formatting, sanitizing, or serializing output
- Submitting pre-prepared data to an external store
- Summarizing files that a capable call will later analyze

**Use capable (Sonnet, GPT-4o) for:**

- Web research and evidence synthesis
- Code analysis and pattern detection
- Comparing findings against criteria
- Writing instructions, documentation, or explanations
- Making file edits where correctness matters
- Most reasoning tasks that don't involve deep ambiguity

**Use thorough (Opus, GPT-5) only when:**

- The problem domain is poorly documented and requires reasoning under genuine uncertainty
- Multiple conflicting sources need to be weighed and a definitive conclusion reached
- A single mistake in the output would require significant rework to fix
- The task cannot be decomposed further into capable-tier subtasks
- You have already gathered all necessary context via cheaper calls

**Never use thorough for:**

- File reading or inventory (quick handles this)
- Straightforward research or implementation (capable handles this)
- Tasks where the answer is already clear but you want extra confidence (that's waste)
- Multiple sequential steps — if you need thorough more than once, rethink the decomposition

## Calling Syntax

Pass `model` to `runSubagent` to override an agent's default:

```
runSubagent(
  agentName: "Explore",
  model: "claude-haiku-4.5",
  prompt: "List all files in src/ with their export signatures",
  description: "Scan src exports"
)
```

Use `list_language_models` (MCP tool) to see all valid model ids.

## Default Models by Pipeline Phase

**Default model for every `runSubagent` call: `claude-haiku-4-5`.** Always pass `model` explicitly.

Do not pre-assign models to agent names or pipeline phases. Assess the specific prompt you are about to send.

| Agent                      | Default | Promote to sonnet when...                                           |
| -------------------------- | ------- | ------------------------------------------------------------------- |
| DevOpsAuditContext         | haiku   | Never — reads files, no synthesis needed                            |
| DevOpsAuditResearch        | haiku   | Prompt asks to reconcile conflicting sources or synthesize guidance |
| DevOpsAuditEvaluate        | haiku   | Criteria are ambiguous or tradeoffs conflict                        |
| DevOpsAuditImplement       | haiku   | Edits span 3+ files with cross-file consistency requirements        |
| DevOpsAuditCommunitySubmit | haiku   | Never — mechanical formatting                                       |

No agent defaults to thorough. Thorough is an orchestrator decision — used when sonnet has already been tried and the output was insufficient.

## Natural Language Signals

When the user describes a task, look for these signals:

- "quickly", "scan", "list", "check", "summarize", "format" → **quick** (haiku)
- "analyze", "research", "evaluate", "compare", "implement", "fix" → **evaluate the prompt** — most of these are cheap unless they involve synthesis or ambiguity
- "deep dive", "thorough", "architecture", "uncertain", "complex", "decide" → **capable** (sonnet), or **thorough** if sonnet proved insufficient (but verify prep work is done first)

If no signal is present, default to **quick**. Promote after reviewing the specific prompt content, not the phase label.

## Example: Well-Routed 4-Step Pipeline

```
step 1 → Context     (haiku — file reading)           — $0.05
step 2 → Research    (haiku — fetch + extract)         — $0.08
step 3 → Evaluate    (sonnet — criteria conflicted)    — $0.25
step 4 → Implement   (haiku — single-file edit)        — $0.08
                                              total ≈  $0.46
```

Compare to pre-assigning sonnet to every "thinking" phase:

```
step 1 → Context     (haiku)                           — $0.05
step 2 → Research    (sonnet — pre-assigned)            — $0.30
step 3 → Evaluate    (sonnet — pre-assigned)            — $0.25
step 4 → Implement   (sonnet — pre-assigned)            — $0.30
                                              total ≈  $0.90
```

Compare to running every step at thorough:

```
step 1 → Context     (thorough: claude-opus-4.6)       — $0.45
step 2 → Research    (thorough: claude-opus-4.6)       — $2.70
step 3 → Evaluate    (thorough: claude-opus-4.6)       — $2.25
step 4 → Implement   (thorough: claude-opus-4.6)       — $2.70
                                              total ≈  $8.10
```

Same pipeline. The well-routed version is half the cost of the phase-mapped version and 18x cheaper than all-thorough — because most prompts are cheap tasks wearing expensive labels.
