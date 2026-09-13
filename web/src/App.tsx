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
import { WorkspacesPage } from "./WorkspacesPage";
import { PluginsPage } from "./PluginsPage";

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

type Page = "dashboard" | "workspaces" | "plugins";

export function App() {
  const [page, setPage] = useState<Page>("workspaces");
  const [state, setState] = useState<State>({
    attention: [],
    nodes: [],
    sessions: [],
  });
  const [realtime] = useState(() => new RealtimeClient());

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
  }, [page]);

  useEffect(() => {
    api
      .latestWorkflow()
      .then((workflow) => setState((current) => ({ ...current, workflow: workflow ?? undefined })))
      .catch(() => setState((current) => ({ ...current, workflow: undefined })));
  }, [page]);

  useEffect(() => {
    api.nodes()
      .then((nodes) => setState((current) => ({ ...current, nodes })))
      .catch(() => setState((current) => ({ ...current, nodes: [] })));
  }, [page]);

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

    realtime.connect();
    realtime.sub("attention");
    realtime.sub("notifications");
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
    };
  }, [realtime]);

  // Poll health lightly while the tab is open (status pill only).
  useEffect(() => {
    const id = window.setInterval(() => {
      void api
        .health()
        .then((health) => setState((current) => ({ ...current, health, error: undefined })))
        .catch((e: unknown) => setState((current) => ({ ...current, error: String(e) })));
    }, 15_000);
    return () => window.clearInterval(id);
  }, []);

  const resolve = async (id: number) => {
    await api.resolveAttention(id);
    setState((current) => ({
      ...current,
      attention: current.attention.filter((item) => item.id !== id),
    }));
  };

  const critical = state.attention.filter((a) => a.priority === "P0Security").length;

  return (
    <div className="app-shell">
      <nav className="app-nav" aria-label="Primary">
        <span className="app-brand">◈ ORBYNODE</span>
        {(["workspaces", "dashboard", "plugins"] as Page[]).map((p) => (
          <button
            key={p}
            type="button"
            className={`nav-link ${page === p ? "active" : ""}`}
            onClick={() => setPage(p)}
          >
            {p === "workspaces" ? "Workspaces" : p === "dashboard" ? "Dashboard" : "Plugins"}
          </button>
        ))}
        <span className="nav-status">
          {state.health && (
            <span className={`status-pill ${state.health.status === "ok" ? "ok" : ""}`}>
              {state.health.status}
            </span>
          )}
          {state.version && <span>v{state.version.version}</span>}
          {critical > 0 && <span className="badge badge-warn">{critical} critical</span>}
        </span>
      </nav>

      <main className="app-main">
        {state.error && <p role="alert" className="error-banner">Daemon unreachable: {state.error}</p>}

        {page === "workspaces" && <WorkspacesPage realtime={realtime} />}

        {page === "plugins" && <PluginsPage />}

        {page === "dashboard" && (
          <div className="dash-grid">
            <section className="card" aria-labelledby="attention-heading">
              <h2 id="attention-heading">Attention Center</h2>
              {state.attention.length === 0 ? (
                <p>Nothing needs you right now.</p>
              ) : (
                state.attention.map((item) => (
                  <div key={item.id} className="attention-item">
                    <span className={`badge ${item.priority === "P0Security" ? "badge-warn" : ""}`}>
                      {item.priority}
                    </span>
                    <span>{item.summary}</span>
                    <button
                      type="button"
                      className="btn btn-ghost btn-small"
                      style={{ marginLeft: "auto" }}
                      onClick={() => void resolve(item.id)}
                    >
                      Resolve
                    </button>
                  </div>
                ))
              )}
            </section>

            <section className="card">
              <h2>Host</h2>
              {state.host ? (
                <>
                  <div className="metric-row">
                    <span>CPU</span>
                    <span>{state.host.cpu_percent.toFixed(1)}%</span>
                  </div>
                  <div className="metric-bar">
                    <div
                      className="metric-fill"
                      style={{ width: `${Math.min(100, state.host.cpu_percent)}%` }}
                    />
                  </div>
                  <div className="metric-row">
                    <span>Memory</span>
                    <span>
                      {Math.round(state.host.mem_used_bytes / 1024 / 1024)} /{" "}
                      {Math.round(state.host.mem_total_bytes / 1024 / 1024)} MB
                    </span>
                  </div>
                  <div className="metric-row">
                    <span>Load</span>
                    <span>{state.host.load_avg[0].toFixed(2)}</span>
                  </div>
                </>
              ) : (
                <p>Host metrics unavailable.</p>
              )}
              {state.workflow && (
                <div className="metric-row">
                  <span>Workflow</span>
                  <span>
                    {state.workflow.status === "waiting_approval" ? (
                      <button
                        type="button"
                        className="btn btn-small"
                        onClick={() => {
                          const id = state.workflow?.id;
                          if (!id) return;
                          void api
                            .approveWorkflow(id)
                            .then((workflow) => setState((current) => ({ ...current, workflow })))
                            .catch(() => undefined);
                        }}
                      >
                        Approve {state.workflow.current_stage}
                      </button>
                    ) : (
                      state.workflow.status
                    )}
                  </span>
                </div>
              )}
            </section>

            <section className="card">
              <h2>Remote Nodes</h2>
              {state.nodes.length === 0 ? (
                <p>No remote nodes paired.</p>
              ) : (
                state.nodes.map((node) => (
                  <div key={node.id} className="metric-row">
                    <span>{node.name}</span>
                    <span
                      className={`badge ${node.status === "online" ? "" : "badge-warn"}`}
                    >
                      {node.status}
                    </span>
                  </div>
                ))
              )}
            </section>

            <section className="card">
              <h2>Sessions</h2>
              {state.sessions.length === 0 ? (
                <p>No active sessions.</p>
              ) : (
                state.sessions.map((session) => (
                  <div key={session.terminal_id} className="metric-row">
                    <span>
                      terminal {session.terminal_id}
                      {session.process ? ` · ${session.process}` : ""}
                    </span>
                    <span>
                      {session.cpu_percent.toFixed(1)}% ·{" "}
                      {Math.round(session.rss_bytes / 1024 / 1024)} MB
                    </span>
                  </div>
                ))
              )}
            </section>
          </div>
        )}
      </main>
    </div>
  );
}
