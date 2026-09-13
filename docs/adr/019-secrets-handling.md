# ADR 019 - Secrets Handling

- Status: Accepted
- Date: 2026-09-13
- Context: Plan §16, §140; Milestones 17-19

## Decision

Secrets are never logged or returned in list responses. Passwords use
Argon2id. API tokens and remote-node secrets are stored as SHA-256 hashes and
shown once at creation. Webhook secrets are write-only. Pairing codes expire.
Sessions use random server-side tokens and per-session CSRF material.

## Consequences

Leakage from database disclosure does not directly expose reusable secrets.
New integrations must use the same creation-time disclosure and hashing model.
