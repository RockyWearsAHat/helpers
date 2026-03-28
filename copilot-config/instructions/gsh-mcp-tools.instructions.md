---
description: "Reference for the gsh MCP server tools available in all workspaces. Use when working with git history, committing, web research, knowledge notes, screenshots, image analysis, or MCP tool calls."
---

# gsh MCP Server Tools

The `gsh` MCP server is installed globally and available in every workspace. It exposes three groups of tools.

## Default Request Preamble

Apply this guidance before processing each user request:

- Prefer direct MCP tool calls when a matching gsh tool exists; avoid terminal emulation for tool behavior checks.
- For diagnostics, use `strict_lint` first.
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
- `cwd` (string, optional) — Absolute path to the git repository to commit in. Auto-detected from the server's working directory when omitted. Pass explicitly when working in a multi-root workspace, a git worktree, or when the target repo differs from the auto-detected root.
- `branch` (string, optional) — Assert that HEAD is on this branch before committing. If the current branch does not match, the commit is aborted with an error. Use this to prevent accidentally committing to the wrong branch.

## Research — Web Search & Knowledge Base

**`search_web`** — Search the web via a local SearXNG instance. Returns ranked results with titles, URLs, and snippets.

**`scrape_webpage`** — Fetch and return the text content of a URL. Use for reading documentation, blog posts, or reference pages.

**`search_knowledge_index`** — Search the knowledge base using TF-IDF indexes. Merges results from two sources:

- **Local index** — built from the workspace's knowledge directory (auto-detected: `knowledge/` in the source repo, `.github/knowledge/` elsewhere).
- **Community index** — pre-built on GitHub (`RockyWearsAHat/github-shell-helpers`), fetched with ETag caching to `~/.cache/gsh/`.

Results are tagged with `source: "local"` or `source: "community"`. Falls back to keyword search if neither index is available. **Prefer this over `search_knowledge_cache`**.

**`build_knowledge_index`** — Build or rebuild the local workspace TF-IDF index. Only affects the workspace index — the community index is pre-built on GitHub. Called automatically after write/update/append operations. Run manually after bulk additions.

**`search_knowledge_cache`** — Keyword search over knowledge files (both workspace knowledge directory and bundled repo knowledge). Use when you need a quick grep-style search or the index is unavailable.

**`read_knowledge_note`** — Read the full content of a knowledge note. Pass just a filename (e.g. `networking-dns.md`) or a workspace-relative path. Resolution order: local workspace → repo knowledge root → GitHub community (fetched with ETag cache). Works for both local and community notes without needing the repo cloned.

**`write_knowledge_note`** — Create a knowledge note. Pass just the filename (e.g. `networking-dns.md`) — the tool auto-detects the correct knowledge directory. Automatically rebuilds the local search index.

**`update_knowledge_note`** — Replace a specific section (by heading) in an existing knowledge note. Automatically rebuilds the local search index.

**`append_to_knowledge_note`** — Append content to an existing knowledge note without replacing it. Automatically rebuilds the local search index.

**`submit_community_research`** — Submit a privacy-safe research conclusion to the community cache. Only call when the workspace has community participation enabled in `.github/devops-audit-community-settings.json`.

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
