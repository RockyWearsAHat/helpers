---
description: "Reference for the gsh MCP server tools available in all workspaces. Use this when working with git history, committing, web research, knowledge notes, screenshots, or image analysis."
applyTo: "**"
---

# gsh MCP Server Tools

The `gsh` MCP server is installed globally and available in every workspace. It exposes three groups of tools.

## Core — Git Checkpoint

**`checkpoint`** — Create a local git commit.

- The caller (the AI) writes the commit message. No AI generation happens inside the tool.
- Stages changes, commits, and optionally pushes.
- Returns the commit hash and summary.

Parameters:

- `message` (required) — The commit message.
- `all` (boolean) — Stage all changes including untracked files (`git add -A`) before committing. Default: `true`.
- `push` (boolean) — Push to remote after committing. Default: `false`.
- `force` (boolean) — Override a mid-session disable. Only use when the user explicitly asked for a checkpoint and the previous call returned `[no-op]`.

## Research — Web Search & Knowledge Cache

**`search_web`** — Search the web via a local SearXNG instance. Returns ranked results with titles, URLs, and snippets.

**`scrape_webpage`** — Fetch and return the text content of a URL. Use for reading documentation, blog posts, or reference pages.

**`search_knowledge_cache`** — Search the workspace's `.github/knowledge/` directory for existing notes. Always check here before doing web research.

**`read_knowledge_note`** — Read the full content of a specific knowledge note file by path.

**`write_knowledge_note`** — Write a new knowledge note to `.github/knowledge/`. Use for saving research findings for future use.

**`update_knowledge_note`** — Overwrite an existing knowledge note with new content.

**`append_to_knowledge_note`** — Append content to an existing knowledge note without replacing it.

**`submit_community_research`** — Submit a privacy-safe research conclusion to the community cache. Only call when the workspace has community participation enabled in `.github/devops-audit-community-settings.json`.

## Vision — Screenshot & Image Analysis

**`take_screenshot`** — Capture a screenshot of the current screen. Returns the image as base64. Requires the gsh-vision VS Code extension to be running.

**`analyze_images`** — Analyze one or more images using a vision model. Pass base64-encoded image data. Use after `take_screenshot` to interpret UI state, errors, or visual content.

## Usage Notes

- The `gsh` server is registered globally via `~/Library/Application Support/Code/User/mcp.json`. It is not workspace-specific.
- If research tools are unavailable (`search_web`, `scrape_webpage`), a local SearXNG instance may not be running. Start it with: `docker run -d --name searxng -p 8888:8080 searxng/searxng:latest`
- If vision tools are unavailable, the gsh-vision VS Code extension may not be active. Install it via the git-shell-helpers installer.
- The `checkpoint` tool always commits to the git repository containing the open workspace. It uses the workspace root, not the server's working directory.
