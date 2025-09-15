use swisspipe::database::{workflow_executions, entities, establish_connection};
use swisspipe::async_execution::CleanupService;
use sea_orm::{ActiveModelTrait, Set, EntityTrait, ColumnTrait, QueryFilter, PaginatorTrait};
use chrono::Utc;
use uuid::Uuid;
use std::sync::Arc;

#[tokio::test]
async fn test_cleanup_service_retention() {
    // Setup database
    let db = establish_connection("sqlite::memory:").await.unwrap();
    let db = Arc::new(db);

    let cleanup_service = CleanupService::new(db.clone(), 3, 1).unwrap(); // Keep 3, check every 1 minute

    // Create a test workflow first (required due to foreign key constraint)
    let workflow_id = Uuid::now_v7().to_string();
    let workflow = entities::ActiveModel {
        id: Set(workflow_id.clone()),
        name: Set("Test Workflow".to_string()),
        description: Set(Some("Test workflow for cleanup service".to_string())),
        start_node_id: Set(None),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    };
    workflow.insert(db.as_ref()).await.unwrap();

    // Create 10 test executions for the same workflow
    let mut execution_ids = Vec::new();
    for i in 0..10 {
        let created_at = Utc::now().timestamp_micros() + (i * 1000); // Different timestamps

        let execution = workflow_executions::ActiveModel {
            id: Set(Uuid::now_v7().to_string()),
            workflow_id: Set(workflow_id.clone()),
            status: Set("completed".to_string()),
            current_node_id: Set(None),
            input_data: Set(Some(format!("{{\"test\": {}}}", i))),
            output_data: Set(Some(format!("{{\"result\": {}}}", i))),
            error_message: Set(None),
            started_at: Set(Some(created_at)),
            completed_at: Set(Some(created_at + 1000)),
            created_at: Set(created_at),
            updated_at: Set(created_at),
        };

        let saved = execution.insert(db.as_ref()).await.unwrap();
        execution_ids.push(saved.id);
    }

    // Verify we have 10 executions
    let count_before = workflow_executions::Entity::find()
        .filter(workflow_executions::Column::WorkflowId.eq(&workflow_id))
        .count(db.as_ref())
        .await
        .unwrap();
    assert_eq!(count_before, 10);

    // Run cleanup
    let deleted_count = cleanup_service.cleanup_old_executions().await.unwrap();
    assert_eq!(deleted_count, 7); // Should delete 7 oldest executions

    // Verify we now have only 3 executions
    let count_after = workflow_executions::Entity::find()
        .filter(workflow_executions::Column::WorkflowId.eq(&workflow_id))
        .count(db.as_ref())
        .await
        .unwrap();
    assert_eq!(count_after, 3);

    // Verify the remaining executions are the most recent ones (should be the last 3)
    let remaining = workflow_executions::Entity::find()
        .filter(workflow_executions::Column::WorkflowId.eq(&workflow_id))
        .all(db.as_ref())
        .await
        .unwrap();

    // The remaining executions should be the ones with the highest created_at timestamps
    let mut remaining_created_times: Vec<i64> = remaining.iter().map(|e| e.created_at).collect();
    remaining_created_times.sort();

    // Should have the 3 most recent timestamps (indices 7, 8, 9 from our original data)
    assert_eq!(remaining_created_times.len(), 3);
}

#[tokio::test]
async fn test_cleanup_service_multiple_workflows() {
    // Setup database
    let db = establish_connection("sqlite::memory:").await.unwrap();
    let db = Arc::new(db);

    let cleanup_service = CleanupService::new(db.clone(), 2, 1).unwrap(); // Keep 2 per workflow

    // Create two different workflow IDs
    let workflow_id_1 = Uuid::now_v7().to_string();
    let workflow_id_2 = Uuid::now_v7().to_string();

    // Create workflows first (required due to foreign key constraint)
    let workflow_1 = entities::ActiveModel {
        id: Set(workflow_id_1.clone()),
        name: Set("Test Workflow 1".to_string()),
        description: Set(Some("Test workflow 1 for cleanup service".to_string())),
        start_node_id: Set(None),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    };
    workflow_1.insert(db.as_ref()).await.unwrap();

    let workflow_2 = entities::ActiveModel {
        id: Set(workflow_id_2.clone()),
        name: Set("Test Workflow 2".to_string()),
        description: Set(Some("Test workflow 2 for cleanup service".to_string())),
        start_node_id: Set(None),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    };
    workflow_2.insert(db.as_ref()).await.unwrap();

    // Create 5 executions for workflow 1
    for i in 0..5 {
        let created_at = Utc::now().timestamp_micros() + (i * 1000);

        let execution = workflow_executions::ActiveModel {
            id: Set(Uuid::now_v7().to_string()),
            workflow_id: Set(workflow_id_1.clone()),
            status: Set("completed".to_string()),
            current_node_id: Set(None),
            input_data: Set(Some(format!("{{\"test\": {}}}", i))),
            output_data: Set(None),
            error_message: Set(None),
            started_at: Set(Some(created_at)),
            completed_at: Set(Some(created_at + 1000)),
            created_at: Set(created_at),
            updated_at: Set(created_at),
        };

        execution.insert(db.as_ref()).await.unwrap();
    }

    // Create 3 executions for workflow 2
    for i in 0..3 {
        let created_at = Utc::now().timestamp_micros() + (i * 1000) + 10000; // Different timestamp range

        let execution = workflow_executions::ActiveModel {
            id: Set(Uuid::now_v7().to_string()),
            workflow_id: Set(workflow_id_2.clone()),
            status: Set("completed".to_string()),
            current_node_id: Set(None),
            input_data: Set(Some(format!("{{\"test\": {}}}", i))),
            output_data: Set(None),
            error_message: Set(None),
            started_at: Set(Some(created_at)),
            completed_at: Set(Some(created_at + 1000)),
            created_at: Set(created_at),
            updated_at: Set(created_at),
        };

        execution.insert(db.as_ref()).await.unwrap();
    }

    // Verify initial counts
    let count_wf1_before = workflow_executions::Entity::find()
        .filter(workflow_executions::Column::WorkflowId.eq(&workflow_id_1))
        .count(db.as_ref())
        .await
        .unwrap();
    assert_eq!(count_wf1_before, 5);

    let count_wf2_before = workflow_executions::Entity::find()
        .filter(workflow_executions::Column::WorkflowId.eq(&workflow_id_2))
        .count(db.as_ref())
        .await
        .unwrap();
    assert_eq!(count_wf2_before, 3);

    // Run cleanup
    let deleted_count = cleanup_service.cleanup_old_executions().await.unwrap();
    assert_eq!(deleted_count, 4); // Should delete 3 from workflow1 and 1 from workflow2

    // Verify final counts - each workflow should have at most 2 executions
    let count_wf1_after = workflow_executions::Entity::find()
        .filter(workflow_executions::Column::WorkflowId.eq(&workflow_id_1))
        .count(db.as_ref())
        .await
        .unwrap();
    assert_eq!(count_wf1_after, 2);

    let count_wf2_after = workflow_executions::Entity::find()
        .filter(workflow_executions::Column::WorkflowId.eq(&workflow_id_2))
        .count(db.as_ref())
        .await
        .unwrap();
    assert_eq!(count_wf2_after, 2);
}

#[tokio::test]
async fn test_cleanup_stats() {
    // Setup database
    let db = establish_connection("sqlite::memory:").await.unwrap();
    let db = Arc::new(db);

    let cleanup_service = CleanupService::new(db.clone(), 3, 1).unwrap();

    // Create test data with multiple workflows
    let workflow_id_1 = Uuid::now_v7().to_string();
    let workflow_id_2 = Uuid::now_v7().to_string();

    // Create workflows first (required due to foreign key constraint)
    let workflow_1 = entities::ActiveModel {
        id: Set(workflow_id_1.clone()),
        name: Set("Test Workflow 1".to_string()),
        description: Set(Some("Test workflow 1 for stats".to_string())),
        start_node_id: Set(None),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    };
    workflow_1.insert(db.as_ref()).await.unwrap();

    let workflow_2 = entities::ActiveModel {
        id: Set(workflow_id_2.clone()),
        name: Set("Test Workflow 2".to_string()),
        description: Set(Some("Test workflow 2 for stats".to_string())),
        start_node_id: Set(None),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    };
    workflow_2.insert(db.as_ref()).await.unwrap();

    // Workflow 1: 5 executions (exceeds retention of 3)
    for i in 0..5 {
        let created_at = Utc::now().timestamp_micros() + (i * 1000);

        let execution = workflow_executions::ActiveModel {
            id: Set(Uuid::now_v7().to_string()),
            workflow_id: Set(workflow_id_1.clone()),
            status: Set("completed".to_string()),
            created_at: Set(created_at),
            updated_at: Set(created_at),
            ..Default::default()
        };

        execution.insert(db.as_ref()).await.unwrap();
    }

    // Workflow 2: 2 executions (within retention limit)
    for i in 0..2 {
        let created_at = Utc::now().timestamp_micros() + (i * 1000) + 10000;

        let execution = workflow_executions::ActiveModel {
            id: Set(Uuid::now_v7().to_string()),
            workflow_id: Set(workflow_id_2.clone()),
            status: Set("completed".to_string()),
            created_at: Set(created_at),
            updated_at: Set(created_at),
            ..Default::default()
        };

        execution.insert(db.as_ref()).await.unwrap();
    }

    // Get cleanup stats
    let stats = cleanup_service.get_cleanup_stats().await.unwrap();

    assert_eq!(stats.total_executions, 7);
    assert_eq!(stats.retention_count, 3);
    assert_eq!(stats.workflow_counts.len(), 2);

    // Check workflow-specific stats
    let wf1_stats = stats.workflow_counts.iter()
        .find(|w| w.workflow_id == workflow_id_1)
        .unwrap();
    assert_eq!(wf1_stats.execution_count, 5);
    assert_eq!(wf1_stats.exceeds_retention, true);

    let wf2_stats = stats.workflow_counts.iter()
        .find(|w| w.workflow_id == workflow_id_2)
        .unwrap();
    assert_eq!(wf2_stats.execution_count, 2);
    assert_eq!(wf2_stats.exceeds_retention, false);
}

#[tokio::test]
async fn test_cleanup_no_executions_to_delete() {
    // Setup database
    let db = establish_connection("sqlite::memory:").await.unwrap();
    let db = Arc::new(db);

    let cleanup_service = CleanupService::new(db.clone(), 5, 1).unwrap(); // Keep 5 executions

    let workflow_id = Uuid::now_v7().to_string();

    // Create a test workflow first (required due to foreign key constraint)
    let workflow = entities::ActiveModel {
        id: Set(workflow_id.clone()),
        name: Set("Test Workflow".to_string()),
        description: Set(Some("Test workflow for no cleanup case".to_string())),
        start_node_id: Set(None),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    };
    workflow.insert(db.as_ref()).await.unwrap();

    // Create only 3 executions (less than retention limit)
    for i in 0..3 {
        let created_at = Utc::now().timestamp_micros() + (i * 1000);

        let execution = workflow_executions::ActiveModel {
            id: Set(Uuid::now_v7().to_string()),
            workflow_id: Set(workflow_id.clone()),
            status: Set("completed".to_string()),
            created_at: Set(created_at),
            updated_at: Set(created_at),
            ..Default::default()
        };

        execution.insert(db.as_ref()).await.unwrap();
    }

    // Run cleanup - should delete nothing
    let deleted_count = cleanup_service.cleanup_old_executions().await.unwrap();
    assert_eq!(deleted_count, 0);

    // Verify all executions are still there
    let count_after = workflow_executions::Entity::find()
        .filter(workflow_executions::Column::WorkflowId.eq(&workflow_id))
        .count(db.as_ref())
        .await
        .unwrap();
    assert_eq!(count_after, 3);
}

#[tokio::test]
async fn test_cleanup_service_validation() {
    // Setup database
    let db = establish_connection("sqlite::memory:").await.unwrap();
    let db = Arc::new(db);

    // Test retention_count = 0 (should fail)
    let result = CleanupService::new(db.clone(), 0, 1);
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("retention_count must be greater than 0"));
    }

    // Test retention_count too large (should fail)
    let result = CleanupService::new(db.clone(), 200_000, 1);
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("retention_count too large"));
    }

    // Test cleanup_interval_minutes = 0 (should fail)
    let result = CleanupService::new(db.clone(), 1000, 0);
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("cleanup_interval_minutes must be greater than 0"));
    }

    // Test cleanup_interval_minutes too large (should fail)
    let result = CleanupService::new(db.clone(), 1000, 2000);
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("cleanup_interval_minutes too large"));
    }

    // Test valid parameters (should succeed)
    let result = CleanupService::new(db.clone(), 1000, 60);
    assert!(result.is_ok());
}