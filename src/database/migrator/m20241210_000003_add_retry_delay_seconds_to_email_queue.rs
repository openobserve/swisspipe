use sea_orm_migration::prelude::*;
use super::m20241208_000001_create_email_queue_table::EmailQueue;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(EmailQueue::Table)
                    .add_column(
                        ColumnDef::new(EmailQueue::RetryDelaySeconds)
                            .integer()
                            .not_null()
                            .default(30)
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(EmailQueue::Table)
                    .drop_column(EmailQueue::RetryDelaySeconds)
                    .to_owned(),
            )
            .await
    }
}