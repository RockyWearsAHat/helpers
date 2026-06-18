#!/usr/bin/env bash

# package-manifest.sh
#
# Usage:
#   source ./scripts/package-manifest.sh
#
# Description:
#   Shared release manifest for installers and package-manager builds.
#   Scripts source these functions to keep shipped file lists aligned.
#
# Options:
#   None.
#
# Examples:
#   source ./scripts/package-manifest.sh
#   helpers_core_commands
#   helpers_man_pages

set -euo pipefail

# Shell commands still shipped as scripts. The git-* CLIs ported to Rust
# (git-upload, git-get, git-initialize, git-fucked-the-push, git-remerge,
# git-resolve, git-scan-for-leaked-envs, git-checkpoint, git-help-i-pushed-an-env)
# are built by `helpers build` as symlinks to the helpers-native binary, not copied here.
helpers_core_commands() {
	printf '%s\n' \
		helpers \
		git-cs-grade.js \
		git-copilot-quickstart
}

# Community-cache knowledge-sharing commands (the AI audit orchestrator was
# removed; these submit/pull community research and remain part of the knowledge
# subsystem).
helpers_audit_commands() {
	printf '%s\n' \
		git-copilot-devops-audit-community-pull \
		git-copilot-devops-audit-community-submit \
		git-copilot-devops-audit-community-research-submit
}

helpers_mcp_commands() {
	printf '%s\n' \
		git-research-mcp \
		git-research-mcp.js \
		helpers-server \
		helpers-server.js \
		helpers-serverd.js
}

# Non-executable support files copied verbatim (no chmod +x) into the staged
# bin/ tree — e.g. the fast C launcher source, compiled by `helpers build`.
helpers_support_files() {
	printf '%s\n' \
		helpers-mcp.c
}

# Rust crate sources (sources only — never the build target/). `helpers build`,
# run by the installer after the tree is staged, compiles these into the
# helpers-native binary (MCP tools + ported git-* CLIs) and git-cs-grade.
helpers_crate_dirs() {
	printf '%s\n' \
		native \
		cs-grade
}

helpers_shell_libs() {
	printf '%s\n' \
		quickstart-detect.sh \
		quickstart-models.sh
}

helpers_mcp_libs() {
	printf '%s\n' \
		mcp-activity-ipc.js \
		mcp-git.js \
		mcp-google-headless.js \
		mcp-native.js \
		mcp-pdf-extract.js \
		mcp-research-tools.js \
		mcp-research.js \
		mcp-utils.js \
		mcp-web-search.js
}

helpers_support_scripts() {
	printf '%s\n' \
		build-knowledge-index.js \
		patch-vscode-apply-all.js \
		patch-vscode-argv.js \
		patch-vscode-folder-switch.js \
		patch-vscode-git-head-display.js \
		patch-vscode-runsubagent-model.js \
		community-cache-pull.sh \
		community-cache-submit.sh \
		community-research-submit.sh
}

helpers_data_dirs() {
	printf '%s\n' \
		claude-config \
		copilot-config \
		community-cache \
		templates
}

helpers_core_man_pages() {
	printf '%s\n' \
		git-checkpoint.1 \
		git-copilot-quickstart.1 \
		git-fucked-the-push.1 \
		git-get.1 \
		git-help-i-pushed-an-env.1 \
		git-initialize.1 \
		git-remerge.1 \
		git-scan-for-leaked-envs.1 \
		git-upload.1
}

# The AI audit orchestrator (and its man page) were removed; no audit man pages.
helpers_audit_man_pages() {
	:
}

helpers_mcp_man_pages() {
	printf '%s\n' \
		git-research-mcp.1
}

helpers_man_pages() {
	helpers_core_man_pages
	helpers_audit_man_pages
	helpers_mcp_man_pages
}