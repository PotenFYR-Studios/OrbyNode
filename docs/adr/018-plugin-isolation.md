# ADR 018 - Plugin Isolation

- Status: Accepted
- Date: 2026-09-13
- Context: Plan §140, §142, §145; Milestone 17

## Decision

Plugins are manifest records and are disabled by default. The current
foundation does not execute arbitrary plugin code. A future plugin runtime
must declare versioned capabilities and permissions, isolate host access
behind explicit APIs, remain revocable, and preserve audit coverage. Arbitrary
native plugins and a public marketplace are deferred.

## Consequences

Milestone 17 can expose safe manifests without accepting untrusted execution.
Plugin isolation must land before enabling third-party plugin execution.
