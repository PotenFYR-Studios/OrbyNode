# ADR 015 - UI Design and Motion

- Status: Accepted
- Date: 2026-09-13
- Context: Plan §148; Vite, React, TypeScript, Bun, and Magic UI are mandated
  for web surfaces.

## Decision

Use Vite + React + TypeScript + Bun for development and static delivery. Use
Magic UI as the component-pattern layer. Motion remains purposeful: respect
`prefers-reduced-motion`, avoid animation on terminal output, and reserve
motion for state transitions and attention feedback. Do not use Next.js or
another React meta-framework.

## Consequences

UI work must remain static-output compatible with daemon embedding. Website
and product UI should share design tokens and component ownership while
keeping runtime responsibilities in the daemon.
