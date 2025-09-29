use swisspipe::database::establish_connection;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

use swisspipe::async_execution::HttpLoopScheduler;
use swisspipe::config::HttpLoopConfig;
use swisspipe::workflow::models::{
    BackoffStrategy, HttpMethod, LoopConfig, WorkflowEvent,
};

/// Generate a unique namespace prefix for loop IDs to prevent conflicts between parallel tests
fn generate_test_namespace(test_name: &str) -> String {
    use std::thread;
    use std::time::{SystemTime, UNIX_EPOCH};

    let thread_id = format!("{:?}", thread::current().id());
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    format!("test_{}_{}_{}_{}", test_name, thread_id, timestamp, Uuid::new_v4())
}

/// Cleanup scheduler state by clearing all loop tasks
async fn cleanup_scheduler_state(scheduler: &HttpLoopScheduler) {
    if let Err(e) = scheduler.clear_all_loop_tasks().await {
        eprintln!("Warning: Failed to clear scheduler state: {}", e);
    }
}

/// Cleanup function to remove old test database files
fn cleanup_old_test_files() {
    use std::fs;

    let prefixes = [
        "test_blocking_behavior_",
        "test_data_preservation_",
        "test_termination_",
        "test_concurrent_",
    ];

    // Read /tmp directory and remove matching files
    if let Ok(entries) = fs::read_dir("/tmp") {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(filename) = path.file_name() {
                if let Some(filename_str) = filename.to_str() {
                    // Check if this file matches any of our test database patterns
                    for prefix in &prefixes {
                        if filename_str.starts_with(prefix) && filename_str.ends_with(".db") {
                            if let Err(e) = fs::remove_file(&path) {
                                eprintln!("Warning: Could not remove old test file {:?}: {}", path, e);
                            } else {
                                println!("Cleaned up old test file: {:?}", path);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Simplified integration tests for HTTP loop blocking behavior
///
/// These tests verify the core blocking functionality without complex workflow setup

#[tokio::test]
async fn test_http_loop_scheduler_blocking_behavior() {
    // Clean up any old test files before starting
    cleanup_old_test_files();

    // Generate unique namespace for this test
    let test_namespace = generate_test_namespace("blocking_behavior");

    // Setup test database with migrations
    let test_db_path = format!("/tmp/test_blocking_behavior_{}.db", Uuid::new_v4());
    let database_url = format!("sqlite:{}?mode=rwc", test_db_path);
    let db = establish_connection(&database_url)
        .await
        .expect("Failed to connect to test database");

    let db = Arc::new(db);

    // Create HTTP loop scheduler
    let http_loop_config = HttpLoopConfig {
        scheduler_interval_seconds: 1,
        max_history_entries: 1000,
        max_response_size_bytes: 1024 * 1024,
        max_iteration_timeout_seconds: 30,
        default_timeout_seconds: 10,
    };

    let scheduler = HttpLoopScheduler::new(db.clone(), http_loop_config)
        .await
        .expect("Failed to create HTTP loop scheduler");

    // Clear any existing scheduler state before starting
    cleanup_scheduler_state(&scheduler).await;

    // Start scheduler service
    scheduler.start_scheduler_service()
        .await
        .expect("Failed to start scheduler service");

    // Test HTTP loop configuration
    let initial_event = WorkflowEvent {
        data: json!({
            "test_id": "blocking_test",
            "counter": 0
        }),
        metadata: HashMap::new(),
        headers: HashMap::new(),
        condition_results: HashMap::new(),
        hil_task: None,
    };

    let loop_config = swisspipe::async_execution::http_loop_scheduler::HttpLoopConfig {
        loop_id: test_namespace.clone(),
        execution_step_id: format!("test_{}", Uuid::new_v4()),
        url: "https://httpbin.org/get".to_string(), // Simple GET request
        method: HttpMethod::Get,
        timeout_seconds: 10,
        headers: HashMap::new(),
        loop_config: LoopConfig {
            max_iterations: Some(2),
            interval_seconds: 1, // Short interval for testing
            backoff_strategy: BackoffStrategy::Fixed(1),
            termination_condition: None,
        },
        initial_event: initial_event.clone(),
    };

    // Test scheduling and blocking behavior
    let start_time = Instant::now();

    // Schedule the loop
    let loop_id = scheduler.schedule_http_loop(loop_config).await
        .expect("Should schedule loop successfully");

    tracing::info!("HTTP loop scheduled with ID: {}", loop_id);

    // Test waiting for completion (blocking behavior)
    let result = scheduler.wait_for_loop_completion(&loop_id).await;

    let execution_time = start_time.elapsed();

    // Verify the loop completed
    assert!(result.is_ok(), "Loop should complete successfully: {result:?}");

    // Verify blocking behavior - should take at least 2+ seconds for 2 iterations
    assert!(
        execution_time >= Duration::from_secs(2),
        "HTTP loop should block for at least 2 seconds, but took {execution_time:?}"
    );

    let final_event = result.unwrap();

    // Debug: Print final event data
    println!("DEBUG: Final event data: {}", serde_json::to_string_pretty(&final_event.data).unwrap());
    println!("DEBUG: Final event metadata: {:?}", final_event.metadata);

    // Verify HTTP response data structure (this is the correct behavior)
    assert!(final_event.data.get("url").is_some(), "HTTP response should contain URL field");
    assert!(final_event.data.get("args").is_some(), "HTTP response should contain args field with query parameters");

    // Original data should appear as query parameters in the HTTP response
    let args = final_event.data.get("args").unwrap();
    assert!(args.get("test_id").is_some(), "Original test_id should appear in query parameters");

    tracing::info!("HTTP loop completed successfully after {:?}", execution_time);
}

#[tokio::test]
async fn test_http_loop_data_preservation() {
    // Clean up any old test files before starting
    cleanup_old_test_files();

    // Generate unique namespace for this test
    let test_namespace = generate_test_namespace("data_preservation");

    // Setup test database
    let test_db_path = format!("/tmp/test_data_preservation_{}.db", Uuid::new_v4());
    let database_url = format!("sqlite:{}?mode=rwc", test_db_path);
    let db = establish_connection(&database_url)
        .await
        .expect("Failed to connect to test database");
    let db = Arc::new(db);

    // Create HTTP loop scheduler
    let http_loop_config = HttpLoopConfig {
        scheduler_interval_seconds: 1,
        max_history_entries: 1000,
        max_response_size_bytes: 1024 * 1024,
        max_iteration_timeout_seconds: 30,
        default_timeout_seconds: 10,
    };

    let scheduler = HttpLoopScheduler::new(db.clone(), http_loop_config)
        .await
        .expect("Failed to create HTTP loop scheduler");

    // Clear any existing scheduler state before starting
    cleanup_scheduler_state(&scheduler).await;

    scheduler.start_scheduler_service()
        .await
        .expect("Failed to start scheduler service");

    // Test data preservation through loop iterations
    let initial_event = WorkflowEvent {
        data: json!({
            "customer_id": "12345",
            "status": "pending",
            "attempt_count": 0
        }),
        metadata: HashMap::new(),
        headers: HashMap::new(),
        condition_results: HashMap::new(),
        hil_task: None,
    };

    let loop_config = swisspipe::async_execution::http_loop_scheduler::HttpLoopConfig {
        loop_id: test_namespace.clone(),
        execution_step_id: format!("data_test_{}", Uuid::new_v4()),
        url: "https://httpbin.org/post".to_string(),
        method: HttpMethod::Post,
        timeout_seconds: 10,
        headers: HashMap::new(),
        loop_config: LoopConfig {
            max_iterations: Some(1), // Just one iteration for data test
            interval_seconds: 1,
            backoff_strategy: BackoffStrategy::Fixed(1),
            termination_condition: None,
        },
        initial_event: initial_event.clone(),
    };

    // Schedule and wait for completion
    let loop_id = scheduler.schedule_http_loop(loop_config).await
        .expect("Should schedule loop successfully");

    let final_event = scheduler.wait_for_loop_completion(&loop_id).await
        .expect("Loop should complete successfully");

    // Verify HTTP response contains original data as POST request body in JSON field
    assert!(final_event.data.get("json").is_some(), "HTTP POST response should contain JSON field");

    let json_data = final_event.data.get("json").unwrap();
    assert_eq!(
        json_data.get("customer_id").unwrap().as_str().unwrap(),
        "12345",
        "Customer ID should appear in POST JSON data"
    );

    assert_eq!(
        json_data.get("status").unwrap().as_str().unwrap(),
        "pending",
        "Status should appear in POST JSON data"
    );
}

#[tokio::test]
async fn test_http_loop_termination_condition() {
    // Clean up any old test files before starting
    cleanup_old_test_files();

    // Generate unique namespace for this test
    let test_namespace = generate_test_namespace("termination_condition");

    // Setup test database
    let test_db_path = format!("/tmp/test_termination_{}.db", Uuid::new_v4());
    let database_url = format!("sqlite:{}?mode=rwc", test_db_path);
    let db = establish_connection(&database_url)
        .await
        .expect("Failed to connect to test database");
    let db = Arc::new(db);

    // Create HTTP loop scheduler
    let http_loop_config = HttpLoopConfig {
        scheduler_interval_seconds: 1,
        max_history_entries: 1000,
        max_response_size_bytes: 1024 * 1024,
        max_iteration_timeout_seconds: 30,
        default_timeout_seconds: 10,
    };

    let scheduler = HttpLoopScheduler::new(db.clone(), http_loop_config)
        .await
        .expect("Failed to create HTTP loop scheduler");

    // Clear any existing scheduler state before starting
    cleanup_scheduler_state(&scheduler).await;

    scheduler.start_scheduler_service()
        .await
        .expect("Failed to start scheduler service");

    // Test termination condition behavior
    let initial_event = WorkflowEvent {
        data: json!({
            "target_reached": false,
            "iteration_count": 0
        }),
        metadata: HashMap::new(),
        headers: HashMap::new(),
        condition_results: HashMap::new(),
        hil_task: None,
    };

    let loop_config = swisspipe::async_execution::http_loop_scheduler::HttpLoopConfig {
        loop_id: test_namespace.clone(),
        execution_step_id: format!("termination_test_{}", Uuid::new_v4()),
        url: "https://httpbin.org/get".to_string(),
        method: HttpMethod::Get,
        timeout_seconds: 10,
        headers: HashMap::new(),
        loop_config: LoopConfig {
            max_iterations: Some(10), // High max, should terminate early
            interval_seconds: 1,
            backoff_strategy: BackoffStrategy::Fixed(1),
            termination_condition: Some(swisspipe::workflow::models::TerminationCondition {
                script: r#"
                    function condition(event) {
                        // Simulate reaching target after 2 iterations
                        if (event.metadata.loop_iteration) {
                            const iteration = parseInt(event.metadata.loop_iteration);
                            return iteration >= 2;
                        }
                        return false;
                    }
                "#.to_string(),
                action: swisspipe::workflow::models::TerminationAction::Success,
            }),
        },
        initial_event: initial_event.clone(),
    };

    let start_time = Instant::now();

    // Schedule and wait for completion
    let loop_id = scheduler.schedule_http_loop(loop_config).await
        .expect("Should schedule loop successfully");

    let final_event = scheduler.wait_for_loop_completion(&loop_id).await
        .expect("Loop should complete successfully");

    let execution_time = start_time.elapsed();

    // Should complete early due to termination condition (around 2-3 seconds instead of 10)
    assert!(
        execution_time < Duration::from_secs(5),
        "Loop should terminate early due to condition, but took {execution_time:?}"
    );

    // Verify final event has HTTP response structure (correct behavior)
    assert!(final_event.data.get("url").is_some(), "HTTP response should contain URL field");
    assert!(final_event.data.get("args").is_some(), "HTTP response should contain args field");

    // Original data should appear as query parameters
    let args = final_event.data.get("args").unwrap();
    assert!(args.get("target_reached").is_some(), "Original data should appear in query parameters");
}

#[tokio::test]
async fn test_concurrent_http_loops() {
    // Clean up any old test files before starting
    cleanup_old_test_files();

    // Generate unique namespace for this test
    let test_namespace = generate_test_namespace("concurrent_loops");

    // Setup test database
    let test_db_path = format!("/tmp/test_concurrent_{}.db", Uuid::new_v4());
    let database_url = format!("sqlite:{}?mode=rwc", test_db_path);
    let db = establish_connection(&database_url)
        .await
        .expect("Failed to connect to test database");
    let db = Arc::new(db);

    // Create HTTP loop scheduler
    let http_loop_config = HttpLoopConfig {
        scheduler_interval_seconds: 1,
        max_history_entries: 1000,
        max_response_size_bytes: 1024 * 1024,
        max_iteration_timeout_seconds: 30,
        default_timeout_seconds: 10,
    };

    let scheduler = Arc::new(
        HttpLoopScheduler::new(db.clone(), http_loop_config)
            .await
            .expect("Failed to create HTTP loop scheduler")
    );

    // Clear any existing scheduler state before starting
    cleanup_scheduler_state(&scheduler).await;

    scheduler.start_scheduler_service()
        .await
        .expect("Failed to start scheduler service");

    // Create two concurrent loops
    let event1 = WorkflowEvent {
        data: json!({"loop_id": 1, "data": "loop_one"}),
        metadata: HashMap::new(),
        headers: HashMap::new(),
        condition_results: HashMap::new(),
        hil_task: None,
    };

    let event2 = WorkflowEvent {
        data: json!({"loop_id": 2, "data": "loop_two"}),
        metadata: HashMap::new(),
        headers: HashMap::new(),
        condition_results: HashMap::new(),
        hil_task: None,
    };

    let config1 = swisspipe::async_execution::http_loop_scheduler::HttpLoopConfig {
        loop_id: format!("{}_1", test_namespace),
        execution_step_id: format!("concurrent_1_{}", Uuid::new_v4()),
        url: "https://httpbin.org/delay/1".to_string(),
        method: HttpMethod::Get,
        timeout_seconds: 10,
        headers: HashMap::new(),
        loop_config: LoopConfig {
            max_iterations: Some(1),
            interval_seconds: 1,
            backoff_strategy: BackoffStrategy::Fixed(1),
            termination_condition: None,
        },
        initial_event: event1,
    };

    let config2 = swisspipe::async_execution::http_loop_scheduler::HttpLoopConfig {
        loop_id: format!("{}_2", test_namespace),
        execution_step_id: format!("concurrent_2_{}", Uuid::new_v4()),
        url: "https://httpbin.org/delay/1".to_string(),
        method: HttpMethod::Get,
        timeout_seconds: 10,
        headers: HashMap::new(),
        loop_config: LoopConfig {
            max_iterations: Some(2),
            interval_seconds: 1,
            backoff_strategy: BackoffStrategy::Fixed(1),
            termination_condition: None,
        },
        initial_event: event2,
    };

    let start_time = Instant::now();

    // Schedule both loops concurrently
    let loop_id1 = scheduler.schedule_http_loop(config1).await
        .expect("Should schedule first loop");
    let loop_id2 = scheduler.schedule_http_loop(config2).await
        .expect("Should schedule second loop");

    // Wait for both to complete concurrently
    let (result1, result2) = tokio::join!(
        scheduler.wait_for_loop_completion(&loop_id1),
        scheduler.wait_for_loop_completion(&loop_id2)
    );

    let execution_time = start_time.elapsed();

    // Both should succeed
    assert!(result1.is_ok(), "First concurrent loop should succeed");
    assert!(result2.is_ok(), "Second concurrent loop should succeed");

    let event1 = result1.unwrap();
    let event2 = result2.unwrap();

    // Verify each got their correct data back in query parameters
    let args1 = event1.data.get("args").unwrap();
    assert_eq!(
        args1.get("loop_id").unwrap().as_str().unwrap(),
        "1",
        "First loop data should appear in query parameters"
    );

    let args2 = event2.data.get("args").unwrap();
    assert_eq!(
        args2.get("loop_id").unwrap().as_str().unwrap(),
        "2",
        "Second loop data should appear in query parameters"
    );

    // Should take at least as long as the longer loop (2+ seconds)
    assert!(
        execution_time >= Duration::from_secs(2),
        "Concurrent loops should still take proper time: {execution_time:?}"
    );
}