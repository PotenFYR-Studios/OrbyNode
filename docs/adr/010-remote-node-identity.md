# ADR 010 - Remote Node Identity and Pairing

## Status

Accepted

## Context

Remote nodes must remain daemon-owned and uniquely identifiable. Pairing
cannot reuse browser sessions or share one secret across machines, and raw
pairing codes must not be recoverable from database access.

## Decision

Persist each node as a distinct record with a random node ID, display name,
device public key, and certificate fingerprint. Registration creates a pending
node; an Owner/Admin creates a single-use, ten-minute pairing code. Only the
SHA-256 hash is persisted, and confirmation is explicit before the node can
send authenticated heartbeats.

Pairing confirmation returns a random 256-bit node secret once and stores only
its SHA-256 hash; every heartbeat presents that secret for constant-time
verification before its timestamp or payload is accepted. Certificate issuance
and mutual TLS are layered on the same identity fields; the wire contract does
not depend on a particular certificate provider.

## Consequences

Node identity, pairing expiry, and revocation are durable. Heartbeats and
aggregated agent snapshots are transient and remain in memory. Later transport
work can replace proof checks with mutual TLS without changing node identity or
the dashboard aggregation model.
