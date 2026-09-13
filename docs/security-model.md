# Security Model

Technical security posture for OrbyNode. For disclosure instructions, see
[../SECURITY.md](../SECURITY.md).

## Trust boundaries

```text
Untrusted browser
      |
Authenticated HTTP/WebSocket
      v
Axum middleware + project/global RBAC
      v
Daemon-owned PTYs, files, services and remote-node APIs
      v
Operating system and authorized project roots
```

Terminal write is remote code execution. Authorization and process ownership
remain server-side.

## Identity and sessions

- Passwords use Argon2id with unique salts.
- Sessions are server-side database records with random tokens.
- Browser cookies are HttpOnly and SameSite-protected.
- Sessions slide and expire; logout deletes the server record.
- Unknown users and bad passwords produce identical failures.
- Failed attempts trigger lockout.

## CSRF

Mutating browser requests require `X-Orbynode-CSRF` in addition to the session
cookie. The token is per session and checked before authorization-sensitive
handlers continue.

## Authorization

Global roles:

- **Owner**: full control; cannot remove the final Owner.
- **Administrator**: administration within configured scope.
- **Operator**: terminal/agent and service control.
- **Developer**: authorized project work.
- **Viewer**: read-only authorized resources.

Project membership maps to explicit permissions. Authorization is revalidated
for REST, WebSocket subscriptions, terminal writes, file access and remote-node
actions.

## Realtime

The multiplexed gateway authenticates clients before subscriptions. Streams
are checked server-side. Event sequences enable replay and exact convergence.
Replay rings and client queues are bounded; droppable traffic may overflow and
require resync, while critical attention and security events are protected.

## Terminal safety

PTY output is treated as data. React escapes rendered text and CSP limits
script origins. Terminal writes require explicit permission. PTYs are owned by
the daemon and are never recreated by client reconnect.

## Files, Git and worktrees

File APIs canonicalize relative paths under an authorized project root.
Traversal and symlink escape are rejected. Git operations are invoked in the
project repository. Worktrees are constrained to the configured worktree base.

## Service previews

Preview proxying is loopback-only. The daemon does not proxy arbitrary hosts.

## Secrets

| Secret | Handling |
| --- | --- |
| Passwords | Argon2id hash. |
| Sessions | Random server-side token; database-backed. |
| CSRF | Random per-session value. |
| API tokens | SHA-256 hash; creation-time secret shown once. |
| Webhooks | Write-only secret; never returned by list APIs. |
| Node pairing | Short-lived hashed code and hashed node secret. |
| Configuration | No mandatory cloud secret store or telemetry. |

Secrets are never logged.

## Plugins and MCP

MCP manifests expose a small versioned tool surface. Plugin manifests are
disabled by default. Arbitrary plugin execution is not enabled; isolation and
capability enforcement are prerequisites.

## Release integrity

Release artifacts provide locked dependency builds, SHA-256 checksums, SBOMs
and Cosign signatures. Updates must verify identity and integrity before
replacement and preserve durable state. Clean-machine install, upgrade and
uninstall tests are v1.0 gates.

## Threat model

| Threat | Position |
| --- | --- |
| Stolen browser session | Revoke session; cookie hardening and CSRF reduce reuse. |
| Malicious LAN user | Localhost default; TLS/trusted overlay for non-loopback. |
| XSS | React escaping, strict CSP and no arbitrary script origins. |
| Terminal-output injection | Output is data, not executable HTML. |
| WebSocket hijacking | Authenticated upgrade and subscription authorization. |
| Brute force | Throttling and temporary lockout. |
| Malicious repository | Project-root containment; agent execution still explicit. |
| Malicious plugin | No arbitrary plugin execution in current baseline. |
| Compromised remote node | Revoke node identity and rotate access. |
| Reverse-proxy spoofing | Identity derives from server session, not headers. |
| Secret leakage | Hash storage, one-time disclosure and no secret logging. |

## Known limitations

- Non-loopback operation requires operator-provided TLS or trusted networking.
- Agent detection is behavioral and cannot guarantee native agent semantics for
  every CLI.
- Plugin execution is intentionally unavailable until isolation is complete.
- Service discovery is best-effort and platform-dependent.
- Clean-machine installer and updater tests are still v1.0 gates.
