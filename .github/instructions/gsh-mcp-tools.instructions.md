---
description: "Reference for the gsh MCP server tools available in all workspaces. Use this when working with git history, committing, web research, knowledge notes, screenshots, or image analysis."
applyTo: "**"
---

# gsh MCP Server Tools

The `gsh` MCP server is installed globally and available in every workspace. It exposes three groups of tools.

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
- `cwd` (string, optional) — Absolute path to the git repository to commit in. The server auto-detects the workspace root via MCP roots when exactly one VS Code workspace folder is open. Pass `cwd` explicitly only when working in a multi-root workspace or when the auto-detected root is not the intended repo.

## Research — Web Search & Knowledge Cache

**`search_web`** — Search the web via a local SearXNG instance. Returns ranked results with titles, URLs, and snippets.

**`scrape_webpage`** — Fetch and return the text content of a URL. Use for reading documentation, blog posts, or reference pages.

**`search_knowledge_index`** — Search the knowledge cache using the prebuilt TF-IDF index. Returns ranked results with relevance scores, characteristic terms, related files clustered by content similarity, and snippets. **Prefer this over `search_knowledge_cache`** when the index exists. Falls back to keyword search automatically if no index is present.

**`build_knowledge_index`** — Build or rebuild the TF-IDF search index (`_index.json`). Computes per-document term vectors and precomputes cosine-similarity clusters between all documents. Called automatically after any write/update/append operation. Run manually after bulk additions to the knowledge base.

**`search_knowledge_cache`** — Keyword search over `.github/knowledge/` files. Use when you need a quick grep-style search or the index is unavailable.

**`read_knowledge_note`** — Read the full content of a specific knowledge note file by path.

**`write_knowledge_note`** — Write a new knowledge note to `.github/knowledge/`. Use for saving research findings for future use. Automatically rebuilds the search index.

**`update_knowledge_note`** — Overwrite an existing knowledge note with new content. Automatically rebuilds the search index.

**`append_to_knowledge_note`** — Append content to an existing knowledge note without replacing it. Automatically rebuilds the search index.

**`submit_community_research`** — Submit a privacy-safe research conclusion to the community cache. Only call when the workspace has community participation enabled in `.github/devops-audit-community-settings.json`.

## Vision — Screenshot & Image Analysis

**`take_screenshot`** — Capture a screenshot of the current screen. Returns the image as base64. Requires the gsh-vision VS Code extension to be running.

**`analyze_images`** — Analyze one or more images using a vision model. Pass base64-encoded image data. Use after `take_screenshot` to interpret UI state, errors, or visual content.

## Usage Notes

- The `gsh` server is registered globally via `~/Library/Application Support/Code/User/mcp.json`. It is not workspace-specific.
- If research tools are unavailable (`search_web`, `scrape_webpage`), a local SearXNG instance may not be running. Start it with: `docker run -d --name searxng -p 8888:8080 searxng/searxng:latest`
- If vision tools are unavailable, the gsh-vision VS Code extension may not be active. Install it via the git-shell-helpers installer.
- The `checkpoint` tool auto-detects the active VS Code workspace root via MCP roots (when exactly one workspace folder is open). Pass `cwd` explicitly when using a multi-root workspace or when you need to commit in a different repo than the detected root.
