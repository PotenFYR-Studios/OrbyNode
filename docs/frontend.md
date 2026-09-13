# Frontend Guide

OrbyNode's web client and website use Vite, React, TypeScript, Bun and Magic
UI. Do not use Next.js or another React meta-framework.

## Commands

Run from the repository root:

```bash
(cd web && bun install)
(cd web && bun run dev)
(cd web && bun run check)
(cd web && bun run build)
```

Run inside `web/`:

```bash
bun install
bun run dev
bun run check
bun run build
```

## Stack responsibilities

| Tool | Responsibility |
| --- | --- |
| Vite | Dev server, proxy, static production build. |
| React | UI composition and state rendering. |
| TypeScript | Strict types and compile-time checks. |
| Bun | Package manager, script runner and local runtime. |
| Magic UI | Reusable interface patterns and primitives. |

## Project files

```text
web/index.html        Vite entry
web/src/main.tsx      React root
web/src/App.tsx       current control-plane surface
web/src/api.ts        typed REST client
web/src/realtime.ts   WebSocket client
web/vite.config.ts    Vite configuration
web/tsconfig.json     TypeScript configuration
```

## API access

Use the typed helpers in `web/src/api.ts`. Keep response shapes explicit. Do
not parse untyped JSON ad hoc.

Mutations must include the session CSRF header when required:

```ts
headers: {
  "x-orbynode-csrf": csrfToken,
}
```

## Realtime access

Use `RealtimeClient` for snapshots and events. The client:

1. opens `/ws`,
2. sends `hello`,
3. subscribes to authorized streams,
4. applies sequenced events,
5. resubscribes or resnapshots after overflow.

Do not poll REST endpoints for live state.

## Terminal data

Terminal output is untrusted data. Do not render terminal bytes as HTML. If
terminal rendering is added, use an escaped terminal renderer and a strict CSP.

## State model

Prefer explicit state slices:

```ts
interface State {
  health?: Health;
  attention: AttentionItem[];
  nodes: RemoteNode[];
  host?: HostMetrics;
  sessions: SessionMetrics[];
  workflow?: WorkflowRun;
}
```

Keep optimistic behavior limited and always reconcile against server events or
snapshots.

## Magic UI

Use Magic UI patterns for reusable interface primitives and motion. Follow
these rules:

- keep motion purposeful,
- respect `prefers-reduced-motion`,
- do not animate raw terminal output,
- reserve motion for state transition and attention feedback,
- own copied component code rather than depending on a meta-framework.

## Styling

Current UI uses inline styles for the minimal control-plane surface. As the UI
grows, use token-based styling with Magic UI-compatible component ownership.
Keep colors, spacing, focus rings and typography consistent.

## Accessibility

- Label interactive controls.
- Use headings and section landmarks.
- Maintain visible keyboard focus.
- Avoid color-only status.
- Preserve live region semantics for attention updates.

## Builds

Development:

```bash
bun run dev
```

Production:

```bash
bun run build
```

Output is static and embeddable by the daemon.

## Embedded delivery

`crates/api/build.rs` embeds `web/dist` into the daemon. For frontend
development, serve from disk:

```bash
ORBYNODE_STATIC_DIR="$PWD/web/dist" cargo run -p orbynode-daemon
```

Do not introduce server-side rendering or a separate web runtime.
