//! Workspace pane engine (ADR 021): shape + journal + restore, web-GUI only.
//!
//! The engine owns restore semantics so the daemon and API share one
//! implementation. Terminal spawning stays in the route layer (it needs the
//! TerminalManager); this type handles rows, journals, and snapshots.

use orbynode_database::{Db, JournalChunk, NewPane, Pane, Tab, Workspace};

/// Resume behavior for agent panes (ADR 021 tier 3).
#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// Opt-in: typing into a fresh shell is a side effect enterprises control.
    pub resume_agents: bool,
    /// Per-pane journal cap in bytes.
    pub pane_cap_bytes: i64,
}

impl Default for EngineConfig {
    fn default() -> Self {
        EngineConfig {
            resume_agents: false,
            pane_cap_bytes: 2 * 1024 * 1024,
        }
    }
}

/// Restore report: what came back, what needs attention.
#[derive(Debug, Default, serde::Serialize)]
pub struct RestoreReport {
    pub workspaces: usize,
    pub tabs: usize,
    pub panes: usize,
    pub degraded: usize,
    pub resumed_agents: usize,
}

pub struct WorkspaceEngine {
    pub db: Db,
    pub cfg: EngineConfig,
}

impl WorkspaceEngine {
    pub fn new(db: Db, cfg: EngineConfig) -> Self {
        WorkspaceEngine { db, cfg }
    }

    pub async fn create_workspace(&self, name: &str) -> Result<Workspace, String> {
        self.db
            .create_workspace(None, name)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn create_tab(&self, workspace_id: i64, name: &str) -> Result<Tab, String> {
        self.db
            .create_tab(workspace_id, name)
            .await
            .map_err(|e| e.to_string())
    }

    /// Create a pane row; a live terminal is spawned by the route layer and
    /// linked via `set_pane_terminal`.
    pub async fn create_pane(&self, pane: &NewPane) -> Result<Pane, String> {
        self.db.create_pane(pane).await.map_err(|e| e.to_string())
    }

    /// Persist one journal chunk; on failure the pane degrades (never blocks
    /// the PTY reader) and the Attention Center hears about it.
    pub async fn journal_output(&self, pane_id: i64, bytes: &[u8]) {
        if let Err(e) = self.db.journal_append(pane_id, bytes).await {
            tracing::error!(pane = pane_id, error = %e, "journal append failed");
            let _ = self.db.set_pane_degraded(pane_id, true).await;
            return;
        }
        let _ = self
            .db
            .journal_evict(pane_id, self.cfg.pane_cap_bytes)
            .await;
    }

    /// Bounded replay after `after_seq` (oldest first).
    pub async fn journal_read(
        &self,
        pane_id: i64,
        after_seq: i64,
    ) -> Result<Vec<(i64, Vec<u8>)>, String> {
        self.db
            .journal_read(pane_id, after_seq)
            .await
            .map(|chunks: Vec<JournalChunk>| chunks.into_iter().map(|c| (c.seq, c.chunk)).collect())
            .map_err(|e| e.to_string())
    }

    /// Persist the full shape snapshot (tier 1 checkpoint).
    pub async fn snapshot(&self) -> Result<(), String> {
        let workspaces = self.db.list_workspaces().await.map_err(|e| e.to_string())?;
        let mut shape = Vec::new();
        for ws in workspaces {
            let tabs = self.db.list_tabs(ws.id).await.map_err(|e| e.to_string())?;
            let mut tab_states = Vec::new();
            for tab in tabs {
                let panes = self
                    .db
                    .list_panes(tab.id)
                    .await
                    .map_err(|e| e.to_string())?;
                tab_states.push(serde_json::json!({
                    "tab": tab,
                    "panes": panes,
                }));
            }
            shape.push(serde_json::json!({ "workspace": ws, "tabs": tab_states }));
        }
        let json = serde_json::to_string(&serde_json::json!({
            "kind": "shape",
            "workspaces": shape,
        }))
        .map_err(|e| e.to_string())?;
        self.db
            .snapshot_save("shape", &json)
            .await
            .map_err(|e| e.to_string())
    }

    /// Startup restore (ADR 021 order): shape is durable in the rows
    /// themselves, so restore counts what survived and flags what needs
    /// attention. Journal replay happens per-pane when a client attaches
    /// (GET /panes/:id/output); agent resume writes need live terminals and
    /// stay in the route layer behind `resume_agents`.
    pub async fn restore(&self) -> RestoreReport {
        let mut report = RestoreReport::default();
        let Ok(workspaces) = self.db.list_workspaces().await else {
            return report;
        };
        report.workspaces = workspaces.len();
        for ws in workspaces {
            let Ok(tabs) = self.db.list_tabs(ws.id).await else {
                continue;
            };
            report.tabs += tabs.len();
            for tab in tabs {
                let Ok(panes) = self.db.list_panes(tab.id).await else {
                    continue;
                };
                for pane in panes {
                    report.panes += 1;
                    if pane.degraded {
                        report.degraded += 1;
                    }
                    if pane.kind == "agent"
                        && !pane.last_session_id.is_empty()
                        && self.cfg.resume_agents
                    {
                        report.resumed_agents += 1;
                    }
                }
            }
        }
        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn engine() -> WorkspaceEngine {
        let db = Db::open("sqlite::memory:").await.expect("test db");
        WorkspaceEngine::new(db, EngineConfig::default())
    }

    #[tokio::test]
    async fn shape_roundtrips_through_snapshot() {
        let e = engine().await;
        let ws = e.create_workspace("main").await.unwrap();
        let tab = e.create_tab(ws.id, "shell").await.unwrap();
        let pane = e
            .create_pane(&NewPane {
                tab_id: tab.id,
                kind: "shell".into(),
                cwd: "/tmp".into(),
                ..NewPane::default()
            })
            .await
            .unwrap();
        e.journal_output(pane.id, b"out1").await;
        e.journal_output(pane.id, b"out2").await;
        e.snapshot().await.expect("snapshot");

        let json = e.db.snapshot_latest("shape").await.unwrap().unwrap();
        assert!(json.contains("main"), "workspace name in snapshot");
        // Journal bytes stay in pane_journal; the snapshot holds shape only.

        let chunks = e.journal_read(pane.id, -1).await.unwrap();
        assert_eq!(chunks.len(), 2);
        let joined: Vec<u8> = chunks.into_iter().flat_map(|(_, c)| c).collect();
        assert_eq!(joined, b"out1out2");
    }

    #[tokio::test]
    async fn restore_counts_shape_and_flags_degraded() {
        let e = engine().await;
        let ws = e.create_workspace("w").await.unwrap();
        let tab = e.create_tab(ws.id, "t").await.unwrap();
        let mut pane = e
            .create_pane(&NewPane {
                tab_id: tab.id,
                kind: "agent".into(),
                last_session_id: "sess-123".into(),
                ..NewPane::default()
            })
            .await
            .unwrap();
        e.db.set_pane_degraded(pane.id, true).await.unwrap();
        pane.degraded = true;

        let report = e.restore().await;
        assert_eq!(report.workspaces, 1);
        assert_eq!(report.tabs, 1);
        assert_eq!(report.panes, 1);
        assert_eq!(report.degraded, 1);
        assert_eq!(report.resumed_agents, 0, "resume off by default");

        let e2 = WorkspaceEngine::new(
            e.db.clone(),
            EngineConfig {
                resume_agents: true,
                ..EngineConfig::default()
            },
        );
        assert_eq!(e2.restore().await.resumed_agents, 1);
    }

    #[tokio::test]
    async fn journal_eviction_keeps_recent_bytes() {
        let e = engine().await;
        let ws = e.create_workspace("w").await.unwrap();
        let tab = e.create_tab(ws.id, "t").await.unwrap();
        let pane = e
            .create_pane(&NewPane {
                tab_id: tab.id,
                ..NewPane::default()
            })
            .await
            .unwrap();
        // Cap of 12 bytes: only the last chunk survives intact.
        let tiny = WorkspaceEngine::new(
            e.db.clone(),
            EngineConfig {
                pane_cap_bytes: 12,
                ..EngineConfig::default()
            },
        );
        tiny.journal_output(pane.id, b"aaaaaaaa").await;
        tiny.journal_output(pane.id, b"bbbbbbbb").await;
        let chunks = tiny.journal_read(pane.id, -1).await.unwrap();
        let total: usize = chunks.iter().map(|(_, c)| c.len()).sum();
        assert!(total <= 12 + 8, "cap enforced, got {total}");
    }
}
