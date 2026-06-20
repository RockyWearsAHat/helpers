#!/usr/bin/env bash
# Assemble an APT repository from the built .deb, laid out for static hosting
# (e.g. GitHub Pages). When a GPG private key is supplied the repository is
# signed (InRelease + Release.gpg) and the matching public keyring is exported,
# which is what `apt` requires by default.
#
#   usage: build-apt-repo.sh <deb-file> <out-dir>
#
# Env:
#   APT_GPG_PRIVATE_KEY  ASCII-armored private key. When set, the Release file is
#                        signed and <out-dir>/helpers-archive-keyring.gpg (the
#                        dearmored public key) is written for clients to trust.
#
# Clients then run:
#   sudo curl -fsSL https://<host>/helpers-archive-keyring.gpg \
#     -o /usr/share/keyrings/helpers-archive-keyring.gpg
#   echo "deb [signed-by=/usr/share/keyrings/helpers-archive-keyring.gpg] \
#     https://<host> stable main" | sudo tee /etc/apt/sources.list.d/helpers.list
#   sudo apt update && sudo apt install helpers
set -euo pipefail

DEB="${1:?usage: build-apt-repo.sh <deb-file> <out-dir>}"
OUT="${2:?missing out dir}"
SUITE="stable"
COMPONENT="main"
ARCH="all" # the .deb is Architecture: all

[ -f "$DEB" ] || { echo "build-apt-repo: no such .deb: $DEB" >&2; exit 1; }
command -v dpkg-scanpackages >/dev/null 2>&1 || { echo "build-apt-repo: dpkg-dev required" >&2; exit 1; }

OUT="$(mkdir -p "$OUT" && cd "$OUT" && pwd)"
BIN_DIR="dists/$SUITE/$COMPONENT/binary-$ARCH"
mkdir -p "$OUT/pool/$COMPONENT" "$OUT/$BIN_DIR"
cp "$DEB" "$OUT/pool/$COMPONENT/"

cd "$OUT"
# Package index over the pool.
dpkg-scanpackages --arch "$ARCH" "pool/$COMPONENT" > "$BIN_DIR/Packages"
gzip -9c "$BIN_DIR/Packages" > "$BIN_DIR/Packages.gz"

# Suite Release file with checksums of the indices.
( cd "dists/$SUITE" && apt-ftparchive \
	-o "APT::FTPArchive::Release::Origin=helpers" \
	-o "APT::FTPArchive::Release::Label=helpers" \
	-o "APT::FTPArchive::Release::Suite=$SUITE" \
	-o "APT::FTPArchive::Release::Codename=$SUITE" \
	-o "APT::FTPArchive::Release::Architectures=$ARCH" \
	-o "APT::FTPArchive::Release::Components=$COMPONENT" \
	release . > Release )

if [ -n "${APT_GPG_PRIVATE_KEY:-}" ]; then
	GNUPGHOME="$(mktemp -d)"; export GNUPGHOME
	printf '%s\n' "$APT_GPG_PRIVATE_KEY" | gpg --batch --import
	keyid="$(gpg --list-secret-keys --with-colons | awk -F: '/^sec:/{print $5; exit}')"
	[ -n "$keyid" ] || { echo "build-apt-repo: imported key has no secret key id" >&2; exit 1; }
	( cd "dists/$SUITE"
	  gpg --batch --yes --default-key "$keyid" --clearsign -o InRelease Release
	  gpg --batch --yes --default-key "$keyid" -abs -o Release.gpg Release )
	gpg --batch --yes --export "$keyid" > "$OUT/helpers-archive-keyring.gpg"
	rm -rf "$GNUPGHOME"
	echo "[build-apt-repo] signed repo + exported helpers-archive-keyring.gpg"
else
	echo "[build-apt-repo] UNSIGNED repo (set APT_GPG_PRIVATE_KEY to sign); clients need [trusted=yes]"
fi

echo "[build-apt-repo] wrote APT repo to $OUT"
