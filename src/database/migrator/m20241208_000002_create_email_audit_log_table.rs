use sea_orm_migration::prelude::*;
use super::m20240908_000001_create_workflow_executions_table::WorkflowExecutions;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(EmailAuditLog::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(EmailAuditLog::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(EmailAuditLog::ExecutionId).string().not_null())
                    .col(ColumnDef::new(EmailAuditLog::NodeId).string().not_null())
                    .col(ColumnDef::new(EmailAuditLog::SmtpConfig).string().not_null())
                    .col(ColumnDef::new(EmailAuditLog::FromEmail).string().not_null())
                    .col(ColumnDef::new(EmailAuditLog::ToEmails).text().not_null())
                    .col(ColumnDef::new(EmailAuditLog::CcEmails).text())
                    .col(ColumnDef::new(EmailAuditLog::BccEmails).text())
                    .col(ColumnDef::new(EmailAuditLog::Subject).text().not_null())
                    .col(ColumnDef::new(EmailAuditLog::EmailSizeBytes).integer().not_null())
                    .col(ColumnDef::new(EmailAuditLog::AttachmentCount).integer().not_null().default(0))
                    .col(ColumnDef::new(EmailAuditLog::Status).string().not_null())
                    .col(ColumnDef::new(EmailAuditLog::ErrorMessage).text())
                    .col(ColumnDef::new(EmailAuditLog::SmtpMessageId).string())
                    .col(ColumnDef::new(EmailAuditLog::SentAt).big_integer())
                    .col(ColumnDef::new(EmailAuditLog::CreatedAt).big_integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-email_audit_log-execution_id")
                            .from(EmailAuditLog::Table, EmailAuditLog::ExecutionId)
                            .to(WorkflowExecutions::Table, WorkflowExecutions::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create performance indices for email audit log queries
        manager
            .create_index(
                Index::create()
                    .name("idx_email_audit_execution_id")
                    .table(EmailAuditLog::Table)
                    .col(EmailAuditLog::ExecutionId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_email_audit_status")
                    .table(EmailAuditLog::Table)
                    .col(EmailAuditLog::Status)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_email_audit_sent_at")
                    .table(EmailAuditLog::Table)
                    .col(EmailAuditLog::SentAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_email_audit_created_at")
                    .table(EmailAuditLog::Table)
                    .col(EmailAuditLog::CreatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_email_audit_from_email")
                    .table(EmailAuditLog::Table)
                    .col(EmailAuditLog::FromEmail)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(EmailAuditLog::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum EmailAuditLog {
    Table,
    Id,
    ExecutionId,
    NodeId,
    SmtpConfig,
    FromEmail,
    ToEmails,
    CcEmails,
    BccEmails,
    Subject,
    EmailSizeBytes,
    AttachmentCount,
    Status,
    ErrorMessage,
    SmtpMessageId,
    SentAt,
    CreatedAt,
}