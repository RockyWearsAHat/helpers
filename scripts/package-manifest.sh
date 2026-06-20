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

# Shell commands still shipped as scripts. The `helpers` CLI and the git-* CLIs
# are the native Rust binary (`helpers-native`, downloaded as a prebuilt by the
# bootstrap and symlinked busybox-style), so they are NOT shipped as scripts.
helpers_core_commands() {
	printf '%s\n' \
		git-copilot-quickstart
}

# The Node-free bootstrap shipped in every package: the installer + the shared
# fetch-prebuilt helper download the native binary for the host, symlink the CLIs,
# and register the MCP server — no Node, no source tree.
helpers_bootstrap_files() {
	printf '%s\n' \
		Helpers-Installer.sh
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

# The MCP server is the native binary (`helpers-native mcp`) — no Node server,
# daemon, or C shim is shipped. Rust crate sources are not shipped either: the
# bootstrap downloads the prebuilt binary (which embeds its agent config), and
# `helpers build --from-source` clones for the niche source-build fallback.

helpers_shell_libs() {
	printf '%s\n' \
		quickstart-detect.sh \
		quickstart-models.sh
}

helpers_support_scripts() {
	printf '%s\n' \
		fetch-prebuilt.sh \
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

helpers_man_pages() {
	helpers_core_man_pages
	helpers_audit_man_pages
}