# OrbyNode — Roadmap

Milestones come from `Plan.md` §123–§143; the ordering is deliberate
(runtime correctness and realtime architecture before feature surface — §153).
This file tracks status; the plan holds the detail.

## Milestones

| #  | Milestone                  | Status          |
| -- | -------------------------- | --------------- |
| 0  | Foundation                 | ✅ Done          |
| 1  | Real PTY Runtime           | Next            |
| 2  | Realtime Core              | —               |
| 3  | Projects and Persistence   | —               |
| 4  | Setup and Authentication   | —               |
| 5  | Native Desktop/Tray        | —               |
| 6  | Agent Detection            | —               |
| 7  | Agent Integrations         | —               |
| 8  | Files and Git              | —               |
| 9  | Tasks and Worktrees        | —               |
| 10 | Services and Previews      | —               |
| 11 | Multi-User RBAC            | —               |
| 12 | Attention Center           | —               |
| 13 | Remote Nodes               | —               |
| 14 | Observability              | —               |
| 15 | Notifications              | —               |
| 16 | Workflows                  | —               |
| 17 | API, MCP, Plugin Foundation| —               |
| 18 | Scale Validation           | —               |
| 19 | Security Hardening         | —               |
| 20 | Release Engineering        | —               |

## Definition of done

A milestone is done when its acceptance criteria from `Plan.md` pass, tests
and lint are green in CI, and any new architectural decision is captured as
an ADR. Compiling is not done.
