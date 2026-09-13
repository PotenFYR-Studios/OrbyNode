# OrbyNode Roadmap

Milestones come from `Plan.md` §123-§143. Runtime correctness and realtime
architecture come before feature surface. This file tracks status; the plan
holds detail.

## Status

| #  | Milestone                  | Status |
| -- | -------------------------- | --- |
| 0  | Foundation                 | ✅ Done |
| 1  | Real PTY Runtime           | ✅ Done |
| 2  | Realtime Core              | ✅ Done |
| 3  | Projects and Persistence   | ✅ Done |
| 4  | Setup and Authentication   | ✅ Done |
| 5  | Native Desktop/Tray        | ✅ Done |
| 6  | Agent Detection            | ✅ Done |
| 7  | Agent Integrations         | ✅ Done |
| 8  | Files and Git              | ✅ Done |
| 9  | Tasks and Worktrees        | ✅ Done |
| 10 | Services and Previews      | ✅ Done |
| 11 | Multi-User RBAC            | ✅ Done |
| 12 | Attention Center           | ✅ Done |
| 13 | Remote Nodes               | ✅ Done |
| 14 | Observability              | ✅ Done |
| 15 | Notifications              | ✅ Done |
| 16 | Workflows                  | ✅ Done |
| 17 | API, MCP, Plugin Foundation| ✅ Done |
| 18 | Scale Validation           | ✅ Done |
| 19 | Security Hardening         | ✅ Done |
| 20 | Release Engineering        | ✅ Done |

## Current focus

The milestone engineering baseline is complete. Focus now shifts to v1.0
release verification rather than new feature surface:

1. Run `scripts/install.sh` in clean Linux and macOS environments.
2. Add Windows installer/PATH support.
3. Enforce Cosign identity policy at install.
4. Test install, upgrade, rollback and clean uninstall per target.
5. Validate the Tauri desktop package separately.
6. Complete end-to-end production performance captures.

See [docs/v1-readiness.md](docs/v1-readiness.md).

## Definition of done

A milestone is done only when its acceptance criteria pass, tests and lint are
green in CI, relevant documentation and ADRs exist, and release/security
consequences are understood. Compiling is not done.

## Non-goals

Do not add hosted SaaS, billing, mandatory cloud accounts, an AI model proxy,
a full VS Code replacement, native mobile apps, Kubernetes orchestration,
enterprise SSO, a public plugin marketplace, arbitrary native plugin execution
or controller clustering in the current release line.
