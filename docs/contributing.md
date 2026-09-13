# Contributing Guide

Thanks for contributing to OrbyNode.

## Ground rules

- Keep changes milestone-scoped.
- Preserve daemon/client separation.
- Preserve local-first security defaults.
- Do not introduce polling.
- Do not introduce unbounded queues.
- Do not weaken authorization, CSRF, CSP or path containment.
- Justify dependencies.
- Add ADRs for substantial architecture.
- Never add AI attribution to commits, PRs, tags or releases.

## Setup

Requirements:

- Rust 1.85+
- Bun 1.1+
- Node 20+ only for CI compatibility

```bash
git clone https://github.com/PotenFYR-Studios/OrbyNode.git
cd OrbyNode
(cd web && bun install)
cargo build
```

## Branches

1. Start from current `main`.
2. Use a short, lowercase name:
   - `feature/workflow-webhooks`
   - `fix/terminal-replay`
   - `docs/api-reference`
3. Keep one logical change per pull request.

## Workflow

1. Update or create an issue.
2. Add tests before or with the fix.
3. Implement the smallest correct change.
4. Update documentation.
5. Add an ADR for material architecture.
6. Run all checks.
7. Rebase onto `main` if needed.
8. Open a focused pull request.

## Required checks

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
(cd web && bun run check)
(cd web && bun run build)
```

Equivalent:

```bash
scripts/check.sh
```

## Commit style

Use imperative subjects:

```text
Fix terminal replay after overflow
Add workflow approval route
Document realtime backpressure
```

Commit bodies should explain:

- user-visible behavior,
- security implications,
- migration needs,
- test coverage,
- remaining risks.

Do not add AI attribution or modify Git identity.

## Pull request description

Include:

1. What changed.
2. Why it changed.
3. User-visible behavior.
4. Security and compatibility notes.
5. Test evidence.
6. Documentation or ADR links.

## Code review

Reviewers prioritize:

1. correctness,
2. security,
3. bounded resource behavior,
4. cross-platform compatibility,
5. maintainability,
6. performance.

Small, focused PRs are reviewed fastest.

## Frontend

Use Vite, React, TypeScript, Bun and Magic UI. Do not use Next.js. Keep live
state realtime-derived. Do not poll.

## Documentation

Update documentation in the same PR when behavior changes:

- `README.md` for user-facing quick start,
- `ARCHITECTURE.md` for structure,
- `docs/adr/` for decisions,
- `docs/rest-api.md` for routes,
- `docs/security-model.md` for threat or hardening changes,
- `ROADMAP.md` for milestone status.

## Reporting bugs

Use GitHub Issues and include:

- commit or release,
- OS and architecture,
- minimal reproduction,
- expected versus actual behavior,
- logs with secrets removed.

## Reporting vulnerabilities

Do not open public issues. Follow [SECURITY.md](SECURITY.md).

## License

Contributions are licensed under [Apache-2.0 with the Commons Clause](LICENSE).
