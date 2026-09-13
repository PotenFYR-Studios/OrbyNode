# Changelog

All notable changes are documented here. Format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- Milestone governance ADRs for UI and motion, collaboration, resource
  efficiency, plugin isolation, secrets handling and update signing.
- `docs/v1-readiness.md` for remaining installer and clean-machine release
  gates.
- `scripts/release.sh` for local locked builds, archives, checksums and SBOMs.
- Bun-first frontend documentation and lockfile.
- Consistent project README, contributing, security, operations and technical
  documentation set.

### Changed

- License changed from MIT to Apache-2.0 with Commons Clause.
- README and repository documentation standardized for OrbyNode.
- Web workflows standardized on Vite, React, TypeScript, Bun and Magic UI.
- CI web jobs use Bun.

### Fixed

- Workflow command execution on Windows.
- Worktree path matching with canonical base comparison.
- Unix-only symlink test isolation.

## [0.0.1] - Pre-release

### Added

- Rust daemon and Axum API foundation.
- Embedded Vite frontend delivery.
- Real PTY lifecycle and WebSocket terminal streaming.
- Realtime event bus with sequences, replay and bounded backpressure.
- SQLite persistence for projects, sessions, terminals, tasks, settings and
  audit history.
- First-run setup, Argon2id authentication, sessions, CSRF and throttling.
- Tauri desktop shell baseline.
- Agent detection for Claude Code, OpenAI Codex, Gemini CLI, OpenCode and
  Hermes.
- Integration install, rollback and safe update baseline.
- Root-contained files, Git operations and task worktrees.
- Service discovery, loopback previews and process observability.
- Multi-user RBAC, project membership and audit logging.
- Attention Center for approvals, failures and blocked work.
- Remote-node identity, pairing, heartbeat and revocation.
- Host, session and agent observability.
- Browser notifications and generic webhook rules.
- Configuration-driven workflow orchestration.
- `/api/v1` tokens, webhooks, events, MCP-style tools and plugin manifests.
- Realtime load baseline.
- Global response security headers and documented threat model.
- Release builds, checksums, SBOMs and Cosign signing.
