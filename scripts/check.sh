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
bun run --cwd web check

echo "==> web build"
bun run --cwd web build

echo "All checks passed."
