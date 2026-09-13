# ADR 013 - Security Hardening Baseline

- Status: Accepted
- Date: 2026-09-13
- Context: Plan §142; Milestone 19

## Decision

Apply response headers globally, document the current threat model and
mitigations, and retain fail-closed authorization and secret handling. Plugin
execution remains disabled until capability enforcement is implemented.

## Consequences

Security regressions in response headers are test-enforced. New interfaces
must extend the documented threat model and retain server-side authorization.
