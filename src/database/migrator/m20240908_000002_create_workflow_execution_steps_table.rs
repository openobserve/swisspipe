use sea_orm_migration::prelude::*;
use super::m20240907_000002_create_nodes_table::Nodes;
use super::m20240908_000001_create_workflow_executions_table::WorkflowExecutions;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(WorkflowExecutionSteps::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(WorkflowExecutionSteps::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(WorkflowExecutionSteps::ExecutionId).string().not_null())
                    .col(ColumnDef::new(WorkflowExecutionSteps::NodeId).string().not_null())
                    .col(ColumnDef::new(WorkflowExecutionSteps::NodeName).string().not_null())
                    .col(ColumnDef::new(WorkflowExecutionSteps::Status).string().not_null())
                    .col(ColumnDef::new(WorkflowExecutionSteps::InputData).text())
                    .col(ColumnDef::new(WorkflowExecutionSteps::OutputData).text())
                    .col(ColumnDef::new(WorkflowExecutionSteps::ErrorMessage).text())
                    .col(ColumnDef::new(WorkflowExecutionSteps::StartedAt).big_integer())
                    .col(ColumnDef::new(WorkflowExecutionSteps::CompletedAt).big_integer())
                    .col(
                        ColumnDef::new(WorkflowExecutionSteps::CreatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-workflow_execution_steps-execution_id")
                            .from(WorkflowExecutionSteps::Table, WorkflowExecutionSteps::ExecutionId)
                            .to(WorkflowExecutions::Table, WorkflowExecutions::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-workflow_execution_steps-node_id")
                            .from(WorkflowExecutionSteps::Table, WorkflowExecutionSteps::NodeId)
                            .to(Nodes::Table, Nodes::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create performance indices
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

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(WorkflowExecutionSteps::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum WorkflowExecutionSteps {
    Table,
    Id,
    ExecutionId,
    NodeId,
    NodeName,
    Status,
    InputData,
    OutputData,
    ErrorMessage,
    StartedAt,
    CompletedAt,
    CreatedAt,
}