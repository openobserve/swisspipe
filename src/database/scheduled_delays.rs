use sea_orm::entity::prelude::*;
use sea_orm::Set;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "scheduled_delays")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub execution_id: String,
    pub current_node_name: String,
    pub next_node_name: String,
    pub scheduled_at: i64, // Unix epoch microseconds - when delay should trigger
    pub created_at: i64,   // Unix epoch microseconds - when delay was scheduled
    pub status: String,    // 'pending', 'triggered', 'cancelled'
    pub workflow_state: String, // JSON serialized WorkflowEvent
    pub scheduler_job_id: Option<String>, // tokio-cron-scheduler job UUID
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
        Self {
            id: Set(Uuid::now_v7().to_string()),
            status: Set("pending".to_string()),
            created_at: Set(now),
            ..Default::default()
        }
    }
}

// Status enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DelayStatus {
    Pending,    // Scheduled but not yet triggered
    Triggered,  // Delay completed, workflow resumed
    Cancelled,  // Delay cancelled (workflow stopped/failed)
}

impl std::fmt::Display for DelayStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DelayStatus::Pending => write!(f, "pending"),
            DelayStatus::Triggered => write!(f, "triggered"),
            DelayStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl From<String> for DelayStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "pending" => DelayStatus::Pending,
            "triggered" => DelayStatus::Triggered, 
            "cancelled" => DelayStatus::Cancelled,
            _ => DelayStatus::Pending,
        }
    }
}