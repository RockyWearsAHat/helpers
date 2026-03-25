# DevOps Audit Community Cache Contract

This document defines how the DevOps audit system should consume and optionally contribute to a shared, versioned community research cache.

Every audit must still do its own live research. The community cache is a starting point and comparison layer, not a replacement for the current audit pass.

The shared cache must contain only generalized GitHub Copilot best practices and general application advice. It must never contain repository-specific context, workspace-specific observations, private file paths, local project descriptions, user-specific content, or anything that could identify or reconstruct a contributor's environment.

## Goals

- Let every audit client start from a broader body of previously verified research.
- Keep rollback and drift detection easy by versioning cache snapshots in GitHub.
- Make automatic pulling the default behavior.
- Keep contribution opt-in because publishing research adds friction and trust implications.
- Prevent community material from overruling current official docs, release notes, or strong product guidance.
- Preserve anonymity and prevent public leakage of repository-specific or personal context.

## Authority Model

Use these trust tiers in order:

1. Current official docs and release notes
2. Current product-team guidance and transcripts
3. Verified file-level repository examples
4. Community cache entries that are still fresh and evidence-backed
5. Local synthesized principles that still pass freshness checks

The community cache is an accelerator, not the source of truth. It can suggest starting points, highlight known anti-patterns, and preserve useful examples, but it must never replace fresh verification for normative claims.

## GitHub Versioning Model

Host the shared cache in a dedicated GitHub repository so every change has normal Git history, reviews, diffs, rollback, tags, and issue tracking.

Recommended layout:

```text
community-cache/
  manifest.json
  CHANGELOG.md
  snapshots/
    2026-03-12/
      manifest.json
      official-sources.json
      public-example-sources.json
      prompting-principles.json
      application-practices.json
      anti-patterns.json
  candidates/
    README.md
```

Recommended rules:

- `community-cache/manifest.json` points to the current recommended snapshot and declares schema version.
- `community-cache/snapshots/YYYY-MM-DD/` is append-only once published except for clearly documented repair commits.
- Tag important snapshot milestones in GitHub so clients can pin or roll back.
- Keep `CHANGELOG.md` human-readable so drift or regressions are easy to inspect.

## Minimum Manifest Fields

The top-level manifest should include at least:

```json
{
  "schemaVersion": 1,
  "recommendedSnapshot": "2026-03-12",
  "publishedAt": "2026-03-12T00:00:00Z",
  "minClientBehaviorVersion": 1,
  "notes": "Community cache is illustrative and must be revalidated against current authoritative sources."
}
```

Each snapshot manifest should include at least:

- `snapshotId`
- `publishedAt`
- `sourceCoverage`
- `freshnessWindowDays`
- `authoritativeSourcesChecked`
- `knownLimitations`
- `breakingChanges`
- `supersedes`

## Client Behavior

Default client behavior should be:

1. Pull the local repo cache first.
2. Attempt to pull the shared community manifest and the recommended snapshot.
3. Run the audit's own live research anyway.
4. If the community cache is unavailable, continue with local and live research instead of failing the audit.
5. Treat the community cache as pre-research context, not as completed proof.
6. Revalidate any normative claim from the community cache against fresher authoritative sources before promoting it into target-state guidance.
7. Before any final conclusion packet is submitted, sanitize it so that only generalized best practices and general application advice remain.

Default publishing behavior should be:

- Do not publish by default.
- If the user has enabled community participation once, automatically submit the final audit conclusion packet after a successful run.
- Submission should happen after the audit is complete, not during mid-research.
- Users who have not enabled community participation remain pull-only.

## Participation Modes

Clients may support three modes:

- `disabled` — do not use the community cache at all
- `pull-only` — default; consume remote cache, never submit conclusions
- `pull-and-auto-submit` — consume remote cache and automatically submit final conclusion packets after successful audits

The opt-in is at the participation setting level, not at the individual audit-run level. Once a user enables `pull-and-auto-submit`, the client should not ask again for each audit.

For safety and reviewability, prefer GitHub pull requests or bot-authored branch updates over direct pushes to published snapshots.

## Final Conclusion Packets

The shared cache should be built from final audit conclusions, not raw notes, partial findings, or mid-research fragments.

The shared cache is for generalized Copilot guidance only. It is not a place to store what a specific repository is, how a private workspace is structured, what a given company does, or what files a contributor had open.

Each successful audit should produce a normalized final conclusion packet containing at least:

- the final principle, anti-pattern, warning, or example
- the final conclusion statement in normalized form
- the recommendation strength
- the general applicability class
- the supporting evidence references
- freshness dates
- whether the conclusion was revalidated against live authoritative sources
- what parts came from prior cache versus fresh research
- what should not be generalized
- the audit version or client behavior version

Each final conclusion packet must exclude:

- repository names unless they are already public source references used as evidence
- private repository descriptions or summaries
- local file paths
- workspace structure details
- stack details learned only from the contributor's private repo
- user prompts or notes that reveal private context
- any example phrased as "this repository should" or "in my workspace"

Allowed content is limited to generalized Copilot best practices such as prompt-design guidance, agent-boundary guidance, instruction-file guidance, skill-structure guidance, routing guidance, tool-use guidance, and general application advice that is safe across repositories.

Once final conclusion packets exist in the shared cache repository, promotion from candidate material into the master cache can be automatic. Client submission and server-side promotion are different concerns.

## Automatic Promotion Pipeline

The clean model is this:

1. Every audit performs its own research and reaches a final conclusion.
2. Clients in `pull-and-auto-submit` mode automatically submit normalized final conclusion packets.
3. GitHub-side automation aggregates matching packets into a single evidence cluster.
4. The cluster earns or loses score over time based on freshness, repetition, source quality, contradiction, and durability.
5. When the cluster clears the promotion threshold, automation moves it into the master cache snapshot.
6. If the cluster later decays or is contradicted by stronger evidence, automation demotes or retires it in a later snapshot.

This keeps promotion automatic without making it arbitrary.

## Candidate Normalization

For automation to merge community conclusions elegantly, each packet should be normalized into fields that GitHub automation can compare directly:

- `candidateId`
- `kind`: principle | anti-pattern | example | warning
- `topic`: prompts | instructions | agents | skills | routing | tooling | other
- `statement`: the concise normalized claim
- `recommendationStrength`
- `applicability`: general | prompt-design | instruction-design | agent-design | skill-design | routing | tool-use | workflow-general
- `evidenceRefs`
- `firstSeenAt`
- `lastSeenAt`
- `clientCount`
- `distinctSubmissionCount`
- `applicabilityConsistency`
- `authoritativeSupport`: none | weak | medium | strong
- `contradictionCount`
- `durabilityDays`
- `status`: candidate | incubating | promoted | deprecated | retired

The important part is the normalized `statement`. Similar user wording must collapse into one comparable claim rather than producing many near-duplicates.

## Ranking Model

Promotion should not be based on popularity alone. Rank each normalized candidate with a weighted evidence score.

Recommended score components:

- `authoritative support score` — strongest factor; boosts claims revalidated against docs, release notes, or product-team guidance
- `cross-submission recurrence score` — how many distinct anonymized submissions independently surfaced the same generalized claim
- `durability score` — how long the claim keeps reappearing without strong contradiction
- `freshness score` — rewards recent revalidation and penalizes stale evidence
- `source diversity score` — rewards support from more than one source type
- `general-applicability confidence score` — rewards claims that remain useful as broad Copilot best practices rather than narrow private-context advice
- `contradiction penalty` — subtracts for conflicts, failed revalidations, or newer evidence that weakens the claim
- `novelty penalty` — prevents a sudden burst of repeated low-quality submissions from instantly promoting a claim

Example conceptual formula:

$$
promotion\_score = 0.35A + 0.20R + 0.15D + 0.10F + 0.10S + 0.10P - 0.20C - 0.10N
$$

Where:

- $A$ = authoritative support
- $R$ = recurrence across distinct anonymized submissions
- $D$ = durability over time
- $F$ = freshness
- $S$ = source diversity
- $P$ = general-applicability confidence
- $C$ = contradiction level
- $N$ = novelty or spam-risk adjustment

The exact weights can change, but the system should always favor evidence quality over volume.

## Promotion States

Use explicit states so automatic behavior stays legible:

- `candidate` — newly submitted, not trusted enough yet
- `incubating` — repeated and promising, but not yet master-cache quality
- `promoted` — accepted into the master cache snapshot
- `watch` — promoted before, but currently showing signs of drift or contradiction
- `deprecated` — no longer recommended, but still useful historically
- `retired` — no longer relevant and should not influence current recommendations

The easiest safe rule is automatic promotion from `incubating` to `promoted` only after both score and time thresholds are met.

## Suggested Promotion Thresholds

A candidate can be auto-promoted only if all of these are true:

- it has support from at least 3 distinct anonymized conclusion submissions
- it has been observed across at least 2 separate time windows
- it has at least one fresh authoritative or product-guidance revalidation, or two strong repo-example revalidations plus no known contradiction
- its durability window exceeds a minimum age such as 21 or 30 days
- its contradiction penalty remains below a defined cap
- it remains general enough to be safe and useful outside the originating private contexts
- its normalized score exceeds the promotion threshold

A candidate should be auto-demoted from the master cache if either of these becomes true:

- stronger contradictory evidence appears and remains unresolved
- the claim goes stale beyond the freshness policy without revalidation

## GitHub-side Automation

The GitHub repository can handle this with scheduled automation and pull-request automation:

1. On new final conclusion packet arrival, update the aggregated candidate index.
2. On a schedule, recompute scores, durability, and contradiction state.
3. Generate a machine-readable promotion report.
4. If a candidate crosses threshold, open or update an automated PR that adds it to the next master snapshot.
5. If a promoted candidate decays, open or update an automated PR that moves it to `watch`, `deprecated`, or `retired`.

This is still automatic, but GitHub history stays readable because the automation changes snapshots through normal commits and PRs.

## Master Cache vs Candidate Pool

Keep two clearly separate stores:

- `candidates/` — noisy, growing, evidence accumulation area
- `snapshots/` — curated master-cache outputs used by clients

Clients should auto-pull only the published snapshot lane by default, not the raw candidate pool.

Clients in `pull-and-auto-submit` mode should submit only their own final conclusion packets, never the raw candidate pool.

## Anti-gaming Rules

To keep the ranking honest, add these controls:

- count distinct anonymized submissions, not raw submission volume
- cap repeated submissions from the same client within a short time window
- reward revalidation more than repetition
- penalize claims that are too tied to private context or narrow local workflows to be safe as public best practice
- require stronger evidence before promoting anything that would materially change platform guidance

## Privacy Boundary

The community cache must never store or expose:

- private repository names or inferred private repo identities
- local file paths or directory layouts
- internal workflow descriptions tied to one contributor's repo
- company, customer, or personal context
- code excerpts from private workspaces
- repo-specific application advice

The community cache may store only:

- generalized Copilot best-practice conclusions
- general application advice that is safe across repositories
- public source references used as evidence
- aggregate anonymous recurrence and durability signals

If a conclusion cannot be restated as a broad Copilot best practice without mentioning a specific repository, it does not belong in the community cache.

## Recommended Operating Principle

Automatic promotion should mean:

- no manual maintainer triage for every good repeated idea
- no per-audit user confirmation once community participation is enabled
- clear thresholds
- readable GitHub history
- reversible promotion
- stronger evidence outranking louder repetition

That gives you the "smart on GitHub" behavior you want without letting the master cache drift into whatever people happened to repeat the most.

## Audit-System Expectations

When the audit research phase uses the community cache, it should report:

- whether the remote cache was checked
- which snapshot was loaded
- which conclusions came from the community cache
- which of those conclusions were revalidated live
- which cached items were rejected as stale, weak, or superseded

When the audit suggests a community contribution, it should produce a candidate contribution packet rather than silently mutating the shared cache.

## Failure Policy

Do not fail the audit just because the community cache is unreachable, stale, or partially malformed.

Instead:

- mark the cache check as blocked or stale
- continue with official docs, release notes, transcripts, and repository examples
- downgrade trust in any unrefreshed cached principle

## Recommended First Implementation

Keep the first implementation simple:

- store the canonical community cache in a GitHub repository
- expose one stable `manifest.json`
- auto-pull the recommended snapshot during research
- keep publishing opt-in and PR-based
- let the local repo cache remain the durable per-repo memory

This gives you history, rollback, auditability, and easy adoption without turning the shared cache into a hidden authority layer.
