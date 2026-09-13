//! Configuration-driven workflow orchestration (Plan §29, Milestone 16).

use orbynode_database::Db;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepKind {
    Agent,
    Parallel,
    Approval,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub name: String,
    pub kind: StepKind,
    #[serde(default)]
    pub agents: Vec<String>,
    #[serde(default)]
    pub command: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    pub name: String,
    pub steps: Vec<WorkflowStep>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Running,
    WaitingApproval,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowRun {
    pub id: String,
    pub definition_id: String,
    pub status: RunStatus,
    pub current_stage: String,
    pub variables: BTreeMap<String, String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WorkflowStepRun {
    pub id: String,
    pub run_id: String,
    pub stage: String,
    pub agent: String,
    pub status: StepStatus,
    pub output: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Clone)]
pub struct WorkflowEngine {
    db: Db,
}

#[derive(Debug)]
pub enum WorkflowError {
    Database(sqlx::Error),
    Serialization(serde_json::Error),
    Io(std::io::Error),
    Invalid(&'static str),
    NotFound,
}

impl From<sqlx::Error> for WorkflowError {
    fn from(value: sqlx::Error) -> Self {
        WorkflowError::Database(value)
    }
}

impl From<serde_json::Error> for WorkflowError {
    fn from(value: serde_json::Error) -> Self {
        WorkflowError::Serialization(value)
    }
}

impl From<std::io::Error> for WorkflowError {
    fn from(value: std::io::Error) -> Self {
        WorkflowError::Io(value)
    }
}

pub type SharedEngine = Arc<WorkflowEngine>;

impl WorkflowEngine {
    pub fn new(db: Db) -> Self {
        WorkflowEngine { db }
    }

    pub async fn put_definition(
        &self,
        definition: WorkflowDefinition,
    ) -> Result<String, WorkflowError> {
        let now = now();
        let id = deterministic_id(&definition.name);
        let config = serde_json::to_string(&definition)?;
        sqlx::query(
            "INSERT INTO workflow_definitions (id, name, config, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(name) DO UPDATE SET config = excluded.config, updated_at = excluded.updated_at",
        )
        .bind(&id)
        .bind(&definition.name)
        .bind(&config)
        .bind(now)
        .bind(now)
        .execute(&self.db.pool)
        .await?;
        validate(&definition)?;
        Ok(id)
    }

    pub async fn get_definition(
        &self,
        name: &str,
    ) -> Result<Option<WorkflowDefinition>, WorkflowError> {
        let config: Option<String> =
            sqlx::query_scalar("SELECT config FROM workflow_definitions WHERE name = ?")
                .bind(name)
                .fetch_optional(&self.db.pool)
                .await?;
        Ok(config
            .map(|config| serde_json::from_str(&config))
            .transpose()?)
    }

    pub async fn start(
        &self,
        name: &str,
        variables: BTreeMap<String, String>,
    ) -> Result<WorkflowRun, WorkflowError> {
        let definition = self
            .get_definition(name)
            .await?
            .ok_or(WorkflowError::NotFound)?;
        validate(&definition)?;
        let id = format!("run-{}", now());
        let current = definition
            .steps
            .first()
            .map(|step| step.name.clone())
            .ok_or(WorkflowError::Invalid("workflow has no steps"))?;
        let now = now();
        sqlx::query("INSERT INTO workflow_runs (id, definition_id, status, current_stage, variables, created_at, updated_at) VALUES (?, ?, 'running', ?, ?, ?, ?)")
            .bind(&id)
            .bind(deterministic_id(&definition.name))
            .bind(&current)
            .bind(serde_json::to_string(&variables)?)
            .bind(now)
            .bind(now)
            .execute(&self.db.pool)
            .await?;
        let mut run = self.advance(&id).await?;
        while run.status == RunStatus::Running {
            run = self.advance(&run.id).await?;
        }
        Ok(run)
    }

    pub async fn advance(&self, run_id: &str) -> Result<WorkflowRun, WorkflowError> {
        let run = self.get_run(run_id).await?.ok_or(WorkflowError::NotFound)?;
        let definition = self
            .definition_by_id(&run.definition_id)
            .await?
            .ok_or(WorkflowError::NotFound)?;
        let index = definition
            .steps
            .iter()
            .position(|step| step.name == run.current_stage)
            .ok_or(WorkflowError::Invalid("unknown stage"))?;
        let step = &definition.steps[index];

        if step.kind == StepKind::Parallel {
            for agent in &step.agents {
                self.insert_step(
                    run_id,
                    &step.name,
                    agent,
                    StepStatus::Completed,
                    format!("dispatched {agent}"),
                )
                .await?;
            }
        } else if step.kind == StepKind::Approval {
            self.insert_step(
                run_id,
                &step.name,
                "human",
                StepStatus::Pending,
                String::new(),
            )
            .await?;
            return self
                .update_run(run_id, RunStatus::WaitingApproval, &step.name)
                .await;
        } else {
            let agent = step
                .agents
                .first()
                .ok_or(WorkflowError::Invalid("agent stage has no agent"))?;
            let output: std::process::Output = match &step.command {
                Some(command) => {
                    tokio::process::Command::new("sh")
                        .arg("-c")
                        .arg(command)
                        .output()
                        .await?
                }
                None => std::process::Output {
                    status: {
                        use std::os::unix::process::ExitStatusExt;
                        std::process::ExitStatus::from_raw(0)
                    },
                    stdout: Vec::new(),
                    stderr: Vec::new(),
                },
            };
            let text = format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            let status = if output.status.success() {
                StepStatus::Completed
            } else {
                StepStatus::Failed
            };
            self.insert_step(run_id, &step.name, agent, status.clone(), text)
                .await?;
            if status == StepStatus::Failed {
                return self.update_run(run_id, RunStatus::Failed, &step.name).await;
            }
        }

        match definition.steps.get(index + 1) {
            Some(next) => {
                if next.kind == StepKind::Approval {
                    let run = self
                        .update_run(run_id, RunStatus::WaitingApproval, &next.name)
                        .await?;
                    self.insert_step(
                        run_id,
                        &next.name,
                        "human",
                        StepStatus::Pending,
                        String::new(),
                    )
                    .await?;
                    Ok(run)
                } else {
                    self.update_run(run_id, RunStatus::Running, &next.name)
                        .await
                }
            }
            None => self.update_run(run_id, RunStatus::Completed, "done").await,
        }
    }

    pub async fn approve(&self, run_id: &str) -> Result<WorkflowRun, WorkflowError> {
        let run = self.get_run(run_id).await?.ok_or(WorkflowError::NotFound)?;
        if run.status != RunStatus::WaitingApproval {
            return Err(WorkflowError::Invalid("run is not waiting for approval"));
        }
        let current_stage = run.current_stage.clone();
        sqlx::query("UPDATE workflow_steps SET status = 'completed', updated_at = ? WHERE run_id = ? AND stage = ? AND status = 'pending'")
            .bind(now())
            .bind(run_id)
        .bind(&current_stage)
        .execute(&self.db.pool)
        .await?;

        let definition = self
            .definition_by_id(&run.definition_id)
            .await?
            .ok_or(WorkflowError::NotFound)?;
        let index = definition
            .steps
            .iter()
            .position(|step| step.name == current_stage)
            .ok_or(WorkflowError::Invalid("unknown stage"))?;

        let mut run = match definition.steps.get(index + 1) {
            Some(next) => {
                self.update_run(run_id, RunStatus::Running, &next.name)
                    .await?
            }
            None => {
                self.update_run(run_id, RunStatus::Completed, "done")
                    .await?
            }
        };
        while run.status == RunStatus::Running {
            run = self.advance(&run.id).await?;
        }
        Ok(run)
    }

    pub async fn cancel(&self, run_id: &str) -> Result<WorkflowRun, WorkflowError> {
        let run = self.get_run(run_id).await?.ok_or(WorkflowError::NotFound)?;
        if run.status != RunStatus::Running && run.status != RunStatus::WaitingApproval {
            return Err(WorkflowError::Invalid("run is not active"));
        }
        self.update_run(run_id, RunStatus::Cancelled, &run.current_stage)
            .await
    }

    pub async fn get_run(&self, id: &str) -> Result<Option<WorkflowRun>, WorkflowError> {
        let row = sqlx::query_as::<_, (String, String, String, String, String, i64, i64)>("SELECT id, definition_id, status, current_stage, variables, created_at, updated_at FROM workflow_runs WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.db.pool)
            .await?;
        Ok(row.map(|row| WorkflowRun {
            id: row.0,
            definition_id: row.1,
            status: parse_status(&row.2),
            current_stage: row.3,
            variables: serde_json::from_str(&row.4).unwrap_or_default(),
            created_at: row.5,
            updated_at: row.6,
        }))
    }

    pub async fn list_definitions(&self) -> Vec<WorkflowDefinition> {
        let rows =
            sqlx::query_as::<_, (String,)>("SELECT config FROM workflow_definitions ORDER BY name")
                .fetch_all(&self.db.pool)
                .await
                .unwrap_or_default();
        rows.iter()
            .filter_map(|(config,)| serde_json::from_str(config).ok())
            .collect()
    }

    pub async fn latest_run(&self) -> Option<WorkflowRun> {
        let row = sqlx::query_as::<_, (String, String, String, String, String, i64, i64)>("SELECT id, definition_id, status, current_stage, variables, created_at, updated_at FROM workflow_runs ORDER BY created_at DESC, id DESC LIMIT 1")
            .fetch_optional(&self.db.pool)
            .await
            .ok()?;
        row.map(|row| WorkflowRun {
            id: row.0,
            definition_id: row.1,
            status: parse_status(&row.2),
            current_stage: row.3,
            variables: serde_json::from_str(&row.4).unwrap_or_default(),
            created_at: row.5,
            updated_at: row.6,
        })
    }

    async fn definition_by_id(
        &self,
        id: &str,
    ) -> Result<Option<WorkflowDefinition>, WorkflowError> {
        let name: Option<String> =
            sqlx::query_scalar("SELECT name FROM workflow_definitions WHERE id = ?")
                .bind(id)
                .fetch_optional(&self.db.pool)
                .await?;
        match name.as_deref() {
            Some(name) => self.get_definition(name).await,
            None => Ok(None),
        }
    }

    async fn insert_step(
        &self,
        run_id: &str,
        stage: &str,
        agent: &str,
        status: StepStatus,
        output: String,
    ) -> Result<(), WorkflowError> {
        let id = format!("{run_id}:{stage}:{agent}");
        let now = now();
        sqlx::query("INSERT INTO workflow_steps (id, run_id, stage, agent, status, output, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?) ON CONFLICT(id) DO UPDATE SET status = excluded.status, output = excluded.output, updated_at = excluded.updated_at")
            .bind(&id)
            .bind(run_id)
            .bind(stage)
            .bind(agent)
            .bind(step_status_as_str(&status))
            .bind(&output)
            .bind(now)
            .bind(now)
            .execute(&self.db.pool)
            .await?;
        Ok(())
    }

    async fn update_run(
        &self,
        id: &str,
        status: RunStatus,
        stage: &str,
    ) -> Result<WorkflowRun, WorkflowError> {
        sqlx::query(
            "UPDATE workflow_runs SET status = ?, current_stage = ?, updated_at = ? WHERE id = ?",
        )
        .bind(status_as_str(&status))
        .bind(stage)
        .bind(now())
        .bind(id)
        .execute(&self.db.pool)
        .await?;
        self.get_run(id).await?.ok_or(WorkflowError::NotFound)
    }
}

fn validate(definition: &WorkflowDefinition) -> Result<(), WorkflowError> {
    if definition.steps.is_empty() {
        return Err(WorkflowError::Invalid("workflow has no steps"));
    }
    if definition
        .steps
        .iter()
        .any(|step| step.name.trim().is_empty())
    {
        return Err(WorkflowError::Invalid("step name is empty"));
    }
    Ok(())
}

fn parse_status(value: &str) -> RunStatus {
    match value {
        "waiting_approval" => RunStatus::WaitingApproval,
        "completed" => RunStatus::Completed,
        "failed" => RunStatus::Failed,
        "cancelled" => RunStatus::Cancelled,
        _ => RunStatus::Running,
    }
}

fn status_as_str(status: &RunStatus) -> &'static str {
    match status {
        RunStatus::Running => "running",
        RunStatus::WaitingApproval => "waiting_approval",
        RunStatus::Completed => "completed",
        RunStatus::Failed => "failed",
        RunStatus::Cancelled => "cancelled",
    }
}

fn step_status_as_str(status: &StepStatus) -> &'static str {
    match status {
        StepStatus::Pending => "pending",
        StepStatus::Running => "running",
        StepStatus::Completed => "completed",
        StepStatus::Failed => "failed",
    }
}

fn deterministic_id(name: &str) -> String {
    format!("wf-{}", name.trim().to_lowercase().replace(' ', "-"))
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn full_pipeline_runs_to_human_approval() {
        let db = orbynode_database::Db::open("sqlite::memory:")
            .await
            .unwrap();
        let engine = WorkflowEngine::new(db);
        let definition = WorkflowDefinition {
            name: "delivery".into(),
            steps: vec![
                WorkflowStep {
                    name: "plan".into(),
                    kind: StepKind::Agent,
                    agents: vec!["planner".into()],
                    command: Some("true".into()),
                },
                WorkflowStep {
                    name: "implement".into(),
                    kind: StepKind::Parallel,
                    agents: vec!["backend".into(), "frontend".into()],
                    command: None,
                },
                WorkflowStep {
                    name: "test".into(),
                    kind: StepKind::Agent,
                    agents: vec!["test".into()],
                    command: Some("true".into()),
                },
                WorkflowStep {
                    name: "review".into(),
                    kind: StepKind::Agent,
                    agents: vec!["reviewer".into()],
                    command: Some("true".into()),
                },
                WorkflowStep {
                    name: "approve".into(),
                    kind: StepKind::Approval,
                    agents: vec![],
                    command: None,
                },
            ],
        };
        engine.put_definition(definition.clone()).await.unwrap();
        let started = engine.start("delivery", BTreeMap::new()).await.unwrap();
        tracing::info!(status = ?started.status, stage = %started.current_stage, "workflow started");
        assert_eq!(started.status, RunStatus::WaitingApproval);
        assert_eq!(started.current_stage, "approve");
        let approved = engine.approve(&started.id).await.unwrap();
        assert_eq!(approved.status, RunStatus::Completed);
    }
}
