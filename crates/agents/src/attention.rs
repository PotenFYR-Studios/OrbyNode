//! Attention Center (Plan §21, Milestone 12): one queue answering "what needs
//! me right now?" - approvals, blocked agents, failures - with priority
//! classes P0..P5 where P0/P1/P2-critical are never dropped under load.

use orbynode_realtime::{Event, Priority as BusPriority, Stream};

/// Priority classes (Plan §21). Ordering is semantic, not decorative.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "PascalCase")]
pub enum AttentionPriority {
    P0Security,
    P1HumanInput,
    P2AgentFailure,
    P3TaskWorkflow,
    P4Metrics,
    P5Presence,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct AttentionItem {
    pub id: u64,
    pub priority: AttentionPriority,
    /// Stable identity for the aggregate source (task, agent, node, ...).
    pub resource: String,
    /// machine-readable source kind: approval, agent.blocked, task.failed, …
    pub kind: String,
    pub summary: String,
    #[serde(rename = "stream")]
    pub source_stream: String,
    pub created_at: i64,
}

/// In-memory attention store fed by bus events; publishes
/// `attention.created`/`attention.resolved` on the `attention` stream
/// (§58, §21: critical never dropped - criticality rides the bus priority).
pub struct AttentionCenter {
    store: std::sync::Mutex<Vec<AttentionItem>>,
    next_id: std::sync::atomic::AtomicU64,
    bus: orbynode_realtime::EventBus,
    /// Optional external delivery callback (notifications, M15).
    notify: Option<std::sync::Arc<dyn Fn(AttentionItem) + Send + Sync>>,
}

impl AttentionCenter {
    pub fn new(bus: orbynode_realtime::EventBus) -> Self {
        AttentionCenter {
            store: std::sync::Mutex::new(Vec::new()),
            next_id: std::sync::atomic::AtomicU64::new(1),
            bus,
            notify: None,
        }
    }

    /// Attach delivery callback for external channels (webhook, desktop, ...).
    pub fn with_notify(
        mut self,
        notify: std::sync::Arc<dyn Fn(AttentionItem) + Send + Sync>,
    ) -> Self {
        self.notify = Some(notify);
        self
    }

    /// Raise an item when no item for the resource is already open.
    pub fn raise_with_resource(
        &self,
        priority: AttentionPriority,
        kind: &str,
        summary: &str,
        source_stream: &str,
        resource: String,
    ) -> Option<u64> {
        let id = self
            .next_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let item = AttentionItem {
            id,
            priority,
            resource,
            kind: kind.to_owned(),
            summary: summary.to_owned(),
            source_stream: source_stream.to_owned(),
            created_at: now_millis(),
        };
        let mut store = self.store.lock().expect("attention store poisoned");
        if store
            .iter()
            .any(|existing| existing.resource == item.resource)
        {
            return None;
        }
        store.push(item.clone());
        drop(store);
        self.publish_created(&item);
        if let Some(notify) = &self.notify {
            notify(item.clone());
        }
        Some(id)
    }

    /// Compatibility wrapper for callers that do not carry a stable resource.
    pub fn raise(
        &self,
        priority: AttentionPriority,
        kind: &str,
        summary: &str,
        source_stream: &str,
    ) -> Option<u64> {
        self.raise_with_resource(
            priority,
            kind,
            summary,
            source_stream,
            format!("{kind}:{summary}"),
        )
    }

    /// Resolve by id (approval answered, failure fixed, ...).
    pub fn resolve(&self, id: u64) -> bool {
        let removed = {
            let mut store = self.store.lock().expect("attention store poisoned");
            let before = store.len();
            store.retain(|item| item.id != id);
            before != store.len()
        };
        if removed {
            self.publish_resolved(id);
        }
        removed
    }

    /// Resolve every open item for a stable resource when its state clears.
    pub fn resolve_resource(&self, resource: &str) -> usize {
        let ids: Vec<u64> = {
            let mut store = self.store.lock().expect("attention store poisoned");
            let ids = store
                .iter()
                .filter(|item| item.resource == resource)
                .map(|item| item.id)
                .collect();
            store.retain(|item| item.resource != resource);
            ids
        };
        for id in &ids {
            self.publish_resolved(*id);
        }
        ids.len()
    }

    /// Queue sorted by priority then age (what needs me first, §135).
    pub fn list(&self) -> Vec<AttentionItem> {
        let mut items = self.store.lock().expect("attention store poisoned").clone();
        items.sort_by_key(|i| (i.priority, i.created_at));
        items
    }

    /// Wire the center into bus events: agent needs-approval/input from the
    /// agent detector, task failures from task moves, terminal exits.
    pub async fn observe(&self, event: &Event) {
        match event.etype.as_str() {
            "agent.detected" => {
                let state = event
                    .data
                    .get("state")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("");
                let Some(terminal) = event
                    .data
                    .get("terminal_id")
                    .and_then(serde_json::Value::as_u64)
                else {
                    return;
                };
                let resource = format!("agent:terminal:{terminal}");
                let kind_state = match state {
                    "NeedsApproval" => Some((
                        AttentionPriority::P1HumanInput,
                        "agent.blocked",
                        "Agent needs approval",
                    )),
                    "NeedsInput" => Some((
                        AttentionPriority::P1HumanInput,
                        "agent.blocked",
                        "Agent needs input",
                    )),
                    "Failed" => Some((
                        AttentionPriority::P2AgentFailure,
                        "agent.failed",
                        "Agent reported a failure",
                    )),
                    _ => None,
                };
                match kind_state {
                    Some((priority, kind, summary)) => {
                        let agent = event
                            .data
                            .get("kind")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("Agent");
                        self.raise_with_resource(
                            priority,
                            kind,
                            &format!("{agent} on terminal {terminal}: {summary}"),
                            event.stream.as_str(),
                            resource,
                        );
                    }
                    None => {
                        self.resolve_resource(&resource);
                    }
                }
            }
            "task.updated" => {
                let Some(id) = event.data.get("id").and_then(serde_json::Value::as_i64) else {
                    return;
                };
                let state = event
                    .data
                    .get("state")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("");
                let title = event
                    .data
                    .get("title")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("Task");
                let kind_state = match state {
                    "needs_input" => Some((
                        AttentionPriority::P1HumanInput,
                        "task.blocked",
                        "Task needs input",
                    )),
                    "failed" => Some((
                        AttentionPriority::P2AgentFailure,
                        "task.failed",
                        "Task failed",
                    )),
                    _ => None,
                };
                let resource = format!("task:{id}");
                match kind_state {
                    Some((priority, kind, summary)) => {
                        self.raise_with_resource(
                            priority,
                            kind,
                            &format!("{title}: {summary}"),
                            event.stream.as_str(),
                            resource,
                        );
                    }
                    None => {
                        self.resolve_resource(&resource);
                    }
                }
            }
            "task.deleted" => {
                if let Some(id) = event.data.get("id").and_then(serde_json::Value::as_i64) {
                    self.resolve_resource(&format!("task:{id}"));
                }
            }
            _ => {}
        }
    }

    fn publish_created(&self, item: &AttentionItem) {
        let priority = match item.priority {
            AttentionPriority::P0Security
            | AttentionPriority::P1HumanInput
            | AttentionPriority::P2AgentFailure => BusPriority::Critical,
            _ => BusPriority::Droppable,
        };
        self.bus.publish(Event {
            stream: Stream::new("attention"),
            etype: "attention.created".into(),
            data: serde_json::to_value(item).unwrap_or_default(),
            priority,
            bytes: Vec::new(),
        });
    }

    fn publish_resolved(&self, id: u64) {
        self.bus.publish(Event {
            stream: Stream::new("attention"),
            etype: "attention.resolved".into(),
            data: serde_json::json!({"id": id}),
            priority: BusPriority::Critical,
            bytes: Vec::new(),
        });
    }
}

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn center() -> AttentionCenter {
        AttentionCenter::new(orbynode_realtime::EventBus::new(
            orbynode_realtime::ReplayConfig::default(),
        ))
    }

    #[test]
    fn raise_and_list_sorted_by_priority() {
        let c = center();
        c.raise(
            AttentionPriority::P3TaskWorkflow,
            "task.failed",
            "task 7 failed",
            "tasks",
        );
        c.raise(
            AttentionPriority::P1HumanInput,
            "agent.blocked",
            "approval needed",
            "terminal:1",
        );
        c.raise(
            AttentionPriority::P2AgentFailure,
            "agent.failed",
            "crashed",
            "terminal:2",
        );
        let list = c.list();
        let priorities: Vec<_> = list.iter().map(|i| i.priority).collect();
        assert_eq!(
            priorities,
            vec![
                AttentionPriority::P1HumanInput,
                AttentionPriority::P2AgentFailure,
                AttentionPriority::P3TaskWorkflow
            ]
        );
    }

    #[test]
    fn duplicate_raises_are_deduped() {
        let c = center();
        assert!(
            c.raise(
                AttentionPriority::P1HumanInput,
                "agent.blocked",
                "needs input",
                "t"
            )
            .is_some()
        );
        assert!(
            c.raise(
                AttentionPriority::P1HumanInput,
                "agent.blocked",
                "needs input",
                "t"
            )
            .is_none(),
            "same kind+summary deduped"
        );
        assert_eq!(c.list().len(), 1);
    }

    #[test]
    fn resolve_removes_and_reports() {
        let c = center();
        let id = c
            .raise(
                AttentionPriority::P1HumanInput,
                "approval",
                "rm -rf build/",
                "terminal:1",
            )
            .unwrap();
        assert!(c.resolve(id));
        assert!(!c.resolve(id), "second resolve is a no-op");
        assert!(c.list().is_empty());
    }

    #[tokio::test]
    async fn agent_events_become_attention_items() {
        let bus = orbynode_realtime::EventBus::new(orbynode_realtime::ReplayConfig::default());
        let c = AttentionCenter::new(bus.clone());
        c.observe(&Event {
            stream: Stream::new("agents"),
            etype: "agent.detected".into(),
            data: serde_json::json!({"terminal_id": 3, "state": "NeedsApproval"}),
            priority: orbynode_realtime::Priority::Critical,
            bytes: Vec::new(),
        })
        .await;
        let list = c.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].priority, AttentionPriority::P1HumanInput);
        assert_eq!(list[0].kind, "agent.blocked");
    }

    #[tokio::test]
    async fn agent_recovery_resolves_attention() {
        let bus = orbynode_realtime::EventBus::new(orbynode_realtime::ReplayConfig::default());
        let c = AttentionCenter::new(bus);
        let event = |state: &str| Event {
            stream: Stream::new("agents"),
            etype: "agent.detected".into(),
            data: serde_json::json!({"terminal_id": 3, "kind": "Claude", "state": state}),
            priority: orbynode_realtime::Priority::Critical,
            bytes: Vec::new(),
        };

        c.observe(&event("NeedsApproval")).await;
        assert_eq!(c.list().len(), 1);
        c.observe(&event("Working")).await;
        assert!(c.list().is_empty());
    }

    #[tokio::test]
    async fn task_events_raise_and_clear_attention() {
        let bus = orbynode_realtime::EventBus::new(orbynode_realtime::ReplayConfig::default());
        let c = AttentionCenter::new(bus);
        let event = |etype: &str, state: &str| Event {
            stream: Stream::new("tasks"),
            etype: etype.into(),
            data: serde_json::json!({"id": 7, "title": "Ship fix", "state": state}),
            priority: orbynode_realtime::Priority::Critical,
            bytes: Vec::new(),
        };

        c.observe(&event("task.updated", "failed")).await;
        let list = c.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].resource, "task:7");

        c.observe(&event("task.updated", "running")).await;
        assert!(c.list().is_empty());
    }
}
