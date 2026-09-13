# ADR 005 - Authentication / Session Model

- Status: Accepted
- Date: 2026-09-13 (Milestone 4)
- Context: Plan §13, §14, §16; Milestone 4 (§127)

## Decision

`crates/auth` owns identity; `crates/api` owns enforcement.

- **Passwords:** Argon2id (`argon2` crate), unique random 16-byte salts,
  default OWASP parameters (m=19456 KiB, t=2, p=1). Hashes live in the `users`
  table; plaintext never stored, logged, or passed as argv (setup wizard takes
  it over the local HTTP POST body).
- **Sessions:** server-side, DB-backed (`sessions` table, schema v2). Cookie:
  `orbynode_session`, 32 bytes of `OsRng` entropy (hex), `HttpOnly`,
  `SameSite=Lax`, `Secure` when serving HTTPS, host-only. Expiration: 7 days
  sliding. Logout and revocation delete the row; revoking a session kills it
  on the next request and its realtime gateway (M11 completes live-socket
  revocation).
- **Throttling:** failed logins per username track in memory; after 5 failures
  within 15 minutes, the account is locked for 5 minutes (server-side, no
  user enumeration - same error for unknown user and bad password).
- **CSRF:** session cookie is SameSite=Lax and all state-changing requests
  require the `X-Orbynode-CSRF` header to match the per-session token issued
  at login. WebSocket `sub` messages are authorized by the session bound at
  upgrade time; the gateway revalidates on each subscription.
- **Response hardening:** strict CSP, frame denial, `nosniff`, `no-referrer`,
  and restrictive permissions-policy headers apply to every response.
- **Setup:** `POST /setup` creates the first Owner **only** when the users
  table is empty; further calls are 403. `GET /setup` reports whether setup is
  pending (wizard gating).
- **Enforcement:** auth extractor middleware guards every state-bearing route
  (terminals REST + WS, gateway WS, projects). `/health`, `/version`, `/login`,
  `/setup`, and static assets stay public. Terminal WS and gateway WS validate
  the session cookie at upgrade; 401 otherwise (M4 acceptance).
- **Local-only default (Plan §17):** the daemon still binds 127.0.0.1; binding
  wider without any Owner existing is refused at startup.

## Consequences

- M4 acceptance: unauthenticated clients get 401 on terminals and realtime
  streams; authenticated clients pass.
- TOTP/passkeys/OIDC (Plan §14 "Later") extend `auth` without protocol change.
- The per-request DB read for session validation is acceptable at M4 scale;
  an in-memory session cache lands if profiling (M18) demands it.
