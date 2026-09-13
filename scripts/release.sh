#!/usr/bin/env sh
# Build a local release artifact, checksum, and dependency inventory.
set -eu

TARGET="${1:-}"
if [ -z "$TARGET" ]; then
  echo "usage: scripts/release.sh <rust-target-triple>" >&2
  exit 2
fi

cd "$(dirname "$0")/.."
bun install --cwd web
bun run --cwd web build
cargo build --release --locked -p orbynode-daemon --target "$TARGET"

BINARY="orbynode-daemon"
if printf '%s' "$TARGET" | grep -q windows; then
  BINARY="$BINARY.exe"
fi

mkdir -p dist
ARTIFACT="dist/orbynode-$TARGET.tar.gz"
tar -czf "$ARTIFACT" -C "target/$TARGET/release" "$BINARY"
if command -v sha256sum >/dev/null 2>&1; then
  sha256sum "$ARTIFACT" > "$ARTIFACT.sha256"
else
  shasum -a 256 "$ARTIFACT" > "$ARTIFACT.sha256"
fi
cargo tree --locked --workspace > "dist/orbynode-$TARGET-sbom.txt"
printf 'created %s\n' "$ARTIFACT"
