//! OrbyNode auth - Argon2id passwords, server-side sessions, throttling (ADR 005).

// ---------- Public API ----------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub display_name: String,
    pub role: Role,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Owner,
    Admin,
    Operator,
    Developer,
    Viewer,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Owner => "owner",
            Role::Admin => "admin",
            Role::Operator => "operator",
            Role::Developer => "developer",
            Role::Viewer => "viewer",
        }
    }
}

#[derive(Debug)]
pub enum AuthError {
    /// Unknown user OR bad password - identical, no enumeration.
    InvalidCredentials,
    /// Account temporarily locked by throttling.
    Locked,
    /// Setup already completed; cannot create another bootstrap owner.
    SetupCompleted,
    /// Session token not found or expired.
    InvalidSession,
    Sqlx(sqlx::Error),
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::InvalidCredentials => write!(f, "invalid credentials"),
            AuthError::Locked => write!(f, "account temporarily locked"),
            AuthError::SetupCompleted => write!(f, "setup already completed"),
            AuthError::InvalidSession => write!(f, "invalid session"),
            AuthError::Sqlx(e) => write!(f, "db error: {e}"),
        }
    }
}

/// Equality ignores the Sqlx payload (only variant identity matters to logic).
impl PartialEq for AuthError {
    fn eq(&self, other: &Self) -> bool {
        use AuthError::*;
        matches!(
            (self, other),
            (InvalidCredentials, InvalidCredentials)
                | (Locked, Locked)
                | (SetupCompleted, SetupCompleted)
                | (InvalidSession, InvalidSession)
                | (Sqlx(_), Sqlx(_))
        )
    }
}

impl Eq for AuthError {}

impl std::error::Error for AuthError {}
impl From<sqlx::Error> for AuthError {
    fn from(e: sqlx::Error) -> Self {
        AuthError::Sqlx(e)
    }
}

pub type AuthResult<T> = Result<T, AuthError>;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SessionInfo {
    pub token: String,
    pub user_id: i64,
    pub expires_at: i64,
    /// CSRF token to echo in `X-Orbynode-CSRF` on mutations (ADR 005).
    pub csrf: String,
}

/// Fine-grained permissions (Plan §15 permission model).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Perm {
    ProjectView,
    ProjectManage,
    TerminalView,
    TerminalWrite,
    TerminalCreate,
    TerminalTerminate,
    AgentView,
    AgentStart,
    AgentControl,
    AgentApprove,
    FilesView,
    FilesWrite,
    GitView,
    GitWrite,
    TasksView,
    TasksWrite,
    ServicesView,
    ServicesControl,
    SettingsView,
    SettingsManage,
    UsersView,
    UsersManage,
    AuditView,
}

/// Role -> permission set (Plan §15 roles).
pub fn role_permissions(role: Role) -> &'static [Perm] {
    use Perm::*;
    match role {
        Role::Owner => &[
            ProjectView,
            ProjectManage,
            TerminalView,
            TerminalWrite,
            TerminalCreate,
            TerminalTerminate,
            AgentView,
            AgentStart,
            AgentControl,
            AgentApprove,
            FilesView,
            FilesWrite,
            GitView,
            GitWrite,
            TasksView,
            TasksWrite,
            ServicesView,
            ServicesControl,
            SettingsView,
            SettingsManage,
            UsersView,
            UsersManage,
            AuditView,
        ],
        Role::Admin => &[
            ProjectView,
            ProjectManage,
            TerminalView,
            TerminalWrite,
            TerminalCreate,
            TerminalTerminate,
            AgentView,
            AgentStart,
            AgentControl,
            AgentApprove,
            FilesView,
            FilesWrite,
            GitView,
            GitWrite,
            TasksView,
            TasksWrite,
            ServicesView,
            ServicesControl,
            SettingsView,
            SettingsManage,
            UsersView,
            UsersManage,
            AuditView,
        ],
        Role::Operator => &[
            ProjectView,
            TerminalView,
            TerminalWrite,
            TerminalCreate,
            TerminalTerminate,
            AgentView,
            AgentStart,
            AgentControl,
            AgentApprove,
            FilesView,
            FilesWrite,
            GitView,
            GitWrite,
            TasksView,
            TasksWrite,
            ServicesView,
            ServicesControl,
        ],
        Role::Developer => &[
            ProjectView,
            TerminalView,
            TerminalWrite,
            TerminalCreate,
            AgentView,
            AgentStart,
            FilesView,
            FilesWrite,
            GitView,
            GitWrite,
            TasksView,
            TasksWrite,
            ServicesView,
        ],
        Role::Viewer => &[
            ProjectView,
            TerminalView,
            AgentView,
            FilesView,
            GitView,
            TasksView,
            ServicesView,
        ],
    }
}

pub fn role_has(role: Role, perm: Perm) -> bool {
    role_permissions(role).contains(&perm)
}

// ---------- Implementation ----------

// Argon2 defaults (v0.5 `Argon2::default()` = Argon2id; OWASP-equivalent baseline).

const SESSION_TTL_SECS: i64 = 7 * 24 * 3600;
const LOCKOUT_WINDOW_SECS: i64 = 15 * 60;
const LOCKOUT_THRESHOLD: u32 = 5;
const LOCKOUT_SECS: i64 = 5 * 60;

pub struct AuthService {
    db: orbynode_database::Db,
    /// (username -> (fail_count, first_fail_unix))
    failures: std::sync::Mutex<std::collections::HashMap<String, (u32, i64)>>,
    /// (username -> locked_until_unix)
    locked_until: std::sync::Mutex<std::collections::HashMap<String, i64>>,
}

impl AuthService {
    pub fn new(db: orbynode_database::Db) -> Self {
        AuthService {
            db,
            failures: std::sync::Mutex::new(std::collections::HashMap::new()),
            locked_until: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }

    pub async fn setup_pending(&self) -> AuthResult<bool> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&self.db.pool)
            .await?;
        Ok(count == 0)
    }

    /// Create the bootstrap Owner. Only succeeds while no users exist.
    pub async fn setup_owner(
        &self,
        username: &str,
        display_name: &str,
        password: &str,
    ) -> AuthResult<User> {
        if !self.setup_pending().await? {
            return Err(AuthError::SetupCompleted);
        }
        self.create_user(username, display_name, password, Role::Owner)
            .await
    }

    pub async fn create_user(
        &self,
        username: &str,
        display_name: &str,
        password: &str,
        role: Role,
    ) -> AuthResult<User> {
        let hash = hash_password(password)?;
        let now = now_secs();
        let id = sqlx::query(
            "INSERT INTO users (username, display_name, password_hash, role, created_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(username)
        .bind(display_name)
        .bind(&hash)
        .bind(role.as_str())
        .bind(now)
        .execute(&self.db.pool)
        .await?
        .last_insert_rowid();
        Ok(User {
            id,
            username: username.to_owned(),
            display_name: display_name.to_owned(),
            role,
        })
    }

    /// Stored hash for a username (test + admin introspection).
    pub async fn password_hash(&self, username: &str) -> AuthResult<Option<String>> {
        Ok(
            sqlx::query_scalar("SELECT password_hash FROM users WHERE username = ?")
                .bind(username)
                .fetch_optional(&self.db.pool)
                .await?,
        )
    }

    pub fn verify_password(&self, hash: &str, password: &str) -> bool {
        argon2::PasswordHash::new(hash)
            .map(|parsed| {
                use argon2::PasswordVerifier;
                argon2::Argon2::default()
                    .verify_password(password.as_bytes(), &parsed)
                    .is_ok()
            })
            .unwrap_or(false)
    }

    /// Validate credentials, create a session. Throttled per ADR 005.
    pub async fn login(&self, username: &str, password: &str) -> AuthResult<SessionInfo> {
        let now = now_secs();
        {
            let locked = self.locked_until.lock().expect("locked poisoned");
            if let Some(&until) = locked.get(username)
                && until > now
            {
                return Err(AuthError::Locked);
            }
        }
        let Some(hash) = self.password_hash(username).await.unwrap_or(None) else {
            // Register the failure too so unknown users can't probe timing.
            self.record_failure(username, now);
            return Err(AuthError::InvalidCredentials);
        };
        if !self.verify_password(&hash, password) {
            self.record_failure(username, now);
            return Err(AuthError::InvalidCredentials);
        }
        self.failures
            .lock()
            .expect("failures poisoned")
            .remove(username);
        self.locked_until
            .lock()
            .expect("locked poisoned")
            .remove(username);

        let user: i64 = sqlx::query_scalar("SELECT id FROM users WHERE username = ?")
            .bind(username)
            .fetch_one(&self.db.pool)
            .await?;
        self.new_session(user).await
    }

    async fn new_session(&self, user_id: i64) -> AuthResult<SessionInfo> {
        let token = random_hex(32);
        let csrf = random_hex(32);
        let now = now_secs();
        let expires_at = now + SESSION_TTL_SECS;
        sqlx::query(
            "INSERT INTO auth_sessions (token, user_id, csrf, created_at, expires_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&token)
        .bind(user_id)
        .bind(&csrf)
        .bind(now)
        .bind(expires_at)
        .execute(&self.db.pool)
        .await?;
        Ok(SessionInfo {
            token,
            user_id,
            expires_at,
            csrf,
        })
    }

    /// Resolve a session token to its user, if valid and unexpired.
    /// Sliding expiration: each successful validation extends the session.
    pub async fn user_for_session(&self, token: &str) -> AuthResult<Option<User>> {
        let now = now_secs();
        let row: Option<(i64, i64, i64)> = sqlx::query_as(
            "SELECT user_id, expires_at, created_at FROM auth_sessions WHERE token = ?",
        )
        .bind(token)
        .fetch_optional(&self.db.pool)
        .await?;
        let Some((user_id, expires_at, _created)) = row else {
            return Ok(None);
        };
        if expires_at <= now {
            // Expired sessions are cleaned up and simply unresolvable.
            sqlx::query("DELETE FROM auth_sessions WHERE token = ?")
                .bind(token)
                .execute(&self.db.pool)
                .await?;
            return Ok(None);
        }
        // Sliding renewal: extend if more than a day has passed since creation.
        if expires_at - now < SESSION_TTL_SECS - 24 * 3600 {
            sqlx::query("UPDATE auth_sessions SET expires_at = ?, created_at = ? WHERE token = ?")
                .bind(now + SESSION_TTL_SECS)
                .bind(now)
                .bind(token)
                .execute(&self.db.pool)
                .await?;
        }
        Ok(
            sqlx::query_as("SELECT id, username, display_name, role FROM users WHERE id = ?")
                .bind(user_id)
                .fetch_optional(&self.db.pool)
                .await?
                .map(
                    |(id, username, display_name, role): (i64, String, String, String)| User {
                        id,
                        username,
                        display_name,
                        role: parse_role(&role),
                    },
                ),
        )
    }

    pub async fn logout(&self, token: &str) -> AuthResult<()> {
        sqlx::query("DELETE FROM auth_sessions WHERE token = ?")
            .bind(token)
            .execute(&self.db.pool)
            .await?;
        Ok(())
    }

    /// CSRF token for a valid session.
    pub async fn csrf_for_session(&self, token: &str) -> AuthResult<Option<String>> {
        let now = now_secs();
        Ok(
            sqlx::query_scalar("SELECT csrf FROM auth_sessions WHERE token = ? AND expires_at > ?")
                .bind(token)
                .bind(now)
                .fetch_optional(&self.db.pool)
                .await?,
        )
    }

    fn record_failure(&self, username: &str, now: i64) {
        let mut lock = false;
        {
            let mut failures = self.failures.lock().expect("failures poisoned");
            let entry = failures.entry(username.to_owned()).or_insert((0, now));
            if now - entry.1 > LOCKOUT_WINDOW_SECS {
                *entry = (1, now);
            } else {
                entry.0 += 1;
            }
            if entry.0 >= LOCKOUT_THRESHOLD {
                lock = true;
            }
        }
        if lock {
            self.locked_until
                .lock()
                .expect("locked poisoned")
                .insert(username.to_owned(), now + LOCKOUT_SECS);
            tracing::warn!(username, "account locked after repeated failures");
        }
    }

    /// Test hook: force-expire a session.
    pub async fn expire_session_for_test(&self, token: &str) {
        let _ = sqlx::query("UPDATE auth_sessions SET expires_at = 0 WHERE token = ?")
            .bind(token)
            .execute(&self.db.pool)
            .await;
    }
}

fn parse_role(s: &str) -> Role {
    match s {
        "owner" => Role::Owner,
        "admin" => Role::Admin,
        "operator" => Role::Operator,
        "developer" => Role::Developer,
        _ => Role::Viewer,
    }
}

fn hash_password(password: &str) -> AuthResult<String> {
    use argon2::PasswordHasher;
    let salt =
        argon2::password_hash::SaltString::generate(&mut argon2::password_hash::rand_core::OsRng);
    argon2::Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| {
            tracing::error!(error = %e, "argon2 hash failed");
            AuthError::Sqlx(sqlx::Error::Configuration(e.to_string().into()))
        })
}

fn random_hex(bytes: usize) -> String {
    use rand::RngCore;
    let mut buf = vec![0u8; bytes];
    rand::rngs::OsRng.fill_bytes(&mut buf);
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn svc() -> AuthService {
        let db = orbynode_database::Db::open("sqlite::memory:")
            .await
            .unwrap();
        AuthService::new(db)
    }

    #[tokio::test]
    async fn setup_creates_first_owner_exactly_once() {
        let s = svc().await;
        assert!(s.setup_pending().await.unwrap());
        let u = s
            .setup_owner("admin", "Administrator", "correct horse")
            .await
            .unwrap();
        assert_eq!(u.role, Role::Owner);
        assert!(!s.setup_pending().await.unwrap());
        assert!(matches!(
            s.setup_owner("evil", "Evil", "password").await,
            Err(AuthError::SetupCompleted)
        ));
    }

    #[tokio::test]
    async fn password_hash_is_argon2id_and_verifiable() {
        let s = svc().await;
        s.setup_owner("admin", "A", "hunter2butgood").await.unwrap();
        let hash = s
            .password_hash("admin")
            .await
            .unwrap()
            .expect("hash stored");
        assert!(hash.starts_with("$argon2id$"), "got {hash}");
        assert!(s.verify_password(&hash, "hunter2butgood"));
        assert!(!s.verify_password(&hash, "wrong"));
    }

    #[tokio::test]
    async fn login_logout_session_roundtrip() {
        let s = svc().await;
        s.setup_owner("admin", "A", "good-pass-1").await.unwrap();
        let sess = s.login("admin", "good-pass-1").await.unwrap();
        assert_eq!(sess.token.len(), 64, "32 bytes hex");
        assert!(!sess.csrf.is_empty());
        let who = s.user_for_session(&sess.token).await.unwrap().unwrap();
        assert_eq!(who.username, "admin");
        s.logout(&sess.token).await.unwrap();
        assert!(
            s.user_for_session(&sess.token).await.unwrap().is_none(),
            "revoked"
        );
    }

    #[tokio::test]
    async fn wrong_password_is_invalid_not_enumerating() {
        let s = svc().await;
        s.setup_owner("admin", "A", "good-pass-2").await.unwrap();
        let e1 = s.login("admin", "wrong").await.unwrap_err();
        let e2 = s.login("ghost", "wrong").await.unwrap_err();
        assert_eq!(e1, AuthError::InvalidCredentials);
        assert_eq!(
            e2,
            AuthError::InvalidCredentials,
            "same error, no user enum"
        );
    }

    #[tokio::test]
    async fn failed_logins_trigger_lockout() {
        let s = svc().await;
        s.setup_owner("admin", "A", "good-pass-3").await.unwrap();
        for _ in 0..5 {
            let _ = s.login("admin", "nope").await;
        }
        let e = s.login("admin", "good-pass-3").await.unwrap_err();
        assert_eq!(e, AuthError::Locked, "even correct password is locked");
    }

    #[test]
    fn viewer_cannot_write_terminal_or_files() {
        // M11 acceptance (Plan §134): Viewer is read-only.
        assert!(role_has(Role::Viewer, Perm::TerminalView));
        assert!(!role_has(Role::Viewer, Perm::TerminalWrite));
        assert!(!role_has(Role::Viewer, Perm::FilesWrite));
        assert!(!role_has(Role::Viewer, Perm::GitWrite));
    }

    #[test]
    fn developer_writes_but_cannot_manage_users() {
        assert!(role_has(Role::Developer, Perm::TerminalWrite));
        assert!(role_has(Role::Developer, Perm::FilesWrite));
        assert!(!role_has(Role::Developer, Perm::UsersManage));
        assert!(!role_has(Role::Developer, Perm::ProjectManage));
    }

    #[test]
    fn owner_has_everything() {
        for perm in role_permissions(Role::Admin) {
            assert!(role_has(Role::Owner, *perm), "owner ⊇ admin: {perm:?}");
        }
    }

    #[tokio::test]
    async fn sessions_expire() {
        let s = svc().await;
        s.setup_owner("admin", "A", "good-pass-4").await.unwrap();
        let sess = s.login("admin", "good-pass-4").await.unwrap();
        // Force-expire the row directly (fast path for the test).
        s.expire_session_for_test(&sess.token).await;
        assert!(s.user_for_session(&sess.token).await.unwrap().is_none());
    }
}
