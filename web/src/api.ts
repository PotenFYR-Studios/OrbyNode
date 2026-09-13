/** Typed client for the daemon REST API. */

import type { Pane, WorkspaceWithTabs } from "./workspace";

export interface Health {
  status: string;
  uptime_secs: number;
}

export interface Version {
  name: string;
  version: string;
}

export interface AttentionItem {
  id: number;
  priority: "P0Security" | "P1HumanInput" | "P2AgentFailure" | "P3TaskWorkflow" | "P4Metrics" | "P5Presence";
  resource: string;
  kind: string;
  summary: string;
  source_stream: string;
  created_at: number;
}

export interface RemoteNode {
  id: string;
  name: string;
  fingerprint: string;
  status: "pending" | "online" | "offline" | "revoked";
  last_seen_at?: number;
  agents: Array<{
    terminal_id: number;
    kind: string;
    state: string;
  }>;
}

export interface HostMetrics {
  cpu_percent: number;
  mem_total_bytes: number;
  mem_used_bytes: number;
  uptime_secs: number;
  load_avg: [number, number, number];
}

export interface SessionMetrics {
  terminal_id: number;
  pid?: number;
  process?: string;
  cpu_percent: number;
  rss_bytes: number;
  runtime_secs: number;
  child_count: number;
  listening_ports: number[];
}

export type WorkflowStatus =
  | "running"
  | "waiting_approval"
  | "completed"
  | "failed"
  | "cancelled";

export interface WorkflowRun {
  id: string;
  definition_id: string;
  status: WorkflowStatus;
  current_stage: string;
  variables: Record<string, string>;
  created_at: number;
  updated_at: number;
}

async function getJson<T>(path: string): Promise<T> {
  const res = await fetch(path);
  if (!res.ok) throw new Error(`${path}: ${res.status}`);
  return res.json() as Promise<T>;
}

export const api = {
  health: () => getJson<Health>("/health"),
  version: () => getJson<Version>("/version"),
  attention: () => getJson<AttentionItem[]>("/attention"),
  nodes: () => getJson<RemoteNode[]>("/nodes"),
  hostMetrics: () => getJson<HostMetrics>("/observability/host"),
  sessionMetrics: () => getJson<SessionMetrics[]>("/observability/sessions"),
  latestWorkflow: () => getJson<WorkflowRun | null>("/workflow-runs/latest"),
  startWorkflow: async (name: string) => {
    const res = await fetch(`/workflows/${encodeURIComponent(name)}/start`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ variables: {} }),
    });
    if (!res.ok) throw new Error(`workflows/${name}/start: ${res.status}`);
    return (await res.json()) as WorkflowRun;
  },
  approveWorkflow: async (id: string) => {
    const res = await fetch(`/workflow-runs/${encodeURIComponent(id)}/approve`, {
      method: "POST",
    });
    if (!res.ok) throw new Error(`workflow-runs/${id}/approve: ${res.status}`);
    return (await res.json()) as WorkflowRun;
  },
  upsertNotificationRule: async (input: {
    event: string;
    project_id?: number;
    channel: string;
    target: string;
    enabled: boolean;
  }) => {
    const res = await fetch("/notifications/rules", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(input),
    });
    if (!res.ok) throw new Error(`notifications/rules: ${res.status}`);
  },
  resolveAttention: async (id: number) => {
    const res = await fetch(`/attention/${id}/resolve`, { method: "POST" });
    if (!res.ok) throw new Error(`attention/${id}/resolve: ${res.status}`);
  },
  listWorkspaces: async () => {
    const data = await getJson<{ workspaces: WorkspaceWithTabs[] }>("/workspaces");
    return data.workspaces;
  },
  listPanes: async (tabId: number) => {
    const data = await getJson<Pane[]>(`/tabs/${tabId}/panes`);
    return data;
  },
};
