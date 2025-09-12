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
                    .table(Nodes::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Nodes::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Nodes::WorkflowId).string().not_null())
                    .col(ColumnDef::new(Nodes::Name).string().not_null())
                    .col(ColumnDef::new(Nodes::NodeType).string().not_null())
                    .col(ColumnDef::new(Nodes::Config).text().not_null())
                    .col(ColumnDef::new(Nodes::PositionX).double().default(0.0))
                    .col(ColumnDef::new(Nodes::PositionY).double().default(0.0))
                    .col(
                        ColumnDef::new(Nodes::CreatedAt)
                            .timestamp()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(Nodes::InputMergeStrategy).string().null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-nodes-workflow_id")
                            .from(Nodes::Table, Nodes::WorkflowId)
                            .to(Workflows::Table, Workflows::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .index(
                        Index::create()
                            .name("idx-nodes-workflow-name")
                            .table(Nodes::Table)
                            .col(Nodes::WorkflowId)
                            .col(Nodes::Name)
                            .unique(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Nodes::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Nodes {
    Table,
    Id,
    WorkflowId,
    Name,
    NodeType,
    Config,
    PositionX,
    PositionY,
    CreatedAt,
    InputMergeStrategy,
}