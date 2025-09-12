use sea_orm_migration::prelude::*;
use super::m20240907_000001_create_workflows_table::Workflows;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(WorkflowExecutions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(WorkflowExecutions::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(WorkflowExecutions::WorkflowId).string().not_null())
                    .col(ColumnDef::new(WorkflowExecutions::Status).string().not_null())
                    .col(ColumnDef::new(WorkflowExecutions::CurrentNodeId).string())
                    .col(ColumnDef::new(WorkflowExecutions::InputData).text())
                    .col(ColumnDef::new(WorkflowExecutions::OutputData).text())
                    .col(ColumnDef::new(WorkflowExecutions::ErrorMessage).text())
                    .col(ColumnDef::new(WorkflowExecutions::StartedAt).big_integer())
                    .col(ColumnDef::new(WorkflowExecutions::CompletedAt).big_integer())
                    .col(
                        ColumnDef::new(WorkflowExecutions::CreatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WorkflowExecutions::UpdatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-workflow_executions-workflow_id")
                            .from(WorkflowExecutions::Table, WorkflowExecutions::WorkflowId)
                            .to(Workflows::Table, Workflows::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create performance indices
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

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(WorkflowExecutions::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum WorkflowExecutions {
    Table,
    Id,
    WorkflowId,
    Status,
    CurrentNodeId,
    InputData,
    OutputData,
    ErrorMessage,
    StartedAt,
    CompletedAt,
    CreatedAt,
    UpdatedAt,
}