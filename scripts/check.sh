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
npm run check --prefix web

echo "==> web build"
npm run build --prefix web

echo "All checks passed."
