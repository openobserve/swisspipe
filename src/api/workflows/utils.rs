use sea_orm::{EntityTrait, ColumnTrait, QueryFilter, PaginatorTrait};
use crate::database::workflow_executions;

/// Check if there are any active executions for the workflow
pub async fn has_active_executions(db: &sea_orm::DatabaseConnection, workflow_id: &str) -> Result<bool, sea_orm::DbErr> {
    let count = workflow_executions::Entity::find()
        .filter(workflow_executions::Column::WorkflowId.eq(workflow_id))
        .filter(workflow_executions::Column::Status.is_in(["running", "pending"]))
        .count(db)
        .await?;
    
    let has_active = count > 0;
    tracing::debug!(
        "Active executions check: workflow_id={}, active_count={}, has_active={}", 
        workflow_id, count, has_active
    );
    
    Ok(has_active)
}