//! OrbyNode notifications - browser bus + generic webhook delivery (Plan §43).

use orbynode_agents::attention::AttentionItem;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::Mutex;

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct NotificationRule {
    pub id: String,
    pub event: String,
    pub project_id: Option<i64>,
    pub channel: String,
    pub target: String,
    pub enabled: bool,
}

#[derive(Clone)]
pub struct NotificationService {
    rules: Arc<RwLock<Vec<NotificationRule>>>,
    recent: Arc<Mutex<HashMap<String, i64>>>,
    client: reqwest::Client,
}

impl NotificationService {
    pub fn new() -> Self {
        NotificationService {
            rules: Arc::new(RwLock::new(Vec::new())),
            recent: Arc::new(Mutex::new(HashMap::new())),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .expect("notification HTTP client"),
        }
    }

    pub async fn list(&self) -> Vec<NotificationRule> {
        self.rules.read().unwrap().clone()
    }

    pub async fn upsert(&self, rule: NotificationRule) {
        let mut rules = self.rules.write().unwrap();
        if let Some(existing) = rules.iter_mut().find(|existing| existing.id == rule.id) {
            *existing = rule;
        } else {
            rules.push(rule);
        }
    }

    pub async fn send(self: std::sync::Arc<Self>, item: AttentionItem) {
        if self.duplicate(&item).await {
            return;
        }
        let rules = self.rules.read().unwrap().clone();
        for rule in rules.iter().filter(|rule| rule.enabled) {
            match rule.channel.as_str() {
                "browser" => continue, // delivered by the `notifications` bus stream
                "webhook" => {
                    let payload = serde_json::json!({
                        "id": item.id,
                        "event": item.kind,
                        "priority": item.priority,
                        "summary": item.summary,
                        "resource": item.resource,
                    });
                    if let Err(error) = self.client.post(&rule.target).json(&payload).send().await {
                        tracing::warn!(error = %error, rule = %rule.id, "webhook delivery failed");
                    }
                }
                unknown => tracing::warn!(channel = %unknown, "unknown notification channel"),
            }
        }
    }

    /// Active-session delivery: bus notifications fan out to every tab, while
    /// `seen` prevents multiple daemon-owned channels repeating one event.
    async fn duplicate(&self, item: &AttentionItem) -> bool {
        let key = format!("{}:{}", item.kind, item.resource);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let mut recent = self.recent.lock().await;
        match recent.get(&key) {
            Some(seen) if now.saturating_sub(*seen) < 300 => true,
            _ => {
                recent.insert(key, now);
                false
            }
        }
    }
}

impl Default for NotificationService {
    fn default() -> Self {
        Self::new()
    }
}

/// Random delivery/rule identifier using the OS CSPRNG.
pub fn new_id() -> String {
    use rand::RngCore;
    let mut bytes = [0_u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::{NotificationRule, NotificationService, new_id};
    use orbynode_agents::attention::{AttentionItem, AttentionPriority};

    fn item(kind: &str, resource: &str) -> AttentionItem {
        AttentionItem {
            id: 1,
            priority: AttentionPriority::P1HumanInput,
            resource: resource.into(),
            kind: kind.into(),
            summary: "test".into(),
            source_stream: "test".into(),
            created_at: 0,
        }
    }

    #[tokio::test]
    async fn rule_upsert_and_list_roundtrip() {
        let service = NotificationService::new();
        let rule = NotificationRule {
            id: new_id(),
            event: "attention.created".into(),
            project_id: Some(3),
            channel: "webhook".into(),
            target: "https://example.invalid/hook".into(),
            enabled: true,
        };
        service.upsert(rule.clone()).await;
        assert_eq!(service.list().await, vec![rule]);
    }

    #[tokio::test]
    async fn repeats_are_deduplicated() {
        let service = NotificationService::new();
        let event = item("agent.blocked", "agent:terminal:7");
        assert!(!service.duplicate(&event).await);
        assert!(service.duplicate(&event).await);
    }
}
