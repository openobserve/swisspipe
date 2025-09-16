use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Settings::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Settings::Key)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Settings::Value).text().not_null())
                    .col(ColumnDef::new(Settings::Description).text())
                    .col(ColumnDef::new(Settings::CreatedAt).big_integer().not_null())
                    .col(ColumnDef::new(Settings::UpdatedAt).big_integer().not_null())
                    .to_owned(),
            )
            .await?;

        // Insert default API base URL setting
        let now = chrono::Utc::now().timestamp();

        // Using values_panic here is acceptable for migrations since:
        // 1. The values are statically known and controlled
        // 2. Migration failures should halt the process anyway
        // 3. The data types are guaranteed to match the schema we just created
        manager
            .exec_stmt(
                Query::insert()
                    .into_table(Settings::Table)
                    .columns([
                        Settings::Key,
                        Settings::Value,
                        Settings::Description,
                        Settings::CreatedAt,
                        Settings::UpdatedAt,
                    ])
                    .values_panic([
                        "api_base_url".into(),
                        "".into(), // Empty by default, will be set by user
                        "Base URL for API endpoints that users can copy for external use. If empty, uses the current browser origin.".into(),
                        now.into(),
                        now.into(),
                    ])
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Settings::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Settings {
    Table,
    Key,
    Value,
    Description,
    CreatedAt,
    UpdatedAt,
}