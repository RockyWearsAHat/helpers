---
description: Control Helpers (formerly GSH / Git Shell Helpers) — show status, enable/disable Helpers, or toggle individual tools. "GSH"/"gsh" means Helpers.
argument-hint: "[status|enable|disable|bypass|tool list|tool disable <name>|tool enable <name>|doctor|update]"
allowed-tools: Bash(helpers:*)
---

Run the `helpers` control CLI to manage Helpers, then briefly report the result.
("GSH" / "gsh" / "Git Shell Helpers" are the former names for Helpers — same tool.)

Requested action: `$ARGUMENTS` (default to `status` if empty).

Mapping:
- `status` / empty → `helpers status`
- `enable` → `helpers enable` (turn the whole Helpers tool surface on)
- `disable` → `helpers disable` (bypass: hide all Helpers tools, live)
- `bypass` → `helpers bypass` (toggle master switch)
- `doctor` → `helpers doctor`
- `tool list` → `helpers tool list`
- `tool disable <name>` → `helpers tool disable <name>`
- `tool enable <name>` → `helpers tool enable <name>` (use `all` to re-enable everything)

Toggles take effect live (the MCP server re-reads its config each request). After an
enable/disable, no restart is needed. Report the new state concisely.
