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
if command -v bun >/dev/null 2>&1; then
  (cd web && bun run check)
else
  npm run check --prefix web
fi

echo "==> web build"
if command -v bun >/dev/null 2>&1; then
  (cd web && bun run build)
else
  npm run build --prefix web
fi

if ! command -v bun >/dev/null 2>&1; then
  echo "Bun is required for docs checks." >&2
  exit 1
fi

echo "==> docs test"
(cd docs && bun test)

echo "==> docs typecheck"
(cd docs && bun run typecheck)

echo "==> docs build"
(cd docs && bun run build)

echo "All checks passed."
