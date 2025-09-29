use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create human_in_loop_tasks table without foreign keys and with Unix epoch microseconds
        manager
            .create_table(
                Table::create()
                    .table(HumanInLoopTasks::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(HumanInLoopTasks::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(HumanInLoopTasks::ExecutionId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HumanInLoopTasks::NodeId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HumanInLoopTasks::NodeExecutionId)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(HumanInLoopTasks::WorkflowId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HumanInLoopTasks::Title)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HumanInLoopTasks::Description)
                            .text(),
                    )
                    .col(
                        ColumnDef::new(HumanInLoopTasks::Status)
                            .string()
                            .not_null()
                            .default("pending"),
                    )
                    .col(
                        ColumnDef::new(HumanInLoopTasks::TimeoutAt)
                            .big_integer(),
                    )
                    .col(
                        ColumnDef::new(HumanInLoopTasks::TimeoutAction)
                            .string(),
                    )
                    .col(
                        ColumnDef::new(HumanInLoopTasks::RequiredFields)
                            .text(),
                    )
                    .col(
                        ColumnDef::new(HumanInLoopTasks::Metadata)
                            .text(),
                    )
                    .col(
                        ColumnDef::new(HumanInLoopTasks::ResponseData)
                            .text(),
                    )
                    .col(
                        ColumnDef::new(HumanInLoopTasks::ResponseReceivedAt)
                            .big_integer(),
                    )
                    .col(
                        ColumnDef::new(HumanInLoopTasks::CreatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HumanInLoopTasks::UpdatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Add indexes for HIL table performance
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_human_in_loop_tasks_status")
                    .table(HumanInLoopTasks::Table)
                    .col(HumanInLoopTasks::Status)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_human_in_loop_tasks_execution_id")
                    .table(HumanInLoopTasks::Table)
                    .col(HumanInLoopTasks::ExecutionId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_human_in_loop_tasks_node_execution_id")
                    .table(HumanInLoopTasks::Table)
                    .col(HumanInLoopTasks::NodeExecutionId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop HIL table
        manager
            .drop_table(Table::drop().table(HumanInLoopTasks::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum HumanInLoopTasks {
    Table,
    Id,
    ExecutionId,
    NodeId,
    NodeExecutionId,
    WorkflowId,
    Title,
    Description,
    Status,
    TimeoutAt,
    TimeoutAction,
    RequiredFields,
    Metadata,
    ResponseData,
    ResponseReceivedAt,
    CreatedAt,
    UpdatedAt,
}

