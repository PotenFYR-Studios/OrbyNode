//! OrbyNode agent detection - process + terminal-output analysis (Plan §10,
//! integration Levels 1-2; native hooks are M7).

pub mod attention;

// ---------- Public API ----------

/// Canonical agent identities (Plan §10). Detection patterns live in
/// [`AgentDetector`]; internal states stay stable across UI labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentKind {
    Claude,
    Codex,
    Gemini,
    OpenCode,
    Hermes,
    /// A recognized shell agent that matches no known manifest (§10).
    Unknown,
}

impl AgentKind {
    /// Process-name patterns for Level 1 detection (executable basename).
    pub fn process_names(self) -> &'static [&'static str] {
        match self {
            AgentKind::Claude => &["claude"],
            AgentKind::Codex => &["codex"],
            AgentKind::Gemini => &["gemini"],
            AgentKind::OpenCode => &["opencode"],
            AgentKind::Hermes => &["hermes"],
            AgentKind::Unknown => &[],
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            AgentKind::Claude => "Claude Code",
            AgentKind::Codex => "OpenAI Codex",
            AgentKind::Gemini => "Gemini CLI",
            AgentKind::OpenCode => "OpenCode",
            AgentKind::Hermes => "Hermes Agent",
            AgentKind::Unknown => "Unknown agent",
        }
    }
}

/// Canonical internal states (Plan §10). Semantics stable; labels are UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "PascalCase")]
pub enum AgentState {
    Starting,
    Working,
    Waiting,
    NeedsInput,
    NeedsApproval,
    Idle,
    Completed,
    Failed,
    Disconnected,
    Unknown,
}

/// Where detection runs against output: state inference rules (Level 2).
#[derive(Debug, Clone)]
pub struct StateRule {
    /// Case-insensitive substring matched against recent terminal output.
    pub pattern: &'static str,
    pub state: AgentState,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct DetectedAgent {
    pub kind: AgentKind,
    pub terminal_id: u64,
    pub state: AgentState,
}

// Implemented below: AgentDetector with per-agent state rules.

// ---------- Implementation ----------

/// Detection + state tracking over terminal output events (Level 2, Plan §11).
/// Process-tree detection (Level 1) hooks in at M7+ with the integration
/// manager; output analysis alone satisfies the M6 acceptance.
pub struct AgentDetector {
    bus: orbynode_realtime::EventBus,
    agents: tokio::sync::Mutex<std::collections::HashMap<u64, TrackedAgent>>,
}

#[derive(Debug, Clone)]
struct TrackedAgent {
    kind: AgentKind,
    state: AgentState,
}

impl AgentDetector {
    pub fn new(bus: orbynode_realtime::EventBus) -> Self {
        AgentDetector {
            bus,
            agents: tokio::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }

    /// Feed one terminal-output event. Detection = identity banner match
    /// (stable per-agent patterns), state = latest matching rule.
    pub async fn observe(
        &self,
        event: &orbynode_realtime::Event,
    ) -> Option<orbynode_realtime::Event> {
        if event.etype != "terminal.output" {
            return None;
        }
        let Ok(text) = std::str::from_utf8(&event.bytes) else {
            return None;
        };
        let terminal_id = event
            .stream
            .as_str()
            .strip_prefix("terminal:")
            .and_then(|id| id.parse::<u64>().ok())?;

        let mut agents = self.agents.lock().await;
        let tracked = match agents.get_mut(&terminal_id) {
            Some(t) => t,
            None => {
                let Some(kind) = detect_identity(text) else {
                    return None; // not an agent terminal
                };
                let t = TrackedAgent {
                    kind,
                    state: AgentState::Starting,
                };
                tracing::info!(
                    terminal = terminal_id,
                    agent = kind.display_name(),
                    "agent detected"
                );
                agents.insert(terminal_id, t);
                agents.get_mut(&terminal_id).unwrap()
            }
        };

        let new_state = infer_state(tracked.kind, text).unwrap_or(tracked.state);
        if new_state != tracked.state {
            tracked.state = new_state;
        }
        // Publish existence/state on the agents stream (§58).
        let _ = &self.bus; // bus publish below keeps the API uniform
        Some(self.publish(terminal_id, tracked))
    }

    fn publish(&self, terminal_id: u64, tracked: &TrackedAgent) -> orbynode_realtime::Event {
        let event = orbynode_realtime::Event {
            stream: orbynode_realtime::Stream::new("agents"),
            etype: "agent.detected".into(),
            data: serde_json::json!({
                "terminal_id": terminal_id,
                "kind": tracked.kind,
                "state": tracked.state,
            }),
            priority: orbynode_realtime::Priority::Critical,
            bytes: Vec::new(),
        };
        self.bus.publish(event.clone());
        event
    }

    /// Terminal exited: mark the agent Disconnected (§10; not Deleted - the
    /// session may be resumable later, M7).
    pub async fn mark_exited(&self, terminal_id: u64) {
        let mut agents = self.agents.lock().await;
        if let Some(t) = agents.get_mut(&terminal_id) {
            t.state = AgentState::Disconnected;
        }
    }

    pub async fn list(&self) -> Vec<DetectedAgent> {
        let agents = self.agents.lock().await;
        let mut out: Vec<DetectedAgent> = agents
            .iter()
            .map(|(id, t)| DetectedAgent {
                kind: t.kind,
                terminal_id: *id,
                state: t.state,
            })
            .collect();
        out.sort_by_key(|a| a.terminal_id);
        out
    }
}

/// Identity detection: per-agent case-insensitive banners.
fn detect_identity(text: &str) -> Option<AgentKind> {
    let lower = text.to_lowercase();
    let candidates: &[(AgentKind, &[&str])] = &[
        (AgentKind::Claude, &["claude code", "✻ welcome to claude"]),
        (AgentKind::Codex, &["openai codex", "codex CLI"]),
        (AgentKind::Gemini, &["gemini-cli", "gemini cli"]),
        (AgentKind::OpenCode, &["opencode"]),
        (AgentKind::Hermes, &["hermes agent", "hermes-agent"]),
    ];
    for (kind, needles) in candidates {
        if needles.iter().any(|n| lower.contains(n)) {
            return Some(*kind);
        }
    }
    None
}

/// State rules (Level 2, Plan §11). Order matters: later approvals override
/// earlier generic activity within the same chunk; explicit prompts win.
fn infer_state(kind: AgentKind, text: &str) -> Option<AgentState> {
    let _ = kind; // per-agent manifests arrive with M7 integrations
    let rules: &[(&str, AgentState)] = &[
        // needs-input / approval prompts are the attention-critical states
        ("do you want to proceed", AgentState::NeedsApproval),
        ("allow?", AgentState::NeedsApproval),
        ("permission requested", AgentState::NeedsApproval),
        ("[y/n]", AgentState::NeedsApproval),
        ("enter your message", AgentState::NeedsInput),
        ("type your message", AgentState::NeedsInput),
        ("waiting for input", AgentState::NeedsInput),
        // generic activity
        ("working", AgentState::Working),
        ("running", AgentState::Working),
        ("esc to interrupt", AgentState::Working),
        // completions
        ("session ended", AgentState::Completed),
        ("task complete", AgentState::Completed),
        ("failed", AgentState::Failed),
        ("error:", AgentState::Failed),
    ];
    let lower = text.to_lowercase();
    rules
        .iter()
        .find(|(pattern, _)| lower.contains(pattern))
        .map(|(_, state)| *state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use orbynode_realtime::{Event, EventBus, Priority, ReplayConfig, Stream};
    use std::time::Duration;

    fn out_event(terminal_id: u64, text: &str) -> Event {
        Event {
            stream: Stream::new(format!("terminal:{terminal_id}")),
            etype: "terminal.output".into(),
            data: serde_json::json!({}),
            priority: Priority::Droppable,
            bytes: text.as_bytes().to_vec(),
        }
    }

    #[tokio::test]
    async fn detects_claude_from_process_banner() {
        let bus = EventBus::new(ReplayConfig::default());
        let detector = AgentDetector::new(bus.clone());
        detector
            .observe(&out_event(7, "\r\n✻ Welcome to Claude Code v1.0\r\n"))
            .await;
        let agents = detector.list().await;
        assert_eq!(agents.len(), 1, "one agent detected");
        assert_eq!(agents[0].kind, AgentKind::Claude);
        assert_eq!(agents[0].terminal_id, 7);
    }

    #[tokio::test]
    async fn detects_codex_and_gemini() {
        let bus = EventBus::new(ReplayConfig::default());
        let detector = AgentDetector::new(bus.clone());
        detector
            .observe(&out_event(1, "OpenAI Codex v0.1 - starting"))
            .await;
        detector
            .observe(&out_event(2, "gemini-cli> type your prompt"))
            .await;
        let mut kinds: Vec<(u64, AgentKind)> = detector
            .list()
            .await
            .into_iter()
            .map(|a| (a.terminal_id, a.kind))
            .collect();
        kinds.sort();
        assert_eq!(kinds, vec![(1, AgentKind::Codex), (2, AgentKind::Gemini)]);
    }

    #[tokio::test]
    async fn state_transitions_from_output_patterns() {
        let bus = EventBus::new(ReplayConfig::default());
        let detector = AgentDetector::new(bus.clone());
        detector.observe(&out_event(3, "Claude Code v1.0")).await;
        assert_eq!(detector.list().await[0].state, AgentState::Starting);

        detector
            .observe(&out_event(3, "● Working… (running tests)"))
            .await;
        assert_eq!(detector.list().await[0].state, AgentState::Working);

        detector
            .observe(&out_event(3, "Do you want to proceed? [y/N]"))
            .await;
        assert_eq!(detector.list().await[0].state, AgentState::NeedsApproval);

        detector.observe(&out_event(3, "Enter your message:")).await;
        assert_eq!(detector.list().await[0].state, AgentState::NeedsInput);
    }

    #[tokio::test]
    async fn plain_output_is_not_an_agent() {
        let bus = EventBus::new(ReplayConfig::default());
        let detector = AgentDetector::new(bus.clone());
        detector.observe(&out_event(9, "$ ls -la\r\ntotal 0")).await;
        assert!(detector.list().await.is_empty(), "plain shell output");
    }

    #[tokio::test]
    async fn exit_marks_disconnected() {
        let bus = EventBus::new(ReplayConfig::default());
        let detector = AgentDetector::new(bus.clone());
        detector.observe(&out_event(4, "Hermes Agent ready")).await;
        assert_eq!(detector.list().await.len(), 1);
        detector.mark_exited(4).await;
        let agents = detector.list().await;
        assert_eq!(agents[0].state, AgentState::Disconnected);
    }

    #[tokio::test]
    async fn state_change_publishes_bus_event() {
        let bus = EventBus::new(ReplayConfig::default());
        let detector = AgentDetector::new(bus.clone());
        let mut rx = bus.subscribe(Stream::new("agents"));
        detector.observe(&out_event(5, "Claude Code v1.0")).await;
        let ev = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("bus event")
            .expect("event");
        assert_eq!(ev.event.etype, "agent.detected");
    }
}
