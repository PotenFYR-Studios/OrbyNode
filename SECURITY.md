# Security Policy

## Supported versions

OrbyNode is pre-v1.0. Security fixes target the latest `main` branch and the
most recent tagged release. Earlier builds are not independently supported.

## Reporting a vulnerability

Report security issues privately through
[GitHub Security Advisories](https://github.com/PotenFYR-Studios/OrbyNode/security/advisories/new)
or the maintainers' private contact listed on the project page. Do not open a
public issue for an exploitable flaw.

Please include:

- affected component and commit or release,
- operating system and architecture,
- minimal reproduction or proof of concept,
- impact assessment and prerequisites,
- suggested mitigation, if known.

You can expect an initial response within a few days. Please keep details
confidential until a coordinated disclosure date.

## Security model

OrbyNode grants terminal access. Treat any account with terminal write as
having remote-code-execution capability on the host. The daemon binds
`127.0.0.1` by default; non-loopback binding is opt-in and must be paired with
authentication plus HTTPS or a trusted overlay.

Every REST request, WebSocket subscription, terminal operation, file operation
and remote-node action is authorized server-side.

## Hardening controls

- Strict CSP, `nosniff`, frame denial, `no-referrer` and restrictive
  permissions-policy headers apply to every response.
- Passwords use Argon2id with unique salts and no plaintext storage.
- Sessions are server-side, HttpOnly and SameSite-protected.
- State-changing browser requests require a per-session CSRF token.
- Login is throttled and accounts are temporarily locked after repeated
  failures. Unknown users and bad passwords produce the same response.
- API-token secrets are shown once and stored only as SHA-256 hashes.
- Remote-node pairing secrets are hashed and pairing codes expire.
- File access canonicalizes paths and rejects traversal or symlinks that
  escape an authorized project root.
- Service previews are restricted to loopback destinations.
- Realtime client queues and replay rings are bounded; slow clients cannot
  block producers or affect other clients.

## Threat model

| Threat | Mitigation |
| --- | --- |
| Stolen session | HttpOnly SameSite cookie, CSRF on mutations, session revocation. |
| Malicious LAN user | Localhost default; non-loopback requires HTTPS/trusted overlay. |
| CSRF | SameSite cookie plus per-session header verification. |
| XSS and terminal-output injection | React escaping and strict CSP; terminal bytes remain data. |
| WebSocket hijacking | Authenticated upgrade and server-side subscription checks. |
| Brute-force login | Failure throttling and temporary account lockout. |
| Path traversal and symlink abuse | Canonical root containment for project files. |
| Malicious repository | File/Git access remains inside the selected project. |
| Malicious plugin | Manifests disabled by default; no arbitrary plugin execution. |
| Compromised node | Per-node identity, hashed pairing secrets, revocation. |
| Reverse-proxy spoofing | Identity comes from server-side sessions, not headers. |
| Secret leakage | Secrets hashed, shown once, never listed, never logged. |

## Data handling

Durable authentication, project, membership, task, terminal metadata, workflow
and audit records are stored in the local SQLite database configured by the
operator. OrbyNode has no mandatory cloud service or telemetry.

## Scope

In scope:

- daemon and background services,
- HTTP and WebSocket APIs,
- web UI and embedded assets,
- authentication, authorization and audit,
- file, Git, workflow and remote-node handling,
- release and update artifacts.

Out of scope:

- the host OS configuration,
- vulnerabilities requiring local root access,
- social engineering of an authorized host user,
- missing v1.0 features explicitly documented as deferred.

## Security updates

Security fixes are announced through GitHub releases and advisories. For
product updates, verify release checksums and Cosign signatures before
installation.
