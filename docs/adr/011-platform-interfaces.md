# ADR 011 - Platform Interfaces

> Superseded by [API Interface Stability](011-api-interface-stability.md).

- Status: Accepted
- Date: 2026-09-13
- Context: Plan §140; Milestone 17

## Decision

Public platform interfaces are versioned under `/api/v1`. The first stable
surface includes bearer-token administration, webhook registrations, event
publication, an MCP tool manifest and invocation endpoint, and plugin
manifests. The realtime protocol remains versioned by `v` in its envelope.

## Consequences

Breaking changes require a new interface version and migration guidance.
Token secrets are only shown once and stored as SHA-256 hashes. Webhook
secrets are scoped per registration and are not returned by list endpoints.
