---
description: "Reference for the gsh MCP server tools available in all workspaces. Use when working with git history, committing, web research, knowledge notes, screenshots, image analysis, or MCP tool calls."
---

# gsh MCP Server Tools

The `gsh` MCP server is installed globally and available in every workspace. It exposes three groups of tools.

## Default Request Preamble

Apply this guidance before processing each user request:

- Prefer direct MCP tool calls when a matching gsh tool exists; avoid terminal emulation for tool behavior checks.
- **After every file edit, call `strict_lint` on the modified file before declaring work complete.** Fix reported errors and warnings or name each one with a reason for leaving it. Do not return "implementation complete" while unresolved issues exist.
- `strict_lint` defaults to `severityFilter: "all"` when omitted (includes error, warning, info, and hint diagnostics).
- Only pass `severityFilter` when the user explicitly asks to narrow severity.
- If there is a reported mismatch with VS Code squiggles, rerun `strict_lint` on the exact file path with default severity before deeper debugging.

## Core — Git Checkpoint

**`checkpoint`** — Create a local git commit with an AI-generated message.

- Stages changes, generates a commit message from the diff via AI, and commits.
- You do not write the message. Just call the tool.
- Optionally pass `context` with extra hints (e.g. `"fixes the login race from PR #42"`).
- Pass `message` only to override the AI output with a specific string.

Parameters:

- `context` (string, optional) — Extra context for the AI message generator. Use when the diff alone doesn't tell the whole story.
- `message` (string, optional) — Manual override. Skips AI generation entirely.
- `all` (boolean) — Stage all changes including untracked files (`git add -A`) before committing. Default: `true`.
- `push` (boolean) — Push to remote after committing. Default: `false`.
- `force` (boolean) — Override a mid-session disable. Only use when the user explicitly asked for a checkpoint and the previous call returned `[no-op]`.
- `cwd` (string, optional) — Absolute path to the git repository to commit in. Auto-detected from the workspace root when omitted. Pass explicitly when working in a multi-root workspace, a git worktree, or when the target repo differs from the auto-detected root.
- `branch` (string, optional) — Assert that HEAD is on this branch before committing. If the current branch does not match, the commit is aborted with an error. Use this to prevent accidentally committing to the wrong branch.

## Core — Workspace Context

**`workspace_context`** — Return the current workspace context: root folders, branch, worktree status, and remote URL for each.

Call this:

- **At the start of a session** to orient yourself (which repo, which branch, what state).
- **Before cross-branch operations** to confirm you're on the right branch.
- **When working on a feature branch** to verify your branch before making changes.

No parameters. Returns one block per workspace root with: root path, branch name, worktree flag, remote URL, and short git status.

## Core — Branch Sessions

Branch sessions give agents isolated working directories via git worktrees. Each session gets its own filesystem checkout under `~/.cache/gsh/worktrees/`, leaving the main workspace untouched. Agents can work on different branches independently — like developers on a team each having their own clone.

**`branch_session_start`** — Start an isolated branch session.

- Creates a worktree for the given branch (or creates the branch if it doesn't exist).
- Returns the absolute path to the worktree.
- **Use this path for all subsequent file operations and terminal commands.**
- If a session already exists for the branch, returns the existing path.

Parameters:

- `branch` (string, required) — The branch to work on.
- `base` (string, optional) — Create the branch from this ref. Only used for new branches. Defaults to HEAD.

**`branch_session_end`** — End an isolated branch session.

- Auto-commits any uncommitted work, then removes the worktree.
- The branch and all its commits are preserved for later merge.
- Pass `discard: true` to throw away uncommitted changes instead of committing them.

Parameters:

- `branch` (string, required) — The branch whose session to end.
- `discard` (boolean, optional) — Discard uncommitted changes instead of auto-committing. Default: false.

**`branch_read_file`** — Read a file from any branch without a worktree.

- Uses `git show` to read directly from the commit tree.
- No checkout or worktree needed — fast and safe for cross-branch inspection.

Parameters:

- `branch` (string, required) — The branch to read from.
- `filePath` (string, required) — Repository-relative path (e.g. `lib/upload-ai-message.sh`).

**`branch_status`** — Show all active branch sessions and local branches.

- Reports: active worktree sessions with their status, the main workspace branch, and all local branches with latest commits.
- No parameters.

## Core — Diagnostics

**`strict_lint`** — Run VS Code’s live diagnostics (errors and warnings) on a file, folder, or the entire workspace.

- Returns the same output as the Problems panel.
- **Call this after every file edit before declaring implementation complete.** If errors or warnings are reported, fix them or explicitly document why they are acceptable.
- Re-run until clean. Do not return “implementation complete” while unresolved issues exist without naming each one and why it was left.

Parameters:

- `filePath` (string, optional) — Absolute path to a specific file to check. Omit to check the whole workspace.
- `folderPath` (string, optional) — Absolute path to a folder to check.
- `severityFilter` (string, optional) — `"all"` (default), `"errors-only"`, or `"warnings-and-above"`.

**`list_language_models`** — List the language models available in VS Code’s language model service.

- Returns each model’s `id`, display name, vendor, and `qualifiedName`.
- Use this when you need to pass a valid model identifier to `runSubagent` or report available models to the user.
- The list is written by the gsh VS Code extension on startup and whenever the model set changes.
- No parameters.

## Research — Web Search & Knowledge Base

**`search_web`** — Search the web via a local SearXNG instance. Returns ranked results with titles, URLs, and snippets.

**`scrape_webpage`** — Fetch and return the text content of a URL. Use for reading documentation, blog posts, or reference pages. If you pass `output_file`, it must include an explicit subdirectory such as `knowledge/note.md` or `.github/knowledge/note.md`; bare filenames are rejected so research does not spill into the workspace root.

**`search_knowledge_index`** — Search the knowledge base using TF-IDF indexes. Merges results from two sources:

- **Local index** — built from the workspace's knowledge directory (auto-detected: `knowledge/` in the source repo, `.github/knowledge/` elsewhere).
- **Community index** — pre-built on GitHub (`RockyWearsAHat/github-shell-helpers`), fetched with ETag caching to `~/.cache/gsh/`.

Results are tagged with `source: "local"` or `source: "community"`. Falls back to keyword search if neither index is available. **Prefer this over `search_knowledge_cache`**.

**`build_knowledge_index`** — Build or rebuild the local workspace TF-IDF index. Only affects the workspace index — the community index is pre-built on GitHub. Called automatically after write/update/append operations. Run manually after bulk additions.

**`search_knowledge_cache`** — Keyword search over knowledge files (both workspace knowledge directory and bundled repo knowledge). Use when you need a quick grep-style search or the index is unavailable.

**`read_knowledge_note`** — Read the full content of a knowledge note. Pass just a filename (e.g. `networking-dns.md`) or a workspace-relative path. Resolution order: local workspace → repo knowledge root → GitHub community (fetched with ETag cache). Works for both local and community notes without needing the repo cloned.

**`write_knowledge_note`** — Create a knowledge note. Pass just the filename (e.g. `networking-dns.md`) — the tool auto-detects the correct knowledge directory. It rebuilds the local search index before returning. Pass `publish: true` when the note is privacy-safe, broadly reusable, and should be auto-submitted to the shared knowledge base if `shareKnowledge` is enabled.

**`update_knowledge_note`** — Replace a specific section (by heading) in an existing knowledge note. Rebuilds the local search index before returning. Pass `publish: true` when the updated note should also be submitted to the shared knowledge base.

**`append_to_knowledge_note`** — Append content to an existing knowledge note without replacing it. Rebuilds the local search index before returning. Pass `publish: true` when the updated note should also be submitted to the shared knowledge base.

**`submit_community_research`** — Submit a privacy-safe knowledge note to the shared knowledge base. Only call when the workspace has `shareKnowledge: true` (or legacy `shareResearch: true`) in `.github/devops-audit-community-settings.json`. The submission rebuilds `knowledge/_index.json` in the PR so the hosted cache stays searchable.

## Vision — Screenshot & Image Analysis

**`take_screenshot`** — Capture a screenshot of the current screen. Returns the image as base64. Requires the gsh-vision VS Code extension to be running.

**`analyze_images`** — Analyze one or more images using a vision model. Pass base64-encoded image data. Use after `take_screenshot` to interpret UI state, errors, or visual content.

## Knowledge-First Protocol

When you encounter uncertainty, need background on an unfamiliar topic, or want to verify an assumption, **check the knowledge base before searching the web**. The knowledge base contains ~950 encyclopedia-quality reference notes covering algorithms, architectures, languages, frameworks, security, infrastructure, and more.

Research order:

1. **`search_knowledge_index`** — TF-IDF search across local + community knowledge. Start here.
2. **`read_knowledge_note`** — Read the full note for any promising result.
3. **`search_knowledge_cache`** — Keyword fallback for exact term matching.
4. **`search_web` + `scrape_webpage`** — Only after exhausting local knowledge, or when you need current/volatile information (API versions, recent releases, pricing).

The community cache (`community-cache/` in the source repo, fetched remotely with ETag caching) contains crowdsourced Copilot customization best practices. Use `search_knowledge_index` (which merges local + community results) to search both at once.

Treat knowledge notes as informed starting context — they are accurate but not infallible. Verify volatile details (version numbers, API signatures, tool flags) against current sources. See `knowledge-philosophy.md` for the full uncertainty gradient.

## Usage Notes

- The `gsh` server is registered by the GitHub Shell Helpers VS Code extension. It can also be configured manually via `~/Library/Application Support/Code/User/mcp.json`.
- If research tools are unavailable (`search_web`, `scrape_webpage`), a local SearXNG instance may not be running. Start it with: `docker run -d --name searxng -p 8888:8080 searxng/searxng:latest`
- If vision tools are unavailable, the gsh-vision VS Code extension may not be active. Install it via the git-shell-helpers installer.
- The `checkpoint` tool auto-detects the active VS Code workspace root via MCP roots (when exactly one workspace folder is open). Pass `cwd` explicitly when using a multi-root workspace or when you need to commit in a different repo than the detected root.
