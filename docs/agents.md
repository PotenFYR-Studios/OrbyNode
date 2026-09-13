# Agents

OrbyNode detects terminal coding agents and converts behavior into stable
control-plane state.

## Supported initial agents

| Agent | Detection | Notes |
| --- | --- | --- |
| Claude Code | Terminal/process patterns | Native hook integration path. |
| OpenAI Codex | Terminal/process patterns | Native hook integration path. |
| Gemini CLI | Terminal patterns | Prompt and approval patterns. |
| OpenCode | Terminal patterns | Initial supported target. |
| Hermes | Terminal patterns | Initial supported target. |

Other terminal processes remain usable as ordinary shells.

## Canonical states

| State | Meaning |
| --- | --- |
| `Starting` | Terminal or agent startup observed. |
| `Working` | Agent is actively producing output. |
| `Waiting` | Agent is paused or waiting on work. |
| `NeedsInput` | User response is required. |
| `NeedsApproval` | Permission or approval is required. |
| `Idle` | No active progress signal. |
| `Completed` | Agent completed successfully. |
| `Failed` | Agent reported failure. |
| `Disconnected` | Terminal exited or connection was lost. |
| `Unknown` | No confident state can be assigned. |

UI labels may differ; internal state remains stable.

## Detection levels

### Level 1 - process detection

Identifies known executable/process-tree combinations. This answers “which
terminal is which agent?” but does not infer detailed state.

### Level 2 - terminal analysis

Configured rules inspect bounded terminal output for banners, prompts,
approval requests, failures and completion markers. Terminal bytes remain
transient; detection produces a semantic event.

### Level 3 - native integration

Agent hooks and integrations supply richer signals when available:

- lifecycle state,
- session identifiers,
- task/model/provider metadata,
- token and context usage,
- permission requests,
- files changed,
- completion and errors,
- native resume identifiers.

Native integration wins over heuristic detection when both are available.

## Attention Center

The Attention Center aggregates:

- approvals,
- blocked agents,
- failures,
- conflicts,
- disconnected nodes,
- warnings.

Critical attention events use the realtime bus and are not dropped in favor of
terminal or metric traffic. Users can resolve items without opening every
terminal.

## Integration manager

The integration surface supports discoverability and reversible lifecycle
actions:

- install,
- update,
- rollback,
- enable/disable,
- inspect.

Safe integrations must be idempotent, preserve unrelated configuration, avoid
silent overwrite, create backups where files are modified and support rollback.

## Agent events

Agent state changes publish realtime events and can raise Attention Center
items. Terminal output remains bounded and droppable under pressure; semantic
approval and failure state is critical.

## Usage and observability

When native integration exposes token, context, model or provider metadata,
OrbyNode records it alongside process and host snapshots. This supports the
“expensive, slow or runaway?” question without polling the daemon.
