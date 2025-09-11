use sea_orm_migration::prelude::*;

mod m20240907_000001_create_workflows_table;
mod m20240907_000002_create_nodes_table;
mod m20240907_000003_create_edges_table;
mod m20240908_000001_create_workflow_executions_table;
mod m20240908_000002_create_workflow_execution_steps_table;
mod m20240908_000003_create_job_queue_table;
mod m20240908_000004_add_performance_indices;
mod m20241208_000001_create_email_queue_table;
mod m20241208_000002_create_email_audit_log_table;
mod m20241210_000001_create_scheduled_delays_table;
mod m20241210_000002_add_job_queue_payload;
mod m20241210_000003_add_retry_delay_seconds_to_email_queue;
mod m20241210_000004_create_node_input_sync_table;
mod m20250911_000001_add_input_merge_strategy_to_nodes;
mod m20250911_000002_convert_node_names_to_ids;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240907_000001_create_workflows_table::Migration),
            Box::new(m20240907_000002_create_nodes_table::Migration),
            Box::new(m20240907_000003_create_edges_table::Migration),
            Box::new(m20240908_000001_create_workflow_executions_table::Migration),
            Box::new(m20240908_000002_create_workflow_execution_steps_table::Migration),
            Box::new(m20240908_000003_create_job_queue_table::Migration),
            Box::new(m20240908_000004_add_performance_indices::Migration),
            Box::new(m20241208_000001_create_email_queue_table::Migration),
            Box::new(m20241208_000002_create_email_audit_log_table::Migration),
            Box::new(m20241210_000001_create_scheduled_delays_table::Migration),
            Box::new(m20241210_000002_add_job_queue_payload::Migration),
            Box::new(m20241210_000003_add_retry_delay_seconds_to_email_queue::Migration),
            Box::new(m20241210_000004_create_node_input_sync_table::Migration),
            Box::new(m20250911_000001_add_input_merge_strategy_to_nodes::Migration),
            Box::new(m20250911_000002_convert_node_names_to_ids::Migration),
        ]
    }
}