use sea_orm_migration::prelude::*;
use super::m20240908_000001_create_workflow_executions_table::WorkflowExecutions;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(JobQueue::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(JobQueue::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(JobQueue::ExecutionId).string().not_null())
                    .col(ColumnDef::new(JobQueue::Priority).integer().default(0))
                    .col(ColumnDef::new(JobQueue::ScheduledAt).big_integer().not_null())
                    .col(ColumnDef::new(JobQueue::ClaimedAt).big_integer())
                    .col(ColumnDef::new(JobQueue::ClaimedBy).string())
                    .col(ColumnDef::new(JobQueue::MaxRetries).integer().default(3))
                    .col(ColumnDef::new(JobQueue::RetryCount).integer().default(0))
                    .col(
                        ColumnDef::new(JobQueue::Status)
                            .string()
                            .not_null()
                            .default("pending"),
                    )
                    .col(ColumnDef::new(JobQueue::ErrorMessage).text())
                    .col(ColumnDef::new(JobQueue::Payload).text().null())
                    .col(ColumnDef::new(JobQueue::CreatedAt).big_integer().not_null())
                    .col(ColumnDef::new(JobQueue::UpdatedAt).big_integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-job_queue-execution_id")
                            .from(JobQueue::Table, JobQueue::ExecutionId)
                            .to(WorkflowExecutions::Table, WorkflowExecutions::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create performance indices for job queue operations
        manager
            .create_index(
                Index::create()
                    .name("idx_job_queue_status_priority")
                    .table(JobQueue::Table)
                    .col((JobQueue::Status, IndexOrder::Asc))
                    .col((JobQueue::Priority, IndexOrder::Desc))
                    .col((JobQueue::ScheduledAt, IndexOrder::Asc))
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
        manager
            .drop_table(Table::drop().table(JobQueue::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum JobQueue {
    Table,
    Id,
    ExecutionId,
    Priority,
    ScheduledAt,
    ClaimedAt,
    ClaimedBy,
    MaxRetries,
    RetryCount,
    Status,
    ErrorMessage,
    Payload,
    CreatedAt,
    UpdatedAt,
}