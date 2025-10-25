use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add sources column to workflow_execution_steps
        manager
            .alter_table(
                Table::alter()
                    .table(WorkflowExecutionSteps::Table)
                    .add_column(
                        ColumnDef::new(WorkflowExecutionSteps::Sources)
                            .text()
                            .default("[]")
                            .not_null()
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop sources column
        manager
            .alter_table(
                Table::alter()
                    .table(WorkflowExecutionSteps::Table)
                    .drop_column(WorkflowExecutionSteps::Sources)
                    .to_owned(),
            )
            .await
    }
}

#[derive(Iden)]
enum WorkflowExecutionSteps {
    Table,
    Sources,
}
