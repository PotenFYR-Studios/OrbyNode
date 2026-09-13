# ADR 020 - Update and Signing Model

- Status: Accepted
- Date: 2026-09-13
- Context: Plan §142-§144; Milestone 20

## Decision

Release artifacts use locked builds, SHA-256 checksums, and Cosign keyless
signatures. SBOMs are generated per target from the locked dependency graph.
The updater must verify checksum, signature, identity, and compatibility
before replacement, preserve configuration and durable state, support rollback,
and never use an unsigned bootstrap endpoint.

## Consequences

Release signing is ready for channel work. A one-command installer and
clean-environment updater tests remain required before calling the product
v1.0.
