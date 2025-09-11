use sea_orm_migration::prelude::*;
use super::m20240907_000002_create_nodes_table::Nodes;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Nodes::Table)
                    .add_column(
                        ColumnDef::new(Nodes::InputMergeStrategy)
                            .string()
                            .null(), // Optional field - defaults to null (FirstWins behavior)
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Nodes::Table)
                    .drop_column(Nodes::InputMergeStrategy)
                    .to_owned(),
            )
            .await
    }
}