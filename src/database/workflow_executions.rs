use sea_orm::entity::prelude::*;
use sea_orm::{ActiveModelTrait, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "workflow_executions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub workflow_id: String,
    pub status: String, // 'pending', 'running', 'completed', 'failed', 'cancelled'
    pub current_node_name: Option<String>,
    pub input_data: Option<String>, // JSON
    pub output_data: Option<String>, // JSON
    pub error_message: Option<String>,
    pub started_at: Option<i64>, // Unix epoch microseconds
    pub completed_at: Option<i64>, // Unix epoch microseconds
    pub created_at: i64, // Unix epoch microseconds
    pub updated_at: i64, // Unix epoch microseconds
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::entities::Entity",
        from = "Column::WorkflowId",
        to = "super::entities::Column::Id"
    )]
    Workflow,
    #[sea_orm(
        has_many = "super::workflow_execution_steps::Entity",
        on_delete = "Cascade"
    )]
    WorkflowExecutionSteps,
    #[sea_orm(has_many = "super::job_queue::Entity")]
    JobQueue,
}

impl Related<super::entities::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Workflow.def()
    }
}

impl Related<super::workflow_execution_steps::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::WorkflowExecutionSteps.def()
    }
}

impl Related<super::job_queue::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::JobQueue.def()
    }
}

impl ActiveModelBehavior for ActiveModel {
    fn new() -> Self {
        let now = chrono::Utc::now().timestamp_micros();
        Self {
            id: Set(Uuid::now_v7().to_string()),
            status: Set("pending".to_string()),
            created_at: Set(now),
            updated_at: Set(now),
            ..ActiveModelTrait::default()
        }
    }

}

// Execution status enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for ExecutionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionStatus::Pending => write!(f, "pending"),
            ExecutionStatus::Running => write!(f, "running"),
            ExecutionStatus::Completed => write!(f, "completed"),
            ExecutionStatus::Failed => write!(f, "failed"),
            ExecutionStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl From<String> for ExecutionStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "pending" => ExecutionStatus::Pending,
            "running" => ExecutionStatus::Running,
            "completed" => ExecutionStatus::Completed,
            "failed" => ExecutionStatus::Failed,
            "cancelled" => ExecutionStatus::Cancelled,
            _ => ExecutionStatus::Pending,
        }
    }
}