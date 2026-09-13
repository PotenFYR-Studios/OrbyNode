# Security Policy

## Reporting a vulnerability

Report security issues to the maintainers privately (GitHub Security Advisories
on this repository, or the contact listed on the project page). Do not open a
public issue for an exploitable flaw.

Please include: affected component, reproduction steps, impact assessment, and
any suggested mitigation. You can expect an initial response within a few days.

## Security model (short version)

OrbyNode grants terminal access - treat any account with terminal write as
remote-code-execution capability on the host. Design rules that follow from
that (full list: `Plan.md` §16, non-negotiables §151):

- The daemon binds `127.0.0.1` by default. LAN/WAN binding (`0.0.0.0`) is
  opt-in and must be paired with authentication.
- Every REST request, WebSocket subscription, and file operation is
  authorized server-side.
- Passwords use Argon2id; sessions are HttpOnly cookies; no plaintext secrets
  in logs or diagnostics.
- No permissive CORS (`Access-Control-Allow-Origin: *` is never combined with
  credentials).
- Updates and installers are signed once release engineering lands (M20).

## Hardening controls

- Every response carries strict CSP, `nosniff`, frame denial, no-referrer, and
  restrictive permissions-policy headers.
- State-changing browser requests require the per-session CSRF token in
  addition to the HttpOnly session cookie.
- Login is throttled and accounts are temporarily locked after repeated
  failures; authentication failures do not distinguish unknown users from bad
  passwords.
- API-token secrets are displayed once and stored only as SHA-256 hashes.
- Remote node pairing secrets are hashed and pairing codes expire.
- File access canonicalizes paths and rejects traversal or symlinks that
  escape the project root.
- Service previews are restricted to loopback destinations.

## Threat model

- **Stolen session:** HttpOnly, SameSite cookies; CSRF tokens on mutations;
  revoke sessions immediately on suspicion.
- **Malicious LAN user:** localhost binding is default. Non-loopback binding
  requires HTTPS or a trusted overlay.
- **XSS and terminal-output injection:** React escapes text and strict CSP
  limits script and frame origins; terminal bytes remain terminal data.
- **WebSocket hijacking:** realtime requires the authenticated session and
  server-side subscription authorization.
- **Reverse-proxy spoofing:** authorization is server-side and does not derive
  identity from proxy headers.
- **Malicious plugin:** manifests are disabled by default; execution is not
  enabled until capability enforcement lands.
- **Compromised node:** per-node identity, revocation, hashed secrets, and
  expiring pairing codes limit reuse.
- **Secret leakage:** API and node secrets are hashed; webhook secrets are
  write-only.

## Scope

The daemon, its API surface, the web UI, and the installers are in scope.
The underlying machines' OS configuration is not.
