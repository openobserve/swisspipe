use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add source_handle_id column to edges table to support 3-handle routing
        manager
            .alter_table(
                Table::alter()
                    .table(Edges::Table)
                    .add_column_if_not_exists(
                        ColumnDef::new(Edges::SourceHandleId)
                            .string()
                            .null() // Nullable to maintain backward compatibility
                    )
                    .to_owned(),
            )
            .await?;

        // Add index for better query performance when filtering by handle
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_edges_source_handle_id")
                    .table(Edges::Table)
                    .col(Edges::SourceHandleId)
                    .to_owned(),
            )
            .await?;

        // Add composite index for efficient handle-specific routing queries
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_edges_from_node_handle")
                    .table(Edges::Table)
                    .col(Edges::FromNodeId)
                    .col(Edges::SourceHandleId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop the indexes first
        manager
            .drop_index(
                Index::drop()
                    .name("idx_edges_from_node_handle")
                    .table(Edges::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_edges_source_handle_id")
                    .table(Edges::Table)
                    .to_owned(),
            )
            .await?;

        // Drop the column
        manager
            .alter_table(
                Table::alter()
                    .table(Edges::Table)
                    .drop_column(Edges::SourceHandleId)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Edges {
    Table,
    FromNodeId,
    SourceHandleId,
}