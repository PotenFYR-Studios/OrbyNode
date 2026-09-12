# ADR 005 — Authentication / Session Model

- Status: Proposed
- Date: Milestone 4 (Setup and Authentication)
- Context: Plan §13, §14, §16; Milestone 4 (§127)

## Decision

To be finalized at the start of Milestone 4. Expected shape:

- Argon2id password hashing, unique salts.
- Server-side sessions with HttpOnly/SameSite cookies, Secure under HTTPS.
- First-run wizard creates the Owner; the password never travels as a process
  argument (Plan §13).
- CSRF protection, failed-login throttling, session revocation, device list.
- Daemon binds `127.0.0.1` by default; `0.0.0.0` requires explicit opt-in and
  a security warning (Plan §17).

## Consequences

- Unauthenticated clients cannot reach terminals or realtime streams — this is
  the M4 acceptance test and it must hold for every later surface.
