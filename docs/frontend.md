# Frontend Stack

OrbyNode's web client and website use:

- **Vite** for dev server and production builds
- **React** for composition
- **TypeScript** in strict mode
- **Bun** as package manager, script runner, and runtime
- **Magic UI** for reusable interface patterns and primitives

Do not use Next.js or another React meta-framework. The daemon already owns
hosting and embedded delivery; Vite builds a static client that can be served
directly and embedded by the daemon.

## Standard commands

```sh
bun install
bun run dev
bun run --cwd web check
bun run build
```

For CI compatibility, npm remains valid for install/build commands, but new
documentation and website instructions must use Bun first.
