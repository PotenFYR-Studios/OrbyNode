# Testing Guide

OrbyNode uses focused Rust tests, API-level integration tests and Bun-based
web checks. Tests live beside the code they cover.

## Required gates

Run all checks before submitting a PR:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
(cd web && bun run check)
(cd web && bun run build)
```

Equivalent full gate:

```bash
scripts/check.sh
```

## Rust test locations

| Area | File | Covers |
| --- | --- | --- |
| Realtime | `crates/realtime/src/lib.rs` | Multiplexing, replay, backpressure. |
| Realtime scale | `crates/realtime/tests/load.rs` | 100-subscriber bounded fan-out. |
| Database | `crates/database/src/lib.rs` | Migrations and repositories. |
| Auth | `crates/auth/src/lib.rs` | Passwords, sessions, throttling. |
| Terminal | `crates/terminal/src/lib.rs` | PTY lifecycle and output. |
| Files | `crates/files/src/lib.rs` | Root containment and file operations. |
| Files/Git | `crates/files/src/git.rs` | Git status and commands. |
| Worktrees | `crates/files/src/worktree.rs` | Worktree isolation. |
| Services | `crates/services/src/lib.rs` | Port discovery and snapshots. |
| Nodes | `crates/nodes/src/registry.rs` | Pairing and heartbeat. |
| Agents | `crates/agents/src/lib.rs` | Detection and state transitions. |
| Attention | `crates/agents/src/attention.rs` | Aggregation and resolution. |
| API | `crates/api/src/*.rs` | HTTP behavior and embedded UI. |
| Workflows | `crates/workflows/src/lib.rs` | Stages, approvals and cancellation. |

## Rust patterns

### Async database test

```rust
#[tokio::test]
async fn creates_project() {
    let db = orbynode_database::Db::open("sqlite::memory:").await.unwrap();
    let project = db.create_project("app", "/tmp/app").await.unwrap();
    assert_eq!(project.name, "app");
}
```

### HTTP route test

```rust
let app = crate::build_router(AppState::default());
let response = app
    .oneshot(Request::get("/health").body(Body::empty()).unwrap())
    .await
    .unwrap();
assert_eq!(response.status(), StatusCode::OK);
```

### Authenticated route test

1. Create an isolated state or test database.
2. Create or sign in as a known user.
3. Extract the returned session token and CSRF token.
4. Send cookie plus CSRF header.
5. Assert status and body.

### Temporary filesystem test

Use a unique temporary directory and clean it after the assertion. File tests
must verify traversal and symlink containment behavior.

### PTY test

PTY tests spawn short-lived shell processes and assert stream, list and
termination behavior. Keep output and timeout bounds explicit.

## Web checks

```bash
(cd web && bun run check)
(cd web && bun run build)
```

Current web gate is strict TypeScript and Vite build. Component and interaction
tests may be added without introducing Next.js.

## Documentation site

The public docs site has Bun tests for its content manifest, routes, link
normalization and prerender helpers. Its production build renders every route
to HTML and fails when a Markdown source or installer artifact is missing.

```bash
(cd docs && bun test)
(cd docs && bun run typecheck)
(cd docs && bun run build)
```

## Security tests

Every security-sensitive change should cover:

- unauthenticated rejection,
- forbidden role rejection,
- CSRF failure,
- path traversal and symlink escape,
- secret non-disclosure,
- optimistic-concurrency conflict,
- bounded queue or replay behavior.

## Windows and macOS considerations

- Do not assume Unix-only path canonicalization in shared tests.
- Gate Unix-only symlink creation behind `#[cfg(unix)]`.
- Prefer `std::process::Command` portability over shell-specific syntax.
- Workflow commands use `sh` on Unix and `cmd /C` on Windows.

## Performance test

`crates/realtime/tests/load.rs` publishes a bounded burst to 100 subscribers
and asserts delivery plus a five-second budget. Increase scale only when a
repeatable end-to-end harness exists.

## Manual smoke checks

1. Start daemon.
2. Complete `/setup`.
3. Verify `/health` and `/version`.
4. Add a project.
5. Create a terminal and type into it.
6. Reload browser; verify terminal reconnects.
7. Add a task and create a worktree.
8. Inspect Git status and diff.
9. Check Attention Center.
10. Restart daemon; verify durable metadata.
