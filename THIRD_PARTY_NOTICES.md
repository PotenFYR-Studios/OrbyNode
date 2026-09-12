# Third-Party Notices

OrbyNode depends on the following third-party open-source packages. Each is
used under its own license; this file documents provenance as required.

## Backend (Rust)

| Crate               | License     | Source                                    |
| ------------------- | ----------- | ----------------------------------------- |
| tokio               | MIT         | https://github.com/tokio-rs/tokio          |
| axum                | MIT         | https://github.com/tokio-rs/axum           |
| tower               | MIT         | https://github.com/tower-rs/tower          |
| tower-http          | MIT/Apache-2.0 | https://github.com/tower-rs/tower-http  |
| serde_json          | MIT/Apache-2.0 | https://github.com/serde-rs/json        |
| tracing             | MIT         | https://github.com/tokio-rs/tracing        |
| tracing-subscriber  | MIT         | https://github.com/tokio-rs/tracing        |

## Frontend (npm)

| Package              | License | Source                                    |
| -------------------- | ------- | ----------------------------------------- |
| react, react-dom     | MIT     | https://github.com/facebook/react          |
| vite                 | MIT     | https://github.com/vitejs/vite             |
| @vitejs/plugin-react | MIT     | https://github.com/vitejs/vite-plugin-react|
| typescript           | Apache-2.0 | https://github.com/microsoft/TypeScript |

This list tracks direct dependencies of record. For the authoritative,
version-pinned set, see `Cargo.toml`/`Cargo.lock` and `web/package-lock.json`.

No third-party code is vendored into this repository. UI inspiration sources
(e.g. Magic UI) are used as documented in `Plan.md` §93: as inspiration and
 selectively as components, never as copied product identity, and any future
adopted component code will be added here with its license.
