use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let now = chrono::Utc::now().timestamp_micros();

        // Insert default from email setting
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
                        "default_from_email".into(),
                        "".into(), // Empty by default, will be set by user
                        "Default from email address for Email nodes when not specified".into(),
                        now.into(),
                        now.into(),
                    ])
                    .to_owned(),
            )
            .await?;

        // Insert default from name setting
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
                        "default_from_name".into(),
                        "".into(), // Empty by default, will be set by user
                        "Default from name for Email nodes when not specified".into(),
                        now.into(),
                        now.into(),
                    ])
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Remove the default email settings
        manager
            .exec_stmt(
                Query::delete()
                    .from_table(Settings::Table)
                    .and_where(Expr::col(Settings::Key).eq("default_from_email"))
                    .to_owned(),
            )
            .await?;

        manager
            .exec_stmt(
                Query::delete()
                    .from_table(Settings::Table)
                    .and_where(Expr::col(Settings::Key).eq("default_from_name"))
                    .to_owned(),
            )
            .await?;

        Ok(())
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