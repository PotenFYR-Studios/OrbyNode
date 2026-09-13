//! OrbyNode persistence — SQLite, WAL, durable state only (ADR 004).

// ---------- Public API ----------

/// A project: repository or authorized working directory.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow, serde::Serialize)]
#[sqlx(rename_all = "snake_case")]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub created_at: i64,
}

/// A session: persistent logical workspace (terminals + layout + context).
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow, serde::Serialize)]
#[sqlx(rename_all = "snake_case")]
pub struct Session {
    pub id: i64,
    pub project_id: i64,
    pub name: String,
    pub created_at: i64,
}

/// Terminal metadata persisted per session (layout reconstruction, §9).
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct SessionTerminalRow {
    pub id: i64,
    pub session_id: i64,
    pub terminal_id: i64,
    pub title: String,
    pub cols: i64,
    pub rows: i64,
    pub position: i64,
}

impl From<SessionTerminalRow> for SessionTerminal {
    fn from(r: SessionTerminalRow) -> Self {
        SessionTerminal {
            id: r.id,
            session_id: r.session_id,
            terminal_id: r.terminal_id as u64,
            title: r.title,
            cols: r.cols as u16,
            rows: r.rows as u16,
            position: r.position as i32,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SessionTerminal {
    pub id: i64,
    pub session_id: i64,
    pub terminal_id: u64,
    pub title: String,
    pub cols: u16,
    pub rows: u16,
    pub position: i32,
}

/// Task states (Plan §26 board columns).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    Backlog,
    Ready,
    Running,
    NeedsInput,
    Review,
    Completed,
    Failed,
}

impl TaskState {
    pub fn as_str(self) -> &'static str {
        match self {
            TaskState::Backlog => "backlog",
            TaskState::Ready => "ready",
            TaskState::Running => "running",
            TaskState::NeedsInput => "needs_input",
            TaskState::Review => "review",
            TaskState::Completed => "completed",
            TaskState::Failed => "failed",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "ready" => TaskState::Ready,
            "running" => TaskState::Running,
            "needs_input" => TaskState::NeedsInput,
            "review" => TaskState::Review,
            "completed" => TaskState::Completed,
            "failed" => TaskState::Failed,
            _ => TaskState::Backlog,
        }
    }
}

/// A unit of work (Plan §26). `version` powers optimistic concurrency (§68).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Task {
    pub id: i64,
    pub project_id: i64,
    pub title: String,
    pub description: String,
    pub state: TaskState,
    pub priority: i32,
    pub assignee: String,
    pub agent: String,
    pub branch: String,
    pub worktree: String,
    pub version: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Raw task row shape (13 columns, sqlx::query_as target).
pub type TaskRow = (
    i64,
    i64,
    String,
    String,
    String,
    i32,
    String,
    String,
    String,
    String,
    i64,
    i64,
    i64,
);

/// Optimistic-concurrency conflict (§68): entity version moved underneath us.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Conflict;

#[derive(Debug)]
pub enum DbError {
    Sqlx(sqlx::Error),
    NotFound,
}

impl From<sqlx::Error> for DbError {
    fn from(e: sqlx::Error) -> Self {
        DbError::Sqlx(e)
    }
}

impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DbError::Sqlx(e) => write!(f, "db error: {e}"),
            DbError::NotFound => write!(f, "not found"),
        }
    }
}

impl std::error::Error for DbError {}

pub type DbResult<T> = Result<T, DbError>;

// Implemented below: Db::open (migrations), project/session/terminal/settings
// repositories.

// ---------- Implementation ----------

/// SQLite handle with WAL + foreign keys; migrations applied on open.
#[derive(Clone)]
pub struct Db {
    pub pool: sqlx::SqlitePool,
}

const MIGRATIONS: &[(i64, &str)] = &[
    (
        1,
        r#"
        CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);

        CREATE TABLE projects (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            path TEXT NOT NULL,
            created_at INTEGER NOT NULL
        );

        CREATE TABLE sessions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            name TEXT NOT NULL,
            created_at INTEGER NOT NULL
        );

        CREATE TABLE session_terminals (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id INTEGER NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
            terminal_id INTEGER NOT NULL,
            title TEXT NOT NULL DEFAULT '',
            cols INTEGER NOT NULL DEFAULT 80,
            rows INTEGER NOT NULL DEFAULT 24,
            position INTEGER NOT NULL DEFAULT 0,
            UNIQUE (session_id, terminal_id)
        );

        CREATE TABLE settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        "#,
    ),
    (
        3,
        r#"
        CREATE TABLE tasks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            title TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            state TEXT NOT NULL DEFAULT 'backlog',
            priority INTEGER NOT NULL DEFAULT 3,
            assignee TEXT NOT NULL DEFAULT '',
            agent TEXT NOT NULL DEFAULT '',
            branch TEXT NOT NULL DEFAULT '',
            worktree TEXT NOT NULL DEFAULT '',
            version INTEGER NOT NULL DEFAULT 1,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );

        CREATE INDEX idx_tasks_project ON tasks (project_id, state);
        "#,
    ),
    (
        2,
        r#"
        CREATE TABLE users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT NOT NULL UNIQUE,
            display_name TEXT NOT NULL DEFAULT '',
            password_hash TEXT NOT NULL,
            role TEXT NOT NULL,
            created_at INTEGER NOT NULL
        );

        CREATE TABLE auth_sessions (
            token TEXT PRIMARY KEY,
            user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            csrf TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            expires_at INTEGER NOT NULL
        );

        CREATE INDEX idx_auth_sessions_expiry ON auth_sessions (expires_at);
        "#,
    ),
];

impl Db {
    /// Open (creating if needed) and apply pending migrations.
    pub async fn open(url: &str) -> DbResult<Self> {
        let pool = sqlx::SqlitePool::connect(url).await?;
        sqlx::query("PRAGMA journal_mode=WAL")
            .execute(&pool)
            .await?;
        sqlx::query("PRAGMA foreign_keys=ON").execute(&pool).await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at INTEGER NOT NULL
            )",
        )
        .execute(&pool)
        .await?;

        for (version, sql) in MIGRATIONS {
            let applied: Option<i64> =
                sqlx::query_scalar("SELECT version FROM schema_migrations WHERE version = ?")
                    .bind(version)
                    .fetch_optional(&pool)
                    .await?;
            if applied.is_none() {
                let mut tx = pool.begin().await?;
                for stmt in sql.split(';').filter(|s| !s.trim().is_empty()) {
                    sqlx::query(stmt).execute(&mut *tx).await?;
                }
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64;
                sqlx::query("INSERT INTO schema_migrations (version, applied_at) VALUES (?, ?)")
                    .bind(version)
                    .bind(now)
                    .execute(&mut *tx)
                    .await?;
                tx.commit().await?;
                tracing::info!(version, "migration applied");
            }
        }
        Ok(Db { pool })
    }

    pub async fn schema_version(&self) -> DbResult<i64> {
        Ok(
            sqlx::query_scalar("SELECT COALESCE(MAX(version), 0) FROM schema_migrations")
                .fetch_one(&self.pool)
                .await?,
        )
    }

    // ---- projects ----

    pub async fn create_project(&self, name: &str, path: &str) -> DbResult<Project> {
        let now = now_secs();
        let id = sqlx::query("INSERT INTO projects (name, path, created_at) VALUES (?, ?, ?)")
            .bind(name)
            .bind(path)
            .bind(now)
            .execute(&self.pool)
            .await?
            .last_insert_rowid();
        Ok(Project {
            id,
            name: name.to_owned(),
            path: path.to_owned(),
            created_at: now,
        })
    }

    pub async fn get_project(&self, id: i64) -> DbResult<Option<Project>> {
        Ok(
            sqlx::query_as("SELECT id, name, path, created_at FROM projects WHERE id = ?")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?,
        )
    }

    pub async fn list_projects(&self) -> DbResult<Vec<Project>> {
        Ok(
            sqlx::query_as("SELECT id, name, path, created_at FROM projects ORDER BY id")
                .fetch_all(&self.pool)
                .await?,
        )
    }

    pub async fn delete_project(&self, id: i64) -> DbResult<()> {
        sqlx::query("DELETE FROM projects WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ---- sessions ----

    pub async fn create_session(&self, project_id: i64, name: &str) -> DbResult<Session> {
        let now = now_secs();
        let id =
            sqlx::query("INSERT INTO sessions (project_id, name, created_at) VALUES (?, ?, ?)")
                .bind(project_id)
                .bind(name)
                .bind(now)
                .execute(&self.pool)
                .await?
                .last_insert_rowid();
        Ok(Session {
            id,
            project_id,
            name: name.to_owned(),
            created_at: now,
        })
    }

    pub async fn list_sessions(&self, project_id: i64) -> DbResult<Vec<Session>> {
        Ok(sqlx::query_as(
            "SELECT id, project_id, name, created_at FROM sessions WHERE project_id = ? ORDER BY id",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?)
    }

    // ---- session terminals (layout) ----

    pub async fn upsert_session_terminal(
        &self,
        session_id: i64,
        terminal_id: u64,
        title: &str,
        cols: u16,
        rows: u16,
        position: i32,
    ) -> DbResult<SessionTerminal> {
        sqlx::query(
            "INSERT INTO session_terminals (session_id, terminal_id, title, cols, rows, position)
             VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT (session_id, terminal_id)
             DO UPDATE SET title = excluded.title, cols = excluded.cols,
                           rows = excluded.rows, position = excluded.position",
        )
        .bind(session_id)
        .bind(terminal_id as i64)
        .bind(title)
        .bind(cols as i64)
        .bind(rows as i64)
        .bind(position)
        .execute(&self.pool)
        .await?;
        let row: SessionTerminalRow = sqlx::query_as(
            "SELECT id, session_id, terminal_id, title, cols, rows, position
             FROM session_terminals WHERE session_id = ? AND terminal_id = ?",
        )
        .bind(session_id)
        .bind(terminal_id as i64)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.into())
    }

    pub async fn list_session_terminals(&self, session_id: i64) -> DbResult<Vec<SessionTerminal>> {
        let rows: Vec<SessionTerminalRow> = sqlx::query_as(
            "SELECT id, session_id, terminal_id, title, cols, rows, position
             FROM session_terminals WHERE session_id = ? ORDER BY position, id",
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    // ---- tasks ----

    pub async fn create_task(
        &self,
        project_id: i64,
        title: &str,
        description: &str,
        priority: i32,
    ) -> DbResult<Task> {
        let now = now_secs();
        let id = sqlx::query(
            "INSERT INTO tasks (project_id, title, description, priority, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(project_id)
        .bind(title)
        .bind(description)
        .bind(priority)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?
        .last_insert_rowid();
        Ok(Task {
            id,
            project_id,
            title: title.to_owned(),
            description: description.to_owned(),
            state: TaskState::Backlog,
            priority,
            assignee: String::new(),
            agent: String::new(),
            branch: String::new(),
            worktree: String::new(),
            version: 1,
            created_at: now,
            updated_at: now,
        })
    }

    fn task_row(row: TaskRow) -> Task {
        Task {
            id: row.0,
            project_id: row.1,
            title: row.2,
            description: row.3,
            state: TaskState::parse(&row.4),
            priority: row.5,
            assignee: row.6,
            agent: row.7,
            branch: row.8,
            worktree: row.9,
            version: row.10,
            created_at: row.11,
            updated_at: row.12,
        }
    }

    const TASK_COLUMNS: &str = "id, project_id, title, description, state, priority, assignee, agent, branch, worktree, version, created_at, updated_at";

    pub async fn get_task(&self, id: i64) -> DbResult<Option<Task>> {
        let row: Option<TaskRow> =
            sqlx::query_as(&format!(
                "SELECT {} FROM tasks WHERE id = ?",
                Self::TASK_COLUMNS
            ))
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(Self::task_row))
    }

    pub async fn list_tasks(&self, project_id: i64) -> DbResult<Vec<Task>> {
        let rows: Vec<TaskRow> =
            sqlx::query_as(&format!(
                "SELECT {} FROM tasks WHERE project_id = ? ORDER BY priority, id",
                Self::TASK_COLUMNS
            ))
            .bind(project_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(Self::task_row).collect())
    }

    /// Move a task between board states with optimistic version check (§68):
    /// fails with `DbError::NotFound`-style conflict when version mismatches.
    pub async fn move_task(
        &self,
        id: i64,
        new_state: TaskState,
        expected_version: i64,
    ) -> DbResult<Result<Task, Conflict>> {
        let now = now_secs();
        let result = sqlx::query(
            "UPDATE tasks SET state = ?, version = version + 1, updated_at = ?
             WHERE id = ? AND version = ?",
        )
        .bind(new_state.as_str())
        .bind(now)
        .bind(id)
        .bind(expected_version)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Ok(Err(Conflict));
        }
        Ok(Ok(self.get_task(id).await?.expect("just updated")))
    }

    /// Attach branch/worktree metadata (worktree-per-task, §25).
    pub async fn set_task_worktree(
        &self,
        id: i64,
        branch: &str,
        worktree: &str,
    ) -> DbResult<()> {
        sqlx::query("UPDATE tasks SET branch = ?, worktree = ?, updated_at = ? WHERE id = ?")
            .bind(branch)
            .bind(worktree)
            .bind(now_secs())
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_task(&self, id: i64) -> DbResult<()> {
        sqlx::query("DELETE FROM tasks WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ---- settings ----

    pub async fn get_setting(&self, key: &str) -> DbResult<Option<String>> {
        Ok(
            sqlx::query_scalar("SELECT value FROM settings WHERE key = ?")
                .bind(key)
                .fetch_optional(&self.pool)
                .await?,
        )
    }

    pub async fn set_setting(&self, key: &str, value: &str) -> DbResult<()> {
        sqlx::query(
            "INSERT INTO settings (key, value) VALUES (?, ?)
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
        )
        .bind(key)
        .bind(value)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
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

    async fn mem_db() -> Db {
        Db::open("sqlite::memory:")
            .await
            .expect("open in-memory db")
    }

    #[tokio::test]
    async fn migrations_apply_cleanly() {
        let db = mem_db().await;
        let v = db.schema_version().await.unwrap();
        assert!(v >= 1, "schema v1 applied, got {v}");
    }

    #[tokio::test]
    async fn project_crud_roundtrip() {
        let db = mem_db().await;
        let p = db.create_project("backend", "/srv/backend").await.unwrap();
        assert_eq!(p.name, "backend");
        assert_eq!(p.path, "/srv/backend");
        let got = db.get_project(p.id).await.unwrap().expect("exists");
        assert_eq!(got, p);
        let all = db.list_projects().await.unwrap();
        assert_eq!(all, vec![p.clone()]);
        db.delete_project(p.id).await.unwrap();
        assert!(db.get_project(p.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn sessions_belong_to_projects() {
        let db = mem_db().await;
        let p = db.create_project("web", "/srv/web").await.unwrap();
        let s = db.create_session(p.id, "main").await.unwrap();
        assert_eq!(s.project_id, p.id);
        let sessions = db.list_sessions(p.id).await.unwrap();
        assert_eq!(sessions, vec![s]);
    }

    #[tokio::test]
    async fn session_terminals_persist_layout() {
        let db = mem_db().await;
        let p = db.create_project("web", "/srv/web").await.unwrap();
        let s = db.create_session(p.id, "main").await.unwrap();
        let t = db
            .upsert_session_terminal(s.id, 42, "build", 120, 40, 0)
            .await
            .unwrap();
        assert_eq!(t.terminal_id, 42);
        assert_eq!(t.cols, 120);
        // Upsert same terminal_id updates rather than duplicating.
        let t2 = db
            .upsert_session_terminal(s.id, 42, "build", 200, 50, 0)
            .await
            .unwrap();
        assert_eq!(t2.id, t.id, "upsert not duplicate");
        let terms = db.list_session_terminals(s.id).await.unwrap();
        assert_eq!(terms.len(), 1);
        assert_eq!(terms[0].rows, 50);
    }

    #[tokio::test]
    async fn settings_roundtrip_and_overwrite() {
        let db = mem_db().await;
        db.set_setting("ui.theme", "dark").await.unwrap();
        db.set_setting("ui.theme", "light").await.unwrap();
        let v = db.get_setting("ui.theme").await.unwrap();
        assert_eq!(v, Some("light".to_owned()));
        assert_eq!(db.get_setting("missing").await.unwrap(), None);
    }


    #[tokio::test]
    async fn task_crud_and_board_flow() {
        let db = mem_db().await;
        let p = db.create_project("app", "/tmp/app").await.unwrap();
        let t = db.create_task(p.id, "Add OAuth", "implement login", 2).await.unwrap();
        assert_eq!(t.state, TaskState::Backlog);
        assert_eq!(t.version, 1);

        // Optimistic move: correct version succeeds, bumps version.
        let moved = db.move_task(t.id, TaskState::Ready, t.version).await.unwrap().unwrap();
        assert_eq!(moved.state, TaskState::Ready);
        assert_eq!(moved.version, 2);

        // Stale version conflicts (§68).
        let conflict = db.move_task(t.id, TaskState::Running, t.version).await.unwrap();
        assert_eq!(conflict, Err(Conflict));

        // Fresh version succeeds.
        let moved = db.move_task(t.id, TaskState::Running, moved.version).await.unwrap().unwrap();
        assert_eq!(moved.state, TaskState::Running);
        assert_eq!(db.list_tasks(p.id).await.unwrap()[0].state, TaskState::Running);
    }

    #[tokio::test]
    async fn task_worktree_metadata_roundtrip() {
        let db = mem_db().await;
        let p = db.create_project("app", "/tmp/app").await.unwrap();
        let t = db.create_task(p.id, "T", "", 3).await.unwrap();
        db.set_task_worktree(t.id, "agent/oauth", "/home/u/.orbynode/worktrees/app/oauth").await.unwrap();
        let got = db.get_task(t.id).await.unwrap().unwrap();
        assert_eq!(got.branch, "agent/oauth");
        assert_eq!(got.worktree, "/home/u/.orbynode/worktrees/app/oauth");
    }

    #[tokio::test]
    async fn tasks_are_scoped_per_project_and_deletable() {
        let db = mem_db().await;
        let p1 = db.create_project("one", "/tmp/one").await.unwrap();
        let p2 = db.create_project("two", "/tmp/two").await.unwrap();
        db.create_task(p1.id, "a", "", 3).await.unwrap();
        let t2 = db.create_task(p2.id, "b", "", 3).await.unwrap();
        assert_eq!(db.list_tasks(p1.id).await.unwrap().len(), 1);
        db.delete_task(t2.id).await.unwrap();
        assert!(db.get_task(t2.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn wal_mode_is_on() {
        // `:memory:` databases always report journal_mode=memory, so use a
        // real temp file to assert WAL actually sticks.
        let dir = std::env::temp_dir().join(format!("orbynode-wal-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.db");
        let url = format!("sqlite://{}?mode=rwc", path.display());
        let db = Db::open(&url).await.unwrap();
        let mode: String = sqlx::query_scalar("PRAGMA journal_mode")
            .fetch_one(&db.pool)
            .await
            .unwrap();
        assert_eq!(mode.to_lowercase(), "wal");
        drop(db);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
