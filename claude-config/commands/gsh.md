---
description: Control Git Shell Helpers — show status, enable/disable GSH, or toggle individual tools
argument-hint: "[status|enable|disable|bypass|tool list|tool disable <name>|tool enable <name>|doctor]"
allowed-tools: Bash(gsh:*)
---

Run the `gsh` control CLI to manage Git Shell Helpers, then briefly report the result.

Requested action: `$ARGUMENTS` (default to `status` if empty).

Mapping:
- `status` / empty → `gsh status`
- `enable` → `gsh enable` (turn the whole GSH tool surface on)
- `disable` → `gsh disable` (bypass: hide all GSH tools, live)
- `bypass` → `gsh bypass` (toggle master switch)
- `doctor` → `gsh doctor`
- `tool list` → `gsh tool list`
- `tool disable <name>` → `gsh tool disable <name>`
- `tool enable <name>` → `gsh tool enable <name>` (use `all` to re-enable everything)

Toggles take effect live (the MCP server re-reads its config each request). After an
enable/disable, no restart is needed. Report the new state concisely.
