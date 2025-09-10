use sea_orm::entity::prelude::*;
use sea_orm::{ActiveModelTrait, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "job_queue")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub execution_id: String,
    pub priority: i32,
    pub scheduled_at: i64, // Unix epoch microseconds
    pub claimed_at: Option<i64>, // Unix epoch microseconds
    pub claimed_by: Option<String>, // worker_id
    pub max_retries: i32,
    pub retry_count: i32,
    pub status: String, // 'pending', 'claimed', 'processing', 'completed', 'failed', 'dead_letter'
    pub error_message: Option<String>,
    pub payload: Option<String>, // JSON payload for special job types
    pub created_at: i64, // Unix epoch microseconds
    pub updated_at: i64, // Unix epoch microseconds
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::workflow_executions::Entity",
        from = "Column::ExecutionId",
        to = "super::workflow_executions::Column::Id"
    )]
    WorkflowExecution,
}

impl Related<super::workflow_executions::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::WorkflowExecution.def()
    }
}

impl ActiveModelBehavior for ActiveModel {
    fn new() -> Self {
        let now = chrono::Utc::now().timestamp_micros();
        let max_retries = std::env::var("SP_WORKFLOW_MAX_RETRIES")
            .ok()
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(0);
            
        Self {
            id: Set(Uuid::now_v7().to_string()),
            priority: Set(0),
            scheduled_at: Set(now),
            max_retries: Set(max_retries),
            retry_count: Set(0),
            status: Set("pending".to_string()),
            payload: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..ActiveModelTrait::default()
        }
    }

}

// Job status enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    Pending,
    Claimed,
    Processing,
    Completed,
    Failed,
    DeadLetter,
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobStatus::Pending => write!(f, "pending"),
            JobStatus::Claimed => write!(f, "claimed"),
            JobStatus::Processing => write!(f, "processing"),
            JobStatus::Completed => write!(f, "completed"),
            JobStatus::Failed => write!(f, "failed"),
            JobStatus::DeadLetter => write!(f, "dead_letter"),
        }
    }
}

impl From<String> for JobStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "pending" => JobStatus::Pending,
            "claimed" => JobStatus::Claimed,
            "processing" => JobStatus::Processing,
            "completed" => JobStatus::Completed,
            "failed" => JobStatus::Failed,
            "dead_letter" => JobStatus::DeadLetter,
            _ => JobStatus::Pending,
        }
    }
}