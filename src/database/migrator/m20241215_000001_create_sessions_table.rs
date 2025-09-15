use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Sessions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Sessions::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Sessions::UserId).string().not_null())
                    .col(ColumnDef::new(Sessions::Email).string().not_null())
                    .col(ColumnDef::new(Sessions::Name).string().not_null())
                    .col(ColumnDef::new(Sessions::GivenName).string())
                    .col(ColumnDef::new(Sessions::FamilyName).string())
                    .col(ColumnDef::new(Sessions::Picture).string())
                    .col(ColumnDef::new(Sessions::Locale).string())
                    .col(ColumnDef::new(Sessions::HostedDomain).string())
                    .col(ColumnDef::new(Sessions::VerifiedEmail).boolean().not_null().default(false))
                    .col(ColumnDef::new(Sessions::CreatedAt).big_integer().not_null())
                    .col(ColumnDef::new(Sessions::LastAccessedAt).big_integer().not_null())
                    .col(ColumnDef::new(Sessions::ExpiresAt).big_integer().not_null())
                    .col(ColumnDef::new(Sessions::IpAddress).string())
                    .col(ColumnDef::new(Sessions::UserAgent).string())
                    .to_owned(),
            )
            .await?;

        // Create performance indices for session operations
        manager
            .create_index(
                Index::create()
                    .name("idx_sessions_user_id")
                    .table(Sessions::Table)
                    .col(Sessions::UserId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_sessions_email")
                    .table(Sessions::Table)
                    .col(Sessions::Email)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_sessions_expires_at")
                    .table(Sessions::Table)
                    .col(Sessions::ExpiresAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_sessions_last_accessed_at")
                    .table(Sessions::Table)
                    .col(Sessions::LastAccessedAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Sessions::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Sessions {
    Table,
    Id,
    UserId,
    Email,
    Name,
    GivenName,
    FamilyName,
    Picture,
    Locale,
    HostedDomain,
    VerifiedEmail,
    CreatedAt,
    LastAccessedAt,
    ExpiresAt,
    IpAddress,
    UserAgent,
}