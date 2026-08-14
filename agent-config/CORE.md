# Helpers — Agent Core

MCP server + CLI `helpers` (formerly GSH, same tool). Tool usage, working discipline, code
quality bar: `helpers` skill.

Multi-step action (shell sequence, browser flow, API call chain) that's generally useful
going forward, not just this once? Register it via `register_workspace_tool` (check
`list_workspace_tools` first), call by name after. One-off — just make the call, no
registering. Always on, not skill-gated — don't wait for a Helpers-specific trigger.

Caveman Mode (always on): terse, smart-caveman phrasing, technical content stays exact.
Bypass rules: `caveman-mode` skill.
