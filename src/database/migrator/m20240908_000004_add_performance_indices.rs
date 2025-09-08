use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Performance indices for workflow_executions table
        manager
            .create_index(
                Index::create()
                    .name("idx_workflow_executions_status")
                    .table(WorkflowExecutions::Table)
                    .col(WorkflowExecutions::Status)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_workflow_executions_workflow_id")
                    .table(WorkflowExecutions::Table)
                    .col(WorkflowExecutions::WorkflowId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_workflow_executions_created_at")
                    .table(WorkflowExecutions::Table)
                    .col(WorkflowExecutions::CreatedAt)
                    .to_owned(),
            )
            .await?;

        // Performance indices for workflow_execution_steps table
        manager
            .create_index(
                Index::create()
                    .name("idx_execution_steps_execution_id")
                    .table(WorkflowExecutionSteps::Table)
                    .col(WorkflowExecutionSteps::ExecutionId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_execution_steps_status")
                    .table(WorkflowExecutionSteps::Table)
                    .col(WorkflowExecutionSteps::Status)
                    .to_owned(),
            )
            .await?;

        // Performance indices for job_queue table
        // Composite index for job polling (most critical)
        manager
            .create_index(
                Index::create()
                    .name("idx_job_queue_status_priority")
                    .table(JobQueue::Table)
                    .col(JobQueue::Status)
                    .col(JobQueue::Priority)
                    .col(JobQueue::ScheduledAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_job_queue_claimed_by")
                    .table(JobQueue::Table)
                    .col(JobQueue::ClaimedBy)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_job_queue_execution_id")
                    .table(JobQueue::Table)
                    .col(JobQueue::ExecutionId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_job_queue_claimed_at")
                    .table(JobQueue::Table)
                    .col(JobQueue::ClaimedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_job_queue_retry_count")
                    .table(JobQueue::Table)
                    .col(JobQueue::RetryCount)
                    .col(JobQueue::Status)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop all indices in reverse order
        manager
            .drop_index(Index::drop().name("idx_job_queue_retry_count").to_owned())
            .await?;

        manager
            .drop_index(Index::drop().name("idx_job_queue_claimed_at").to_owned())
            .await?;

        manager
            .drop_index(Index::drop().name("idx_job_queue_execution_id").to_owned())
            .await?;

        manager
            .drop_index(Index::drop().name("idx_job_queue_claimed_by").to_owned())
            .await?;

        manager
            .drop_index(Index::drop().name("idx_job_queue_status_priority").to_owned())
            .await?;

        manager
            .drop_index(Index::drop().name("idx_execution_steps_status").to_owned())
            .await?;

        manager
            .drop_index(Index::drop().name("idx_execution_steps_execution_id").to_owned())
            .await?;

        manager
            .drop_index(Index::drop().name("idx_workflow_executions_created_at").to_owned())
            .await?;

        manager
            .drop_index(Index::drop().name("idx_workflow_executions_workflow_id").to_owned())
            .await?;

        manager
            .drop_index(Index::drop().name("idx_workflow_executions_status").to_owned())
            .await?;

        Ok(())
    }
}

#[derive(Iden)]
enum WorkflowExecutions {
    Table,
    Status,
    WorkflowId,
    CreatedAt,
}

#[derive(Iden)]
enum WorkflowExecutionSteps {
    Table,
    ExecutionId,
    Status,
}

#[derive(Iden)]
enum JobQueue {
    Table,
    Status,
    Priority,
    ScheduledAt,
    ClaimedBy,
    ExecutionId,
    ClaimedAt,
    RetryCount,
}