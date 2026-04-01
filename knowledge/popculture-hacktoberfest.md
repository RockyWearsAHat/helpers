# Hacktoberfest — Incentives Gone Wrong, Then Corrected

## What Hacktoberfest Is

Hacktoberfest is an annual event sponsored by DigitalOcean designed to encourage open source contribution. The original incentive was simple:

- **October goal**: Make four pull requests to any open source repository on GitHub
- **Reward**: Get a free T-shirt (and stickers)

The premise was noble: "October is a great time to draw new people into open source." The event grew exponentially and became the world's largest open source contribution event by volume.

## The 2020 Spam Wave

In 2020, Hacktoberfest suffered a reputation collapse. Thousands of low-quality pull requests flooded open source repositories:

- **Example**: Adding a single space, then commenting "Hacktoberfest"
- **Mass submissions**: Poetry, haikus, and random comments added to documentation
- **Cryptocurrency spam**: Repositories received PRs adding links to crypto wallets
- **Typos**: PRs "fixing" valid spellings to wrong spellings to hit the four-PR quota
- **Automation**: Scripts that naively created PRs to thousands of repositories

Why it happened:

1. **Perverse incentives**: The T-shirt was a fixed reward for a counted metric (4 PRs). Quantity was measured; quality was not.
2. **No vetting**: DigitalOcean didn't require maintainers to opt-in. Any repository received Hacktoberfest traffic.
3. **No governance**: Maintainers reported their issues being marked spam-flooded before they could react.
4. **Scaling failure**: The event's scale grew faster than mechanisms to prevent abuse.

**The technical lesson**: This is a textbook example of **Goodhart's Law**: "When a measure becomes a target, it ceases to be a good measure."

PR count was meant to be a proxy for contribution. People optimized for the measure itself (submitting 4 PRs, any PRs), breaking its validity as a proxy for actual contribution.

## The Maintainer Revolt (2020)

Maintainers publicly complained:

- Linus Torvalds participation (Rust community): "This isn't helping"
- Many disabled the 'hacktoberfest' label that registered repositories for the event
- High-profile projects set up special filters or created a separate label to quarantine Hacktoberfest PRs for triage much later
- Communities documented the incident as a case study in event design failure

DigitalOcean received significant criticism for:

- Not requiring opt-in from maintainers
- Not setting quality standards
- Not protecting open source communities from spam by their own event

## The Corrective (2021-present)

DigitalOcean responded with concrete changes:

1. **Opt-in requirement**: Maintainers now explicitly register their projects as participating
2. **Quality gate**: New requirement—first PR must be merged before subsequent ones count toward the 4-PR goal (prevents spam)
3. **Maintainer control**: Participating projects can mark PRs as invalid, removing them from the count
4. **Reduced focus on T-shirt**: The reward structure became less prominent; focus shifted to "contributing back to open source" as the primary goal

The 2021 event ran much more cleanly. The correction didn't eliminate all spam, but it dramatically reduced low-quality contributions.

## The Broader Technical Insight: Incentive Design

Hacktoberfest is a textbook case of incentive design failure, later corrected. The lesson generalizes:

### The Problem: Proxy Collapse

When designing systems that reward behavior, measures collapse under optimization:

- **Measure**: "Number of PRs submitted" was meant to approximate "contribution quality"
- **Reality**: Optimizing for PR count only optimizes PR count, not quality
- **Result**: The measure stops correlating with the thing being measured

Related examples:

- **Code coverage** as a test quality metric: teams add tests covering code but not testing meaningful behavior
- **Customer satisfaction surveys**: if tied to bonuses, staff pressures customers to rate highly regardless of service quality
- **Commit count as productivity**: engineers make trivial commits to inflate their numbers
- **Stack Overflow reputation**: users upvote clever jokes over accurate answers

### The Fix: Multi-dimensional Metrics + Governance

Hacktoberfest's correction involved:

1. **Adding friction**: Opt-in means bad-faith participants need more effort
2. **Gating the reward**: First PR must be merged—this filters out low-quality submissions automatically because maintainers won't merge spam
3. **Explicit governance**: Maintainers have authority to mark PRs as non-participating, reversing the automatic counting
4. **Reputation feedback loop**: Spammers get labeled and their PRs get marked invalid; this creates social cost to spam

These are standard techniques in incentive design.

## Modern Hacktoberfest: Success Stories

Post-correction, Hacktoberfest has become genuinely useful:

- Universities use it to teach students about open source in a bounded event context
- Junior developers get structured introduction to pull requests
- Marginalized groups in tech participate in increased numbers (women, people of color report higher participation during Hacktoberfest)
- Legitimate open source projects get contributions and new maintainers

## Technical Cultures View

Different open source cultures have different relationships to Hacktoberfest:

- **Welcoming projects**: Linux documentation, accessibility projects, educational codebases explicitly market Hacktoberfest as a good entry point
- **High-barrier projects**: Linux kernel, cryptography libraries, performance-critical systems discourage Hacktoberfest participation
- **Middle ground**: Most projects participate but have triage procedures ready

This reflects a real divide: some projects can affordably handle beginner-oriented contributions (good test coverage, clear onboarding, modular codebase). Others have such high technical barriers that beginner contributions are costly to review.

## See Also

- open-source-practices.md — how to evaluate and triage external contributions
- process-code-review.md — review burden and contributor onboarding
- popculture-open-source-burnout.md — the broader context of maintainer scarcity making spam costly