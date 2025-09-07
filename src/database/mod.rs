pub mod entities;
pub mod nodes;
pub mod edges;

use sea_orm::{Database, DatabaseConnection, DbErr, ConnectionTrait};

pub async fn establish_connection(database_url: &str) -> Result<DatabaseConnection, DbErr> {
    Database::connect(database_url).await
}

pub async fn create_tables(db: &DatabaseConnection) -> Result<(), DbErr> {
    use sea_orm::{Schema, Statement};
    
    let backend = db.get_database_backend();
    let _schema = Schema::new(backend);
    
    // Create tables manually with SQL for now
    let sql_statements = vec![
        r#"CREATE TABLE IF NOT EXISTS workflows (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            start_node_name TEXT NOT NULL,
            created_at TEXT DEFAULT (datetime('now')),
            updated_at TEXT DEFAULT (datetime('now'))
        )"#,
        r#"CREATE TABLE IF NOT EXISTS nodes (
            id TEXT PRIMARY KEY,
            workflow_id TEXT NOT NULL,
            name TEXT NOT NULL,
            node_type TEXT NOT NULL,
            config TEXT NOT NULL,
            position_x REAL DEFAULT 0,
            position_y REAL DEFAULT 0,
            created_at TEXT DEFAULT (datetime('now')),
            FOREIGN KEY (workflow_id) REFERENCES workflows(id) ON DELETE CASCADE,
            UNIQUE(workflow_id, name)
        )"#,
        r#"CREATE TABLE IF NOT EXISTS edges (
            id TEXT PRIMARY KEY,
            workflow_id TEXT NOT NULL,
            from_node_name TEXT NOT NULL,
            to_node_name TEXT NOT NULL,
            condition_result INTEGER,
            created_at TEXT DEFAULT (datetime('now')),
            FOREIGN KEY (workflow_id) REFERENCES workflows(id) ON DELETE CASCADE
        )"#,
    ];
    
    for sql in sql_statements {
        let statement = Statement::from_string(backend, sql);
        db.execute(statement).await?;
    }
    
    // Add position columns to existing nodes table (will fail silently if already exist)
    let alter_statements = vec![
        "ALTER TABLE nodes ADD COLUMN position_x REAL DEFAULT 0",
        "ALTER TABLE nodes ADD COLUMN position_y REAL DEFAULT 0",
    ];
    
    for sql in alter_statements {
        let statement = Statement::from_string(backend, sql);
        let _ = db.execute(statement).await; // Ignore errors for existing columns
    }
    
    Ok(())
}