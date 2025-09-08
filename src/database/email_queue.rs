use sea_orm::entity::prelude::*;
use sea_orm::{ActiveModelTrait, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "email_queue")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub execution_id: Option<String>,
    pub node_id: Option<String>,
    pub smtp_config: String,
    pub priority: String,
    pub email_config: String, // JSON serialized EmailConfig
    pub template_context: String, // JSON serialized template context
    pub status: String,
    pub queued_at: i64, // Unix epoch microseconds
    pub scheduled_at: Option<i64>, // Unix epoch microseconds
    pub processed_at: Option<i64>, // Unix epoch microseconds
    pub sent_at: Option<i64>, // Unix epoch microseconds
    pub max_wait_minutes: i32,
    pub retry_count: i32,
    pub max_retries: i32,
    pub error_message: Option<String>,
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
        let now = chrono::Utc::now().timestamp_micros();
        Self {
            id: Set(Uuid::now_v7().to_string()),
            smtp_config: Set("default".to_string()),
            priority: Set("normal".to_string()),
            status: Set("queued".to_string()),
            queued_at: Set(now),
            max_wait_minutes: Set(60),
            retry_count: Set(0),
            max_retries: Set(3),
            created_at: Set(now),
            updated_at: Set(now),
            ..ActiveModelTrait::default()
        }
    }
}

// Email queue status enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmailQueueStatus {
    Queued,
    Processing,
    Sent,
    Failed,
    Expired,
}

impl std::fmt::Display for EmailQueueStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmailQueueStatus::Queued => write!(f, "queued"),
            EmailQueueStatus::Processing => write!(f, "processing"),
            EmailQueueStatus::Sent => write!(f, "sent"),
            EmailQueueStatus::Failed => write!(f, "failed"),
            EmailQueueStatus::Expired => write!(f, "expired"),
        }
    }
}

impl From<String> for EmailQueueStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "queued" => EmailQueueStatus::Queued,
            "processing" => EmailQueueStatus::Processing,
            "sent" => EmailQueueStatus::Sent,
            "failed" => EmailQueueStatus::Failed,
            "expired" => EmailQueueStatus::Expired,
            _ => EmailQueueStatus::Queued,
        }
    }
}