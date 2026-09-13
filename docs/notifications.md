# Notifications

Notification routing for attention, workflow and user-defined events.

## Current channels

| Channel | Status |
| --- | --- |
| Browser realtime | Implemented. |
| Desktop | Delivered through realtime events surfaced by Tauri. |
| Generic webhook | Implemented. |

Slack, Discord, Microsoft Teams, Telegram and email are later channel targets.
Use a generic webhook bridge in the meantime.

## Browser and desktop

The web client connects to the realtime `notifications` stream. When the
browser permission is granted, it creates a local notification from event
priority and summary.

Desktop notifications remain a client concern. The daemon remains the source of
event truth.

## Rules

### List rules

```http
GET /notifications/rules
```

### Create or update rule

```http
POST /notifications/rules
Content-Type: application/json
X-Orbynode-CSRF: <csrf>
```

```json
{
  "event": "attention.created",
  "channel": "webhook",
  "target": "https://example.invalid/hook",
  "enabled": true
}
```

Optional fields:

| Field | Meaning |
| --- | --- |
| `project_id` | Limit rule to one project. |
| `enabled` | Disable without deleting. |

### Send test

```http
POST /notifications/test
X-Orbynode-CSRF: <csrf>
```

Test notifications are also audited.

## Generic webhook delivery

A webhook rule posts a JSON payload to the configured HTTPS or HTTP target.
Treat webhook URLs and secrets as sensitive.

Recommended receiver behavior:

1. Verify request source and TLS.
2. Parse JSON defensively.
3. Handle retries idempotently.
4. Return a non-2xx only when delivery should be considered failed.
5. Do not log secret material.

## Event sources

Expected event families include:

| Event | Meaning |
| --- | --- |
| `attention.created` | New approval, failure or blocked agent. |
| `attention.resolved` | Attention item resolved. |
| `workflow.updated` | Workflow status changed. |
| `notification.test` | User-triggered test. |

New event types are additive and documented before use.

## Persistence

Rules are in-memory in the current notification service. Delivery secrets are
never logged. Platform webhook registrations are separate durable `/api/v1`
records.

## Security

- Notification routes require authentication and CSRF.
- Webhook targets can exfiltrate event metadata; configure carefully.
- Do not place secrets in event summaries.
- Do not expose daemon internals through webhook payloads.

## Failure behavior

- Browser delivery depends on client connection and notification permission.
- Webhook delivery is best-effort in the current baseline.
- A failed external receiver must not block the daemon.
- Test deliveries should be used after changing rules.

## Later channels

Planned channels:

- Slack,
- Discord,
- Microsoft Teams,
- Telegram,
- email.

Each channel should use the same rule model, explicit target configuration,
secret handling and auditable test delivery.
