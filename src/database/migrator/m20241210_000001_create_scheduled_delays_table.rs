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
                    .table(ScheduledDelays::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ScheduledDelays::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ScheduledDelays::ExecutionId).string().not_null())
                    .col(ColumnDef::new(ScheduledDelays::CurrentNodeName).string().not_null())
                    .col(ColumnDef::new(ScheduledDelays::NextNodeName).string().not_null())
                    .col(ColumnDef::new(ScheduledDelays::ScheduledAt).big_integer().not_null())
                    .col(ColumnDef::new(ScheduledDelays::CreatedAt).big_integer().not_null())
                    .col(
                        ColumnDef::new(ScheduledDelays::Status)
                            .string()
                            .not_null()
                            .default("pending"),
                    )
                    .col(ColumnDef::new(ScheduledDelays::WorkflowState).text().not_null())
                    .col(ColumnDef::new(ScheduledDelays::SchedulerJobId).string())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-scheduled_delays-execution_id")
                            .from(ScheduledDelays::Table, ScheduledDelays::ExecutionId)
                            .to(WorkflowExecutions::Table, WorkflowExecutions::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create performance indices for scheduled delay operations
        manager
            .create_index(
                Index::create()
                    .name("idx_scheduled_delays_status_scheduled_at")
                    .table(ScheduledDelays::Table)
                    .col((ScheduledDelays::Status, IndexOrder::Asc))
                    .col((ScheduledDelays::ScheduledAt, IndexOrder::Asc))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_scheduled_delays_execution_id")
                    .table(ScheduledDelays::Table)
                    .col(ScheduledDelays::ExecutionId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_scheduled_delays_scheduler_job_id")
                    .table(ScheduledDelays::Table)
                    .col(ScheduledDelays::SchedulerJobId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ScheduledDelays::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum ScheduledDelays {
    Table,
    Id,
    ExecutionId,
    CurrentNodeName,
    NextNodeName,
    ScheduledAt,
    CreatedAt,
    Status,
    WorkflowState,
    SchedulerJobId,
}