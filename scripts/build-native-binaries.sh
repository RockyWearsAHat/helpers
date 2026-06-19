#!/usr/bin/env bash
# Build the prebuilt `helpers-native` binary for ONE release target tag and pack
# it as dist/helpers-native-<tag>.tar.gz. These tarballs are attached to the
# GitHub release; `helpers build` downloads the matching one so installs need no
# Rust toolchain. git-cs-grade and the git-* CLIs are folded into this one binary.
#
# Plain `cargo build --target` + cross-linkers are used (NO Docker/`cross`), so
# the native crate's path-dependency on ../cs-grade resolves with normal
# filesystem access. The toolchains each tag needs are installed by the CI job
# (see .github/workflows/build-installer.yml: build-natives).
#
#   usage: scripts/build-native-binaries.sh <tag>
#   tags:  macos-universal linux-x86_64 linux-aarch64 linux-x86_64-musl
#          windows-x86_64 windows-arm64
set -euo pipefail

tag="${1:?usage: build-native-binaries.sh <tag>}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
OUT="${OUT_DIR:-dist}"
mkdir -p "$OUT"
stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT
MANIFEST="native/Cargo.toml"

# Build the native crate for one rust triple (adding the target if missing).
build_triple() {
	rustup target add "$1" >/dev/null 2>&1 || true
	cargo build --release --manifest-path "$MANIFEST" --target "$1"
}

case "$tag" in
macos-universal)
	build_triple aarch64-apple-darwin
	build_triple x86_64-apple-darwin
	lipo -create -output "$stage/helpers-native" \
		"native/target/aarch64-apple-darwin/release/helpers-native" \
		"native/target/x86_64-apple-darwin/release/helpers-native"
	;;
linux-x86_64)
	build_triple x86_64-unknown-linux-gnu
	cp "native/target/x86_64-unknown-linux-gnu/release/helpers-native" "$stage/"
	;;
linux-aarch64)
	export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER="aarch64-linux-gnu-gcc"
	export CC_aarch64_unknown_linux_gnu="aarch64-linux-gnu-gcc"
	build_triple aarch64-unknown-linux-gnu
	cp "native/target/aarch64-unknown-linux-gnu/release/helpers-native" "$stage/"
	;;
linux-x86_64-musl)
	export CC_x86_64_unknown_linux_musl="musl-gcc"
	build_triple x86_64-unknown-linux-musl
	cp "native/target/x86_64-unknown-linux-musl/release/helpers-native" "$stage/"
	;;
windows-x86_64)
	build_triple x86_64-pc-windows-gnu
	cp "native/target/x86_64-pc-windows-gnu/release/helpers-native.exe" "$stage/"
	;;
windows-arm64)
	build_triple aarch64-pc-windows-msvc
	cp "native/target/aarch64-pc-windows-msvc/release/helpers-native.exe" "$stage/"
	;;
*)
	echo "build-native-binaries.sh: unknown target tag: $tag" >&2
	exit 2
	;;
esac

tar -czf "$OUT/helpers-native-$tag.tar.gz" -C "$stage" .
echo "built $OUT/helpers-native-$tag.tar.gz"
