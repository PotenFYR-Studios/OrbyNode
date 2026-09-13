use orbynode_database::Db;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeStatus {
    Pending,
    Online,
    Offline,
    Revoked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Node {
    pub id: String,
    pub name: String,
    pub fingerprint: String,
    pub public_key: String,
    pub status: NodeStatus,
    pub last_seen_at: Option<i64>,
    pub created_at: i64,
    pub paired_at: Option<i64>,
    /// Node-held proof secret; excluded from dashboard serialization.
    #[serde(skip)]
    pub secret: String,
}

pub type NodeRecord = Node;

#[derive(Debug, Clone, Serialize)]
pub struct Pairing {
    pub node_id: String,
    pub code: String,
    pub expires_at: i64,
    /// Returned once after pairing confirmation; stored only as a hash.
    pub secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heartbeat {
    pub node_id: String,
    pub timestamp: i64,
    pub agents: Vec<serde_json::Value>,
}

#[derive(Clone)]
pub struct NodeRegistry {
    db: Db,
    pending: Arc<Mutex<HashMap<String, String>>>,
}

impl NodeRegistry {
    pub fn new(db: Db) -> Self {
        NodeRegistry {
            db,
            pending: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn register(
        &self,
        name: &str,
        fingerprint: &str,
        public_key: &str,
    ) -> anyhow::Result<Node> {
        let now = now();
        let id = random_hex(16);
        let node = Node {
            id: id.clone(),
            name: name.to_owned(),
            fingerprint: fingerprint.to_owned(),
            public_key: public_key.to_owned(),
            status: NodeStatus::Pending,
            last_seen_at: None,
            created_at: now,
            paired_at: None,
            secret: String::new(),
        };
        sqlx::query("INSERT INTO nodes (id, name, fingerprint, public_key, status, created_at) VALUES (?, ?, ?, ?, 'pending', ?)")
            .bind(&node.id)
            .bind(name)
            .bind(fingerprint)
            .bind(public_key)
            .bind(now)
            .execute(&self.db.pool)
            .await?;
        Ok(node)
    }

    pub async fn create_pairing(&self, node_id: &str) -> anyhow::Result<Pairing> {
        let code = format!("{}-{}", random_hex(4), random_hex(4));
        let expires_at = now() + 600;
        sqlx::query("INSERT INTO pairing_codes (code_hash, node_id, expires_at, created_at) VALUES (?, ?, ?, ?)")
            .bind(hash(&code))
            .bind(node_id)
            .bind(expires_at)
            .bind(now())
            .execute(&self.db.pool)
            .await?;
        self.pending
            .lock()
            .await
            .insert(hash(&code), node_id.to_owned());
        Ok(Pairing {
            node_id: node_id.to_owned(),
            code,
            expires_at,
            secret: String::new(),
        })
    }

    pub async fn confirm_pairing(&self, code: &str) -> anyhow::Result<Node> {
        let digest = hash(code);
        let expires_at: i64 =
            sqlx::query_scalar("SELECT expires_at FROM pairing_codes WHERE code_hash = ?")
                .bind(&digest)
                .fetch_one(&self.db.pool)
                .await
                .map_err(|_| anyhow::anyhow!("invalid pairing code"))?;
        if expires_at <= now() {
            return Err(anyhow::anyhow!("pairing code expired"));
        }
        let id: String = self
            .pending
            .lock()
            .await
            .get(&digest)
            .ok_or_else(|| anyhow::anyhow!("invalid pairing code"))?
            .clone();
        let secret = random_hex(32);
        let paired_at = now();
        sqlx::query(
            "UPDATE nodes SET status = 'online', paired_at = ?, secret_hash = ? WHERE id = ? AND status = 'pending'",
        )
        .bind(paired_at)
        .bind(hash(&secret))
        .bind(&id)
        .execute(&self.db.pool)
        .await?;
        sqlx::query("DELETE FROM pairing_codes WHERE code_hash = ?")
            .bind(&digest)
            .execute(&self.db.pool)
            .await?;
        self.pending.lock().await.remove(&digest);
        let mut node = self
            .node(&id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("node missing"))?;
        node.secret = secret;
        Ok(node)
    }

    pub async fn heartbeat(&self, heartbeat: Heartbeat, proof: &str) -> anyhow::Result<()> {
        let now = now();
        let expected: Option<String> =
            sqlx::query_scalar("SELECT secret_hash FROM nodes WHERE id = ? AND status = 'online'")
                .bind(&heartbeat.node_id)
                .fetch_optional(&self.db.pool)
                .await?;
        if expected.as_deref() != Some(proof) {
            return Err(anyhow::anyhow!("invalid node proof"));
        }
        let updated = sqlx::query(
            "UPDATE nodes SET status = 'online', last_seen_at = ? WHERE id = ? AND status = 'online'",
        )
        .bind(now)
        .bind(&heartbeat.node_id)
        .execute(&self.db.pool)
        .await?
        .rows_affected();
        if updated == 0 {
            return Err(anyhow::anyhow!("node not paired"));
        }
        self.pending.lock().await.remove(&heartbeat.node_id);
        Ok(())
    }

    pub async fn revoke(&self, id: &str) -> anyhow::Result<()> {
        sqlx::query("UPDATE nodes SET status = 'revoked' WHERE id = ?")
            .bind(id)
            .execute(&self.db.pool)
            .await?;
        Ok(())
    }

    pub async fn nodes(&self) -> anyhow::Result<Vec<Node>> {
        let rows = sqlx::query_as::<_, (String, String, String, String, String, Option<i64>, i64, Option<i64>)>(
            "SELECT id, name, fingerprint, public_key, status, last_seen_at, created_at, paired_at FROM nodes ORDER BY name",
        )
        .fetch_all(&self.db.pool)
        .await?;
        Ok(rows.into_iter().map(Node::from_row).collect())
    }

    pub async fn node(&self, id: &str) -> anyhow::Result<Option<Node>> {
        let row = sqlx::query_as::<_, (String, String, String, String, String, Option<i64>, i64, Option<i64>)>(
            "SELECT id, name, fingerprint, public_key, status, last_seen_at, created_at, paired_at FROM nodes WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.db.pool)
        .await?;
        Ok(row.map(Node::from_row))
    }
}

impl Node {
    fn from_row(
        row: (
            String,
            String,
            String,
            String,
            String,
            Option<i64>,
            i64,
            Option<i64>,
        ),
    ) -> Self {
        Node {
            id: row.0,
            name: row.1,
            fingerprint: row.2,
            public_key: row.3,
            status: match row.4.as_str() {
                "online" => NodeStatus::Online,
                "revoked" => NodeStatus::Revoked,
                _ => NodeStatus::Pending,
            },
            last_seen_at: row.5,
            created_at: row.6,
            paired_at: row.7,
            secret: String::new(),
        }
    }
}

fn hash(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn random_hex(bytes: usize) -> String {
    let mut buffer = vec![0_u8; bytes];
    rand::rngs::OsRng.fill_bytes(&mut buffer);
    buffer.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
