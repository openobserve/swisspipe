use sea_orm_migration::prelude::*;
use sea_orm::ConnectionTrait;
use super::m20240908_000001_create_workflow_executions_table::WorkflowExecutions;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(NodeInputSync::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(NodeInputSync::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(NodeInputSync::ExecutionId).string().not_null())
                    .col(ColumnDef::new(NodeInputSync::NodeId).string().not_null())
                    .col(ColumnDef::new(NodeInputSync::ExpectedInputCount).integer().not_null())
                    .col(ColumnDef::new(NodeInputSync::ReceivedInputs).text().not_null().default("[]"))
                    .col(ColumnDef::new(NodeInputSync::TimeoutAt).big_integer())
                    .col(ColumnDef::new(NodeInputSync::Status).string().not_null().default("waiting"))
                    .col(
                        ColumnDef::new(NodeInputSync::CreatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(NodeInputSync::UpdatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-node_input_sync-execution_id")
                            .from(NodeInputSync::Table, NodeInputSync::ExecutionId)
                            .to(WorkflowExecutions::Table, WorkflowExecutions::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .index(
                        Index::create()
                            .name("idx-node_input_sync-execution-node")
                            .col(NodeInputSync::ExecutionId)
                            .col(NodeInputSync::NodeId)
                            .unique(),
                    )
                    .to_owned(),
            )
            .await?;

        // Create the status index separately
        manager
            .get_connection()
            .execute_unprepared("CREATE INDEX IF NOT EXISTS idx_node_input_sync_status ON node_input_sync (status)")
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(NodeInputSync::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum NodeInputSync {
    Table,
    Id,
    ExecutionId,
    NodeId,
    ExpectedInputCount,
    ReceivedInputs,
    TimeoutAt,
    Status,
    CreatedAt,
    UpdatedAt,
}