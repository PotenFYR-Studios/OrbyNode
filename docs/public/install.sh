#!/usr/bin/env sh
# Bootstrap installer skeleton. Downloads a release asset, verifies its
# checksum, Cosign signature and certificate identity, and installs the daemon
# under the user's local bin directory.
set -eu

REPO="PotenFYR-Studios/OrbyNode"
INSTALL_DIR="${ORBYNODE_INSTALL_DIR:-$HOME/.local/bin}"
VERSION="${ORBYNODE_VERSION:-latest}"
# Expected certificate identity for keyless Cosign verification. Override for
# fork builds; pin before production rollouts.
IDENTITY="${ORBYNODE_IDENTITY:-PotenFYR-Studios}"
ISSUER="${ORBYNODE_ISSUER:-https://token.actions.githubusercontent.com}"

log() { printf '==> %s\n' "$*"; }
fail() { printf 'error: %s\n' "$*" >&2; exit 1; }

command -v curl >/dev/null 2>&1 || command -v wget >/dev/null 2>&1 || fail "curl or wget required"
command -v sha256sum >/dev/null 2>&1 || command -v shasum >/dev/null 2>&1 || fail "sha256sum or shasum required"

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux)
    platform="unknown-linux-gnu"
    # musl systems (Alpine, Void) report themselves through ldd; the gnu
    # binary would not run there.
    case "$(ldd --version 2>&1)" in
      *musl*) platform="unknown-linux-musl" ;;
    esac
    ;;
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

# Override to serve release assets from a mirror or a local test server.
BASE="${ORBYNODE_BASE_URL:-https://github.com/$REPO/releases/download/$VERSION}"
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

log "verifying signature and certificate identity"
SIG_OK=0
if command -v cosign >/dev/null 2>&1; then
  curl -fsSL "$BASE/$ASSET.sig" -o "$TMP/$ASSET.sig" || fail "release signature missing"
  curl -fsSL "$BASE/$ASSET.pem" -o "$TMP/$ASSET.pem" || fail "release certificate missing"
  if cosign verify-blob --certificate "$TMP/$ASSET.pem" --signature "$TMP/$ASSET.sig" \
      --certificate-identity-regexp "https://github.com/$IDENTITY/\\.github/workflows/release\\.yml@.*" \
      --certificate-oidc-issuer "$ISSUER" \
      "$TMP/$ASSET" >/dev/null 2>&1; then
    SIG_OK=1
  else
    [ "${ORBYNODE_ALLOW_UNSIGNED:-0}" = "1" ] || fail "signature or certificate identity verification failed"
    log "WARNING: continuing with failed signature verification (ORBYNODE_ALLOW_UNSIGNED=1)"
  fi
else
  curl -fsSL "$BASE/$ASSET.sig" -o "$TMP/$ASSET.sig" || fail "release signature missing"
  log "cosign not found; verified signature presence only. Install cosign for full verification."
fi

log "installing to $INSTALL_DIR"
mkdir -p "$INSTALL_DIR"
tar -xzf "$TMP/$ASSET" -C "$TMP"
# Stash the bundled uninstaller so upgrades/clean removal stay self-contained.
if [ -f "$TMP/uninstall.sh" ]; then
  mkdir -p "$INSTALL_DIR/../share/orbynode"
  mv "$TMP/uninstall.sh" "$INSTALL_DIR/../share/orbynode/uninstall.sh"
  chmod 755 "$INSTALL_DIR/../share/orbynode/uninstall.sh"
fi
INSTALL_NAME="orbynode"
INSTALL_SOURCE="$TMP/orbynode-daemon"
[ -f "$INSTALL_SOURCE" ] || fail "archive did not contain orbynode-daemon"
mv "$INSTALL_SOURCE" "$INSTALL_DIR/$INSTALL_NAME"
chmod 755 "$INSTALL_DIR/$INSTALL_NAME"

log "installed $INSTALL_DIR/$INSTALL_NAME"
if [ "$SIG_OK" = "1" ]; then
  log "signature, checksum and certificate identity verified (identity: $IDENTITY)"
else
  log "checksum verified; signature presence checked"
fi
log "start with: $INSTALL_NAME"
log "installer does not launch the daemon or modify services automatically"
