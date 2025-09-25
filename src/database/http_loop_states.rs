use sea_orm::entity::prelude::*;
use sea_orm::{ActiveModelTrait, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "http_loop_states")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub execution_step_id: String,
    pub current_iteration: i32,
    pub max_iterations: Option<i32>,
    pub next_execution_at: Option<i64>, // Unix epoch microseconds
    pub consecutive_failures: i32,
    pub loop_started_at: i64, // Unix epoch microseconds
    pub last_response_status: Option<i32>,
    pub last_response_body: Option<String>,
    pub iteration_history: String, // JSON array of iteration results
    pub status: String, // 'running', 'completed', 'failed'
    pub termination_reason: Option<String>, // 'Success', 'MaxIterations', 'Failure'
    // Configuration fields for proper resumption
    pub url: String, // HTTP URL for requests
    pub method: String, // HTTP method (GET, POST, etc.)
    pub timeout_seconds: i64, // Request timeout
    pub headers: String, // JSON-encoded headers
    pub loop_configuration: String, // JSON-encoded LoopConfig (termination conditions, backoff strategy)
    pub initial_event: String, // JSON-encoded initial WorkflowEvent
    pub created_at: i64, // Unix epoch microseconds
    pub updated_at: i64, // Unix epoch microseconds
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::workflow_execution_steps::Entity",
        from = "Column::ExecutionStepId",
        to = "super::workflow_execution_steps::Column::Id"
    )]
    WorkflowExecutionStep,
}

impl Related<super::workflow_execution_steps::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::WorkflowExecutionStep.def()
    }
}

impl ActiveModelBehavior for ActiveModel {
    fn new() -> Self {
        let now = chrono::Utc::now().timestamp_micros();
        Self {
            id: Set(Uuid::new_v4().to_string()),
            created_at: Set(now),
            updated_at: Set(now),
            status: Set("running".to_string()),
            current_iteration: Set(0),
            consecutive_failures: Set(0),
            loop_started_at: Set(now),
            iteration_history: Set("[]".to_string()),
            // Default values for configuration fields
            url: Set(String::new()),
            method: Set("GET".to_string()),
            timeout_seconds: Set(30),
            headers: Set("{}".to_string()),
            loop_configuration: Set("{}".to_string()),
            initial_event: Set("{}".to_string()),
            ..ActiveModelTrait::default()
        }
    }

    fn before_save<'life0, 'async_trait, C>(
        mut self,
        _db: &'life0 C,
        _insert: bool,
    ) -> core::pin::Pin<
        Box<
            dyn core::future::Future<Output = Result<Self, DbErr>>
                + core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
        C: 'async_trait + ConnectionTrait,
    {
        Box::pin(async move {
            self.updated_at = Set(chrono::Utc::now().timestamp_micros());
            Ok(self)
        })
    }
}

/// Status values for HTTP loop states
#[derive(Debug, Clone, PartialEq)]
pub enum LoopStatus {
    Running,
    Completed,
    Failed,
    Paused,
    Cancelled,
}

impl std::fmt::Display for LoopStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoopStatus::Running => write!(f, "running"),
            LoopStatus::Completed => write!(f, "completed"),
            LoopStatus::Failed => write!(f, "failed"),
            LoopStatus::Paused => write!(f, "paused"),
            LoopStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl std::str::FromStr for LoopStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "running" => Ok(LoopStatus::Running),
            "completed" => Ok(LoopStatus::Completed),
            "failed" => Ok(LoopStatus::Failed),
            "paused" => Ok(LoopStatus::Paused),
            "cancelled" => Ok(LoopStatus::Cancelled),
            _ => Err(format!("Invalid loop status: {s}")),
        }
    }
}

/// Termination reasons for HTTP loops
#[derive(Debug, Clone, PartialEq)]
pub enum LoopTerminationReason {
    Success,
    MaxIterations,
    Failure,
    Stopped,
}

impl std::fmt::Display for LoopTerminationReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoopTerminationReason::Success => write!(f, "Success"),
            LoopTerminationReason::MaxIterations => write!(f, "MaxIterations"),
            LoopTerminationReason::Failure => write!(f, "Failure"),
            LoopTerminationReason::Stopped => write!(f, "Stopped"),
        }
    }
}

/// Iteration result stored in the iteration history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IterationResult {
    pub iteration: u32,
    pub timestamp: i64,
    pub http_status: Option<i32>,
    pub success: bool,
    pub response_snippet: Option<String>, // First 1000 chars of response
    pub error_message: Option<String>,
}

impl Model {
    /// Parse iteration history JSON into structured data
    pub fn get_iteration_history(&self) -> Result<Vec<IterationResult>, serde_json::Error> {
        serde_json::from_str(&self.iteration_history)
    }

    /// Add a new iteration result to the history with configurable limit
    pub fn add_iteration_result_with_limit(&mut self, result: IterationResult, max_entries: usize) -> Result<(), serde_json::Error> {
        let mut history = self.get_iteration_history().unwrap_or_default();
        history.push(result);

        // Keep only last N iterations to prevent unbounded growth
        if history.len() > max_entries {
            let skip_count = history.len() - max_entries;
            history = history.into_iter().skip(skip_count).collect();
        }

        self.iteration_history = serde_json::to_string(&history)?;
        Ok(())
    }

    /// Add a new iteration result to the history (legacy method with default limit)
    pub fn add_iteration_result(&mut self, result: IterationResult) -> Result<(), serde_json::Error> {
        // Use default limit of 100 for backward compatibility
        self.add_iteration_result_with_limit(result, 100)
    }

    /// Calculate success rate based on iteration history
    pub fn calculate_success_rate(&self) -> f64 {
        let history = self.get_iteration_history().unwrap_or_default();
        if history.is_empty() {
            return 0.0;
        }

        let successful = history.iter().filter(|r| r.success).count();
        successful as f64 / history.len() as f64
    }
}