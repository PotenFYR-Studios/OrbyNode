# Web UI

Stack: Vite, React, TypeScript, Bun, Magic UI.

## Commands

```sh
bun install
bun run dev
bun run check
bun run build
```

Run Bun scripts from `web/`. From the repository root, use:

```sh
(cd web && bun install)
(cd web && bun run check)
(cd web && bun run build)
```

Use Magic UI components for reusable interface primitives. Do not introduce
Next.js, another meta-framework, or npm scripts into this workspace.
