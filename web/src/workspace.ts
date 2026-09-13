/**
 * Workspace model + client for the pane surface (ADR 021).
 * Web GUI is the only pane interface; every pane is a real PTY behind
 * /panes/:id/input and the terminal:<id> realtime stream.
 */

import { api } from "./api";

export interface Workspace {
  id: number;
  project_id: number | null;
  name: string;
  position: number;
  created_at: number;
}

export interface Tab {
  id: number;
  workspace_id: number;
  name: string;
  position: number;
  active_pane_id: number | null;
}

export interface Pane {
  id: number;
  tab_id: number;
  terminal_id: number | null;
  kind: "shell" | "agent" | string;
  cwd: string;
  title: string;
  split_dir: string;
  split_ratio: number | null;
  position: number;
  last_session_id: string;
  degraded: boolean;
}

export interface TabWithPanes {
  tab: Tab;
  panes: Pane[];
}

export interface WorkspaceWithTabs {
  workspace: Workspace;
  tabs: TabWithPanes[];
}

async function send<T>(path: string, method: string, body?: unknown): Promise<T> {
  const res = await fetch(path, {
    method,
    headers: body === undefined ? undefined : { "content-type": "application/json" },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  if (!res.ok) throw new Error(`${method} ${path}: ${res.status}`);
  return (res.status === 204 ? (undefined as T) : ((await res.json()) as T));
}

export const workspaceApi = {
  list: () => api.listWorkspaces(),
  createWorkspace: (name: string) =>
    send<Workspace>("/workspaces", "POST", { name }),
  renameWorkspace: (id: number, name: string) =>
    send<void>(`/workspaces/${id}`, "PATCH", { name }),
  deleteWorkspace: (id: number) => send<void>(`/workspaces/${id}`, "DELETE"),
  createTab: (workspaceId: number, name: string) =>
    send<Tab>(`/workspaces/${workspaceId}/tabs`, "POST", { name }),
  renameTab: (id: number, name: string) => send<void>(`/tabs/${id}`, "PATCH", { name }),
  deleteTab: (id: number) => send<void>(`/tabs/${id}`, "DELETE"),
  listPanes: (tabId: number) => api.listPanes(tabId),
  createPane: (tabId: number, opts?: { cwd?: string; kind?: string; title?: string }) =>
    send<Pane>(`/tabs/${tabId}/panes`, "POST", {
      tab_id: tabId,
      kind: opts?.kind ?? "shell",
      cwd: opts?.cwd ?? "",
      title: opts?.title ?? "",
    }),
  closePane: (id: number) => send<void>(`/panes/${id}`, "DELETE"),
  renamePane: (id: number, title: string) => send<void>(`/panes/${id}`, "PATCH", { title }),
  splitPane: (id: number, dir: string, ratio: number, position: number) =>
    send<void>(`/panes/${id}`, "PATCH", {
      split_dir: dir,
      split_ratio: ratio,
      position,
    }),
  sendInput: (id: number, data: string) =>
    send<void>(`/panes/${id}/input`, "POST", { data }),
  output: (id: number, sinceSeq: number) =>
    send<{ chunks: Array<{ seq: number; text: string }> }>(
      `/panes/${id}/output?since_seq=${sinceSeq}`,
      "GET",
    ),
};

/** Agent resume command per known agent (tier-3 restore, docs/agents.md). */
export function resumeCommand(kind: string, sessionId: string): string | null {
  if (!sessionId) return null;
  switch (kind) {
    case "claude":
      return `claude --resume ${sessionId}`;
    case "codex":
      return `codex resume ${sessionId}`;
    case "gemini":
      return `gemini --resume ${sessionId}`;
    case "opencode":
      return `opencode --resume ${sessionId}`;
    case "hermes":
      return `hermes --resume ${sessionId}`;
    default:
      return null;
  }
}
