# ADR 016 - Multi-User Collaboration

- Status: Accepted
- Date: 2026-09-13
- Context: Plan §15, §134; Milestone 11

## Decision

Authorization is server-side and project-scoped. Global roles provide
administration, while project membership maps to explicit permissions. Every
REST request, WebSocket subscription, terminal mutation, file operation, and
remote-node action revalidates the resolved user. Membership changes publish
realtime events and close immediately unauthorized subscriptions.

## Consequences

Client state is eventually consistent only after server authorization.
Project-scoped permissions may be extended without breaking global roles.
