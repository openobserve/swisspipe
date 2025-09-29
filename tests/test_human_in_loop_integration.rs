use swisspipe::{
    database::{establish_connection, entities, human_in_loop_tasks},
    workflow::{
        engine::WorkflowEngine,
        models::{NodeType, WorkflowEvent},
    },
    hil::HilService,
};
use sea_orm::{ActiveModelTrait, Set, EntityTrait, ColumnTrait, QueryFilter, DatabaseConnection};
use serde_json::json;
use std::{collections::HashMap, sync::Arc};
use uuid::Uuid;
use chrono::Utc;

/// Setup a test database and return connection
async fn setup_test_db() -> Arc<DatabaseConnection> {
    let db = establish_connection("sqlite::memory:").await.unwrap();
    Arc::new(db)
}

/// Create a test workflow with HIL node
async fn create_test_hil_workflow(db: &DatabaseConnection, workflow_name: &str) -> (String, String, String) {
    let workflow_id = Uuid::new_v4().to_string();
    let trigger_node_id = Uuid::new_v4().to_string();
    let hil_node_id = Uuid::new_v4().to_string();

    // Create workflow
    let workflow = entities::ActiveModel {
        id: Set(workflow_id.clone()),
        name: Set(workflow_name.to_string()),
        description: Set(Some("Test HIL workflow".to_string())),
        start_node_id: Set(Some(trigger_node_id.clone())),
        enabled: Set(true),
        created_at: Set(Utc::now().timestamp_micros()),
        updated_at: Set(Utc::now().timestamp_micros()),
        ..Default::default()
    };
    workflow.insert(db).await.unwrap();

    // Create trigger node
    let trigger_node_config = json!({
        "method": "POST",
        "path": "/test"
    });

    let trigger_node = swisspipe::database::nodes::ActiveModel {
        id: Set(trigger_node_id.clone()),
        workflow_id: Set(workflow_id.clone()),
        name: Set("Trigger".to_string()),
        node_type: Set("trigger".to_string()),
        config: Set(trigger_node_config.to_string()),
        position_x: Set(100.0),
        position_y: Set(100.0),
        created_at: Set(Utc::now().timestamp_micros()),
        input_merge_strategy: Set(None),
    };
    trigger_node.insert(db).await.unwrap();

    // Create HIL node
    let hil_node_config = json!({
        "title": "Approval Required",
        "description": "Please review this request",
        "timeout_seconds": 3600,
        "timeout_action": "denied",
        "required_fields": ["decision", "comments"],
        "metadata": {
            "priority": "high",
            "department": "security"
        }
    });

    let hil_node = swisspipe::database::nodes::ActiveModel {
        id: Set(hil_node_id.clone()),
        workflow_id: Set(workflow_id.clone()),
        name: Set("Human Approval".to_string()),
        node_type: Set("human_in_loop".to_string()),
        config: Set(hil_node_config.to_string()),
        position_x: Set(200.0),
        position_y: Set(100.0),
        created_at: Set(Utc::now().timestamp_micros()),
        input_merge_strategy: Set(None),
    };
    hil_node.insert(db).await.unwrap();

    // Create edge from trigger to HIL
    let edge = swisspipe::database::edges::ActiveModel {
        id: Set(Uuid::new_v4().to_string()),
        workflow_id: Set(workflow_id.clone()),
        from_node_id: Set(trigger_node_id.clone()),
        to_node_id: Set(hil_node_id.clone()),
        condition_result: Set(None),
        source_handle_id: Set(None),
        created_at: Set(Utc::now().timestamp_micros()),
    };
    edge.insert(db).await.unwrap();

    (workflow_id, trigger_node_id, hil_node_id)
}

#[tokio::test]
async fn test_hil_workflow_creation_and_basic_execution() {
    let db = setup_test_db().await;
    let (workflow_id, _trigger_node_id, _hil_node_id) = create_test_hil_workflow(&db, "Basic HIL Test").await;

    // Create workflow engine
    let engine = WorkflowEngine::new(db.clone()).unwrap();

    // Load workflow
    let workflow = engine.load_workflow(&workflow_id).await.unwrap();
    assert_eq!(workflow.name, "Basic HIL Test");
    assert_eq!(workflow.nodes.len(), 2); // trigger + HIL
    assert_eq!(workflow.edges.len(), 1); // trigger -> HIL

    // Find HIL node
    let hil_node = workflow.nodes.iter().find(|n| {
        matches!(n.node_type, NodeType::HumanInLoop { .. })
    });
    assert!(hil_node.is_some(), "HIL node should exist in workflow");

    if let NodeType::HumanInLoop { title, description, timeout_seconds, .. } = &hil_node.unwrap().node_type {
        assert_eq!(title, "Approval Required");
        assert_eq!(description.as_ref().unwrap(), "Please review this request");
        assert_eq!(timeout_seconds.unwrap(), 3600);
    }
}

#[tokio::test]
async fn test_hil_task_creation_and_database_persistence() {
    let db = setup_test_db().await;
    let (workflow_id, _trigger_node_id, hil_node_id) = create_test_hil_workflow(&db, "HIL DB Test").await;

    // Create HIL service
    let hil_service = Arc::new(HilService::new(db.clone()));

    // Create test event
    let mut event = WorkflowEvent {
        data: json!({"user": "test_user", "action": "create_resource"}),
        metadata: HashMap::new(),
        headers: HashMap::new(),
        condition_results: HashMap::new(),
        hil_task: None,
    };
    event.metadata.insert("execution_id".to_string(), Uuid::new_v4().to_string());

    // Create HIL node type for testing
    let node_type = NodeType::HumanInLoop {
        title: "Test Approval".to_string(),
        description: Some("Test description".to_string()),
        timeout_seconds: Some(7200),
        timeout_action: Some("denied".to_string()),
        required_fields: Some(vec!["decision".to_string(), "reason".to_string()]),
        metadata: Some(json!({"test_flag": true})),
    };

    // Create HIL task
    let node_execution_id = Uuid::new_v4().to_string();
    let hil_params = swisspipe::hil::service::HilTaskParams {
        execution_id: &event.metadata["execution_id"],
        workflow_id: &workflow_id,
        node_id: &hil_node_id,
        node_execution_id: &node_execution_id,
        config: &node_type,
        event: &event,
    };

    let (task_id, _resumption_state) = hil_service
        .create_hil_task_and_prepare_resumption(hil_params)
        .await
        .unwrap();

    // Verify task was created in database
    let hil_task = human_in_loop_tasks::Entity::find()
        .filter(human_in_loop_tasks::Column::Id.eq(&task_id))
        .one(db.as_ref())
        .await
        .unwrap();

    assert!(hil_task.is_some(), "HIL task should be created in database");
    let task = hil_task.unwrap();
    assert_eq!(task.title, "Test Approval");
    assert_eq!(task.status, "pending");
    assert_eq!(task.workflow_id, workflow_id);
    assert_eq!(task.node_id, hil_node_id);
    assert_eq!(task.node_execution_id, node_execution_id);
    assert!(task.timeout_at.is_some());
    assert_eq!(task.timeout_action.as_ref().unwrap(), "denied");

    // Verify required fields were stored correctly
    let required_fields: Vec<String> = serde_json::from_value(
        task.required_fields.unwrap()
    ).unwrap();
    assert_eq!(required_fields, vec!["decision", "reason"]);

    // Verify metadata was stored correctly
    let stored_metadata: serde_json::Value = task.metadata.unwrap();
    assert_eq!(stored_metadata["test_flag"], true);
}

#[tokio::test]
async fn test_hil_task_database_operations() {
    let db = setup_test_db().await;
    let (workflow_id, _trigger_node_id, hil_node_id) = create_test_hil_workflow(&db, "HIL DB Operations").await;

    let execution_id = Uuid::new_v4().to_string();
    let node_execution_id = Uuid::new_v4().to_string();
    let task_id = Uuid::new_v4().to_string();

    // Create a HIL task directly in the database
    let hil_task = human_in_loop_tasks::ActiveModel {
        id: Set(task_id.clone()),
        execution_id: Set(execution_id.clone()),
        node_id: Set(hil_node_id.clone()),
        node_execution_id: Set(node_execution_id.clone()),
        workflow_id: Set(workflow_id.clone()),
        title: Set("Database Test Task".to_string()),
        description: Set(Some("Testing database operations".to_string())),
        status: Set("pending".to_string()),
        timeout_at: Set(Some((Utc::now() + chrono::Duration::hours(1)).timestamp_micros())),
        timeout_action: Set(Some("denied".to_string())),
        required_fields: Set(Some(json!(["approval"]))),
        metadata: Set(Some(json!({"test": "database"}))),
        response_data: Set(None),
        response_received_at: Set(None),
        created_at: Set(Utc::now().timestamp_micros()),
        updated_at: Set(Utc::now().timestamp_micros()),
    };
    hil_task.insert(db.as_ref()).await.unwrap();

    // Test retrieving by task ID
    let retrieved_task = human_in_loop_tasks::Entity::find()
        .filter(human_in_loop_tasks::Column::Id.eq(&task_id))
        .one(db.as_ref())
        .await
        .unwrap();

    assert!(retrieved_task.is_some(), "Task should be retrievable by ID");
    let task = retrieved_task.unwrap();
    assert_eq!(task.title, "Database Test Task");
    assert_eq!(task.status, "pending");

    // Test updating task status
    let updated_task = human_in_loop_tasks::ActiveModel {
        id: Set(task_id.clone()),
        status: Set("completed".to_string()),
        response_data: Set(Some(json!({"decision": "approved"}))),
        response_received_at: Set(Some(Utc::now().timestamp_micros())),
        updated_at: Set(Utc::now().timestamp_micros()),
        ..Default::default()
    };
    updated_task.update(db.as_ref()).await.unwrap();

    // Verify the update
    let final_task = human_in_loop_tasks::Entity::find()
        .filter(human_in_loop_tasks::Column::Id.eq(&task_id))
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(final_task.status, "completed");
    assert!(final_task.response_data.is_some());
    let response_data: serde_json::Value = final_task.response_data.unwrap();
    assert_eq!(response_data["decision"], "approved");
}

#[tokio::test]
async fn test_hil_task_query_operations() {
    let db = setup_test_db().await;
    let (workflow_id, _trigger_node_id, hil_node_id) = create_test_hil_workflow(&db, "HIL Query Test").await;

    // Create multiple HIL tasks with different statuses
    let execution_id = Uuid::new_v4().to_string();
    let tasks_data = vec![
        ("pending", "Task 1", None),
        ("pending", "Task 2", None),
        ("completed", "Task 3", Some(json!({"decision": "approved"}))),
        ("completed", "Task 4", Some(json!({"decision": "denied"}))),
    ];

    let mut task_ids = Vec::new();
    for (status, title, response_data) in tasks_data {
        let task_id = Uuid::new_v4().to_string();
        let node_execution_id = Uuid::new_v4().to_string();

        let hil_task = human_in_loop_tasks::ActiveModel {
            id: Set(task_id.clone()),
            execution_id: Set(execution_id.clone()),
            node_id: Set(hil_node_id.clone()),
            node_execution_id: Set(node_execution_id),
            workflow_id: Set(workflow_id.clone()),
            title: Set(title.to_string()),
            description: Set(Some("Query test task".to_string())),
            status: Set(status.to_string()),
            timeout_at: Set(Some((Utc::now() + chrono::Duration::hours(2)).timestamp_micros())),
            timeout_action: Set(Some("denied".to_string())),
            required_fields: Set(Some(json!(["decision"]))),
            metadata: Set(Some(json!({"query_test": true}))),
            response_data: Set(response_data.map(|v| v)),
            response_received_at: Set(if status == "completed" {
                Some(Utc::now().timestamp_micros())
            } else {
                None
            }),
            created_at: Set(Utc::now().timestamp_micros()),
            updated_at: Set(Utc::now().timestamp_micros()),
        };
        hil_task.insert(db.as_ref()).await.unwrap();
        task_ids.push(task_id);
    }

    // Query pending tasks
    let pending_tasks = human_in_loop_tasks::Entity::find()
        .filter(human_in_loop_tasks::Column::Status.eq("pending"))
        .filter(human_in_loop_tasks::Column::WorkflowId.eq(&workflow_id))
        .all(db.as_ref())
        .await
        .unwrap();

    assert_eq!(pending_tasks.len(), 2, "Should have 2 pending tasks");

    // Query completed tasks
    let completed_tasks = human_in_loop_tasks::Entity::find()
        .filter(human_in_loop_tasks::Column::Status.eq("completed"))
        .filter(human_in_loop_tasks::Column::WorkflowId.eq(&workflow_id))
        .all(db.as_ref())
        .await
        .unwrap();

    assert_eq!(completed_tasks.len(), 2, "Should have 2 completed tasks");

    // Verify completed tasks have response data
    for task in completed_tasks {
        assert!(task.response_data.is_some(), "Completed task should have response data");
        assert!(task.response_received_at.is_some(), "Completed task should have response timestamp");
    }
}

#[tokio::test]
async fn test_hil_timeout_detection() {
    let db = setup_test_db().await;
    let (workflow_id, _trigger_node_id, hil_node_id) = create_test_hil_workflow(&db, "HIL Timeout Test").await;

    // Create a task that has already timed out
    let execution_id = Uuid::new_v4().to_string();
    let node_execution_id = Uuid::new_v4().to_string();
    let task_id = Uuid::new_v4().to_string();

    // Set timeout in the past
    let past_timeout = (Utc::now() - chrono::Duration::minutes(30)).timestamp_micros();

    let hil_task = human_in_loop_tasks::ActiveModel {
        id: Set(task_id.clone()),
        execution_id: Set(execution_id.clone()),
        node_id: Set(hil_node_id.clone()),
        node_execution_id: Set(node_execution_id.clone()),
        workflow_id: Set(workflow_id.clone()),
        title: Set("Timeout Test Task".to_string()),
        description: Set(Some("Testing timeout behavior".to_string())),
        status: Set("pending".to_string()),
        timeout_at: Set(Some(past_timeout)),
        timeout_action: Set(Some("denied".to_string())),
        required_fields: Set(Some(json!(["decision"]))),
        metadata: Set(Some(json!({"test": "timeout"}))),
        response_data: Set(None),
        response_received_at: Set(None),
        created_at: Set(Utc::now().timestamp_micros()),
        updated_at: Set(Utc::now().timestamp_micros()),
    };
    hil_task.insert(db.as_ref()).await.unwrap();

    // HIL service would be used for timeout processing in production
    let _hil_service = Arc::new(HilService::new(db.clone()));

    // Check for timed-out tasks
    let timed_out_tasks = human_in_loop_tasks::Entity::find()
        .filter(human_in_loop_tasks::Column::Status.eq("pending"))
        .filter(human_in_loop_tasks::Column::TimeoutAt.lt(Utc::now().timestamp_micros()))
        .all(db.as_ref())
        .await
        .unwrap();

    assert_eq!(timed_out_tasks.len(), 1, "Should detect 1 timed-out task");
    assert_eq!(timed_out_tasks[0].id, task_id, "Should find the correct timed-out task");

    // Verify task properties
    let timed_out_task = &timed_out_tasks[0];
    assert_eq!(timed_out_task.timeout_action.as_ref().unwrap(), "denied");
    assert!(timed_out_task.timeout_at.unwrap() < Utc::now().timestamp_micros());
}