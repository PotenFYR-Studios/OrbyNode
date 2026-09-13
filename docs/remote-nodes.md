# Remote Nodes

OrbyNode can aggregate coding agents from multiple machines under one
authenticated control plane.

## Model

| Role | Responsibility |
| --- | --- |
| Controller | Main OrbyNode daemon. Stores node identity and aggregated state. |
| Node | Remote OrbyNode-compatible daemon that owns local PTYs and agents. |
| Operator | Authenticated controller user who registers, pairs and revokes nodes. |

The controller never executes a remote node's local PTY directly. Aggregation
comes from authenticated node state and heartbeats.

## Lifecycle

1. **Register** an identity on the controller.
2. **Create pairing** to issue a short-lived challenge/code.
3. **Confirm pairing** on the node using the one-time secret.
4. **Heartbeat** from node with proof material.
5. **Aggregate** node agents in the controller.
6. **Revoke** when host is decommissioned or compromised.

## Identity

Each node has:

- stable node ID,
- human-readable name,
- unique fingerprint,
- public identity material,
- status.

Statuses:

| Status | Meaning |
| --- | --- |
| `pending` | Registered but not paired. |
| `online` | Heartbeat accepted. |
| `offline` | Heartbeat missing or expired. |
| `revoked` | Rejected until re-registered. |

## Pairing

Pairing secrets and codes are one-time and hashed. Pairing challenges expire.
The controller does not store reusable plaintext node secrets.

Operator flow:

```http
POST /nodes
POST /nodes/{id}/pairing
```

Node flow:

1. Receive code out-of-band.
2. Confirm before expiry.
3. Store node-side secret securely.
4. Begin heartbeat.

## Heartbeat

A node heartbeat updates last-seen state and presents proof bound to its
secret. The controller rejects:

- unknown IDs,
- revoked nodes,
- invalid proofs,
- stale pairing state.

Heartbeats maintain online status. Missing heartbeats lead to offline state.

## Agent aggregation

A node reports agent summaries such as:

```json
{
  "terminal_id": 12,
  "kind": "codex",
  "state": "NeedsApproval"
}
```

The controller merges local and node agent state for one Attention Center view.
Live terminal control remains local to the owning daemon and is bounded by
remote authorization.

## API

### List nodes

```http
GET /nodes
```

### Register

```http
POST /nodes
Content-Type: application/json
X-Orbynode-CSRF: <csrf>
```

### Create pairing

```http
POST /nodes/{id}/pairing
X-Orbynode-CSRF: <csrf>
```

### Revoke

```http
POST /nodes/{id}/revoke
X-Orbynode-CSRF: <csrf>
```

### Node agents

```http
GET /nodes/{id}/agents
```

## Security

- Pairing codes and node secrets are hashed.
- Pairing expires.
- Revocation is immediate at authorization checks.
- Heartbeat proof avoids reusable plaintext secrets.
- Node registration and revocation are audited.
- Remote access must use HTTPS or a trusted overlay.

## Failure and recovery

| Situation | Behavior |
| --- | --- |
| Heartbeat lost | Node becomes offline. |
| Invalid proof | Heartbeat rejected. |
| Node revoked | Further heartbeats rejected. |
| Node lost secret | Re-register and pair again. |
| Controller restored from backup | Pairing/heartbeat state follows database. |

## Operational guidance

- Use unique node names per physical or virtual machine.
- Store node credentials in the node's protected configuration.
- Rotate by revoking and re-pairing.
- Monitor offline nodes from the dashboard.
- Do not share one node identity across hosts.
- Decommission nodes immediately after host teardown.

## Current limitations

- Live remote terminal streaming is not yet exposed through this baseline.
- Certificate-backed transport is modeled by identity and fingerprint work but
  clean-machine deployment remains a release gate.
- Automatic node software updates are not implemented.
- Network partitions converge through next accepted heartbeat and controller
  snapshot state.
