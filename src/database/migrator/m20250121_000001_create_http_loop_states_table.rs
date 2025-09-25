use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(HttpLoopStates::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(HttpLoopStates::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(HttpLoopStates::ExecutionStepId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HttpLoopStates::CurrentIteration)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(HttpLoopStates::MaxIterations)
                            .integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(HttpLoopStates::NextExecutionAt)
                            .big_integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(HttpLoopStates::ConsecutiveFailures)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(HttpLoopStates::LoopStartedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HttpLoopStates::LastResponseStatus)
                            .integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(HttpLoopStates::LastResponseBody)
                            .text()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(HttpLoopStates::IterationHistory)
                            .text()
                            .not_null()
                            .default("[]"),
                    )
                    .col(
                        ColumnDef::new(HttpLoopStates::Status)
                            .string()
                            .not_null()
                            .default("running"),
                    )
                    .col(
                        ColumnDef::new(HttpLoopStates::TerminationReason)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(HttpLoopStates::CreatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(HttpLoopStates::UpdatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Add index on execution_step_id for fast lookups
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx-http_loop_states-execution_step_id")
                    .table(HttpLoopStates::Table)
                    .col(HttpLoopStates::ExecutionStepId)
                    .to_owned(),
            )
            .await?;

        // Add index on next_execution_at for scheduler queries
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx-http_loop_states-next_execution_at")
                    .table(HttpLoopStates::Table)
                    .col(HttpLoopStates::NextExecutionAt)
                    .to_owned(),
            )
            .await?;

        // Add index on status for filtering active loops
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx-http_loop_states-status")
                    .table(HttpLoopStates::Table)
                    .col(HttpLoopStates::Status)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(HttpLoopStates::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum HttpLoopStates {
    Table,
    Id,
    ExecutionStepId,
    CurrentIteration,
    MaxIterations,
    NextExecutionAt,
    ConsecutiveFailures,
    LoopStartedAt,
    LastResponseStatus,
    LastResponseBody,
    IterationHistory,
    Status,
    TerminationReason,
    CreatedAt,
    UpdatedAt,
}