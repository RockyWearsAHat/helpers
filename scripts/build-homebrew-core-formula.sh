#!/usr/bin/env bash
# Generate a homebrew-core-style FROM-SOURCE formula for submitting helpers to
# Homebrew/homebrew-core. Unlike the binary tap formula (build-pkg-manifests.sh),
# homebrew-core builds its own bottles from source, so this formula compiles the
# native crate with cargo instead of downloading a prebuilt binary.
#
#   usage: build-homebrew-core-formula.sh [version] [out-file]
#
# Defaults: version from ./VERSION, out-file dist/homebrew-core/helpers.rb.
# Downloads the tagged source tarball to compute its SHA-256, so the tag
# (v<version>) must already exist on GitHub.
set -euo pipefail

VERSION="${1:-$(tr -d '\n' < VERSION | xargs)}"
OUT="${2:-dist/homebrew-core/helpers.rb}"
SLUG="RockyWearsAHat/helpers"
URL="https://github.com/${SLUG}/archive/refs/tags/v${VERSION}.tar.gz"

mkdir -p "$(dirname "$OUT")"
tmp="$(mktemp)"
curl -fsSL "$URL" -o "$tmp"
if command -v sha256sum >/dev/null 2>&1; then SHA="$(sha256sum "$tmp" | awk '{print $1}')"; else SHA="$(shasum -a 256 "$tmp" | awk '{print $1}')"; fi
rm -f "$tmp"

cat > "$OUT" <<RB
class Helpers < Formula
  desc "AI-agent tooling (MCP server + CLI) as a single native binary"
  homepage "https://github.com/${SLUG}"
  url "${URL}"
  sha256 "${SHA}"
  license "MIT"
  head "https://github.com/${SLUG}.git", branch: "main"

  depends_on "rust" => :build

  def install
    # The runtime is one crate (native/). Builds the single helpers-native binary.
    system "cargo", "install", *std_cargo_args(path: "native")

    # Expose the busybox-dispatched CLIs as symlinks to helpers-native.
    %w[helpers git-resolve git-remerge git-fucked-the-push git-initialize git-get
       git-scan-for-leaked-envs git-upload git-checkpoint git-help-i-pushed-an-env].each do |name|
      bin.install_symlink "helpers-native" => name
    end
  end

  test do
    assert_match "Helpers", shell_output("#{bin}/helpers status 2>&1")
  end
end
RB

echo "[build-homebrew-core-formula] wrote ${OUT} (v${VERSION}, sha ${SHA})"
