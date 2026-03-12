---
name: copilot-research
description: "Research method for current Copilot customization guidance."
user-invocable: false
---

# Copilot Research

Gather real, current, verified Copilot customization knowledge that directly applies to this project.

The research must be thorough. Surface-level research produces surface-level results. The quality of everything that follows depends on how well this step understands what Copilot can do today, what has changed recently, what guidance is now outdated or deprecated, which sources should shape the normative target state, and what the correct patterns are for this specific type of project.

Thorough means breadth, depth, triangulation, and clarity. You must cover the topic from multiple source types, go past overview material into the pages, files, and transcripts that actually define behavior and design intent, and cross-check major conclusions so that one weak source cannot distort the result.

The goal is not total knowledge of the entire system. The goal is enough understanding to define the intended current best-practices setup for this workspace confidently and to remove any confusion that would block evaluation or implementation.

The point of this phase is not to produce a pile of consulted sources. The point is to define the clean target state that the evaluator and implementer should aim for.

The point is also to discover better implementations. If the current setup technically works but the evidence shows a cleaner, clearer, more maintainable, more efficient, or more accurately routed design, you are expected to surface that.

Research should clarify uncertainty. It should not leave the next phase more confused than before. If the output still feels muddy, overloaded, hard to explain, unclear about which guidance is current versus outdated, or unclear about which sources deserve the most weight, keep researching until you can state the best-practices target state clearly.

## What You Are Researching

You are not researching the project itself. The project is fine.

You are researching how Copilot customization works right now, what the correct file formats and field names are, what patterns exist for projects like this one, and what common mistakes look like. The goal is to know enough that when you look at the `.github/` folder, you can immediately tell what is correct, what is outdated, what is broken, what is missing, and what could be made cleaner or clearer even if it is not strictly broken.

## Runtime Boundaries

The context agent already read the workspace. Do not repeat that work.

- Do not search the workspace for more Copilot files during the research phase.
- Do not treat workspace files as evidence of best practice.
- Only read workspace artifacts that were explicitly handed to you for this phase, such as the context report or the persistent audit research cache.
- Spend the research budget on external evidence: official docs, release notes, repository examples, transcripts, and model validation.
- If you catch yourself reading local source files or `.github/` files to answer a research question, stop and switch back to external sources.

Use the context only to understand the project type, the active workflows, the current Copilot surface area, and the user's focus. Do not let current repo wording anchor you to the current implementation if the documentation and strong examples point toward a better design.

Use the user's focus and the inferred project intent aggressively. If the audit was launched with a focus, optimize the research toward that focus. If no focus was given, infer likely goals from the project type, source layout, and current customization intent, then research toward the workflows that most matter.

### File Type Purposes

One of the most important things to verify is the intended purpose of each file type. These purposes define what content belongs where. Putting content in the wrong file type causes real behavioral problems — for example, an agent file stuffed with task instructions will cause the model to do the work itself instead of loading its skill.

Research and verify the current intended purpose of each file type. Do not assume — these could change. Report what you find so the evaluator can check every file against it.

You are expected to come back with normative expectations, not just observations. Say what belongs in each file type, what does not, and what a clean setup for this project type should optimize for.

You are expected to convert complex findings into plain English. If you cannot explain the core idea simply, you have not understood it well enough yet.

You are also expected to distinguish current guidance from stale guidance. If you find older examples, older videos, older docs, or community advice that conflicts with newer official guidance or release notes, say so explicitly and treat the newer authoritative guidance as controlling unless you have strong evidence otherwise.

Do not throw away an example just because it is older, imperfect, or technically malformed. Older examples can still teach useful prompting patterns, workflow structure, or design tradeoffs. The important distinction is this: examples are usually illustrative evidence, while official docs, release notes, and strong product-team guidance are the primary normative evidence for what should be recommended now.

## Source Priority

1. Official VS Code and GitHub Copilot documentation and release notes — this is the most reliable source
2. Product-team guidance — Burke Holland's channel is especially useful for understanding how Copilot customization actually works in practice, not for templates but for understanding the design and intent behind features ([https://www.youtube.com/@BurkeHolland](https://www.youtube.com/@BurkeHolland))
3. Well-maintained reference repositories — Awesome Copilot ([https://github.com/github/awesome-copilot](https://github.com/github/awesome-copilot)) and Anthropic's skills repo ([https://github.com/anthropics/skills](https://github.com/anthropics/skills))
   3 1/2. Public repositories that compile skills/agents/prompts, such as Antigravity or ANVIL
4. Public repositories with similar project types that use `.github/` customization
5. Community sources — only when backed up by something stronger

Do not treat any single source as enough on its own. Cross-check everything.

When evidence conflicts, prefer the most current authoritative source that is still relevant to the exact feature being audited. Do not flatten old and new guidance into a compromise. Call out deprecations, renamed fields, replaced patterns, and examples that were valid once but should not drive the target state now. At the same time, preserve useful example patterns as examples when they still illuminate how people achieved better results.

Do not use Awesome Copilot, Anthropic skills, or any other meta-repository as your main source of truth for requirements. They are useful for examples, discovery, and common patterns, not for turning optional ideas into mandatory rules unless the official docs or stronger product guidance agree.

Do not treat product-team video evidence as optional flavor. It is one of the best sources for how the system is intended to be used in practice, how pieces fit together, and what high-quality agentic workflows actually look like. When product-team guidance and user-derived examples disagree, the product-team guidance should usually carry more weight for the normative recommendation.

## How to Actually Research (This Is the Critical Part)

Fetching a webpage and reading the landing page is not research. Most useful information is not on the first page you visit. You must navigate, explore, and follow links to find the actual content.

Do not stop when you have enough material to sound convincing. Stop when you have enough material to be precise and simple.

Prefer the built-in `fetch` tool for official documentation pages and the built-in `githubRepo` tool for reading public repository files. Use terminal commands as a fallback when those tools cannot retrieve a specific artifact. Do not claim the research was blocked just because one tool path failed if another documented path is still available. If one tactic stalls, change tactics and keep going.

Your first meaningful actions in this phase should usually be external: fetch the official docs or inspect public repository examples. Do not begin by searching the workspace.

### Official Documentation

The official docs are spread across multiple pages. Do not stop at the landing page. Typical structure:

1. Start at the main customization page (e.g. `https://docs.github.com/en/copilot/customizing-copilot`)
2. That page will have links to subpages about specific topics — instruction files, custom agents, prompt files, skills, etc. **Follow those links and read those pages.**
3. For VS Code docs, check `https://code.visualstudio.com/docs/copilot/copilot-customization` and its subpages.
4. Check release notes by searching for "copilot" in recent VS Code changelogs: `https://code.visualstudio.com/updates/`

Freshness requirements:

1. Use release notes and current docs to validate that guidance is still current before promoting it to target-state guidance.
2. If you rely on an older repo example or older video because it explains something especially well, verify that the pattern still matches current docs or recent product guidance before treating it as normative. It can still be cited as an example even when it is not current enough to define the recommendation.
3. If you cannot verify freshness for an important normative claim, mark it as uncertain rather than silently treating it as current best practice.

If a page mentions a feature but doesn't explain the details, look for a "learn more" link or search the docs site for that feature name. Do not assume the first page you land on has everything.

If a page gives a summary table, look for the page that defines the behavior behind the table. If a page gives an example, look for the guide or reference that explains why the example is structured that way.

### Repositories

Fetching a repository's landing page gives you the README. The README rarely contains the actual customization files. To see how a repository sets up Copilot:

- First try the built-in `githubRepo` tool to inspect the repository and read the actual `.github/` files.
- If the built-in repo tool cannot access the needed file, fall back to `gh api` in the terminal.

```bash
# List the .github folder structure of a repo
gh api repos/OWNER/REPO/git/trees/main?recursive=1 2>/dev/null | \
  jq -r '.tree[].path' | grep '^\.github/'

# Read a specific file from a repo
gh api repos/OWNER/REPO/contents/.github/copilot-instructions.md 2>/dev/null | \
  jq -r '.content' | base64 -d

# Read an instruction file
gh api repos/OWNER/REPO/contents/.github/instructions/FILENAME 2>/dev/null | \
  jq -r '.content' | base64 -d

# Read an agent file
gh api repos/OWNER/REPO/contents/.github/agents/FILENAME 2>/dev/null | \
  jq -r '.content' | base64 -d
```

When you find a relevant repository:

1. List its `.github/` folder to see what files exist
2. Read the actual files that are relevant — instruction files, agent files, prompt files, skill files
3. Read at least one additional project artifact that explains why their Copilot setup fits the project: build config, CI workflow, test setup, contributing guide, or a representative code area tied to the same workflow
4. Note what patterns they use: frontmatter fields, `applyTo` patterns, file naming, folder structure, routing, and how the customization matches actual project workflows
5. Do this for at least 3 repositories to see what is common vs unusual

Do not overweight examples just because there are many of them. Use them to learn how people structured prompts, agents, skills, and workflows in practice. Use docs, release notes, and strong product-team guidance to decide which of those patterns should actually shape the recommendation now.

Prefer real projects over definition repositories. A strong project example has actual source code, build/test workflows, and Copilot files that appear tailored to that reality.

Do not collect repositories mechanically. Pick repositories that actually teach you something different, then extract the specific pattern they prove.

### Skills and Related Workflow Patterns

Skill discovery matters. Look for many real `SKILL.md` files and other task-oriented Copilot assets that map to the audited workspace's workflows.

When researching skills:

1. Prefer skills that are closely related to the audited workspace's actual work: build, test, UI, debugging, deployment, security, domain workflows, or the user-provided focus.
2. Read the actual `SKILL.md` files, not just repo listings.
3. Extract useful skill patterns: when the skill is invoked, how narrowly it is scoped, what sources it uses, how it shapes outputs, and why it is effective.
4. Reject irrelevant skill examples even if they are popular.

The goal is not to recommend random skills. The goal is to understand what kinds of skills or workflow-specific guidance would actually improve this workspace.

### Awesome Copilot and Curated Lists

These repositories are link collections, not documentation. Their value is in the links they contain, not in their own README.

```bash
# Get the README of awesome-copilot, which contains curated links
gh api repos/github/awesome-copilot/contents/README.md 2>/dev/null | \
  jq -r '.content' | base64 -d

# Then follow the links that are relevant to customization
```

Read the README to find links. Then follow the links that relate to customization, agents, instructions, or workspace setup. The list itself is not the research — it's the starting point.

Do not stop with a curated-list README. If your report cites curated guidance, it should mostly be to support or contrast conclusions already grounded in official docs and real project repositories.

### Burke Holland Videos

Search for recent videos, then download and read the transcripts of the most relevant ones:

If transcript tooling is unavailable locally, report that specific limitation and continue with the rest of the research instead of downgrading the entire pass to shallow research.

```bash
# Search for recent videos
yt-dlp --flat-playlist --print "%(id)s %(title)s %(upload_date)s %(channel)s" \
  "ytsearch10:Burke Holland copilot customization" 2>/dev/null

# Download transcript for a relevant video
yt-dlp --write-auto-sub --write-subs --sub-lang en --sub-format vtt --skip-download \
  -o "/tmp/yt-transcript-%(id)s" "https://www.youtube.com/watch?v=VIDEO_ID" 2>/dev/null

# Read the transcript
cat /tmp/yt-transcript-VIDEO_ID.en.vtt 2>/dev/null
```

Pick videos by title relevance and recency. Read the transcripts for specific claims about how Copilot customization works, what fields do what, what changed, and what mistakes to avoid.

This is not optional in a normal pass. You are expected to use product-team transcript evidence to understand practical workflow design, not just syntax rules.

Transcript selection rules:

1. Prefer Burke Holland videos first when they are relevant to Copilot customization, agents, prompt files, skills, workflow design, or end-to-end agentic development.
2. Prefer recent videos, but allow slightly older ones if they are clearly more relevant to the feature or workflow under review and you verify that their guidance still matches current docs or release notes for normative use.
3. Pick transcripts that help answer how the system is intended to be composed and used efficiently, not just what fields exist.
4. If the audit has a user focus, prioritize transcripts that illuminate that workflow.

Transcript usage rules:

1. Extract concrete workflow principles, not just factual snippets.
2. Note where transcript guidance reinforces, clarifies, or tensions with the official docs.
3. Use transcript insights to improve the `Target-state blueprint`, especially around workflow structure, role separation, and practical efficiency.
4. If the transcript shows a more effective pattern than the current repo setup, surface that as an improvement opportunity.

If no relevant transcript can be retrieved, document the exact commands attempted, the exact failure, and why the remaining research is still usable. A transcript blocker should be rare, specific, and visible.

## Thoroughness Rules

Your research is only thorough if all of the following are true:

1. **Breadth**: You used multiple source types with meaningful coverage.
2. **Depth**: You went past overview pages and READMEs into the pages, files, and transcripts that actually define how things work.
3. **Triangulation**: Major conclusions are checked across at least two source types whenever possible.
4. **Precision**: You distinguish required, recommended, optional, illustrative, deprecated, stale, environment-specific, and unverified claims.
5. **Actionability**: The result tells the evaluator and implementer what correct or better looks like without forcing them back into research mode.
6. **Clarity**: You can explain the key parts of the relevant system and workflow in plain English without hiding behind jargon, vague wording, or source dumping.
7. **Uncertainty Removal**: Blocker-level confusion is resolved, not merely restated. Any remaining uncertainty must be narrow, explicit, and non-blocking to the current best-practices target state or the research is incomplete.
8. **Freshness**: The report makes clear why the recommended guidance is current, and it explicitly distinguishes between examples that are still useful to learn from and patterns that should no longer drive the recommendation.
9. **Resilience**: When one source, repo, page, or tool path fails, you keep pursuing other viable paths before reporting a blocker.

If any of these are missing, keep researching.

## Research Loop

Use this loop until the output is ready:

1. Gather an initial set of official docs and strong examples.
2. Extract tentative conclusions.
3. Stress-test those conclusions against release notes, repository files, transcripts, skills, and conflicting evidence.
4. Fill the gaps where the conclusion is still vague, weak, source-thin, potentially outdated, or overly dependent on one type of source.
5. Only then turn the conclusion into target-state guidance.

Do not jump directly from first-pass reading to final recommendations.

### GitHub Code Search

Use this to find real-world examples of specific patterns:

```bash
# Find repos using .agent.md files
gh search code "filename:.agent.md" --sort=indexed --limit=15

# Find repos using prompt files
gh search code "filename:.prompt.md path:.github/prompts" --sort=indexed --limit=15

# Find repos using skill files
gh search code "filename:SKILL.md path:.github/skills" --sort=indexed --limit=15

# Find repos using specific frontmatter fields
gh search code "applyTo" --filename=.instructions.md --sort=indexed --limit=10

# Find how people use copilot-instructions.md
gh search code "filename:copilot-instructions.md path:.github" --sort=indexed --limit=10
```

When a search returns results, pick 2-3 promising repos and actually read their `.github/` files (see the repository section above for how).

### Model Validation

```bash
gh api https://api.githubcopilot.com/models \
  -H "Authorization: Bearer $(gh auth token)" \
  -H "Copilot-Integration-Id: vscode-chat" 2>/dev/null
```

### When You Hit a Dead End

If a page has no useful information:

- Check if the page links to subpages that have the actual content
- Search the same site for more specific terms
- Try a different source from the priority list
- If a repository README is unhelpful, check its `.github/` folder directly
- If one tool path fails, switch tools before concluding the evidence is unavailable
- If one repo example is noisy or malformed, mine additional examples and compare common threads instead of discarding the line of inquiry

Do not report "I checked the docs" if you only read one page and it was a landing page. Go deeper.

## What You Must Cover

Every research pass must address all of these. If you skip one, say which and why.

- Current file formats and frontmatter fields for instruction files, agent files, prompt files, and skill files
- Valid folder structure under `.github/` (what goes where)
- Current valid tool names for frontmatter `tools:` field
- Current valid model IDs (verified with the API, not assumed)
- Recent VS Code release notes for Copilot-related changes
- Recent deprecations or breaking changes (field names that changed, features that were removed)
- How 3+ real repositories structure their `.github/` Copilot files (read the actual files, not just the README)
- How those same repositories connect their customization to real project workflows beyond `.github/`
- At least 1 recent product-team video transcript for practical insights, and preferably 2 when the workflow is broad or architecture-heavy
- Known common mistakes and what the correct version looks like
- Concrete improvement opportunities or likely bugs in the audited setup, even when the current setup is not outright broken

Minimum evidence bar:

- At least 10 concrete external references total
- At least 4 official references across docs and release notes
- At least 3 real project repositories explored at the file level
- At least 1 transcript or equivalent product-team primary source
- At least 3 major conclusions supported by more than one source type
- At least 3 concrete patterns learned from real external skills, agents, prompts, or instruction files that are relevant to this workspace

If transcript tooling works and relevant Burke Holland videos exist, omitting transcript evidence is a research failure.

If you do not meet this bar, the research is incomplete.

## Output

Return findings in a structured format covering:

- `Status`: complete / incomplete
- `Coverage checklist`: one line per mandatory topic with `verified`, `blocked`, or `missing`
- `Evidence ledger`: the concrete sources that support each major conclusion
- `Reference matrix`: numbered references with exact URLs and what each reference establishes
- `Target-state blueprint`: the clean intended current workflow and file design for this project type and user focus
- `Freshness notes`: what evidence is most current, what changed recently, which older patterns are still useful as examples, and which ones should not drive the current recommendation
- `Source weighting`: which sources are authoritative for normative claims, which are illustrative, and how conflicts were resolved
- `Implementation cues`: concrete statements the evaluator can translate into per-file keep/fix/merge/move/delete decisions
- `Improvement opportunities`: specific ways the audited setup could be better even if parts of it are technically valid
- `Likely bugs or anti-patterns`: concrete failure risks, misleading guidance, routing problems, over-broad scope, duplication, or dead/inert files to look for
- `Transcript takeaways`: concrete workflow and system-design lessons learned from product-team videos, plus how they change or sharpen the target state
- `Triangulated conclusions`: major conclusions with the source types that support them
- `Related skill patterns`: concrete skill ideas or structures discovered in external repositories that map to this workspace's workflows
- `Plain-English explanation`: a simple explanation of how the relevant part of the system works and how the pieces fit together
- `Confusion ledger`: remaining uncertainties, why they remain, whether they block action, and what would resolve them

- What the docs currently say (summarized, not copied)
- Valid model IDs (verified, not assumed)
- Relevant video or post evidence with dates
- Patterns from similar repositories
- Confirmed mistakes and pitfalls to watch for
- What has changed recently that affects customization
- Open questions where you could not find a clear answer
- Specific recommendations for this project type

The `Target-state blueprint` must be the most actionable part of the report. It should say, in plain language, what the ideal `.github/` setup should look like for this project and what qualities it should prioritize: clarity, brevity, routing accuracy, workflow fit, maintainability, and any focus-specific needs the user provided.

The `Implementation cues` must be concrete enough that the evaluator can produce an implementation-ready plan without having to go back out to the web.

The `Reference matrix` should be dense and specific. Prefer many precise references over a few broad overview pages.

The `Transcript takeaways` must not be filler. They should capture what the product-team source teaches about composing agents, prompts, instructions, skills, tool use, iteration flow, or other practical workflow patterns that the docs alone do not make vivid.

The `Related skill patterns` section must focus on actual useful skill designs, especially skills that map closely to the audited workspace's build, test, UI, debugging, security, deployment, or domain workflows. Do not pad this with irrelevant skill examples.

The `Plain-English explanation` must be simple enough that a non-expert could follow it. No jargon unless you define it immediately. If the explanation still sounds complicated, keep researching.

The `Confusion ledger` should usually be short. A long confusion ledger means the research is not ready. If a confusion item would make evaluation or implementation guess about the intended current best-practices target state, the overall status must be `incomplete`.

If the models API check fails, report the exact failure and fall back to current official Copilot model documentation. Do not silently skip model validation.

If any mandatory topic is still `missing` rather than `verified` or `blocked`, the overall status must be `incomplete` and you must say the research is not ready for evaluation.

For every major conclusion, cite the evidence source in plain text inside the report body, such as the doc page title and URL, repository name and file path, release notes month, or video title and date. If a conclusion has no external evidence, label it unverified instead of presenting it as established.

Be honest about what you verified and what you could not. Never present unverified information as confirmed.

Keep the report compact. The goal is a dense evidence-backed handoff with a clear target state, not a long narrative.

Compact does not mean shallow. If the report would still look plausible after deleting most of the URLs, repo paths, specific references, triangulated conclusions, or plain-English explanation, it is too weak.

## Before You Finish: Check Your Own Results

Activity is not results. Fetching 10 pages and running 4 searches means nothing if you did not learn anything from them.

Research is done when you can answer YES to all of the following. If you cannot, you are not done.

### 1. Can you describe the ideal `.github/` setup for this project?

Not in vague terms. You should be able to say exactly:

- What files should exist (instruction files, agent files, prompt files, skills)
- What each file's frontmatter should look like (which fields, which values)
- What each file's content should cover for this specific project
- What should NOT exist (unnecessary files, redundant content)
- If you implement this right now, for every single line of the file, are you absolutely certain there will be no syntax errors with the most recent version of Copilot?

If you cannot do this, you do not understand the target state well enough.

### 2. Do you have evidence from real repositories?

You should have read the actual `.github/` files (not just READMEs) from at least 3 repositories that use Copilot customization [NOT DEFINITION REPOSITORIES, BUT PROJECTS]. You should know:

- What patterns they use that have helped them reach this point, what is the current point and state? Value repositories with higher stars while avoiding purely instruction repositories as this will give us the most popular and helpful projects that have actually used copilot to help them achieve their current state.
- What they do differently from this project
- What of their ideas and uses would improve this project's setup

If you only read landing pages and search results, or READMEs, you have not seen how anyone actually does this.

### 3. Do you know what has changed recently?

Copilot customization changes frequently. You should be able to name:

- Specific features or fields that were added, changed, or deprecated recently
- Whether anything in the current `.github/` setup uses outdated syntax or patterns

If you cannot name a single recent change, you did not check.

### 4. Could you rewrite any file in `.github/` from scratch right now?

This is the real test. If someone deleted every Copilot file in `.github/` and asked you to rebuild them from scratch for this project, could you do it correctly based on what you learned? Could you write valid frontmatter, accurate content, and proper file structure without guessing?

If the answer is no, your research is not done. Go learn what you are missing.

### 5. Are your findings specific or generic?

Read through your report. If you replaced this project's name with any other project and the report would still make sense, it is too generic. Your findings should be specific to this project's type, stack, and workflows.

### 6. Did you reach real understanding, not just source collection?

You should be able to explain not only what the docs say, but why strong examples are structured the way they are and how those patterns should or should not transfer to this workspace.

If you cannot do that, you are still at source collection, not research.

### 7. Could you explain the relevant system simply to a beginner?

If you cannot explain the relevant component, workflow, or structure in clear, simple English without sounding confused yourself, you are not done. Research is supposed to remove confusion that blocks defining the best-practices system for this workspace.

If blocker-level confusion remains, return `incomplete`.
