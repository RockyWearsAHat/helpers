# Code Review Culture — Rituals, Nitpicking, and Knowledge Transfer

## The LGTM Stamp as Cultural Object

"LGTM" (Looks Good To Me) is a four-letter stamp applied to pull requests when a reviewer approves code. In GitHub and GitLab, it's often just a thumbs-up emoji or a checkbox. But in email-based code review systems (like Linux kernel development on mailing lists), LGTM carries weight—it's a signed assertion of approval.

The cultural phenomenon:

- **LGTM as abbreviated approval**: A reviewer adds LGTM and the code moves forward
- **LGTM inflation**: In some cultures, LGTM is given cautiously; in others, it's given freely
- **The "ship it" derivative**: "Ship it" is LGTM's cousin—an explicit signal to merge and deploy
- **Cargo cult LGTM**: Teams that have code review processes but add LGTM without meaningful review

What's embedded: LGTM is a **responsibility transfer**. The reviewer is saying "I read this, I didn't see critical problems, I'll accept the risk of this merging." This is a meaningful assertion.

## Nitpicking: The Review Anti-Pattern

"Nitpicking" in code review is commenting on minor issues (style, naming, formatting) while ignoring or avoiding substantive feedback on logic, architecture, or correctness.

### When nitpicking is valuable

Code style matters for systems that live for years:

- **Inconsistent style is a tax on future readers**: maintainers spend effort learning variable naming schemes, indentation patterns, comment conventions
- **Standards reduce context switching**: team members switching between files written in different styles pay a cognitive overhead
- **Lint and formatters automate most of it**: tools like prettier, black, gofmt, and cargo fmt now handle this automatically

### When nitpicking is pathological

- **"This variable name is bad but I can't explain why"**: opinionated feedback without reasoning
- **Blocking merge on style**: holding up a bug fix because of formatting
- **Nitpicking as gatekeeping**: using minor comments to assert dominance ("I'm the reviewer, I set standards")
- **Nitpicking as avoidance**: pointing out minor issues to avoid responsibility for reviewing substantive changes

The toxicity is that nitpicking feels productive to the reviewer (they've made comments, maintained control) while frustrating the author (feedback is low-value and doesn't unblock them).

## Ship it vs. One More Thing

"Ship it" is a decision gate: "This code is good enough; merge and deploy."

The tension:

- **Ship it culture**: "Iterate in production. We'll fix bugs if they happen. Velocity beats perfection."
- **One more thing culture**: "Wait, what about this edge case? What about this performance optimization? Fix this first."

Both are valid philosophies for different contexts:

**Ship it makes sense when:**

- Iteration cycles are fast (web applications, feature toggles allow instant rollback)
- The cost of fixing bugs in production is lower than the cost of review delays
- You have good monitoring and rapid response procedures

**One more thing makes sense when:**

- Mistakes are costly (kernel code, financial systems, medical devices)
- Iteration cycles are slow (hardware, release cycles measured in quarters)
- The cost of finding bugs post-deploy is very high

The pathology emerges when teams don't align on which context they're in. A perfectionistic reviewer blocking a web developer's deployment, or a "ship it" developer pushing unmaintainable code to a database team, both create friction.

## PR Descriptions as Art

A well-written pull request description is itself a communication artifact. The best descriptions contain:

1. **What problem does this solve?** (context)
2. **Why this solution and not others?** (design rationale)
3. **How to test?** (verification instructions)
4. **Any gotchas or concerns?** (transparency)

A description that mastered these becomes documentation. Future maintainers reading git history see not just code but the reasoning behind it.

**Examples of PR art:**

- **Linux kernel**: Longstanding tradition of detailed commit messages explaining why changes were made, referencing related commits, and documenting alternative approaches that were considered
- **Google Paxos**: Simple algorithm with terrible descriptions. Decades later, Lamport rewrote it as "Paxos Made Simple," teaching the community better pedagogy
- **Open source anthropology**: Major projects often have exemplary PRs in their history that became reference material

**Anti-pattern: "Drive-by PRs"**

- Title: "Fixed thing"
- Description: empty or "See code"
- Reviewer's burden: reverse-engineer intent from code

## Code Review as Knowledge Transfer

The deepest function of code review is not gatekeeping but **knowledge transfer**.

### Asymmetric review

- **Senior→Junior**: "Here's the pattern we use. Here's why. Here's the gotcha you might hit."
- **Junior→Senior**: "We tried this approach. Here's the constraint we uncovered." (Seniors learn too.)

### Distributed review

When a team reviews code together:

- Local knowledge gets distributed (the person who knows this subsystem reviews it)
- New contributors see how decisions are made
- Institutional knowledge becomes written (comments explain not just what but why)
- Consensus builds around standards

### The review as documentation

Comments like:

```
// Don't use Map.entries(); it's slower than .keys() loop in Chrome <95
// due to iterator allocation. See: https://...perf-study
```

Are more valuable than code review as "bug catching." They're **teaching moments** that prevent future mistakes.

## Approval Cultures Vary Widely

Different communities have radically different review standards:

### Strict approval

- **Linux kernel**: Changes go through multiple reviews. Maintainers sign off, adding their name. If the patch causes problems, your reputation is tied to it.
- **Financial systems**: Segregation of duty. Change requires approval from person who wrote code + person who didn't + ops who deploy.
- **Medical device firmware**: FDA traceability. Every change documented with rationale.

### Loose approval

- **Startup web development**: "If it doesn't break tests, ship it. Revert fast if problems emerge."
- **Academic code**: "Running in production? Not really. Ship it when it's done."

### Approval theater

- **LGTM without reading**: Reviewer's name on a box that checks "we did review"
- **Senior person rubber-stamping**: "I trust you, merge it" (deferring review, not doing it)
- **Too many approvals**: 5 people approve a change to `README.md` because process requires it

## The Embedded Technical Insight

Code review optimizes for competing values that can't be simultaneously maximized:

1. **Speed to market** (fewer reviews = faster merge)
2. **Code quality** (more reviews = higher quality)
3. **Knowledge distribution** (more reviewers = broader learning)
4. **Reviewer bandwidth** (review takes time)
5. **Accountability** (named approval creates responsibility)

Systems that ignores one of these usually fails:

- **Too much review**: slow, bottlenecked on reviewers, junior devs frustrated, "merge and revert" becomes faster than review iteration
- **Too little review**: bugs in production, knowledge silos, senior people don't know what's deployed
- **No naming**: no accountability, rubber-stamping
- **Named single reviewer**: fast for them, no knowledge distribution, single point of failure

The best systems balance:**

- Proportional review depth to change risk (one line to README ≠ algorithm change)
- Clear approval criteria (not subjective)
- Explicit tradeoffs (we tolerate this risk level to maintain velocity)
- Knowledge transfer as a goal, not just bug catching

## See Also

- process-code-review.md — structured approaches to making review effective
- code-review-practices.md — specific practices and anti-patterns
- process-code-ownership.md — how review relates to expertise silos