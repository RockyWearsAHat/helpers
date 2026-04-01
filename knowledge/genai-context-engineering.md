# Context Engineering for AI Agents — Window Management, Compaction, Resets & Handoffs

## Overview

The context window is both the agent's working memory and its greatest constraint. A 200K token model with 200K tokens available doesn't actually have 200K usable tokens. In practice, models exhibit **context anxiety** (premature task completion as they near token limits) and **context degradation** (reduced reasoning quality beyond ~130K tokens). Worse, long sequences of accumulated conversation don't scale efficiently—every token in the context costs money, and all tokens weight equally regardless of importance.

**Context engineering** is the discipline of managing information flow to maximize agent coherence, minimize token waste, and prevent degradation over long task executions. It differs crucially from compression: Compaction summarizes earlier parts of the same conversation; context engineering resets the conversation and hands off explicit state to the next agent via structured artifacts.

## The Compaction Trap

### Why Compaction Alone Fails

Compaction—summarizing earlier conversation turns and replacing them inline—seems like an obvious solution. Instead of keeping:
```
[Turn 1: User asks for feature]
[Turn 2: Agent drafts architecture]
[Turn 3: User reviews]
[Turn 4: Agent refines]
[Turn 5: User approves]
```

Compaction reduces to:
```
[Summary: User asked for feature X with requirements Y. 
Agent drafted architecture Z. User approved with feedback about Z'].

[Turn 6: Agent continues building...]
```

**Apparent win**: Keep context growth bounded; agent continues without interruption.

**Real failure modes**:

1. **Summary loss**: Nuance disappears. "User mentioned performance is critical" might vanish if the compaction algorithm only extracts "User approved." Later, the agent makes a decision that violates that constraint.

2. **Context anxiety persists**: The model doesn't *know* it got a fresh context. It's still the same conversation, just truncated. Anthropic's research found that Claude Sonnet 4.5 exhibited context anxiety strongly enough that even with compaction, the model began wrapping up work prematurely as it approached context limits.

3. **No clean separation**: Compacted context is a lossy summary sitting in the same conversation. The agent can't make a hard "context switch"—resetting assumptions, clearing bad decisions, or starting fresh on a new approach.

### Anthropic's Finding: Context Resets Are Essential

Testing with Claude Sonnet 4.5 revealed that **context resets** (completely end the conversation, start fresh with a new agent) outperformed compaction on long-running tasks. The reset:
- Gives the model a psychological "clean slate"
- Eliminates lingering context anxiety
- Allows explicit handoff of state and next steps

**Trade-off**: Adding orchestration complexity and latency (start a new agent session) but ensuring coherent, high-quality work.

## Context Reset Architecture: Handoff Artifacts

When you reset context, agents must immediately understand the state-of-work. This requires **handoff artifacts**—structured files that carry the prior session's context forward.

### Pattern 1: Feature List Files (JSON)

Store work as explicit, machine-readable items:
```json
{
  "project": "Music Production App",
  "sprint": 3,
  "features": [
    {
      "id": "recording-1",
      "category": "core_daw",
      "description": "User can record audio from microphone to a track",
      "acceptance_criteria": [
        "Microphone permission is requested on first record click",
        "Recording displays waveform in real-time",
        "Stop button ends recording and saves to database",
        "Recorded clip appears on timeline at cursor position"
      ],
      "passes": false,
      "tested": false,
      "notes": "Audio context setup; may need browser API handling"
    },
    {
      "id": "mixing-1",
      "category": "mixer",
      "description": "User can adjust track volume with slider 0-100%",
      "acceptance_criteria": [
        "Slider responds to mouse drag",
        "Volume updates are reflected in playback immediately",
        "Volume persists across sessions"
      ],
      "passes": true,
      "tested": true,
      "notes": "Completed Sprint 1"
    }
  ]
}
```

**Why JSON, not Markdown?**
- Models are less likely to accidentally overwrite or reformat JSON
- Clear structure prevents "I'll just delete that requirement" edits
- Easy to parse and validate programmatically
- Strongly enforces only updating the `passes` field, ignoring other changes

**Agent instruction**: "Update the `passes` field to `true` only after you have tested the feature end-to-end and verified all acceptance criteria are met."

### Pattern 2: Progress Tracking Files (claude-progress.txt)

Narrative summary of what happened in the previous session, written for humans and the next agent:

```
=== Session 4 (Sprint 3, Iteration 2) ===
Time: 1 hour 45 minutes
Model: Claude Opus 4.5
Completed features: 5/20 in sprint

Work completed:
- Finished track volume mixing with persistent state
- Fixed waveform rendering for large audio files (>30 sec)
- Added pan control (left/right balance)

Bugs found and fixed:
- Volume slider sometimes didn't update playback (db level calculation error)
- Pan control caused crackling on large files (buffer alignment issue)

Known issues (not yet fixed):
- Master bus compression UI shows number only, not visual feedback
  (Priority: LOW — cosmetic issue, backend works fine)
- Playback stutters after 2+ hours in session (memory leak suspected)
  (Priority: HIGH — only affects long sessions, but annoying)

Architecture notes:
- Audio context now properly initialized on page load
- Using Web Audio API's GainNode for volume/pan
- Database schema added 'pan_value' column to TrackMix table

Next session should:
1. URGENT: Diagnose memory leak causing stuttering
2. Improve compression UI with visual feedback (fader only)
3. Begin Sprint 4: Add EQ filters

Questions for next agent:
- Should we use a dedicated library for EQ (e.g., tone.js) or build custom?
- Current testing only covers Chrome; need Safari/Firefox validation

Code health:
- 3 new TODO comments added (all documented)
- No breaking changes to existing APIs
- All tests passing (18/18 in test suite)
```

This file:
- Helps the next agent context-switch quickly
- Documents decisions and rationale
- Flags known issues by priority
- Suggests next steps without dictating them

### Pattern 3: Git Commit History

Each session's agent commits work with descriptive messages:

```bash
commit 8f3a7c: Implement track volume mixing with Web Audio GainNode
  - Add volume slider to mixer UI (0-100%)
  - Persist volume state to database via TrackMix table
  - Fix: Volume updates now reflect in playback immediately
  - Add: Pan control (L/R balance) for each track

commit 5d2e1b: Fix waveform rendering for large audio files
  - Previous: Crashed on files >30 seconds
  - Root cause: Buffer size exceeded canvas dimensions
  - Solution: Implement chunked rendering with viewport scaling
  - Test: Added waveform-rendering.test.ts (8 new cases)

commit 3c9f4a: Add Web Audio API initialization at page load
  - Moved audio context creation from first-play to page load
  - Fixes: Race condition where context wasn't ready before first click
  - Note: Requires user gesture for browser security; tested with click handlers
```

Next agent can:
- Understand recent work: `git log --oneline -10`
- See what changed: `git show HEAD` or `git diff HEAD~3..HEAD`
- Revert bad changes: `git revert <commit>`

## Context Window Management Strategies

### Token Budgeting

Budget tokens across the session:

```
Initial context:
- System prompt: 2,000 tokens
- Feature requirements (JSON): 15,000 tokens
- Current codebase (read from files): 50,000 tokens
- Progress tracking (text): 1,000 tokens
- Git log (last 20 commits): 3,000 tokens
Total: 71,000 tokens

Available for work: 200,000 - 71,000 = 129,000 tokens

Estimate per task:
- Implement one feature: ~5,000-8,000 tokens
- Write tests: ~2,000 tokens
- Commit and document: ~1,000 tokens

Expected tasks in session: 8-10 features

Decision: Implement 6-8 features, then trigger context reset.
```

This prevents the agent from running out of context mid-feature, which is catastrophic (half-implemented feature left undocumented).

### Information Density Over Completeness

Don't include everything. Include only what the agent needs right now:

**Bad (bloat)**:
```
Here's the entire application codebase:
[20,000 lines of code]
```

**Good (targeted)**:
```
The architecture uses a React frontend (components in /ui) 
and FastAPI backend (routes in /api).

For this sprint (implementing the mixer), you'll need:
- /ui/Mixer.tsx (existing component; add volume slider)
- /api/routes/mixer.py (where you'll add volume endpoint)
- /db/schema.sql (you may need to add a column)

Don't modify: /ui/Transport.tsx, /api/routes/auth.py (unrelated)
```

The second approach focuses the agent on the task and keeps context lean.

### Vocabulary Stability

Establish naming conventions and stick to them. Example:

```
Feature states: "passes" (boolean), not "completed", "done", or "working"
File references: Absolute paths from project root, not relative
Database fields: snake_case (track_volume), not camelCase (trackVolume)
Commit messages: "Implement X" or "Fix: Y", not "Added X" or "Fixed Y"
```

Consistency means the agent doesn't burn tokens on style decisions or ambiguous naming.

## When Context Resets Are Necessary

1. **Long-running tasks** (>2 hours): Token accumulation compounds; context anxiety emerges
2. **Model degradation observed**: The agent's reasoning becomes incoherent or it starts forgetting constraints
3. **Incremental work** (feature-by-feature): Each feature is a natural reset point
4. **Handoff to different agent/model**: Switching from Opus (planning) to Haiku (cheap execution) requires a context boundary
5. **After failure/recovery**: When an agent goes off-rails, a reset cleanly separates the failed attempt from the next iteration

## The New Model Advantage

Anthropic reported that Claude Opus 4.6 **largely eliminated context anxiety** compared to Sonnet 4.5. This means:
- Compaction is more viable with Opus 4.6 (in a single session)
- Context resets are still useful for cost management and clarity, but not as critical for coherence
- Longer sessions become possible without artificial resets

However, the handoff artifact patterns (feature lists, progress tracking, git history) remain valuable regardless of model capability—they reduce cognitive load and enable efficient collaboration.

## Cost Implications

| Strategy | Token Cost | Latency | Quality | Notes |
|----------|-----------|---------|---------|-------|
| **Compaction (same session)** | High (all tokens counted) | Low | Good (no reset) | Simpler; loses nuance; context anxiety persists |
| **Context reset (new agent)** | Medium (reset overhead) | Higher | Excellent (clean slate) | Best for long tasks; requires good handoff artifacts |
| **No management** | Very high | Low | Poor (degrades) | Agent runs out of context; output incoherent |

## Key Principles

1. **Resets > compaction for long tasks.** Compaction is easier operationally but doesn't fully solve the problem on current models (Sonnet 4.5).
2. **Handoff artifacts are load-bearing.** Without them, context resets become impossible—the next agent has no idea what happened.
3. **Token budgeting prevents catastrophe.** Estimate task size and reset before the agent runs out of space mid-work.
4. **Incremental work naturally aligns with resets.** Implementing one feature per session, then resetting, provides natural stopping points.
5. **Model improvement changes the equation.** Opus 4.6's elimination of context anxiety moves the boundary; evaluate with your actual model.

## See Also

- [Effective Harnesses for Long-Running Agents](genai-agent-harness-design.md) — Initializer agents, progress file patterns, init.sh
- [Agent Architecture](genai-agent-architecture.md) — Planner-generator-evaluator, multi-agent patterns
- [LLM Cost Optimization](genai-lm-cost-optimization.md) — Token economics, model selection

---

**Sources**: Anthropic Engineering research (2026); "Harness Design for Long-Running Application Development," https://www.anthropic.com/engineering/harness-design-long-running-apps; Claude context window documentation