#!/usr/bin/env bash
# lib/env-git-ops.sh — Git operations for git-help-i-pushed-an-env
# Provides: check_git_repo, get_repo_root, check_dependencies, check_push_access,
#           github_file_link, github_commit_link, get_github_repos

check_git_repo() {
	if ! git rev-parse --is-inside-work-tree &>/dev/null; then
		log_error "Not inside a git repository"
		exit 2
	fi
}

get_repo_root() {
	git rev-parse --show-toplevel
}

check_dependencies() {
	local missing=()

	if ! command -v git-filter-repo &>/dev/null; then
		log_warn "git-filter-repo not found. Will use git-filter-branch (slower)."
		log_info "Consider installing: pip install git-filter-repo"
	fi

	if [ "$all_public" = true ] || [ "$all_repos" = true ]; then
		if ! command -v gh &>/dev/null; then
			log_error "GitHub CLI (gh) is required for --all-public and --all-repos"
			log_info "Install with: brew install gh"
			exit 2
		fi
		if ! gh auth status &>/dev/null; then
			log_error "GitHub CLI not authenticated. Run: gh auth login"
			exit 2
		fi
	fi

	if [ "$run_review" = true ]; then
		if ! command -v git-scan-for-leaked-envs &>/dev/null; then
			local script_dir
			script_dir="$(dirname "$(realpath "$0")")"
			if [ ! -x "$script_dir/git-scan-for-leaked-envs" ]; then
				log_error "git-scan-for-leaked-envs not found. Required for --review flag."
				exit 2
			fi
		fi
	fi
}

check_push_access() {
	typeset repo="$1"

	typeset current_user=""
	current_user=$(gh api user --jq '.login' 2>/dev/null) || current_user=""

	if [ -z "$current_user" ]; then
		echo "unknown"
		return
	fi

	typeset repo_owner="${repo%%/*}"
	if [ "$repo_owner" = "$current_user" ]; then
		echo "owner"
		return
	fi

	typeset permission=""
	permission=$(gh api "repos/$repo/collaborators/$current_user/permission" --jq '.permission' 2>/dev/null) || permission=""

	case "$permission" in
		admin|write|maintain)
			echo "collaborator"
			;;
		read|"")
			echo "readonly"
			;;
		*)
			echo "readonly"
			;;
	esac
}

github_file_link() {
	local repo="$1"
	local filepath="$2"
	local commit="${3:-HEAD}"

	local encoded_path
	encoded_path=$(printf '%s' "$filepath" | /usr/bin/sed 's/ /%20/g; s/#/%23/g')

	if [ "$commit" = "HEAD" ]; then
		echo "https://github.com/$repo/blob/HEAD/$encoded_path"
	else
		echo "https://github.com/$repo/blob/$commit/$encoded_path"
	fi
}

github_commit_link() {
	local repo="$1"
	local commit="$2"
	echo "https://github.com/$repo/commit/$commit"
}

get_github_repos() {
	local visibility="$1"
	case "$visibility" in
		public)
			gh repo list --visibility public --limit 1000 --json nameWithOwner --jq '.[].nameWithOwner'
			;;
		private)
			gh repo list --visibility private --limit 1000 --json nameWithOwner --jq '.[].nameWithOwner'
			;;
		all)
			gh repo list --limit 1000 --json nameWithOwner --jq '.[].nameWithOwner'
			;;
	esac
}
