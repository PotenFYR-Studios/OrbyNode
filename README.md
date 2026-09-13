<!-- markdownlint-disable -->
<div align="center">

<img src="https://capsule-render.vercel.app/api?type=waving&color=0:6366f1,50:22d3ee,100:a3e635&height=220&section=header&text=OrbyNode&fontSize=52&fontColor=ffffff&fontAlignY=34&desc=The%20Control%20Plane%20for%20Coding%20Agents&descSize=18&descAlignY=55&animation=twinkling" width="100%" alt="OrbyNode Banner"/>

[![Typing SVG](https://readme-typing-svg.demolab.com?font=Fira+Code&weight=600&size=20&pause=1200&color=22D3EE&center=true&vCenter=true&width=850&lines=Persistent+agents+across+browser+disconnects;Real+PTYs%2C+files%2C+Git%2C+tasks+and+previews;Multi-user+RBAC+and+multi-machine+control;One+Attention+Center+for+what+needs+you+now)](https://orbynode.docs.potenfyr.in)

<p align="center">
  <a href="https://potenfyr.in"><img src="https://img.shields.io/badge/Website-potenfyr.in-22d3ee?style=for-the-badge&logo=googlechrome&logoColor=white&labelColor=1c1e26" alt="Website" /></a>
  <a href="https://orbynode.docs.potenfyr.in"><img src="https://img.shields.io/badge/Docs-orbynode.docs.potenfyr.in-6366f1?style=for-the-badge&logo=gitbook&logoColor=white&labelColor=1c1e26" alt="Documentation" /></a>
  <a href="https://discord.com/invite/zUaN2FPBec"><img src="https://img.shields.io/badge/Discord-Join%20us-5865F2?style=for-the-badge&logo=discord&logoColor=white&labelColor=1c1e26" alt="Discord" /></a>
  <a href="mailto:support@potenfyr.in"><img src="https://img.shields.io/badge/Email-support%40potenfyr.in-a3e635?style=for-the-badge&logo=gmail&logoColor=white&labelColor=1c1e26" alt="Email" /></a>
  <a href="https://github.com/PotenFYR-Studios/OrbyNode"><img src="https://komarev.com/ghpvc/?username=PotenFYR-Studios-OrbyNode&color=22d3ee&style=for-the-badge&label=VIEW&labelColor=1c1e26" alt="View" /></a>
</p>

[![Release](https://img.shields.io/github/v/release/PotenFYR-Studios/OrbyNode?style=flat-square&display_name=release&labelColor=1c1e26&color=2ea043)](https://github.com/PotenFYR-Studios/OrbyNode/releases/latest)
[![CI Build](https://img.shields.io/github/actions/workflow/status/PotenFYR-Studios/OrbyNode/ci.yml?style=flat-square&labelColor=1c1e26&color=2ea043)](https://github.com/PotenFYR-Studios/OrbyNode/actions/workflows/ci.yml)
[![Release Build](https://img.shields.io/github/actions/workflow/status/PotenFYR-Studios/OrbyNode/release.yml?style=flat-square&label=release&labelColor=1c1e26&color=2ea043)](https://github.com/PotenFYR-Studios/OrbyNode/actions/workflows/release.yml)
[![License](https://img.shields.io/badge/License-Apache--2.0%20%2B%20Commons%20Clause-6366f1?style=flat-square&labelColor=1c1e26)](LICENSE)

[Docs](https://orbynode.docs.potenfyr.in) · [Getting started](docs/getting-started.md) · [Architecture](ARCHITECTURE.md) · [API](docs/rest-api.md) · [Contributing](CONTRIBUTING.md) · [Security](SECURITY.md)

</div>

---

## 🚀 What is OrbyNode?

OrbyNode is the **self-hosted control plane for coding agents**. It runs persistent terminal agents, owns their PTYs, files, tasks, Git workflows, services and machine state in one daemon, and reconnects your browser without interrupting the work.

The browser and desktop shell are replaceable clients. The daemon is the runtime. Closing a client never kills an agent.

## ✨ Highlights

| Capability | Why it matters |
| --- | --- |
| **Persistent PTY runtime** | Real terminals survive disconnects, UI restarts and network changes. |
| **Realtime control plane** | One authenticated multiplexed WebSocket with snapshots, replay and bounded queues. |
| **Agent awareness** | Detect Claude, Codex, Gemini, OpenCode and Hermes state, approvals and failures. |
| **Files and Git** | Inspect, edit and review changes without leaving the control plane. |
| **Tasks and worktrees** | Isolate agents per task and keep merge flows under review. |
| **Services and previews** | Discover local ports and proxy development previews safely. |
| **Multi-user RBAC** | Owner, admin, operator, developer and viewer roles with project membership. |
| **Remote nodes** | Pair machines with expiring secrets and aggregate agents across hosts. |
| **Observability** | Host, session, token/context and workflow signals with an Attention Center. |
| **APIs and workflows** | Versioned `/api/v1`, MCP-style tools, webhooks and configuration-driven orchestration. |

## ⚡ Quick start

Requirements: Rust 1.85+, Bun 1.1+, Node 20+ for CI compatibility.

```bash
git clone https://github.com/PotenFYR-Studios/OrbyNode.git
cd OrbyNode
(cd web && bun install && bun run build)
cargo run --release -p orbynode-daemon
```

Open <http://127.0.0.1:7676>, complete first-run setup, and create the first Owner.

The daemon binds `127.0.0.1` by default. Non-loopback binding is opt-in and must be paired with authentication and HTTPS or a trusted overlay.

## 🧠 How it works

```text
Browser / PWA / Tauri shell
          |
   HTTP(S) + WebSocket
          v
    OrbyNode daemon
    /      |      \
  PTYs   SQLite   Realtime bus
   |       |        |
 Agents  durable  snapshots,
 tasks   state    replay, events
```

1. The daemon creates and owns terminal PTYs.
2. Output is captured once and fanned out through bounded realtime channels.
3. REST handles commands; WebSocket state uses snapshots and event replay.
4. Projects, sessions, terminals, tasks, settings and audit data live in SQLite.
5. Clients reconnect, resume from a sequence, and resynchronize when replay is unavailable.

## 🧱 Architecture

| Crate | Role |
| --- | --- |
| `orbynode-core` | Small shared configuration and path types. |
| `orbynode-terminal` | Real PTY lifecycle and bounded output fan-out. |
| `orbynode-realtime` | Event bus, sequences, replay, subscriptions and backpressure. |
| `orbynode-database` | SQLite with WAL, migrations and durable repositories. |
| `orbynode-auth` | Argon2id, sessions, throttling and RBAC primitives. |
| `orbynode-agents` | Agent detection and Attention Center. |
| `orbynode-files` | Root-contained files, Git and worktrees. |
| `orbynode-services` | Port discovery, service metadata and process snapshots. |
| `orbynode-nodes` | Remote-node identity, pairing, heartbeat and revocation. |
| `orbynode-notifications` | Browser events and generic webhooks. |
| `orbynode-workflows` | Configuration-driven pipeline execution. |
| `orbynode-api` | Axum routes, auth middleware, gateway and embedded UI. |

The web client uses **Vite + React + TypeScript + Bun + Magic UI**. Next.js and other meta-frameworks are not used.

## 🛠️ Development

```bash
(cd web && bun install) # install frontend dependencies
cargo build             # build backend
scripts/check.sh        # run every required gate
```

Run the daemon with live frontend assets:

```bash
(cd web && bun run build)
ORBYNODE_STATIC_DIR=web/dist cargo run -p orbynode-daemon
```

## 📦 Releases

Automated release builds target Linux x86_64 and ARM64, macOS Intel and Apple Silicon, and Windows x86_64 and ARM64. Artifacts include locked daemon builds, SHA-256 checksums, SBOM inventory and Cosign signatures.

Local release subset:

```bash
scripts/release.sh <rust-target-triple>
```

See [docs/v1-readiness.md](docs/v1-readiness.md) for remaining clean-machine installer and upgrade gates.

## 📚 Documentation

- [Documentation index](docs/README.md)
- [Getting started](docs/getting-started.md)
- [Architecture](ARCHITECTURE.md)
- [Operations](docs/operations.md)
- [Documentation site](https://orbynode.docs.potenfyr.in)
- [REST API](docs/rest-api.md)
- [Realtime protocol](docs/realtime.md)
- [Agents](docs/agents.md)
- [Workflows](docs/workflows.md)
- [Remote nodes](docs/remote-nodes.md)
- [Notifications](docs/notifications.md)
- [Security model](docs/security-model.md)
- [Frontend](docs/frontend.md)
- [Development](docs/development.md)
- [Testing](docs/testing.md)
- [Contributing](docs/contributing.md)
- [Troubleshooting](docs/troubleshooting.md)
- [Performance](docs/performance.md)
- [Release readiness](docs/v1-readiness.md)
- [Changelog](docs/changelog.md)
- [ADRs](docs/adr/)

## 💬 Support

- Bugs, ideas and questions: [GitHub Issues](https://github.com/PotenFYR-Studios/OrbyNode/issues)
- Documentation: <https://orbynode.docs.potenfyr.in>
- Community: [Discord](https://discord.com/invite/zUaN2FPBec)

## 🤝 Contributing

Contributions are welcome. Keep changes milestone-scoped, add ADRs for substantial architecture, and run the full check suite before opening a pull request. See [CONTRIBUTING.md](CONTRIBUTING.md) and the [good first issues](https://github.com/PotenFYR-Studios/OrbyNode/labels/good%20first%20issue). Security concerns: please use [SECURITY.md](SECURITY.md) (private vulnerability reporting), not public issues.

<a href="https://github.com/PotenFYR-Studios/OrbyNode/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=PotenFYR-Studios/OrbyNode" alt="OrbyNode contributors" />
</a>
<a href="https://github.com/PotenFYR-Studios/OrbyNode/stargazers">
  <img src="https://img.shields.io/github/stars/PotenFYR-Studios/OrbyNode?style=social&label=Stars" alt="Live star count" />
</a>
<a href="https://github.com/PotenFYR-Studios/OrbyNode/network/members">
  <img src="https://img.shields.io/github/forks/PotenFYR-Studios/OrbyNode?style=social&label=Forks" alt="Live fork count" />
</a>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/PotenFYR-Studios/FYRwall/output/github-snake-dark.svg" />
  <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/PotenFYR-Studios/FYRwall/output/github-snake.svg" />
  <img alt="Contribution snake animation" src="https://raw.githubusercontent.com/PotenFYR-Studios/FYRwall/output/github-snake.svg" width="100%" />
</picture>

---

## ⭐ Star History

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=potenfyr-studios/authcore,potenfyr-studios/statfyr,potenfyr-studios/discord-botlists,potenfyr-studios/shell-eggs,potenfyr-studios/prog-language-eggs,potenfyr-studios/minecraft-eggs,potenfyr-studios/database-eggs,potenfyr-studios/apicordon,potenfyr-studios/ojaj,potenfyr-studios/fyrwall,potenfyr-studios/echoingdeaths,potenfyr-studios/orbynode&type=Date&theme=dark" />
  <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=potenfyr-studios/authcore,potenfyr-studios/statfyr,potenfyr-studios/discord-botlists,potenfyr-studios/shell-eggs,potenfyr-studios/prog-language-eggs,potenfyr-studios/minecraft-eggs,potenfyr-studios/database-eggs,potenfyr-studios/apicordon,potenfyr-studios/ojaj,potenfyr-studios/fyrwall,potenfyr-studios/echoingdeaths,potenfyr-studios/orbynode&type=Date" />
  <img alt="Star history chart for all PotenFYR Studios public repositories" src="https://api.star-history.com/svg?repos=potenfyr-studios/authcore,potenfyr-studios/statfyr,potenfyr-studios/discord-botlists,potenfyr-studios/shell-eggs,potenfyr-studios/prog-language-eggs,potenfyr-studios/minecraft-eggs,potenfyr-studios/database-eggs,potenfyr-studios/apicordon,potenfyr-studios/ojaj,potenfyr-studios/fyrwall,potenfyr-studios/echoingdeaths,potenfyr-studios/orbynode&type=Date" width="80%" />
</picture>

Every public PotenFYR Studios repository on one live chart, served by [star-history.com](https://star-history.com).

---

## 🔐 Security

Terminal access is remote-code-execution capability. Do not open public issues for exploitable flaws; follow the confidential process in [SECURITY.md](SECURITY.md).

## 📜 License

Licensed under **Apache-2.0 with Commons Clause**; the [LICENSE](LICENSE) file is authoritative. In plain terms: fork, modify, use, self-host and redistribute OrbyNode for free, including commercial use and building products or services around it. The Commons Clause withholds selling the software itself, or charging for a product or service whose value derives entirely or substantially from its functionality, and PotenFYR names and trademarks stay unlicensed. Keep the required license notices, including the Commons Clause notice.

---

<div align="center">

<img src="https://capsule-render.vercel.app/api?type=waving&color=0:a3e635,50:22d3ee,100:6366f1&height=120&section=footer&text=Made%20with%20%E2%9D%A4%EF%B8%8F%20by%20PotenFYR%20Studios&fontSize=22&fontColor=ffffff&animation=twinkling" width="100%" alt="footer"/>

</div>
<!-- markdownlint-enable -->
