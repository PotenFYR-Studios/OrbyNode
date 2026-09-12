# OrbyNode

> The self-hosted control plane for coding agents.

OrbyNode runs, monitors, and coordinates terminal-based AI coding agents from
a browser. One daemon owns the agents — close the browser, switch networks,
come back later: everything keeps running and reconnects cleanly.

**Status:** early foundation (Milestone 0). See [ROADMAP.md](ROADMAP.md).

## Quick start (from source)

Requirements: Rust 1.85+, Node 20+.

```sh
npm ci --prefix web && npm run build --prefix web
cargo run --release -p orbynode-daemon
```

Open <http://127.0.0.1:7676>. The daemon binds localhost only by default.

## Documentation

- [ARCHITECTURE.md](ARCHITECTURE.md) — how the system is put together
- [ROADMAP.md](ROADMAP.md) — milestones and status
- [SECURITY.md](SECURITY.md) — security model and reporting
- [CONTRIBUTING.md](CONTRIBUTING.md) — development workflow
- [docs/adr/](docs/adr/) — architecture decision records

## License

[MIT](LICENSE)
