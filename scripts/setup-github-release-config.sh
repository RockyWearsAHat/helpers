#!/usr/bin/env bash

# setup-github-release-config.sh
#
# Usage:
#   bash ./scripts/setup-github-release-config.sh [--repo owner/name] [--env release] [--defaults-only]
#
# Description:
#   Configure GitHub Actions release variables and secrets for this repository.
#   Reads values from the current shell environment and writes them via `gh`.
#   Use `.github/release-config.example.env` as a template for a local private env file.
#   Pass `--env release` to scope them to a GitHub Environment instead of repo-wide scope.
#
# Options:
#   --repo <owner/name>  Target GitHub repository. Defaults to the current gh repo.
#   --env <name>         Store variables and secrets in the named GitHub Environment.
#   --defaults-only      Only set safe variables; do not attempt to write secrets.
#   --help               Show this help text.
#
# Examples:
#   bash ./scripts/setup-github-release-config.sh --defaults-only
#   bash ./scripts/setup-github-release-config.sh --env release --defaults-only
#   set -a; source ~/.config/gsh-release.env; set +a
#   bash ./scripts/setup-github-release-config.sh --env release

set -euo pipefail

TARGET_REPO=""
TARGET_ENVIRONMENT=""
DEFAULTS_ONLY=false
SCOPE_LABEL="repository"
declare -a SCOPE_ARGS=()

usage() {
	grep '^#' "$0" | sed 's/^# \{0,1\}//'
}

resolve_repo() {
	if [ -n "$TARGET_REPO" ]; then
		printf '%s\n' "$TARGET_REPO"
		return 0
	fi
	gh repo view --json nameWithOwner --jq '.nameWithOwner'
}

ensure_environment() {
	local repo="$1"
	local environment_name="$2"
	gh api --method PUT "repos/${repo}/environments/${environment_name}" >/dev/null
	printf '[setup-github-release-config] Ensured GitHub Environment %s exists\n' "$environment_name"
}

infer_bool() {
	local explicit="$1"
	local fallback="$2"
	if [ -n "$explicit" ]; then
		printf '%s\n' "$explicit"
	else
		printf '%s\n' "$fallback"
	fi
}

set_variable() {
	local repo="$1"
	local name="$2"
	local value="$3"
	gh variable set "$name" --repo "$repo" "${SCOPE_ARGS[@]}" --body "$value" >/dev/null
	printf '[setup-github-release-config] Set %s variable %s=%s\n' "$SCOPE_LABEL" "$name" "$value"
}

set_repo_variable() {
	local repo="$1"
	local name="$2"
	local value="$3"
	gh variable set "$name" --repo "$repo" --body "$value" >/dev/null
	printf '[setup-github-release-config] Set repository variable %s=%s\n' "$name" "$value"
}

set_secret_if_present() {
	local repo="$1"
	local name="$2"
	local value="$3"
	if [ -z "$value" ]; then
		printf '[setup-github-release-config] %s secret %s not set locally; leaving unchanged\n' "$SCOPE_LABEL" "$name"
		return 0
	fi
	printf '%s' "$value" | gh secret set "$name" --repo "$repo" "${SCOPE_ARGS[@]}" >/dev/null
	printf '[setup-github-release-config] Updated %s secret %s\n' "$SCOPE_LABEL" "$name"
}

while [ $# -gt 0 ]; do
	case "$1" in
		--repo)
			TARGET_REPO="$2"
			shift 2
			;;
		--env)
			TARGET_ENVIRONMENT="$2"
			shift 2
			;;
		--defaults-only)
			DEFAULTS_ONLY=true
			shift
			;;
		--help)
			usage
			exit 0
			;;
		*)
			printf '[setup-github-release-config] Unknown option: %s\n' "$1" >&2
			usage >&2
			exit 1
			;;
	esac
done

if ! command -v gh >/dev/null 2>&1; then
	printf '[setup-github-release-config] ERROR: gh CLI is required.\n' >&2
	exit 1
fi

gh auth status >/dev/null 2>&1 || {
	printf '[setup-github-release-config] ERROR: gh is not authenticated. Run gh auth login first.\n' >&2
	exit 1
}

REPO="$(resolve_repo)"

if [ -n "$TARGET_ENVIRONMENT" ]; then
	ensure_environment "$REPO" "$TARGET_ENVIRONMENT"
	SCOPE_ARGS=(--env "$TARGET_ENVIRONMENT")
	SCOPE_LABEL="environment:${TARGET_ENVIRONMENT}"
fi

macos_signing_default=false
if [ -n "${INSTALLER_CERT_BASE64:-}" ] && [ -n "${INSTALLER_CERT_PASSWORD:-}" ] && [ -n "${PKG_SIGN_IDENTITY:-}" ]; then
	macos_signing_default=true
fi

npm_publish_default=false
if [ -n "${NPM_TOKEN:-}" ]; then
	npm_publish_default=true
fi

homebrew_publish_default=false
if [ -n "${HOMEBREW_TAP_TOKEN:-}" ] && [ -n "${HOMEBREW_TAP_REPOSITORY:-}" ]; then
	homebrew_publish_default=true
fi

aur_publish_default=false
if [ -n "${AUR_SSH_PRIVATE_KEY:-}" ]; then
	aur_publish_default=true
fi

ENABLE_MACOS_SIGNING_VALUE="$(infer_bool "${ENABLE_MACOS_SIGNING:-}" "$macos_signing_default")"
ENABLE_NPM_PUBLISH_VALUE="$(infer_bool "${ENABLE_NPM_PUBLISH:-}" "$npm_publish_default")"
ENABLE_HOMEBREW_PUBLISH_VALUE="$(infer_bool "${ENABLE_HOMEBREW_PUBLISH:-}" "$homebrew_publish_default")"
ENABLE_AUR_PUBLISH_VALUE="$(infer_bool "${ENABLE_AUR_PUBLISH:-}" "$aur_publish_default")"
AUR_PACKAGE_NAME_VALUE="${AUR_PACKAGE_NAME:-github-shell-helpers}"
RELEASE_ENVIRONMENT_VALUE="${RELEASE_ENVIRONMENT:-${TARGET_ENVIRONMENT:-release}}"

set_repo_variable "$REPO" RELEASE_ENVIRONMENT "$RELEASE_ENVIRONMENT_VALUE"

set_variable "$REPO" ENABLE_MACOS_SIGNING "$ENABLE_MACOS_SIGNING_VALUE"
set_variable "$REPO" ENABLE_NPM_PUBLISH "$ENABLE_NPM_PUBLISH_VALUE"
set_variable "$REPO" ENABLE_HOMEBREW_PUBLISH "$ENABLE_HOMEBREW_PUBLISH_VALUE"
set_variable "$REPO" ENABLE_AUR_PUBLISH "$ENABLE_AUR_PUBLISH_VALUE"
set_variable "$REPO" AUR_PACKAGE_NAME "$AUR_PACKAGE_NAME_VALUE"

if [ -n "${HOMEBREW_TAP_REPOSITORY:-}" ]; then
	set_variable "$REPO" HOMEBREW_TAP_REPOSITORY "$HOMEBREW_TAP_REPOSITORY"
else
	printf '[setup-github-release-config] Variable HOMEBREW_TAP_REPOSITORY not set locally; leaving unchanged\n'
fi

if [ "$DEFAULTS_ONLY" = true ]; then
	printf '[setup-github-release-config] Defaults-only mode complete for %s (%s)\n' "$REPO" "$SCOPE_LABEL"
	exit 0
fi

set_secret_if_present "$REPO" INSTALLER_CERT_BASE64 "${INSTALLER_CERT_BASE64:-}"
set_secret_if_present "$REPO" INSTALLER_CERT_PASSWORD "${INSTALLER_CERT_PASSWORD:-}"
set_secret_if_present "$REPO" PKG_SIGN_IDENTITY "${PKG_SIGN_IDENTITY:-}"
set_secret_if_present "$REPO" NOTARIZE_APPLE_ID "${NOTARIZE_APPLE_ID:-}"
set_secret_if_present "$REPO" NOTARIZE_PASSWORD "${NOTARIZE_PASSWORD:-}"
set_secret_if_present "$REPO" NOTARIZE_TEAM_ID "${NOTARIZE_TEAM_ID:-}"
set_secret_if_present "$REPO" NPM_TOKEN "${NPM_TOKEN:-}"
set_secret_if_present "$REPO" HOMEBREW_TAP_TOKEN "${HOMEBREW_TAP_TOKEN:-}"
set_secret_if_present "$REPO" AUR_SSH_PRIVATE_KEY "${AUR_SSH_PRIVATE_KEY:-}"

printf '[setup-github-release-config] Release configuration complete for %s (%s)\n' "$REPO" "$SCOPE_LABEL"