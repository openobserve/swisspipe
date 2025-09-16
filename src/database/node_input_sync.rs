use sea_orm::entity::prelude::*;
use sea_orm::{ActiveModelTrait, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "node_input_sync")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub execution_id: String,
    pub node_id: String,
    pub expected_input_count: i32,
    pub received_inputs: String, // JSON array of WorkflowEvents
    pub timeout_at: Option<i64>, // Unix epoch microseconds
    pub status: String, // "waiting", "ready", "completed", "timeout"
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
        Self {
            id: Set(Uuid::new_v4().to_string()),
            created_at: Set(now),
            updated_at: Set(now),
            status: Set("waiting".to_string()),
            received_inputs: Set("[]".to_string()),
            ..ActiveModelTrait::default()
        }
    }

    fn before_save<'life0, 'async_trait, C>(
        mut self,
        _db: &'life0 C,
        insert: bool,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self, DbErr>> + Send + 'async_trait>>
    where
        Self: 'async_trait,
        C: ConnectionTrait + 'life0,
        'life0: 'async_trait,
    {
        Box::pin(async move {
            if !insert {
                self.updated_at = Set(chrono::Utc::now().timestamp_micros());
            }
            Ok(self)
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncStatus {
    Waiting,   // Waiting for more inputs
    Ready,     // All inputs received, ready to execute
    Completed, // Node has been executed
    Timeout,   // Timeout exceeded
}

impl std::fmt::Display for SyncStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status_str = match self {
            SyncStatus::Waiting => "waiting",
            SyncStatus::Ready => "ready", 
            SyncStatus::Completed => "completed",
            SyncStatus::Timeout => "timeout",
        };
        write!(f, "{status_str}")
    }
}

impl From<String> for SyncStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "waiting" => SyncStatus::Waiting,
            "ready" => SyncStatus::Ready,
            "completed" => SyncStatus::Completed,
            "timeout" => SyncStatus::Timeout,
            _ => SyncStatus::Waiting,
        }
    }
}