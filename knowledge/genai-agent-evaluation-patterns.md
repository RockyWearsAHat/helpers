# Agent Evaluation Patterns — Self-Evaluation Failures, Calibration & Grading Criteria

## Overview

Agents are notoriously poor at evaluating their own work. When asked "Does this code work?", they confidently praise broken implementations. When asked "Is this design beautiful?", they declare generic, template-ridden output to be magnificent. This isn't a flaw in reasoning—it's a structural problem: **the same model that generated the work is asked to judge it, and models are incentivized to be agreeable about LLM-generated outputs**.

Anthropic's 2026 research identified **separation of concerns** as the key solution: remove evaluation from the generator and create a dedicated evaluator agent. But creating an effective evaluator requires careful prompt design, few-shot calibration, and explicit grading criteria that turn subjective judgments into measurable signals.

## The Core Problem: Agents Praise Their Own Work

### Why Self-Evaluation Fails

When Claude is asked to evaluate code it just wrote, it exhibits consistent bias:

1. **Confirmation bias**: The model rationalizes why the code probably works, even when tests fail
2. **Sunk cost fallacy**: "I spent 2 hours building this feature; it can't be broken"
3. **Agreeable nature**: LLMs are trained to be helpful; criticizing their own work feels rude
4. **Pattern matching on adjacent work**: If similar code worked yesterday, this similar code must work today (false reasoning)

Example: An agent generates a React component with a missing `useEffect` dependency. Asked to evaluate, it says: "The component correctly renders the user data and updates when props change. No issues detected." In fact, the component causes stale data bugs on navigation. The evaluator's prompt was too vague (just "Is this good?"), so the model ran quick pattern matching and returned positive.

### Why This Matters for Long Tasks

On codebases with 50, 100, or 200 features, bugs accumulate like sediment. Each missed bug becomes technical debt that slows subsequent agents. After 5 sessions, the codebase is full of stubbed features, half-implemented functionality, and latent race conditions—all blessed by the generator's self-evaluation.

This is invisible failure: the agent reports success; the user discovers broken features only when testing manually.

## The Evaluator-Generator Separation Pattern

### Architecture

Create two distinct agents with different prompts:

**Generator Agent** (produces work)
- Focus: Implementation quality, feature completeness, code correctness
- Tool access: Code editor, git, version control, test runner
- Model: Opus 4.5 (reasoning-heavy)
- Prompt tone: Ambitious, forward-looking ("make this app fully featured")

**Evaluator Agent** (judges work)
- Focus: Finding bugs, verifying acceptance criteria, identifying edge cases
- Tool access: Browser automation (Playwright MCP), running tests, inspecting database state
- Model: Opus 4.5 (same reasoning capability, but calibrated for skepticism)
- Prompt tone: Skeptical, thorough ("test edge cases; find problems; don't assume it works")

Then: **Generator → Evaluator → Feedback loop → Generator**

The evaluator provides specific, actionable feedback (not just "bad" or "good"):
```
Feature: User can update profile picture

FAILED — Criteria not met:

Bug 1 (Critical): Profile picture doesn't update after upload
- Reproduced: Upload new photo, page reload, photo still shows old image
- Root cause: Frontend uploads file but doesn't refetch updated user data
- How to fix: Call GET /api/me after upload completes to refresh profile state

Bug 2 (Medium): Image scaling doesn't preserve aspect ratio
- Reproduced: Upload non-square image, it appears stretched
- Expected: Image should maintain aspect ratio, centered in square frame
- Note: CSS works fine for existing images; issue is in upload preview

Bug 3 (Low): Error message is unclear on unsupported file types
- Current: "Invalid file"
- Better: "Please upload JPG, PNG, or GIF only"

Criteria met:
✓ User can select and upload image file
✓ UI gives feedback during upload (spinner shows)
✓ Upload completes without crashing
✗ Image persists after page reload (BUG 1)
✗ Image displays correctly without distortion (BUG 2)

Recommendation: Fix bugs 1 & 2 before moving forward. Bug 3 can wait.
```

The generator then fixes these specific issues.

## Calibrating the Evaluator: Few-Shot Prompting

Out of the box, even a skeptical evaluator prompt produces mediocre QA. The key is **few-shot calibration**: Show the evaluator examples of good and bad grading, so it learns your standard.

### Example: Frontend Design Evaluation

**User defines 4 grading criteria**:

1. **Design Quality**: Does the design feel cohesive? Colors, typography, layout, imagery combine into a distinct identity?
2. **Originality**: Evidence of custom decisions, not template defaults? No telltale "AI slop" patterns like purple gradients over white cards?
3. **Craft**: Technical execution—typography hierarchy, spacing, color harmony, contrast. Competence check.
4. **Functionality**: Usability independent of aesthetics. Can users understand the interface, find actions, complete tasks?

**Few-shot examples in evaluator prompt**:

```
I'm showing you examples of designs I graded. Study my feedback 
and use it to calibrate how you grade new designs.

===== EXAMPLE 1: POOR DESIGN =====
Design: Landing page for "Acme Corp" with:
- Generic centered hero section
- Stock photo of business people shaking hands
- Three feature cards with icons from Material Design
- Blue call-to-action button (very standard)
- No distinctive visual identity

My grading:
- Design Quality: 2/10 (Feels generic, no cohesion beyond "corporate blue")
- Originality: 1/10 (Entirely stock components and patterns; indistinguishable from 100 other startup sites)
- Craft: 7/10 (Technically correct grid, readable typography, color contrast fine)
- Functionality: 9/10 (Users can find the email signup)

Overall: FAIL. This is "AI slop." The craft is fine, but the lack of originaldemanding iterating.

===== EXAMPLE 2: EXCELLENT DESIGN =====
Design: Landing page for art museum with:
- Dark theme (charcoal background)
- Asymmetric layout with large hero image on left, text on right
- Custom serif typography (not system font)
- Subtle animation on scroll (gallery cards slide in)
- Distinctive color palette (mustard yellow accent on charcoal)
- Clear visual hierarchy: heading > museum description > call-to-action

My grading:
- Design Quality: 9/10 (Cohesive dark/mustard palette; each element reinforces the "cultured, artistic" vibe)
- Originality: 9/10 (Asymmetric layout is not default; custom serif choice shows intent; animations suggest care)
- Craft: 9/10 (Spacing is clean; contrast is high; readability is excellent; no alignment issues)
- Functionality: 8/10 (Clear CTA; users know where to click; navigation is intuitive)

Overall: PASS. This feels intentional, distinctive, and well-executed.

===== EXAMPLE 3: MEDIOCRE DESIGN =====
Design: To-do app with:
- Light neutral background
- Functional layout (works well)
- No color beyond grayscale
- Standard sans-serif
- Useful but uninspired

My grading:
- Design Quality: 5/10 (Not ugly, but not distinctive either; feels like default)
- Originality: 4/10 (Competent but unremarkable; could be any to-do app)
- Craft: 8/10 (Technically sound; no errors; readable; good spacing)
- Functionality: 9/10 (Task workflows are intuitive and fast)

Overall: BORDERLINE. Functionally excellent but aesthetically forgettable. 
For a to-do app, shipping this would be acceptable but unremarkable. 
For a design task, I'd iterate to add personality.
```

After these examples, the evaluator understands:
- "AI slop" (generic patterns, stock photos, defaults) scores low on originality
- Intentional design choices (custom palette, asymmetry, animations) score high
- Craft and functionality alone don't overcome originality failures
- The standard is not perfection, but evidence of deliberate decision-making

### Objective vs. Subjective Criteria

Some criteria are measurable; others aren't:

**Objective** (easy to grade):
- "User can submit a form": Test it; does it submit or error? Binary.
- "API endpoint returns 200 OK": Call it; check response code.
- "All test cases pass": Run suite; count passes.
- "No SQL injection vulnerabilities": Static analysis; automated checks.

**Subjective** (requires calibration):
- "UI design is polished": Needs examples to understand the standard
- "Code is readable": What counts as readable? (Need style guide)
- "Error messages are helpful": Some are, some aren't (Needs examples)
- "Performance is acceptable": Depends on target hardware, use case (Needs benchmarks)

**Strategy**: For subjective criteria, always include few-shot examples. For objective criteria, include test code or automated checks.

## Grading Criteria Design

### Pattern: Hard Thresholds + Soft Balance

Define hard failure and soft trade-offs:

**Hard failures** (one failure = feature is not done):
- Functional correctness: Feature does what spec says, or it fails
- No critical bugs: Crashes, data loss, security issues fail the build
- Tests pass: If test suite exists, all tests must pass

**Soft balance** (evaluated holistically):
- Code quality vs. speed: Refactored code is nice but not required to pass
- Comprehensive vs. minimal: Full-featured is better than MVP, but MVP is acceptable
- Edge cases vs. happy path: Happy path must work; edge cases are nice-to-have

**Example grading contract**:
```
Feature: "User can upload and share audio files"

Hard thresholds (any failure = REJECTED):
□ Upload endpoint accepts audio files (mp3, wav, flac, aac)
□ Uploaded file is saved to storage (S3 or local disk)
□ Sharing generates a unique, shareable link
□ Link expiration time matches spec (default: 7 days)
□ No crashes when uploading malformed audio

Soft criteria (evaluated together, no binary pass/fail):
□ Upload progress bar shows meaningful feedback
□ File validation happens client-side before upload
□ User is notified of upload speed/completion time
□ Error messages are clear and actionable

Overall rule:
- All hard thresholds met + at least 2/4 soft criteria → PASS
- Any hard threshold failed → REJECT (no credit for soft)
```

## Testing Tools: Browser Automation & API Verification

Evaluators need to test end-to-end, not just read code. Two key tool categories:

### 1. Browser Automation (Playwright MCP)

Playwright lets evaluators interact with the live app like a user:

```
I'll now test the chat feature end-to-end.

[Tool: Playwright] Get page https://localhost:3000
[Tool: Playwright] Click on "New Chat" button
[Tool: Playwright] Type in input: "Hello Claude"
[Tool: Playwright] Press Enter
[Tool: Playwright] Wait for response to appear (max 5 sec)
[Tool: Playwright] Take screenshot

Screenshot shows: Message appears in chat history, response is loading...
✓ No crash
✓ Message was sent
✓ UI state updated

Wait for response completion...
[Tool: Playwright] Wait for response text to appear
[Tool: Playwright] Take screenshot

Screenshot shows: Full response visible, message is in chat history, 
database query shows both messages saved
✓ Response arrived
✓ Both messages in database
✓ No truncation or corruption

Now test edge case: Send two messages rapidly...
[Tool: Playwright] Type "First"
[Tool: Playwright] Press Enter
[Tool: Playwright] Type "Second"  
[Tool: Playwright] Press Enter
[Tool: Playwright] Wait for both responses
[Tool: Playwright] Check database

Database shows: Both messages queued correctly; responses arrived in order
✓ No message loss
✓ Correct ordering
```

Evaluators with Playwright can:
- Verify UI behavior matches spec
- Check database state after actions
- Test edge cases (rapid clicks, long inputs, network delays)
- Catch UI bugs without manual testing

### 2. API Verification & Database Inspection

For backend features, evaluators can inspect state directly:

```
Feature: "User authentication tokens expire after 7 days"

Testing:
1. Create a user account
   [Tool: curl] POST /api/register 
   → Returns user_id=42, token with exp=now+7days

2. Query database for stored token
   [Tool: read] SELECT * FROM api_tokens WHERE user_id=42
   → Shows: exp=1711584000, created_at=1710979200
   → Difference = 604,800 seconds = 7 days ✓

3. Test token expiration
   [Tool: Python] import time; time.sleep(600) [wait 10 min]
   [Tool: curl] GET /api/me (using stored token)
   → Returns 200 OK (token still valid, only 10 min expired) ✓
   
4. Simulate 7-day wait (can't actually wait)
   [Tool: Database] UPDATE api_tokens SET exp=now-1
   [Tool: curl] GET /api/me (using expired token)
   → Returns 401 Unauthorized ✓
```

## Handling Evaluator Leniency

Even calibrated evaluators sometimes score generously. Solutions:

1. **Escalation**: If evaluator marks feature as passing but you (human) see bugs, update the few-shot examples and retrain
2. **Multiple independent evaluators**: Have two evaluators grade independently; require consensus on pass/fail
3. **Objective checks first**: Let automated tests run before evaluator; evaluator only checks subjective criteria
4. **Spot checks**: Manually verify a random sample of "passing" features each week; track evaluator accuracy

## Failure Modes & Fixes

| Failure | Cause | Fix |
|---------|-------|-----|
| Evaluator says pass; feature is broken | Evaluation too brief; no Playwright testing | Add browser automation; require testing all paths |
| Evaluator never gives pass | Too harsh; criteria unrealistic | Adjust few-shot examples; soften standards |
| Evaluator misses subtle bugs | Overlooking edge cases | Add edge case testing to few-shot; emphasize "break it" mode |
| Inconsistent grading (same feature, different score) | Criteria are vague | Add objective checks; use explicit thresholds |
| Evaluator gets tired after 50 features | Token cost of evaluation | Limit evaluations; run fewer iterations; use cheaper model for final sign-off |

## Cost-Quality Trade-Off

| Strategy | Cost | Quality | Latency |
|----------|------|---------|---------|
| **No evaluation** | 0% | Poor (buggy shipped) | Fast |
| **Generator self-eval** | Low | Terrible (blind spots) | Medium |
| **Single dedicated evaluator** | Medium (+20% to harness) | Good (catches major bugs) | Slow (more iterations) |
| **Evaluator + generator dual-review** | High (+30% to harness) | Excellent (comprehensive) | Very Slow |
| **Automated tests + evaluator** | Medium | Excellent (test + human check) | Medium |

**Recommendation**: Start with automated tests + dedicated evaluator. If evaluation cost is high, reduce to spot-check (evaluate 1 in 5 features, trust the progress on others).

## See Also

- [Agent Architecture](genai-agent-architecture.md) — Evaluator-optimizer pattern, design feedback loops
- [Agent Harness Design](genai-agent-harness-design.md) — Integrating evaluators into multi-session harnesses
- [Tool Design](genai-agent-tool-design.md) — Browser automation and API tools for evaluators
- [LLM Function Calling](genai-function-calling.md) — How evaluators invoke tools

---

**Sources**: Anthropic Engineering research (Prithvi Rajasekaran, 2026); "Harness Design for Long-Running Application Development," https://www.anthropic.com/engineering/harness-design-long-running-apps; Playwright documentation