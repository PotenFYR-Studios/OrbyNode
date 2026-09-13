import { useEffect, useState } from "react";
import {
  api,
  type AttentionItem,
  type Health,
  type HostMetrics,
  type RemoteNode,
  type SessionMetrics,
  type Version,
  type WorkflowRun,
} from "./api";
import { RealtimeClient } from "./realtime";

interface State {
  health?: Health;
  version?: Version;
  error?: string;
  attention: AttentionItem[];
  nodes: RemoteNode[];
  host?: HostMetrics;
  sessions: SessionMetrics[];
  workflow?: WorkflowRun;
}

export function App() {
  const [state, setState] = useState<State>({
    attention: [],
    nodes: [],
    sessions: [],
  });

  useEffect(() => {
    if (!("Notification" in window)) return;
    if (Notification.permission === "default") void Notification.requestPermission();
  }, []);

  useEffect(() => {
    Promise.all([api.health(), api.version()])
      .then(([health, version]) => setState((current) => ({ ...current, health, version })))
      .catch((e: unknown) => setState((current) => ({ ...current, error: String(e) })));
  }, []);

  useEffect(() => {
    Promise.all([api.hostMetrics(), api.sessionMetrics()])
      .then(([host, sessions]) => setState((current) => ({ ...current, host, sessions })))
      .catch(() => setState((current) => ({ ...current, sessions: [] })));
  }, []);

  useEffect(() => {
    api
      .latestWorkflow()
      .then((workflow) => setState((current) => ({ ...current, workflow: workflow ?? undefined })))
      .catch(() => setState((current) => ({ ...current, workflow: undefined })));
  }, []);

  useEffect(() => {
    api.nodes()
      .then((nodes) => setState((current) => ({ ...current, nodes })))
      .catch(() => setState((current) => ({ ...current, nodes: [] })));
  }, []);

  useEffect(() => {
    let mounted = true;
    const sync = async () => {
      try {
        const attention = await api.attention();
        if (mounted) setState((current) => ({ ...current, attention }));
      } catch {
        // Auth and connection failures are shown by the health status.
      }
    };
    void sync();

    const realtime = new RealtimeClient();
    const removeListener = realtime.on("attention", (event) => {
      if (event.etype === "attention.resolved") {
        const resolved = (event.data as { id?: number }).id;
        setState((current) => ({
          ...current,
          attention: current.attention.filter((item) => item.id !== resolved),
        }));
        return;
      }
      void sync();
    });
    realtime.connect();
    realtime.sub("attention");
    realtime.sub("notifications");
    const removeNotificationListener = realtime.on("notifications", (event) => {
      const data = event.data as { summary?: string; priority?: string };
      if ("Notification" in window && Notification.permission === "granted") {
        new Notification("OrbyNode", {
          body: `${data.priority ?? ""} ${data.summary ?? ""}`.trim(),
        });
      }
    });
    return () => {
      mounted = false;
      removeListener();
      removeNotificationListener();
      realtime.close();
    };
  }, []);

  const resolve = async (id: number) => {
    await api.resolveAttention(id);
    setState((current) => ({
      ...current,
      attention: current.attention.filter((item) => item.id !== id),
    }));
  };

  return (
    <main style={{ fontFamily: "system-ui, sans-serif", maxWidth: 640, margin: "4rem auto", padding: "0 1rem" }}>
      <h1>OrbyNode</h1>
      <p>The self-hosted control plane for coding agents.</p>
      {state.error && <p role="alert">Daemon unreachable: {state.error}</p>}
      <section aria-labelledby="attention-heading">
        <h2 id="attention-heading">Attention Center</h2>
        {state.attention.length === 0 ? (
          <p>Nothing needs you right now.</p>
        ) : (
          <ul>
            {state.attention.map((item) => (
              <li key={item.id}>
                <strong>{item.priority}</strong> - {item.summary}
                <button type="button" onClick={() => void resolve(item.id)}>
                  Resolve
                </button>
              </li>
            ))}
          </ul>
        )}
      </section>
      <section aria-labelledby="workflows-heading">
        <h2 id="workflows-heading">Workflows</h2>
        {state.workflow ? (
          <p>
            <strong>{state.workflow.status}</strong> - {state.workflow.current_stage}
            {state.workflow.status === "waiting_approval" && (
              <button
                type="button"
                onClick={() => {
                  const id = state.workflow?.id;
                  if (!id) return;
                  void api
                    .approveWorkflow(id)
                    .then((workflow) => setState((current) => ({ ...current, workflow })))
                    .catch(() => undefined);
                }}
              >
                Approve
              </button>
            )}
          </p>
        ) : (
          <button
            type="button"
            onClick={() =>
              void api
                .startWorkflow("delivery")
                .then((workflow) => setState((current) => ({ ...current, workflow })))
                .catch(() => undefined)
            }
          >
            Start delivery workflow
          </button>
        )}
      </section>
      <section aria-labelledby="nodes-heading">
        <h2 id="nodes-heading">Remote Nodes</h2>
        {state.nodes.length === 0 ? (
          <p>No remote nodes paired.</p>
        ) : (
          <ul>
            {state.nodes.map((node) => (
              <li key={node.id}>
                <strong>{node.name}</strong> - {node.status}
                {node.agents.length > 0 && (
                  <ul>
                    {node.agents.map((agent) => (
                      <li key={`${node.id}:${String(agent.terminal_id)}`}>
                        {String(agent.kind)} on terminal {String(agent.terminal_id)}: {String(agent.state)}
                      </li>
                    ))}
                  </ul>
                )}
              </li>
            ))}
          </ul>
        )}
      </section>
      <section aria-labelledby="observability-heading">
        <h2 id="observability-heading">Observability</h2>
        {state.host ? (
          <p>
            CPU {state.host.cpu_percent.toFixed(1)}% · RAM{" "}
            {Math.round(state.host.mem_used_bytes / 1024 / 1024)} /{" "}
            {Math.round(state.host.mem_total_bytes / 1024 / 1024)} MB · load{" "}
            {state.host.load_avg[0].toFixed(2)}
          </p>
        ) : (
          <p>Host metrics unavailable.</p>
        )}
        {state.sessions.length > 0 && (
          <table>
            <thead>
              <tr>
                <th>Terminal</th>
                <th>CPU</th>
                <th>RAM</th>
                <th>Runtime</th>
              </tr>
            </thead>
            <tbody>
              {state.sessions.map((session) => (
                <tr key={session.terminal_id}>
                  <td>{session.terminal_id}</td>
                  <td>{session.cpu_percent.toFixed(1)}%</td>
                  <td>{Math.round(session.rss_bytes / 1024 / 1024)} MB</td>
                  <td>{session.runtime_secs}s</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>
      <section aria-labelledby="notifications-heading">
        <h2 id="notifications-heading">Notifications</h2>
        <button
          type="button"
          onClick={() =>
            void api
              .upsertNotificationRule({
                event: "attention.created",
                channel: "webhook",
                target: "https://example.invalid/hook",
                enabled: true,
              })
              .catch(() => undefined)
          }
        >
          Add test webhook rule
        </button>
      </section>
      {state.health && (
        <dl>
          <dt>Status</dt>
          <dd>{state.health.status}</dd>
          <dt>Uptime</dt>
          <dd>{state.health.uptime_secs}s</dd>
          <dt>Version</dt>
          <dd>
            {state.version?.name} {state.version?.version}
          </dd>
        </dl>
      )}
    </main>
  );
}
