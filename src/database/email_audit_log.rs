use sea_orm::entity::prelude::*;
use sea_orm::{ActiveModelTrait, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "email_audit_log")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub execution_id: String,
    pub node_id: String,
    pub smtp_config: String,
    pub from_email: String,
    pub to_emails: String, // JSON array of email addresses
    pub cc_emails: Option<String>, // JSON array of email addresses
    pub bcc_emails: Option<String>, // JSON array of email addresses
    pub subject: String,
    pub email_size_bytes: i32,
    pub attachment_count: i32,
    pub status: String, // 'sent', 'failed', 'partial'
    pub error_message: Option<String>,
    pub smtp_message_id: Option<String>,
    pub sent_at: Option<i64>, // Unix epoch microseconds
    pub created_at: i64, // Unix epoch microseconds
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
            email_size_bytes: Set(0),
            attachment_count: Set(0),
            status: Set("sent".to_string()),
            created_at: Set(now),
            ..ActiveModelTrait::default()
        }
    }
}

// Email audit status enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmailAuditStatus {
    Sent,
    Failed,
    Partial,
}

impl std::fmt::Display for EmailAuditStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmailAuditStatus::Sent => write!(f, "sent"),
            EmailAuditStatus::Failed => write!(f, "failed"),
            EmailAuditStatus::Partial => write!(f, "partial"),
        }
    }
}

impl From<String> for EmailAuditStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "sent" => EmailAuditStatus::Sent,
            "failed" => EmailAuditStatus::Failed,
            "partial" => EmailAuditStatus::Partial,
            _ => EmailAuditStatus::Sent,
        }
    }
}