use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Workflows::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Workflows::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Workflows::Name).string().not_null())
                    .col(ColumnDef::new(Workflows::Description).string())
                    .col(ColumnDef::new(Workflows::StartNodeName).string().not_null())
                    .col(
                        ColumnDef::new(Workflows::CreatedAt)
                            .timestamp()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Workflows::UpdatedAt)
                            .timestamp()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Workflows::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Workflows {
    Table,
    Id,
    Name,
    Description,
    StartNodeName,
    CreatedAt,
    UpdatedAt,
}