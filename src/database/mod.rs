pub mod entities;
pub mod nodes;
pub mod edges;
pub mod migrator;
pub mod workflow_executions;
pub mod workflow_execution_steps;
pub mod job_queue;
pub mod email_queue;
pub mod email_audit_log;

use sea_orm::{Database, DatabaseConnection, DbErr};
use sea_orm_migration::MigratorTrait;
use migrator::Migrator;

pub async fn establish_connection(database_url: &str) -> Result<DatabaseConnection, DbErr> {
    let db = Database::connect(database_url).await?;
    
    // Run pending migrations
    Migrator::up(&db, None).await?;
    
    Ok(db)
}

