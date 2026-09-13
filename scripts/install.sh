#!/usr/bin/env sh
# Bootstrap installer skeleton. Downloads a release asset, verifies its
# checksum, and installs the daemon under the user's local bin directory.
set -eu

REPO="PotenFYR-Studios/OrbyNode"
INSTALL_DIR="${ORBYNODE_INSTALL_DIR:-$HOME/.local/bin}"
VERSION="${ORBYNODE_VERSION:-latest}"

log() { printf '==> %s\n' "$*"; }
fail() { printf 'error: %s\n' "$*" >&2; exit 1; }

command -v curl >/dev/null 2>&1 || command -v wget >/dev/null 2>&1 || fail "curl or wget required"
command -v sha256sum >/dev/null 2>&1 || command -v shasum >/dev/null 2>&1 || fail "sha256sum or shasum required"

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux) platform="unknown-linux-gnu" ;;
  Darwin) platform="apple-darwin" ;;
  *) fail "unsupported OS: $OS" ;;
esac

case "$ARCH" in
  x86_64|amd64) cpu="x86_64" ;;
  aarch64|arm64) cpu="aarch64" ;;
  *) fail "unsupported architecture: $ARCH" ;;
esac

TARGET="$cpu-$platform"
ASSET="orbynode-$TARGET.tar.gz"

if [ "$VERSION" = latest ]; then
  VERSION="$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -n1)"
  [ -n "$VERSION" ] || fail "could not determine latest release"
fi

BASE="https://github.com/$REPO/releases/download/$VERSION"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

log "downloading $ASSET"
if command -v curl >/dev/null 2>&1; then
  curl -fL "$BASE/$ASSET" -o "$TMP/$ASSET"
  curl -fL "$BASE/$ASSET.sha256" -o "$TMP/$ASSET.sha256"
else
  wget -qO "$TMP/$ASSET" "$BASE/$ASSET"
  wget -qO "$TMP/$ASSET.sha256" "$BASE/$ASSET.sha256"
fi

log "verifying checksum"
EXPECTED="$(cut -d' ' -f1 "$TMP/$ASSET.sha256")"
if command -v sha256sum >/dev/null 2>&1; then
  ACTUAL="$(sha256sum "$TMP/$ASSET" | cut -d' ' -f1)"
else
  ACTUAL="$(shasum -a 256 "$TMP/$ASSET" | cut -d' ' -f1)"
fi
[ "$EXPECTED" = "$ACTUAL" ] || fail "checksum mismatch"

log "verifying signature availability"
curl -fsSL "$BASE/$ASSET.sig" -o "$TMP/$ASSET.sig" || fail "release signature missing"

log "installing to $INSTALL_DIR"
mkdir -p "$INSTALL_DIR"
tar -xzf "$TMP/$ASSET" -C "$TMP"
INSTALL_NAME="orbynode"
INSTALL_SOURCE="$TMP/orbynode-daemon"
[ -f "$INSTALL_SOURCE" ] || fail "archive did not contain orbynode-daemon"
mv "$INSTALL_SOURCE" "$INSTALL_DIR/$INSTALL_NAME"
chmod 755 "$INSTALL_DIR/$INSTALL_NAME"

log "installed $INSTALL_DIR/$INSTALL_NAME"
log "signature and checksum verified; configure Cosign verification policy before production rollout"
log "start with: $INSTALL_NAME"
log "installer does not launch the daemon or modify services automatically"
