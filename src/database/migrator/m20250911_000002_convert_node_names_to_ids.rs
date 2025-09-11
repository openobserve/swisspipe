use sea_orm_migration::prelude::*;
use super::m20240907_000001_create_workflows_table::Workflows;
use super::m20240907_000003_create_edges_table::Edges;
use super::m20240908_000001_create_workflow_executions_table::WorkflowExecutions;
use super::m20240908_000002_create_workflow_execution_steps_table::WorkflowExecutionSteps;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Step 1: Columns should already exist from previous migration attempt
        // Check if we need to add columns (for safety in case running on fresh DB)
        let db = manager.get_connection();
        
        // Check if columns exist, add them only if they don't
        let edges_has_from_node_id = db.execute_unprepared("SELECT from_node_id FROM edges LIMIT 1").await.is_ok();
        if !edges_has_from_node_id {
            manager
                .alter_table(
                    Table::alter()
                        .table(Edges::Table)
                        .add_column(ColumnDef::new(Alias::new("from_node_id")).string())
                        .to_owned(),
                )
                .await?;
        }
        
        let edges_has_to_node_id = db.execute_unprepared("SELECT to_node_id FROM edges LIMIT 1").await.is_ok();
        if !edges_has_to_node_id {
            manager
                .alter_table(
                    Table::alter()
                        .table(Edges::Table)
                        .add_column(ColumnDef::new(Alias::new("to_node_id")).string())
                        .to_owned(),
                )
                .await?;
        }
        
        let workflows_has_start_node_id = db.execute_unprepared("SELECT start_node_id FROM workflows LIMIT 1").await.is_ok();
        if !workflows_has_start_node_id {
            manager
                .alter_table(
                    Table::alter()
                        .table(Workflows::Table)
                        .add_column(ColumnDef::new(Alias::new("start_node_id")).string())
                        .to_owned(),
                )
                .await?;
        }
        
        let executions_has_current_node_id = db.execute_unprepared("SELECT current_node_id FROM workflow_executions LIMIT 1").await.is_ok();
        if !executions_has_current_node_id {
            manager
                .alter_table(
                    Table::alter()
                        .table(WorkflowExecutions::Table)
                        .add_column(ColumnDef::new(Alias::new("current_node_id")).string())
                        .to_owned(),
                )
                .await?;
        }
        
        let steps_has_node_id_ref = db.execute_unprepared("SELECT node_id_ref FROM workflow_execution_steps LIMIT 1").await.is_ok();
        if !steps_has_node_id_ref {
            manager
                .alter_table(
                    Table::alter()
                        .table(WorkflowExecutionSteps::Table)
                        .add_column(ColumnDef::new(Alias::new("node_id_ref")).string())
                        .to_owned(),
                )
                .await?;
        }

        // Step 2: Update the ID columns with values from the name-to-ID mapping
        // This requires raw SQL as we need to join tables and update based on node names
        
        // Update edges.from_node_id based on nodes.name -> nodes.id mapping
        db.execute_unprepared(
            "UPDATE edges SET from_node_id = (
                SELECT nodes.id FROM nodes 
                WHERE nodes.name = edges.from_node_name 
                AND nodes.workflow_id = edges.workflow_id
            )"
        ).await?;

        // Update edges.to_node_id based on nodes.name -> nodes.id mapping
        db.execute_unprepared(
            "UPDATE edges SET to_node_id = (
                SELECT nodes.id FROM nodes 
                WHERE nodes.name = edges.to_node_name 
                AND nodes.workflow_id = edges.workflow_id
            )"
        ).await?;

        // Update workflows.start_node_id based on nodes.name -> nodes.id mapping
        db.execute_unprepared(
            "UPDATE workflows SET start_node_id = (
                SELECT nodes.id FROM nodes 
                WHERE nodes.name = workflows.start_node_name 
                AND nodes.workflow_id = workflows.id
            )"
        ).await?;

        // Update workflow_executions.current_node_id based on nodes.name -> nodes.id mapping
        db.execute_unprepared(
            "UPDATE workflow_executions SET current_node_id = (
                SELECT nodes.id FROM nodes 
                WHERE nodes.name = workflow_executions.current_node_name 
                AND nodes.workflow_id = workflow_executions.workflow_id
            ) WHERE workflow_executions.current_node_name IS NOT NULL"
        ).await?;

        // Update workflow_execution_steps.node_id_ref based on existing node_id
        db.execute_unprepared(
            "UPDATE workflow_execution_steps SET node_id_ref = node_id"
        ).await?;

        // Step 3: SQLite doesn't support adding foreign key constraints to existing tables
        // Skip foreign key constraints - they would be nice to have but not essential
        // The application logic will handle referential integrity

        // Step 4: Create indices for the new ID columns
        manager
            .create_index(
                Index::create()
                    .name("idx_edges_from_node_id")
                    .table(Edges::Table)
                    .col(Alias::new("from_node_id"))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_edges_to_node_id")
                    .table(Edges::Table)
                    .col(Alias::new("to_node_id"))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_workflow_executions_current_node_id")
                    .table(WorkflowExecutions::Table)
                    .col(Alias::new("current_node_id"))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Foreign key constraints were never added due to SQLite limitations, so skip dropping them

        // Drop the indices
        manager
            .drop_index(
                Index::drop()
                    .name("idx_edges_from_node_id")
                    .table(Edges::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_edges_to_node_id")
                    .table(Edges::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_workflow_executions_current_node_id")
                    .table(WorkflowExecutions::Table)
                    .to_owned(),
            )
            .await?;

        // Drop the new columns
        manager
            .alter_table(
                Table::alter()
                    .table(Edges::Table)
                    .drop_column(Alias::new("from_node_id"))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Edges::Table)
                    .drop_column(Alias::new("to_node_id"))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Workflows::Table)
                    .drop_column(Alias::new("start_node_id"))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(WorkflowExecutions::Table)
                    .drop_column(Alias::new("current_node_id"))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(WorkflowExecutionSteps::Table)
                    .drop_column(Alias::new("node_id_ref"))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}