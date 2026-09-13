# OrbyNode

> The self-hosted control plane for coding agents.

OrbyNode runs, monitors, and coordinates terminal-based AI coding agents from
a browser. One daemon owns the agents - close the browser, switch networks,
come back later: everything keeps running and reconnects cleanly.

**Status:** Milestone 20 engineering baseline. See [ROADMAP.md](ROADMAP.md).

## Quick start (from source)

Requirements: Rust 1.85+, Bun 1.1+, Node 20+ for CI compatibility.

```sh
(cd web && bun install) && (cd web && bun run build)
cargo run --release -p orbynode-daemon
```

Open <http://127.0.0.1:7676>. The daemon binds localhost only by default.

## Documentation

- [ARCHITECTURE.md](ARCHITECTURE.md) - how the system is put together
- [ROADMAP.md](ROADMAP.md) - milestones and status
- [SECURITY.md](SECURITY.md) - security model and reporting
- [CONTRIBUTING.md](CONTRIBUTING.md) - development workflow
- [docs/adr/](docs/adr/) - architecture decision records
- [docs/performance.md](docs/performance.md) - performance baseline

## Release artifacts

Automated builds produce Linux x86_64, Linux ARM64, macOS Intel, macOS Apple
Silicon, Windows x86_64, and Windows ARM64 daemon archives. Each archive has a
SHA-256 checksum and an Sigstore/Cosign signature. Release notes document
breaking interface changes and the SBOM location.

Build locally with:

```sh
(cd web && bun install) && (cd web && bun run build)
cargo build --release -p orbynode-daemon
```

The desktop Tauri shell is excluded from the daemon workspace. Build it after
the web assets with your platform's Tauri prerequisites.

## License

[MIT](LICENSE)
