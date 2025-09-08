use sea_orm_migration::prelude::*;
use super::m20240908_000001_create_workflow_executions_table::WorkflowExecutions;
use super::m20240907_000002_create_nodes_table::Nodes;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(EmailQueue::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(EmailQueue::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(EmailQueue::ExecutionId).string())
                    .col(ColumnDef::new(EmailQueue::NodeId).string())
                    .col(ColumnDef::new(EmailQueue::SmtpConfig).string().not_null())
                    .col(ColumnDef::new(EmailQueue::Priority).string().not_null().default("normal"))
                    .col(ColumnDef::new(EmailQueue::EmailConfig).text().not_null())
                    .col(ColumnDef::new(EmailQueue::TemplateContext).text().not_null())
                    .col(ColumnDef::new(EmailQueue::Status).string().not_null().default("queued"))
                    .col(ColumnDef::new(EmailQueue::QueuedAt).big_integer().not_null())
                    .col(ColumnDef::new(EmailQueue::ScheduledAt).big_integer())
                    .col(ColumnDef::new(EmailQueue::ProcessedAt).big_integer())
                    .col(ColumnDef::new(EmailQueue::SentAt).big_integer())
                    .col(ColumnDef::new(EmailQueue::MaxWaitMinutes).integer().not_null().default(60))
                    .col(ColumnDef::new(EmailQueue::RetryCount).integer().not_null().default(0))
                    .col(ColumnDef::new(EmailQueue::MaxRetries).integer().not_null().default(3))
                    .col(ColumnDef::new(EmailQueue::ErrorMessage).text())
                    .col(ColumnDef::new(EmailQueue::CreatedAt).big_integer().not_null())
                    .col(ColumnDef::new(EmailQueue::UpdatedAt).big_integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-email_queue-execution_id")
                            .from(EmailQueue::Table, EmailQueue::ExecutionId)
                            .to(WorkflowExecutions::Table, WorkflowExecutions::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-email_queue-node_id")
                            .from(EmailQueue::Table, EmailQueue::NodeId)
                            .to(Nodes::Table, Nodes::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create performance indices for email queue operations
        manager
            .create_index(
                Index::create()
                    .name("idx_email_queue_status_priority")
                    .table(EmailQueue::Table)
                    .col((EmailQueue::Status, IndexOrder::Asc))
                    .col((EmailQueue::Priority, IndexOrder::Desc))
                    .col((EmailQueue::QueuedAt, IndexOrder::Asc))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_email_queue_scheduled_at")
                    .table(EmailQueue::Table)
                    .col(EmailQueue::ScheduledAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_email_queue_execution_id")
                    .table(EmailQueue::Table)
                    .col(EmailQueue::ExecutionId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_email_queue_queued_at")
                    .table(EmailQueue::Table)
                    .col(EmailQueue::QueuedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_email_queue_status")
                    .table(EmailQueue::Table)
                    .col(EmailQueue::Status)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_email_queue_priority")
                    .table(EmailQueue::Table)
                    .col(EmailQueue::Priority)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(EmailQueue::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum EmailQueue {
    Table,
    Id,
    ExecutionId,
    NodeId,
    SmtpConfig,
    Priority,
    EmailConfig,
    TemplateContext,
    Status,
    QueuedAt,
    ScheduledAt,
    ProcessedAt,
    SentAt,
    MaxWaitMinutes,
    RetryCount,
    MaxRetries,
    ErrorMessage,
    CreatedAt,
    UpdatedAt,
}