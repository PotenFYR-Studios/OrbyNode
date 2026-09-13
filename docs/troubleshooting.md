# Troubleshooting

Common OrbyNode failures and recovery paths.

## Daemon does not start

### Port already in use

```bash
lsof -i :7676
ORBYNODE_BIND=127.0.0.1:7680 cargo run -p orbynode-daemon
```

Stop the conflicting process or choose another localhost port.

### Data directory cannot be created

Check parent-directory permissions and disk space. OrbyNode does not require
root for a per-user deployment.

### Database fails to open

1. Stop all daemon processes.
2. Check disk space and filesystem permissions.
3. Back up the entire data directory.
4. Try opening the database with SQLite tooling.
5. Restore a consistent backup if integrity checks fail.

Do not delete `orbynode.db` to “fix” startup unless the data is disposable.

## Setup and authentication

### `/setup` returns forbidden

An Owner already exists. Sign in with an existing account or use an
Administrator to create another user.

### Login returns unauthorized

- Check username and password.
- Wait out temporary lockout after repeated failures.
- Confirm the daemon has not been pointed at another data directory.

### Mutation returns forbidden for CSRF

Browser mutations require the session's `X-Orbynode-CSRF` header. Re-authenticate
or refresh your client state; do not disable CSRF.

## UI and static assets

### Placeholder page appears

`web/dist` was missing at build time. Build the frontend:

```bash
(cd web && bun install)
(cd web && bun run build)
cargo run --release -p orbynode-daemon
```

For frontend development:

```bash
ORBYNODE_STATIC_DIR="$PWD/web/dist" cargo run -p orbynode-daemon
```

### Asset returns 404

Path traversal and encoded dot segments are rejected fail-closed. Use the
hashed Vite asset path emitted in `index.html`.

## Terminals

### Terminal disappears after daemon restart

Live PTY processes are transient by design. Terminal metadata and layouts are
durable; OS processes are not. Relaunch the terminal or supported agent resume
flow.

### Terminal output stops

1. Check the PTY process still exists.
2. Reload the browser to create a fresh WebSocket.
3. Inspect daemon logs for terminal errors.
4. Terminate and recreate the terminal if the PTY is unrecoverable.

### Viewer cannot type

Expected. Terminal write requires Operator or greater and appropriate
permissions.

## Files and Git

### File operation says outside root

OrbyNode canonicalizes paths and rejects traversal or symlink escape. Check
that the project path is correct and that the requested file is inside it.

### Git command fails

- Verify Git is installed.
- Confirm repository ownership and safe-directory settings.
- Inspect the Git error returned by the API.
- Ensure branches and worktrees are not locked by another Git process.

## Services and previews

### Preview returns bad gateway

Previews target loopback ports only. Verify the service is listening on
`127.0.0.1:<port>` and still running.

### Service is not discovered

Only local listening ports visible to the daemon are discovered. Discovery is
best-effort and platform-dependent.

## Remote nodes

### Node remains pending

Complete pairing before expiry. Pairing codes and secrets are one-time and
hashed.

### Node heartbeat rejected

Check node identity, secret and revocation state. Re-pair the node if its
identity material is lost or compromised.

### Node offline

Inspect the remote daemon, network connectivity, clock and heartbeat logs.
After fixing the cause, allow the next heartbeat or reconnect.

## Workflows

### Run stays waiting for approval

Expected at an approval stage. Approve from the workflow API or UI.

### Run failed at a command stage

Inspect the step output. Command stages execute the configured command; a
non-zero exit marks the stage failed.

### Cancel returns conflict

Only active or approval-waiting runs can be cancelled.

## Performance

### Slow browser with heavy terminal output

Realtime queues are bounded. Droppable output may overflow and require
resynchronization; critical events are protected. Reload or resubscribe the
affected stream.

### High CPU or RAM

Use `/observability/host` and `/observability/sessions` to identify runaway
processes. Terminate the terminal or service intentionally.

## Logs and diagnostics

```bash
ORBYNODE_LOG_FORMAT=json RUST_LOG=debug cargo run -p orbynode-daemon
```

Redact secrets before sharing logs. OrbyNode avoids logging credentials and
token material.
