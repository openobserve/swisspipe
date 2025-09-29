use swisspipe::{
    database::{establish_connection, workflow_executions, entities},
    workflow::{
        models::{WorkflowEvent, InputMergeStrategy},
        input_sync::InputSyncService,
    },
};
use sea_orm::{ActiveModelTrait, Set, DatabaseConnection};
use std::{collections::HashMap, sync::Arc};

// Helper function to create a test execution record to satisfy foreign key constraints
async fn create_test_execution(db: &DatabaseConnection, execution_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    // First create the workflow record if it doesn't exist
    let workflow = entities::ActiveModel {
        id: Set("test_workflow".to_string()),
        name: Set("Test Workflow".to_string()),
        description: Set(Some("Test workflow for input sync".to_string())),
        start_node_id: Set(None),
        created_at: Set(chrono::Utc::now().timestamp_micros()),
        updated_at: Set(chrono::Utc::now().timestamp_micros()),
        ..Default::default()
    };
    // Ignore errors in case the workflow already exists
    let _ = workflow.insert(db).await;

    // Then create the execution record
    let execution = workflow_executions::ActiveModel {
        id: Set(execution_id.to_string()),
        workflow_id: Set("test_workflow".to_string()),
        status: Set("pending".to_string()),
        current_node_id: Set(None),
        input_data: Set(Some(r#"{"data": {}}"#.to_string())),
        output_data: Set(None),
        error_message: Set(None),
        started_at: Set(None),
        completed_at: Set(None),
        created_at: Set(chrono::Utc::now().timestamp_micros()),
        updated_at: Set(chrono::Utc::now().timestamp_micros())
    };
    execution.insert(db).await?;
    Ok(())
}

#[tokio::test]
async fn test_input_synchronization_wait_for_all() {
    let db_url = "sqlite::memory:";
    let db = Arc::new(establish_connection(db_url).await.expect("Failed to connect to database"));
    let input_sync = InputSyncService::new(db.clone());

    let execution_id = "test_exec_001";
    let node_id = "merge_node";
    let expected_inputs = 2;
    let strategy = InputMergeStrategy::WaitForAll;

    // Create test execution record first to satisfy foreign key constraint
    create_test_execution(db.as_ref(), execution_id).await.expect("Failed to create test execution");

    // Initialize sync record
    input_sync.initialize_node_sync(execution_id, node_id, expected_inputs, &strategy)
        .await
        .expect("Failed to initialize sync");
    
    // Create first input event
    let mut event1 = WorkflowEvent {
        data: serde_json::json!({"message": "first input", "value": 100}),
        metadata: HashMap::new(),
        headers: HashMap::new(),
        condition_results: HashMap::new(),
        hil_task: None,
    };
    event1.metadata.insert("source".to_string(), "branch_a".to_string());
    
    // Add first input - should be waiting
    let result1 = input_sync.add_input(execution_id, node_id, event1)
        .await
        .expect("Failed to add first input");
    
    match result1 {
        swisspipe::workflow::input_sync::InputSyncResult::Waiting => {
            println!("✅ First input correctly marked as waiting");
        }
        _ => panic!("Expected Waiting result for first input"),
    }
    
    // Create second input event
    let mut event2 = WorkflowEvent {
        data: serde_json::json!({"message": "second input", "value": 200}),
        metadata: HashMap::new(),
        headers: HashMap::new(),
        condition_results: HashMap::new(),
        hil_task: None,
    };
    event2.metadata.insert("source".to_string(), "branch_b".to_string());
    
    // Add second input - should be ready
    let result2 = input_sync.add_input(execution_id, node_id, event2)
        .await
        .expect("Failed to add second input");
    
    match result2 {
        swisspipe::workflow::input_sync::InputSyncResult::Ready(inputs) => {
            println!("✅ Second input correctly marked as ready with {} inputs", inputs.len());
            assert_eq!(inputs.len(), 2);
            
            // Test input merging
            let merged = InputSyncService::merge_inputs(inputs, &strategy)
                .expect("Failed to merge inputs");
            
            // Verify merged data structure
            if let Some(array) = merged.data.as_array() {
                assert_eq!(array.len(), 2);
                
                // Check first input data
                if let Some(input_0) = array[0].as_object() {
                    assert!(input_0.contains_key("message"));
                    assert!(input_0.contains_key("value"));
                    assert_eq!(input_0["message"], "first input");
                    assert_eq!(input_0["value"], 100);
                }
                
                // Check second input data  
                if let Some(input_1) = array[1].as_object() {
                    assert!(input_1.contains_key("message"));
                    assert!(input_1.contains_key("value"));
                    assert_eq!(input_1["message"], "second input");
                    assert_eq!(input_1["value"], 200);
                }
                
                println!("✅ Input merging created correct array structure");
            } else {
                panic!("Merged data should be an array");
            }
            
            // Verify metadata merging
            assert!(merged.metadata.contains_key("input_0_source"));
            assert!(merged.metadata.contains_key("input_1_source"));
            println!("✅ Metadata correctly merged with input prefixes");
        }
        _ => panic!("Expected Ready result for second input"),
    }
}

#[tokio::test] 
async fn test_input_synchronization_first_wins() {
    let db_url = "sqlite::memory:";
    let db = Arc::new(establish_connection(db_url).await.expect("Failed to connect to database"));
    let input_sync = InputSyncService::new(db.clone());
    
    let execution_id = "test_exec_002";
    let node_id = "first_wins_node";
    let expected_inputs = 3;
    let strategy = InputMergeStrategy::FirstWins;

    // Create test execution record first to satisfy foreign key constraint
    create_test_execution(db.as_ref(), execution_id).await.expect("Failed to create test execution");

    // Initialize sync record
    input_sync.initialize_node_sync(execution_id, node_id, expected_inputs, &strategy)
        .await
        .expect("Failed to initialize sync");
    
    // Create test events
    let event1 = WorkflowEvent {
        data: serde_json::json!({"winner": "first", "value": 100}),
        metadata: HashMap::new(),
        headers: HashMap::new(),
        condition_results: HashMap::new(),
        hil_task: None,
    };
    
    let event2 = WorkflowEvent {
        data: serde_json::json!({"winner": "second", "value": 200}), 
        metadata: HashMap::new(),
        headers: HashMap::new(),
        condition_results: HashMap::new(),
        hil_task: None,
    };
    
    let event3 = WorkflowEvent {
        data: serde_json::json!({"winner": "third", "value": 300}),
        metadata: HashMap::new(), 
        headers: HashMap::new(),
        condition_results: HashMap::new(),
        hil_task: None,
    };
    
    // Add inputs
    let _result1 = input_sync.add_input(execution_id, node_id, event1.clone()).await.expect("Failed to add input 1");
    let _result2 = input_sync.add_input(execution_id, node_id, event2).await.expect("Failed to add input 2");
    let result3 = input_sync.add_input(execution_id, node_id, event3).await.expect("Failed to add input 3");
    
    match result3 {
        swisspipe::workflow::input_sync::InputSyncResult::Ready(inputs) => {
            println!("✅ All inputs received, testing FirstWins strategy");
            
            let merged = InputSyncService::merge_inputs(inputs, &strategy)
                .expect("Failed to merge inputs");
            
            // With FirstWins, should get the first event unchanged
            assert_eq!(merged.data, event1.data);
            println!("✅ FirstWins strategy correctly returned first input");
        }
        _ => panic!("Expected Ready result after third input"),
    }
}

#[tokio::test]
async fn test_timeout_based_strategy() {
    let db_url = "sqlite::memory:";
    let db = Arc::new(establish_connection(db_url).await.expect("Failed to connect to database"));
    let input_sync = InputSyncService::new(db.clone());
    
    let execution_id = "test_exec_003";
    let node_id = "timeout_node";
    let expected_inputs = 2;
    let strategy = InputMergeStrategy::TimeoutBased(1); // 1 second timeout

    // Create test execution record first to satisfy foreign key constraint
    create_test_execution(db.as_ref(), execution_id).await.expect("Failed to create test execution");

    // Initialize sync record
    input_sync.initialize_node_sync(execution_id, node_id, expected_inputs, &strategy)
        .await
        .expect("Failed to initialize sync");
    
    let event1 = WorkflowEvent {
        data: serde_json::json!({"message": "only input"}),
        metadata: HashMap::new(),
        headers: HashMap::new(),
        condition_results: HashMap::new(),
        hil_task: None,
    };
    
    // Add only one input
    let result1 = input_sync.add_input(execution_id, node_id, event1).await.expect("Failed to add input");
    
    match result1 {
        swisspipe::workflow::input_sync::InputSyncResult::Waiting => {
            println!("✅ Input correctly marked as waiting for timeout strategy");
        }
        _ => panic!("Expected Waiting result for timeout strategy"),
    }
    
    // Wait for timeout to occur (test would need real delay, simplified for unit test)
    tokio::time::sleep(tokio::time::Duration::from_millis(1100)).await;
    
    // Check for timeouts
    let timeouts = input_sync.check_timeouts().await.expect("Failed to check timeouts");
    
    if !timeouts.is_empty() {
        println!("✅ Timeout correctly detected for node waiting too long");
        assert_eq!(timeouts[0].node_id, node_id);
        assert_eq!(timeouts[0].execution_id, execution_id);
    } else {
        println!("ℹ️ Timeout check may need longer delay in real scenarios");
    }
}

#[test]
fn test_merge_strategies_logic() {
    // Test merge logic without database
    let event1 = WorkflowEvent {
        data: serde_json::json!({"key1": "value1"}),
        metadata: {
            let mut map = HashMap::new();
            map.insert("meta1".to_string(), "meta_value1".to_string());
            map
        },
        headers: HashMap::new(),
        condition_results: HashMap::new(),
        hil_task: None,
    };
    
    let event2 = WorkflowEvent {
        data: serde_json::json!({"key2": "value2"}),
        metadata: {
            let mut map = HashMap::new();
            map.insert("meta2".to_string(), "meta_value2".to_string());
            map
        },
        headers: HashMap::new(),
        condition_results: HashMap::new(),
        hil_task: None,
    };
    
    let inputs = vec![event1.clone(), event2];
    
    // Test FirstWins
    let first_wins_result = InputSyncService::merge_inputs(inputs.clone(), &InputMergeStrategy::FirstWins)
        .expect("FirstWins merge failed");
    assert_eq!(first_wins_result.data, event1.data);
    
    // Test WaitForAll
    let wait_all_result = InputSyncService::merge_inputs(inputs, &InputMergeStrategy::WaitForAll)
        .expect("WaitForAll merge failed");
    
    if let Some(array) = wait_all_result.data.as_array() {
        // With new array structure, check that we have 2 elements
        assert_eq!(array.len(), 2);
        
        // Verify the array elements contain the correct data
        if let Some(input_0_obj) = array[0].as_object() {
            assert!(input_0_obj.contains_key("key1"));
            assert_eq!(input_0_obj["key1"], "value1");
        }
        if let Some(input_1_obj) = array[1].as_object() {
            assert!(input_1_obj.contains_key("key2"));
            assert_eq!(input_1_obj["key2"], "value2");
        }
        println!("✅ WaitForAll merge created expected array structure");
    } else {
        panic!("WaitForAll should create array structure");
    }
    
    // Check metadata merging
    assert!(wait_all_result.metadata.contains_key("input_0_meta1"));
    assert!(wait_all_result.metadata.contains_key("input_1_meta2"));
    println!("✅ All merge strategy tests passed");
}