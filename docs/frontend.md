# Frontend Guide

OrbyNode's web client and website use Vite, React, TypeScript, Bun and Magic
UI. Do not use Next.js or another React meta-framework.

## Applications

OrbyNode has two independent static React applications:

| Directory | Purpose | Delivery |
| --- | --- | --- |
| `web/` | Authenticated product client | Embedded in the daemon binary |
| `docs/` | Public documentation site | Prerendered and deployed to GitHub Pages |

They share stack rules and visual language, not runtime code or build output.

## Commands

Product client:

```bash
(cd web && bun install --frozen-lockfile)
(cd web && bun run dev)
(cd web && bun run check)
(cd web && bun run build)
```

Documentation site:

```bash
(cd docs && bun install --frozen-lockfile)
(cd docs && bun run dev)
(cd docs && bun test)
(cd docs && bun run typecheck)
(cd docs && bun run build)
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
web/index.html          product-client Vite entry
web/src/main.tsx        product-client React root
web/src/App.tsx         current control-plane surface
web/src/api.ts          typed REST client
web/src/realtime.ts     WebSocket client
docs/index.html         documentation Vite entry
docs/src/App.tsx        docs routes and shared shell
docs/src/manifest.ts    navigation, routes and metadata
docs/src/content.ts     build-time Markdown ingestion
docs/scripts/           asset sync and static prerendering
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

Run commands from the application directory. Both apps use `bun run dev` and
`bun run build`.

- `web/dist/` is static and embeddable by the daemon.
- `docs/dist/` contains a prerendered HTML file for every documentation route,
  plus the sitemap, CNAME and installer entrypoints used by the public domain.

## Embedded delivery

`crates/api/build.rs` embeds `web/dist` into the daemon. For frontend
development, serve from disk:

```bash
ORBYNODE_STATIC_DIR="$PWD/web/dist" cargo run -p orbynode-daemon
```

Do not introduce server-side rendering or a separate web runtime.
