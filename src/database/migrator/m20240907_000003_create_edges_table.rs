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
                    .table(Edges::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Edges::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Edges::WorkflowId).string().not_null())
                    .col(ColumnDef::new(Edges::FromNodeId).string().not_null())
                    .col(ColumnDef::new(Edges::ToNodeId).string().not_null())
                    .col(ColumnDef::new(Edges::ConditionResult).boolean())
                    .col(
                        ColumnDef::new(Edges::CreatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-edges-workflow_id")
                            .from(Edges::Table, Edges::WorkflowId)
                            .to(Workflows::Table, Workflows::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Edges::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Edges {
    Table,
    Id,
    WorkflowId,
    FromNodeId,
    ToNodeId,
    ConditionResult,
    CreatedAt,
}