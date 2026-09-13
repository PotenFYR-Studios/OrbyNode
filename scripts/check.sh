#!/usr/bin/env sh
# Full local check: same gates as CI.
set -e
cd "$(dirname "$0")/.."

echo "==> cargo fmt"
cargo fmt --check

echo "==> cargo clippy"
cargo clippy --workspace --all-targets -- -D warnings

echo "==> cargo test"
cargo test --workspace

echo "==> web typecheck"
(cd web && bun run check)

echo "==> web build"
(cd web && bun run build)

echo "All checks passed."
