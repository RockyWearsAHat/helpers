---
name: devops-audit-community-submit
description: "Turn a completed audit into privacy-safe community cache conclusion packets and submit them when community participation is enabled. Extracts ALL generalizable conclusions, not just one."
user-invocable: false
---

# DevOps Audit Community Submit

This runs only after the audit itself is complete.

## Purpose

Take the accepted final audit result and extract ALL generalizable GitHub Copilot best-practice conclusions suitable for the public community cache. Each conclusion becomes one privacy-safe packet.

## Rules

- Never submit raw notes, workspace descriptions, or file inventories.
- Never submit repository-specific guidance.
- Never include private paths, private repo names, or project descriptions.
- Submit only generalized Copilot best practices and general application advice.
- Submit only if community participation is enabled for the client.
- If community participation is disabled or the shared cache repo is not configured, skip submission and report why.
- Extract as many generalizable conclusions as the audit supports — do not limit to one.

## Packet Shape

Write one JSON packet per conclusion that matches `community-cache/schemas/final-conclusion.schema.json`.

Each `statement` should be one concise generalized claim such as:

- "Keep prompt files as thin entrypoints and move methodology into skills."
- "Do not judge prompt quality by tone or confidence; judge it by goals, boundaries, and evidence expectations."
- "Validate YAML frontmatter syntax before judging content quality."

Each packet must use only public evidence references and generalized wording.

## Extraction Process

1. Read the accepted final audit result.
2. Identify ALL generalizable conclusions:
   - Principles discovered or reinforced during the audit.
   - Anti-patterns detected that would apply broadly.
   - Workflow patterns that proved effective.
   - Deprecations or compatibility issues encountered.
3. For each conclusion, classify its kind (principle, anti-pattern, example, warning), topic, and applicability using the schema enums.
4. Check each against the existing cache search-index to avoid submitting duplicates of already-known conclusions. If a conclusion is substantially similar to an existing cache entry, skip it.
5. Write only conclusions that add new knowledge to the cache.

## Deduplication

Before submitting, read `community-cache/snapshots/<current>/search-index.json` and compare each candidate conclusion against existing entries in the relevant topic. Skip conclusions that:

- Restate an existing principle with no new evidence or nuance.
- Cover exactly the same anti-pattern already documented.
- Add no new evidence refs beyond what the cache already has.

## Submission Path

1. Read the accepted final audit result.
2. Extract all generalizable conclusions (target: 3-10 per audit).
3. Deduplicate against the current cache snapshot.
4. Write each unique packet to a temporary JSON file.
5. Run `git-copilot-devops-audit-community-submit <temp-packet>` for each packet.
6. Report the aggregate submission results.

## Skip Conditions

Skip submission entirely if any of these are true:

- community participation is not enabled
- the shared cache repo is not configured

Skip individual conclusions if:

- the conclusion is still repository-specific
- the conclusion cannot be phrased safely as broad Copilot guidance
- no strong evidence supports the conclusion
- the conclusion duplicates an existing cache entry

## Output

Return:

- how many packets were submitted (e.g., "5 of 7 candidate conclusions submitted")
- the generalized statements that were chosen
- what evidence supported each
- why each was safe for the public cache
- for skipped conclusions: the exact reason each was skipped (duplicate, repo-specific, weak evidence)
