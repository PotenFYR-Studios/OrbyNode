# Contributing to OrbyNode

Thanks for helping build OrbyNode. Keep changes milestone-scoped (see
[ROADMAP.md](ROADMAP.md)); substantial architectural decisions go through an
[ADR](docs/adr/) first.

## Code of conduct

Be respectful, precise and constructive. Assume good intent, review code rather
than people, and keep discussions focused on user impact, correctness,
security and maintainability.

## Development setup

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

## Project layout

| Path | Purpose |
| --- | --- |
| `crates/core` | Small shared configuration and path types. |
| `crates/terminal` | Real PTY lifecycle and bounded output fan-out. |
| `crates/realtime` | Event bus, replay, subscriptions and backpressure. |
| `crates/database` | SQLite migrations and durable repositories. |
| `crates/auth` | Argon2id, sessions, throttling and permissions. |
| `crates/api` | Axum routes, middleware, gateway and embedded UI. |
| `web` | Vite + React + TypeScript + Bun + Magic UI client. |
| `docs/adr` | Architecture decision records. |

## Workflow

1. Open or assign an issue and keep the change focused.
2. Create a short-lived branch from `main`.
3. Build the web UI so the daemon embeds real assets:
   ```bash
   (cd web && bun run build)
   ```
4. Run the daemon:
   ```bash
   cargo run -p orbynode-daemon
   ```
   For frontend iteration, use:
   ```bash
   ORBYNODE_STATIC_DIR=web/dist cargo run -p orbynode-daemon
   ```
5. Add or update tests next to the code they cover.
6. Update documentation and an ADR for material behavior or architecture.
7. Run every check and commit with a focused, imperative message.

## Checks

All checks must pass locally and in CI:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
(cd web && bun run check)
(cd web && bun run build)
(cd docs && bun test)
(cd docs && bun run typecheck)
(cd docs && bun run build)
```

You can run the same gates with:

```bash
scripts/check.sh
```

## Frontend standards

Use **Vite + React + TypeScript + Bun + Magic UI**. Do not use Next.js or
another React meta-framework. Keep UI state derived from daemon snapshots and
realtime events; do not introduce polling.

## Commit and pull requests

- Use focused, imperative commit subjects, for example `Fix terminal replay`.
- Explain user-visible behavior, risks and test coverage in the body.
- Keep pull requests small and reviewable.
- Do not add AI attribution to commits, pull requests, tags or releases.
- Do not modify Git identity configuration.

## Issue reports

Include:

- OrbyNode commit or release,
- operating system and architecture,
- minimal reproduction,
- expected and actual behavior,
- relevant logs with secrets redacted.

## Security

Never weaken authentication, authorization, path containment, CSP or bounded
buffers to make a change easier. For vulnerabilities, do not open a public
issue. Follow [SECURITY.md](SECURITY.md).

## License

By contributing, you agree that contributions are licensed under
[Apache-2.0 with the Commons Clause](LICENSE).
