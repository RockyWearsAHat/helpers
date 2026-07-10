#!/usr/bin/env bash
# Generate package-manager manifests (Homebrew formula, Scoop manifest, Winget
# manifests) from the prebuilt native binaries. Run in the release-publish job,
# where the per-platform tarballs/zips and their SHA-256s are available.
#
#   usage: build-pkg-manifests.sh <version> <assets-dir> <out-dir>
#
# <assets-dir> must contain helpers-native-<tag>.tar.gz (and, for Windows,
# helpers-native-windows-*.zip). Writes:
#   <out-dir>/homebrew/helpers.rb
#   <out-dir>/scoop/helpers.json
#   <out-dir>/winget/RockyWearsAHat.Helpers.{yaml,installer.yaml,locale.en-US.yaml}
set -euo pipefail

VERSION="${1:?usage: build-pkg-manifests.sh <version> <assets-dir> <out-dir>}"
ASSETS="${2:?missing assets dir}"
OUT="${3:?missing out dir}"
REPO_SLUG="RockyWearsAHat/helpers"
BASE="https://github.com/${REPO_SLUG}/releases/download/v${VERSION}"

# SHA-256 of a file (GNU sha256sum or macOS shasum).
sha256() {
	if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | awk '{print $1}'; else shasum -a 256 "$1" | awk '{print $1}'; fi
}

mkdir -p "$OUT/homebrew" "$OUT/scoop" "$OUT/winget"

# ── Homebrew (macOS universal binary) ────────────────────────────────────────
mac_tar="$ASSETS/helpers-native-macos-universal.tar.gz"
if [ -f "$mac_tar" ]; then
	mac_sha="$(sha256 "$mac_tar")"
	cat > "$OUT/homebrew/helpers.rb" <<RB
class Helpers < Formula
  desc "AI-agent tooling (MCP server + CLI) as a single native binary"
  homepage "https://github.com/${REPO_SLUG}"
  url "${BASE}/helpers-native-macos-universal.tar.gz"
  sha256 "${mac_sha}"
  version "${VERSION}"
  license "MIT"

  depends_on "git"

  def install
    bin.install "helpers-native"
    %w[helpers git-resolve git-remerge git-fucked-the-push git-initialize git-get
       git-scan-for-leaked-envs git-upload git-checkpoint git-help-i-pushed-an-env].each do |name|
      bin.install_symlink "helpers-native" => name
    end
  end

  def caveats
    "Run 'helpers install' to register the MCP server with your AI agent."
  end

  test do
    assert_match "Helpers", shell_output("#{bin}/helpers status 2>&1")
  end
end
RB
	echo "[build-pkg-manifests] wrote homebrew/helpers.rb"
fi

# ── Scoop (Windows x64 + arm64 zips) ─────────────────────────────────────────
win64_zip="$ASSETS/helpers-native-windows-x86_64.zip"
winarm_zip="$ASSETS/helpers-native-windows-arm64.zip"
if [ -f "$win64_zip" ]; then
	win64_sha="$(sha256 "$win64_zip")"
	winarm_arch=""
	if [ -f "$winarm_zip" ]; then
		winarm_sha="$(sha256 "$winarm_zip")"
		winarm_arch=",
        \"arm64\": {
            \"url\": \"${BASE}/helpers-native-windows-arm64.zip\",
            \"hash\": \"${winarm_sha}\"
        }"
	fi
	cat > "$OUT/scoop/helpers.json" <<JSON
{
    "version": "${VERSION}",
    "description": "AI-agent tooling (MCP server + CLI) as a single native binary",
    "homepage": "https://github.com/${REPO_SLUG}",
    "license": "MIT",
    "architecture": {
        "64bit": {
            "url": "${BASE}/helpers-native-windows-x86_64.zip",
            "hash": "${win64_sha}"
        }${winarm_arch}
    },
    "bin": [
        ["helpers-native.exe", "helpers", "cli"]
    ],
    "checkver": "github",
    "autoupdate": {
        "architecture": {
            "64bit": { "url": "https://github.com/${REPO_SLUG}/releases/download/v\$version/helpers-native-windows-x86_64.zip" },
            "arm64": { "url": "https://github.com/${REPO_SLUG}/releases/download/v\$version/helpers-native-windows-arm64.zip" }
        }
    }
}
JSON
	echo "[build-pkg-manifests] wrote scoop/helpers.json"
fi

# ── Winget (portable-in-zip, x64 + arm64) ────────────────────────────────────
if [ -f "$win64_zip" ]; then
	pkgid="RockyWearsAHat.Helpers"
	rel_date="$(date -u +%Y-%m-%d)"
	win64_sha="$(sha256 "$win64_zip")"
	installers=""
	add_installer() { # arch zip
		[ -f "$2" ] || return 0
		local sha; sha="$(sha256 "$2")"
		installers="${installers}  - Architecture: $1
    InstallerUrl: ${BASE}/$(basename "$2")
    InstallerSha256: ${sha}
    NestedInstallerFiles:
      - RelativeFilePath: helpers-native.exe
        PortableCommandAlias: helpers
"
	}
	add_installer x64 "$win64_zip"
	add_installer arm64 "$winarm_zip"
	cat > "$OUT/winget/${pkgid}.installer.yaml" <<YAML
PackageIdentifier: ${pkgid}
PackageVersion: ${VERSION}
InstallerType: zip
NestedInstallerType: portable
Commands:
  - helpers
ReleaseDate: ${rel_date}
Installers:
${installers}ManifestType: installer
ManifestVersion: 1.6.0
YAML
	cat > "$OUT/winget/${pkgid}.locale.en-US.yaml" <<YAML
PackageIdentifier: ${pkgid}
PackageVersion: ${VERSION}
PackageLocale: en-US
Publisher: RockyWearsAHat
PackageName: Helpers
License: MIT
ShortDescription: AI-agent tooling (MCP server + CLI) as a single native binary.
PackageUrl: https://github.com/${REPO_SLUG}
ManifestType: defaultLocale
ManifestVersion: 1.6.0
YAML
	cat > "$OUT/winget/${pkgid}.yaml" <<YAML
PackageIdentifier: ${pkgid}
PackageVersion: ${VERSION}
DefaultLocale: en-US
ManifestType: version
ManifestVersion: 1.6.0
YAML
	echo "[build-pkg-manifests] wrote winget/${pkgid}.*.yaml"
fi
