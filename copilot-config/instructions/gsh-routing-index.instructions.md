---
description: "Always-on routing index for gsh MCP tools. Keeps key tool-call routing rules in every request without loading the full reference."
applyTo: "**"
---

## ⚠️ STOP — IF THE USER WANTS A TOOL, READ THIS FIRST

If the user says anything like: "save it as a tool", "save this as a tool", "build a tool", "make it a tool", "reusable tool", "save as reusable", or any phrasing that implies creating a runnable script that an agent can call:

**Write the implementation yourself, then CALL `register_workspace_tool` IMMEDIATELY.** Do not investigate "how this workspace stores tools". Do not search for `*prompt*` files or similar patterns. Do not LIST or READ any prompts folder. Do not READ memory to check what format to use. Do not reason about `VSCODE_USER_PROMPTS_FOLDER` at all. There is no discovery step — go directly to implementation.

The correct sequence:
1. Write the script implementation (JS: `module.exports = async (args) => { ... }` or shell)
2. Call `register_workspace_tool(name, description, script, language)` — this saves the file and makes it callable

When registering multiple tools in one request:
1. Finish ALL `register_workspace_tool` / `unregister_workspace_tool` calls first.
2. Then call `reload_window_ready({ keyword: "window reloaded" })` exactly once.
3. Wait for the keyword `window reloaded` to appear in chat before continuing the original request.

If `register_workspace_tool` is not found or not available in this session: write the script file directly into the workspace as a `.js` or `.sh` file using file creation tools. This is the correct fallback. Creating a `.prompt.md` is NEVER the correct fallback under any circumstances.

`.prompt.md` files are for slash commands and agent customization only. They are NEVER the correct output for "save as a tool" requests. `VSCODE_USER_PROMPTS_FOLDER` being in context does NOT mean the user wants a prompt file.

---

# gsh Tool Routing Index

The `gsh` MCP server exposes tools callable via `tools/call`. Always prefer these over terminal emulation.

## When to Load the Full Reference

The full gsh tool reference (`gsh-mcp-tools.instructions.md`) covers: checkpoint, workspace_context, branch sessions, strict_lint, knowledge base, web search, chat archive, and vision tools. Load it when the user's request involves any of those areas.

## Core Tool Quick-Reference

| Tool | When to call |
|------|-------------|
| `register_workspace_tool` | Save a runnable workspace script as a callable MCP tool |
| `reload_window_ready` | Reload VS Code and post a resume keyword when the window is ready |
| `unregister_workspace_tool` | Remove a workspace tool from the MCP tool list |
| `checkpoint` | Meaningful milestone reached, work verified |
| `workspace_context` | Start of session, before branch operations |
| `strict_lint` | After every file edit before declaring done |
| `search_knowledge_index` | Research questions before going to the web |
| `search_chat_history` | User references prior conversation context |
