use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add configuration fields to http_loop_states table for proper resumption
        // SQLite requires each column addition to be a separate ALTER TABLE statement

        manager
            .alter_table(
                Table::alter()
                    .table(HttpLoopStates::Table)
                    .add_column(
                        ColumnDef::new(HttpLoopStates::Url)
                            .string()
                            .not_null()
                            .default(""),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(HttpLoopStates::Table)
                    .add_column(
                        ColumnDef::new(HttpLoopStates::Method)
                            .string()
                            .not_null()
                            .default("GET"),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(HttpLoopStates::Table)
                    .add_column(
                        ColumnDef::new(HttpLoopStates::TimeoutSeconds)
                            .big_integer()
                            .not_null()
                            .default(30),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(HttpLoopStates::Table)
                    .add_column(
                        ColumnDef::new(HttpLoopStates::Headers)
                            .text()
                            .not_null()
                            .default("{}"),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(HttpLoopStates::Table)
                    .add_column(
                        ColumnDef::new(HttpLoopStates::LoopConfiguration)
                            .text()
                            .not_null()
                            .default("{}"),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(HttpLoopStates::Table)
                    .add_column(
                        ColumnDef::new(HttpLoopStates::InitialEvent)
                            .text()
                            .not_null()
                            .default("{}"),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Remove the configuration fields
        // SQLite requires each column drop to be a separate ALTER TABLE statement

        manager
            .alter_table(
                Table::alter()
                    .table(HttpLoopStates::Table)
                    .drop_column(HttpLoopStates::InitialEvent)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(HttpLoopStates::Table)
                    .drop_column(HttpLoopStates::LoopConfiguration)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(HttpLoopStates::Table)
                    .drop_column(HttpLoopStates::Headers)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(HttpLoopStates::Table)
                    .drop_column(HttpLoopStates::TimeoutSeconds)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(HttpLoopStates::Table)
                    .drop_column(HttpLoopStates::Method)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(HttpLoopStates::Table)
                    .drop_column(HttpLoopStates::Url)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum HttpLoopStates {
    Table,
    Url,
    Method,
    TimeoutSeconds,
    Headers,
    LoopConfiguration,
    InitialEvent,
}