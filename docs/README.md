# OrbyNode Documentation

Welcome to the OrbyNode documentation. OrbyNode is the self-hosted control
plane for coding agents: persistent terminal agents, real PTYs, files, Git,
tasks, services and machine state in one daemon.

## Getting started

| Page | What's inside |
|---|---|
| [Getting started](getting-started.md) | Quick start, first-run setup, configuration, daily use |
| [Installation](installation.md) | Installer script, manual download, per-platform requirements |
| [Releases](releases.md) | Release targets, signature verification, upgrade/rollback |

## Core concepts

| Page | What's inside |
|---|---|
| [Architecture](../ARCHITECTURE.md) | Daemon-centric design, crates, data flow |
| [Realtime](realtime.md) | Snapshots, sequenced events, bounded replay |
| [Agents](agents.md) | Agent detection model and Attention Center states |
| [Workspaces](../docs/adr/021-herdr-parity-web-workspaces.md) | Workspaces, tabs, panes and crash recovery (ADR 021) |
| [Security model](security-model.md) | Authentication, RBAC, CSRF, audit, hardening |

## Operations

| Page | What's inside |
|---|---|
| [Operations](operations.md) | Filesystem layout, logs, backup/restore, upgrades |
| [Troubleshooting](troubleshooting.md) | Common symptoms and fixes |
| [Remote nodes](remote-nodes.md) | Pairing, identity, revocation |
| [Notifications](notifications.md) | Attention, workflow and user-defined event routing |
| [Performance](performance.md) | Baselines, limits, measurement requirements |

## Integrations

| Page | What's inside |
|---|---|
| [REST API](rest-api.md) | Public `/api/v1` surface: auth, projects, panes, workspaces |
| [Workflows](workflows.md) | Configuration-driven orchestration |
| [Frontend](frontend.md) | Product client and public docs-site architecture |

## Contributing

| Page | What's inside |
|---|---|
| [Contributing](contributing.md) | Process, commit discipline, review expectations |
| [Development](development.md) | Build, run, test, debug from source |
| [Testing](testing.md) | Test patterns and coverage expectations |
| [Changelog](changelog.md) | Release notes and breaking changes |

## Reference

| Page | What's inside |
|---|---|
| [FAQ](faq.md) | Common questions on security, architecture, licensing |
| [License](license.md) | Apache-2.0 + Commons Clause in plain language |
| [Roadmap](../ROADMAP.md) | Completed milestones, current release focus, non-goals |
| [Security policy](../SECURITY.md) | Private disclosure process and supported versions |
| [v1 readiness](v1-readiness.md) | Release gates for the v1.0 label |

## Architecture decision records

ADRs live in [adr/](adr/) and capture substantial architecture decisions.
Notable:

- [ADR 001](adr/001-daemon-architecture.md) — daemon-centric architecture
- [ADR 021](adr/021-herdr-parity-web-workspaces.md) — Herdr-parity web workspaces and crash recovery

## Community

- Bugs, ideas and questions: [GitHub Issues](https://github.com/PotenFYR-Studios/OrbyNode/issues)
- Documentation site: <https://orbynode.docs.potenfyr.in>
- Community: [Discord](https://discord.com/invite/zUaN2FPBec)
- Security disclosure: [SECURITY.md](../SECURITY.md)
