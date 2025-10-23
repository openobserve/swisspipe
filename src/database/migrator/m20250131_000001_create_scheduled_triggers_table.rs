use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ScheduledTriggers::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ScheduledTriggers::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ScheduledTriggers::WorkflowId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ScheduledTriggers::TriggerNodeId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ScheduledTriggers::ScheduleName)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ScheduledTriggers::CronExpression)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ScheduledTriggers::Timezone)
                            .string()
                            .not_null()
                            .default("UTC"),
                    )
                    .col(
                        ColumnDef::new(ScheduledTriggers::TestPayload)
                            .json_binary()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ScheduledTriggers::Enabled)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(ScheduledTriggers::StartDate)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ScheduledTriggers::EndDate)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ScheduledTriggers::LastExecutionTime)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ScheduledTriggers::NextExecutionTime)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ScheduledTriggers::ExecutionCount)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(ScheduledTriggers::FailureCount)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(ScheduledTriggers::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(ScheduledTriggers::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index for efficient querying of due schedules
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_scheduled_triggers_next_execution")
                    .table(ScheduledTriggers::Table)
                    .col(ScheduledTriggers::NextExecutionTime)
                    .to_owned(),
            )
            .await?;

        // Create index for workflow lookups
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_scheduled_triggers_workflow")
                    .table(ScheduledTriggers::Table)
                    .col(ScheduledTriggers::WorkflowId)
                    .to_owned(),
            )
            .await?;

        // Create composite index for enabled schedules with next execution time
        // This optimizes the get_enabled_schedules() query
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_scheduled_triggers_enabled_next_exec")
                    .table(ScheduledTriggers::Table)
                    .col(ScheduledTriggers::Enabled)
                    .col(ScheduledTriggers::NextExecutionTime)
                    .to_owned(),
            )
            .await?;

        // Create unique constraint on workflow_id + trigger_node_id
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_scheduled_triggers_unique_workflow_node")
                    .table(ScheduledTriggers::Table)
                    .col(ScheduledTriggers::WorkflowId)
                    .col(ScheduledTriggers::TriggerNodeId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ScheduledTriggers::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ScheduledTriggers {
    Table,
    Id,
    WorkflowId,
    TriggerNodeId,
    ScheduleName,
    CronExpression,
    Timezone,
    TestPayload,
    Enabled,
    StartDate,
    EndDate,
    LastExecutionTime,
    NextExecutionTime,
    ExecutionCount,
    FailureCount,
    CreatedAt,
    UpdatedAt,
}
