use sea_orm::entity::prelude::*;
use sea_orm::{ActiveModelTrait, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "workflow_execution_steps")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub execution_id: String,
    pub node_id: String, // Node ID reference
    pub node_name: String, // Node name for easy display
    pub status: String, // 'pending', 'running', 'completed', 'failed', 'skipped', 'cancelled'
    pub input_data: Option<String>, // JSON
    pub output_data: Option<String>, // JSON
    pub error_message: Option<String>,
    pub started_at: Option<i64>, // Unix epoch microseconds
    pub completed_at: Option<i64>, // Unix epoch microseconds
    pub created_at: i64, // Unix epoch microseconds
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::workflow_executions::Entity",
        from = "Column::ExecutionId",
        to = "super::workflow_executions::Column::Id",
        on_delete = "Cascade"
    )]
    WorkflowExecution,
    #[sea_orm(
        belongs_to = "super::nodes::Entity",
        from = "Column::NodeId",
        to = "super::nodes::Column::Id"
    )]
    Node,
}

impl Related<super::workflow_executions::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::WorkflowExecution.def()
    }
}

impl Related<super::nodes::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Node.def()
    }
}

impl ActiveModelBehavior for ActiveModel {
    fn new() -> Self {
        Self {
            id: Set(Uuid::now_v7().to_string()),
            status: Set("pending".to_string()),
            created_at: Set(chrono::Utc::now().timestamp_micros()),
            ..ActiveModelTrait::default()
        }
    }
}

// Step status enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
    Cancelled,
}

impl std::fmt::Display for StepStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StepStatus::Pending => write!(f, "pending"),
            StepStatus::Running => write!(f, "running"),
            StepStatus::Completed => write!(f, "completed"),
            StepStatus::Failed => write!(f, "failed"),
            StepStatus::Skipped => write!(f, "skipped"),
            StepStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl From<String> for StepStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "pending" => StepStatus::Pending,
            "running" => StepStatus::Running,
            "completed" => StepStatus::Completed,
            "failed" => StepStatus::Failed,
            "skipped" => StepStatus::Skipped,
            "cancelled" => StepStatus::Cancelled,
            _ => StepStatus::Pending,
        }
    }
}