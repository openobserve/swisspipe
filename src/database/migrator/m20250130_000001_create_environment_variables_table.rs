use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(EnvironmentVariables::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(EnvironmentVariables::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(EnvironmentVariables::Name)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(EnvironmentVariables::ValueType)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(EnvironmentVariables::Value)
                            .text()
                            .not_null(),
                    )
                    .col(ColumnDef::new(EnvironmentVariables::Description).text())
                    .col(
                        ColumnDef::new(EnvironmentVariables::CreatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(EnvironmentVariables::UpdatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index on name for fast lookups
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_environment_variables_name")
                    .table(EnvironmentVariables::Table)
                    .col(EnvironmentVariables::Name)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(EnvironmentVariables::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum EnvironmentVariables {
    Table,
    Id,
    Name,
    ValueType,
    Value,
    Description,
    CreatedAt,
    UpdatedAt,
}
