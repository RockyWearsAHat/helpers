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
#   gsh_core_commands
#   gsh_man_pages

set -euo pipefail

gsh_core_commands() {
	printf '%s\n' \
		git-upload \
		git-get \
		git-initialize \
		git-checkpoint \
		git-fucked-the-push \
		git-remerge \
		git-resolve \
		git-scan-for-leaked-envs \
		git-help-i-pushed-an-env \
		git-copilot-quickstart
}

gsh_audit_commands() {
	printf '%s\n' \
		git-copilot-devops-audit \
		git-copilot-devops-audit-community-pull \
		git-copilot-devops-audit-community-submit \
		git-copilot-devops-audit-community-research-submit
}

gsh_mcp_commands() {
	printf '%s\n' \
		git-research-mcp \
		git-research-mcp.js \
		git-shell-helpers-mcp \
		git-shell-helpers-mcp.js
}

gsh_shell_libs() {
	printf '%s\n' \
		env-batch-ops.sh \
		env-cache.sh \
		env-git-ops.sh \
		env-history-clean.sh \
		env-patterns.sh \
		env-scan.sh \
		env-ui.sh \
		quickstart-detect.sh \
		quickstart-models.sh \
		upload-ai-message.sh \
		upload-diff-analysis.sh \
		upload-spinner.sh \
		upload-test-detection.sh \
		upload-test-output.sh
}

gsh_mcp_libs() {
	printf '%s\n' \
		mcp-activity-ipc.js \
		mcp-branch-sessions.js \
		mcp-checkpoint.js \
		mcp-git.js \
		mcp-google-headless.js \
		mcp-knowledge-index.js \
		mcp-knowledge-rw.js \
		mcp-language-models.js \
		mcp-pdf-extract.js \
		mcp-research-tools.js \
		mcp-research.js \
		mcp-session-memory.js \
		mcp-strict-lint.js \
		mcp-utils.js \
		mcp-web-search.js \
		mcp-workspace-context.js
}

gsh_support_scripts() {
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

gsh_data_dirs() {
	printf '%s\n' \
		copilot-config \
		community-cache \
		templates
}

gsh_core_man_pages() {
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

gsh_audit_man_pages() {
	printf '%s\n' \
		git-copilot-devops-audit.1
}

gsh_mcp_man_pages() {
	printf '%s\n' \
		git-research-mcp.1
}

gsh_man_pages() {
	gsh_core_man_pages
	gsh_audit_man_pages
	gsh_mcp_man_pages
}