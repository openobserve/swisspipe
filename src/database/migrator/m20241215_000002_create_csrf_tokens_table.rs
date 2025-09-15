use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(CsrfTokens::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CsrfTokens::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(CsrfTokens::Token).string().not_null().unique_key())
                    .col(ColumnDef::new(CsrfTokens::CreatedAt).big_integer().not_null())
                    .col(ColumnDef::new(CsrfTokens::ExpiresAt).big_integer().not_null())
                    .col(ColumnDef::new(CsrfTokens::Used).boolean().not_null().default(false))
                    .col(ColumnDef::new(CsrfTokens::IpAddress).string())
                    .col(ColumnDef::new(CsrfTokens::UserAgent).string())
                    .to_owned(),
            )
            .await?;

        // Create performance indices for CSRF token operations
        manager
            .create_index(
                Index::create()
                    .name("idx_csrf_tokens_token")
                    .table(CsrfTokens::Table)
                    .col(CsrfTokens::Token)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_csrf_tokens_expires_at")
                    .table(CsrfTokens::Table)
                    .col(CsrfTokens::ExpiresAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_csrf_tokens_used")
                    .table(CsrfTokens::Table)
                    .col(CsrfTokens::Used)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(CsrfTokens::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum CsrfTokens {
    Table,
    Id,
    Token,
    CreatedAt,
    ExpiresAt,
    Used,
    IpAddress,
    UserAgent,
}