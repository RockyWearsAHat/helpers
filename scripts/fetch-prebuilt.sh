#!/usr/bin/env bash
# Node-free bootstrap: download the prebuilt `helpers-native` binary for THIS host
# from the GitHub release, install it, symlink the `helpers`/`git-*` CLIs to it,
# and register the MCP server via the (Rust) `helpers install`. No Node, no source,
# no build toolchain. Used by Helpers-Installer.sh and every package's postinstall.
#
#   usage: fetch-prebuilt.sh <bin-dir> [version] [--register]
#
#   <bin-dir>   where to place the binary + symlinks (e.g. ~/bin, /usr/bin)
#   [version]   release version to fetch (default: latest published)
#   --register  also run `helpers install --agent auto` after placing the binary
set -euo pipefail

REPO_SLUG="RockyWearsAHat/github-shell-helpers"
BIN_DIR="${1:?usage: fetch-prebuilt.sh <bin-dir> [version] [--register]}"
VERSION="${2:-}"
REGISTER="no"
for a in "$@"; do [ "$a" = "--register" ] && REGISTER="yes"; done
[ "${VERSION:-}" = "--register" ] && VERSION=""

# ── detect the release target tag for this host ──────────────────────────────
detect_tag() {
	local os arch libc
	os="$(uname -s 2>/dev/null || echo unknown)"
	arch="$(uname -m 2>/dev/null || echo unknown)"
	case "$arch" in
	x86_64 | amd64) arch="x86_64" ;;
	arm64 | aarch64) arch="aarch64" ;;
	esac
	case "$os" in
	Darwin) echo "macos-universal" ;;
	Linux)
		libc="gnu"
		if [ -f /etc/alpine-release ] || (ldd --version 2>&1 | grep -qi musl); then libc="musl"; fi
		if [ "$libc" = "musl" ]; then echo "linux-${arch}-musl"; else echo "linux-${arch}"; fi
		;;
	MINGW* | MSYS* | CYGWIN* | Windows_NT) echo "windows-${arch}" ;;
	*) echo "" ;;
	esac
}

# ── resolve the latest published version when none was given ──────────────────
latest_version() {
	local hdr=("-H" "Accept: application/vnd.github+json")
	[ -n "${GITHUB_TOKEN:-${GH_TOKEN:-}}" ] && hdr+=("-H" "Authorization: Bearer ${GITHUB_TOKEN:-$GH_TOKEN}")
	curl -fsSL "${hdr[@]}" "https://api.github.com/repos/${REPO_SLUG}/releases?per_page=30" 2>/dev/null |
		grep -oE '"tag_name":[[:space:]]*"v[^"]+"' | sed -E 's/.*"v([^"]+)".*/\1/' |
		sort -t. -k1,1n -k2,2n -k3,3n | tail -n 1
}

TAG="$(detect_tag)"
if [ -z "$TAG" ]; then
	echo "[fetch-prebuilt] No prebuilt for $(uname -s)/$(uname -m). Build from source: helpers build --from-source" >&2
	exit 3
fi
if [ -z "$VERSION" ]; then VERSION="$(latest_version || true)"; fi

EXE=""
case "$TAG" in windows-*) EXE=".exe" ;; esac
asset="helpers-native-${TAG}.tar.gz"
urls=()
[ -n "$VERSION" ] && urls+=("https://github.com/${REPO_SLUG}/releases/download/v${VERSION}/${asset}")
urls+=("https://github.com/${REPO_SLUG}/releases/latest/download/${asset}")

tmp="$(mktemp -d "${TMPDIR:-/tmp}/helpers-bin.XXXXXX")"
trap 'rm -rf "$tmp"' EXIT
got="no"
for url in "${urls[@]}"; do
	if curl -fsSL -o "$tmp/bin.tar.gz" "$url" 2>/dev/null; then got="yes"; break; fi
done
if [ "$got" != "yes" ]; then
	echo "[fetch-prebuilt] Could not download $asset (tried v${VERSION:-?} and latest)." >&2
	exit 1
fi
( cd "$tmp" && tar -xf bin.tar.gz )
[ -f "$tmp/helpers-native${EXE}" ] || { echo "[fetch-prebuilt] archive missing helpers-native${EXE}" >&2; exit 1; }

mkdir -p "$BIN_DIR"
install -m 0755 "$tmp/helpers-native${EXE}" "$BIN_DIR/helpers-native${EXE}" 2>/dev/null ||
	{ cp "$tmp/helpers-native${EXE}" "$BIN_DIR/helpers-native${EXE}" && chmod 0755 "$BIN_DIR/helpers-native${EXE}"; }

# Symlink the busybox CLIs (helpers + git-*) to the one binary.
for name in helpers git-resolve git-remerge git-fucked-the-push git-initialize git-get \
	git-scan-for-leaked-envs git-upload git-checkpoint git-help-i-pushed-an-env git-cs-grade; do
	ln -sf "helpers-native${EXE}" "$BIN_DIR/${name}${EXE}" 2>/dev/null ||
		cp "$BIN_DIR/helpers-native${EXE}" "$BIN_DIR/${name}${EXE}" 2>/dev/null || true
done

echo "[fetch-prebuilt] Installed helpers-native (${TAG}) + CLI symlinks to ${BIN_DIR}"

if [ "$REGISTER" = "yes" ]; then
	"$BIN_DIR/helpers${EXE}" install --agent auto ||
		echo "[fetch-prebuilt] 'helpers install' reported an issue — run 'helpers doctor'." >&2
fi
