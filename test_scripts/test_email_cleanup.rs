// Temporary test script to verify email cleanup logic
// This can be run with: cargo run --bin test_email_cleanup

use std::env;
use sea_orm::{Database, EntityTrait, ColumnTrait, QueryFilter};
use swisspipe::{email::EmailService, database::{email_queue, email_queue::Entity as EmailQueue}};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env::set_var("SMTP_HOST", "localhost");
    env::set_var("SMTP_FROM_EMAIL", "test@example.com");
    
    let db_url = "sqlite:./data/swisspipe.db?mode=rwc";
    let db = Database::connect(db_url).await?;
    
    println!("Checking email queue before cleanup...");
    let count_before = EmailQueue::find()
        .filter(email_queue::Column::Status.eq("sent"))
        .count(&db)
        .await?;
    println!("Found {} sent emails in queue", count_before);
    
    // Create email service and run cleanup
    let email_service = EmailService::new(std::sync::Arc::new(db.clone()))?;
    let cleaned = email_service.cleanup_expired_emails().await?;
    
    println!("Cleanup completed, processed {} items", cleaned);
    
    let count_after = EmailQueue::find()
        .filter(email_queue::Column::Status.eq("sent"))
        .count(&db)
        .await?;
    println!("Found {} sent emails in queue after cleanup", count_after);
    
    Ok(())
}