use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(WorkflowVersions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(WorkflowVersions::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(WorkflowVersions::WorkflowId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WorkflowVersions::VersionNumber)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WorkflowVersions::WorkflowSnapshot)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WorkflowVersions::CommitMessage)
                            .string_len(100)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WorkflowVersions::CommitDescription)
                            .string_len(1000)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(WorkflowVersions::ChangedBy)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WorkflowVersions::CreatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_workflow_versions_workflow_id")
                            .from(WorkflowVersions::Table, WorkflowVersions::WorkflowId)
                            .to(Workflows::Table, Workflows::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create unique constraint on workflow_id + version_number
        manager
            .create_index(
                Index::create()
                    .name("idx_workflow_versions_workflow_version")
                    .table(WorkflowVersions::Table)
                    .col(WorkflowVersions::WorkflowId)
                    .col(WorkflowVersions::VersionNumber)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Create index on workflow_id for fast lookup
        manager
            .create_index(
                Index::create()
                    .name("idx_workflow_versions_workflow_id")
                    .table(WorkflowVersions::Table)
                    .col(WorkflowVersions::WorkflowId)
                    .to_owned(),
            )
            .await?;

        // Create index on created_at for sorting
        manager
            .create_index(
                Index::create()
                    .name("idx_workflow_versions_created_at")
                    .table(WorkflowVersions::Table)
                    .col(WorkflowVersions::CreatedAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(WorkflowVersions::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum WorkflowVersions {
    Table,
    Id,
    WorkflowId,
    VersionNumber,
    WorkflowSnapshot,
    CommitMessage,
    CommitDescription,
    ChangedBy,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Workflows {
    Table,
    Id,
}
