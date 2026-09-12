# Security Policy

## Reporting a vulnerability

Report security issues to the maintainers privately (GitHub Security Advisories
on this repository, or the contact listed on the project page). Do not open a
public issue for an exploitable flaw.

Please include: affected component, reproduction steps, impact assessment, and
any suggested mitigation. You can expect an initial response within a few days.

## Security model (short version)

OrbyNode grants terminal access — treat any account with terminal write as
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

## Scope

The daemon, its API surface, the web UI, and the installers are in scope.
The underlying machines' OS configuration is not.
