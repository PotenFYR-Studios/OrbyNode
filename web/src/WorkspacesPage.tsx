/**
 * Workspace pane surface (ADR 021): workspace sidebar, tab strip, pane grid.
 * Web GUI is the only pane interface; every pane is a real PTY.
 */

import { useEffect, useMemo, useState } from "react";
import type { RealtimeClient } from "./realtime";
import {
  workspaceApi,
  resumeCommand,
  type Pane,
  type WorkspaceWithTabs,
} from "./workspace";
import { TerminalView } from "./TerminalView";
import { FileExplorer } from "./FileExplorer";

interface Props {
  realtime: RealtimeClient;
}

export function WorkspacesPage({ realtime }: Props) {
  const [workspaces, setWorkspaces] = useState<WorkspaceWithTabs[]>([]);
  const [activeWs, setActiveWs] = useState<number | null>(null);
  const [activeTab, setActiveTab] = useState<number | null>(null);
  const [focusedPane, setFocusedPane] = useState<number | null>(null);
  const [projectId, setProjectId] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);

  const reload = () => {
    workspaceApi
      .list()
      .then((list) => {
        setWorkspaces(list);
        if (list.length > 0) {
          setActiveWs((cur) => cur ?? list[0]!.workspace.id);
        }
      })
      .catch((e: unknown) => setError(String(e)));
  };

  useEffect(reload, []);
  useEffect(() => {
    // Workspace/pane events refresh the tree (created, closed, restored).
    const remove = realtime.on("workspace", (ev) => {
      if (ev.etype.startsWith("workspace.") || ev.etype.startsWith("pane.")) reload();
    });
    realtime.sub("workspace");
    return remove;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [realtime]);

  const current = useMemo(
    () => workspaces.find((w) => w.workspace.id === activeWs) ?? null,
    [workspaces, activeWs],
  );
  const tabs = current?.tabs ?? [];
  const tab = tabs.find((t) => t.tab.id === activeTab) ?? tabs[0] ?? null;
  const panes = tab?.panes ?? [];

  const createWorkspace = () => {
    const name = prompt("Workspace name")?.trim();
    if (!name) return;
    void workspaceApi
      .createWorkspace(name)
      .then((ws) => {
        reload();
        setActiveWs(ws.id);
      })
      .catch((e: unknown) => setError(String(e)));
  };

  const createTab = () => {
    if (!activeWs) return;
    const name = prompt("Tab name")?.trim() || "tab";
    void workspaceApi
      .createTab(activeWs, name)
      .then(() => reload())
      .catch((e: unknown) => setError(String(e)));
  };

  const createPane = (kind: "shell" | "agent") => {
    if (!tab) return;
    void workspaceApi
      .createPane(tab.tab.id, { kind, title: kind === "agent" ? "agent" : "shell" })
      .then(() => reload())
      .catch((e: unknown) => setError(String(e)));
  };

  const closePane = (id: number) => {
    void workspaceApi.closePane(id).then(reload).catch((e: unknown) => setError(String(e)));
  };

  const resumeAgent = (pane: Pane) => {
    const cmd = resumeCommand(pane.kind, pane.last_session_id);
    if (!cmd) return;
    void workspaceApi
      .sendInput(pane.id, `${cmd}\n`)
      .then(() => setFocusedPane(pane.id))
      .catch((e: unknown) => setError(String(e)));
  };

  return (
    <div className="workspace-page">
      {error && <p role="alert" className="error-banner">{error}</p>}
      <div className="workspace-layout">
        <aside className="ws-sidebar">
          <div className="ws-sidebar-head">
            <h2>Workspaces</h2>
            <button type="button" className="btn btn-ghost" onClick={createWorkspace} title="New workspace">
              +
            </button>
          </div>
          <ul className="ws-list">
            {workspaces.map(({ workspace }) => (
              <li key={workspace.id}>
                <button
                  type="button"
                  className={`ws-item ${workspace.id === activeWs ? "active" : ""}`}
                  onClick={() => {
                    setActiveWs(workspace.id);
                    setActiveTab(null);
                  }}
                >
                  {workspace.name}
                </button>
              </li>
            ))}
            {workspaces.length === 0 && <li className="ws-empty">No workspaces yet.</li>}
          </ul>
          {projectId !== null && (
            <div className="ws-explorer">
              <h3>Files</h3>
              <FileExplorer projectId={projectId} realtime={realtime} />
            </div>
          )}
        </aside>

        <section className="ws-main">
          <div className="tab-strip" role="tablist">
            {tabs.map(({ tab: t, panes: ps }) => (
              <button
                key={t.id}
                type="button"
                role="tab"
                aria-selected={t.id === tab?.tab.id}
                className={`tab ${t.id === tab?.tab.id ? "active" : ""}`}
                onClick={() => setActiveTab(t.id)}
              >
                {t.name}
                <span className="tab-count">{ps.length}</span>
              </button>
            ))}
            <button type="button" className="btn btn-ghost" onClick={createTab} title="New tab">
              +
            </button>
          </div>

          <div className="pane-toolbar">
            <button type="button" className="btn" onClick={() => createPane("shell")}>
              New shell
            </button>
            <button type="button" className="btn" onClick={() => createPane("agent")}>
              New agent pane
            </button>
            <label className="project-picker">
              Project
              <input
                type="number"
                min={1}
                placeholder="id"
                value={projectId ?? ""}
                onChange={(e) =>
                  setProjectId(e.target.value === "" ? null : Number(e.target.value))
                }
              />
            </label>
          </div>

          {tab && panes.length === 0 && (
            <div className="pane-empty">
              <p>No panes in this tab.</p>
              <button type="button" className="btn btn-primary" onClick={() => createPane("shell")}>
                Open a shell
              </button>
            </div>
          )}

          <div className="pane-grid" style={{ gridTemplateColumns: `repeat(${Math.ceil(panes.length / 2) || 1}, 1fr)` }}>
            {panes.map((pane) => (
              <div
                key={pane.id}
                className={`pane-card ${focusedPane === pane.id ? "focused" : ""} ${pane.degraded ? "degraded" : ""}`}
                onMouseDown={() => setFocusedPane(pane.id)}
              >
                <header className="pane-head">
                  <span className={`pane-dot kind-${pane.kind}`} aria-hidden />
                  <span className="pane-title">{pane.title || `pane ${pane.id}`}</span>
                  {pane.kind === "agent" && pane.last_session_id && (
                    <button
                      type="button"
                      className="btn btn-small"
                      onClick={() => resumeAgent(pane)}
                      title={`Resume session ${pane.last_session_id}`}
                    >
                      Resume
                    </button>
                  )}
                  {pane.degraded && <span className="badge badge-warn">degraded</span>}
                  <button
                    type="button"
                    className="btn btn-ghost btn-small pane-close"
                    onClick={() => closePane(pane.id)}
                    aria-label={`Close pane ${pane.id}`}
                  >
                    ✕
                  </button>
                </header>
                <TerminalView
                  paneId={pane.id}
                  terminalId={pane.terminal_id}
                  realtime={realtime}
                  focused={focusedPane === pane.id}
                />
              </div>
            ))}
          </div>
        </section>
      </div>
    </div>
  );
}
