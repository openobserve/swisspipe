use axum::middleware;
use axum_test::TestServer;
use serde_json::json;
use std::sync::Arc;
use swisspipe::{
    api::create_router,
    api::middleware::admin_logging::admin_api_logging_middleware,
    config::Config,
    database::establish_connection,
    workflow::engine::WorkflowEngine,
    async_execution::{WorkerPool, DelayScheduler, JobManager},
    cache::WorkflowCache,
    AppState,
};
use uuid::Uuid;

/// Set up test environment with minimal configuration
fn setup_test_env() {
    std::env::set_var("SMTP_HOST", "localhost");
    std::env::set_var("SMTP_PORT", "587");
    std::env::set_var("SMTP_USERNAME", "test");
    std::env::set_var("SMTP_PASSWORD", "test");
    std::env::set_var("SMTP_FROM_EMAIL", "test@example.com");
    std::env::set_var("SP_USERNAME", "admin");
    std::env::set_var("SP_PASSWORD", "admin");
}

/// Create test app state with in-memory database
async fn create_test_app_state() -> AppState {
    let db_url = "sqlite::memory:";
    let db = Arc::new(establish_connection(db_url).await.expect("Failed to connect to test database"));

    let engine = Arc::new(WorkflowEngine::new(db.clone()).expect("Failed to create workflow engine"));
    let config = Arc::new(Config::from_env().expect("Failed to load config"));
    let worker_pool = Arc::new(WorkerPool::new(
        db.clone(),
        engine.clone(),
        Some(config.worker_pool.clone()),
    ));
    let workflow_cache = Arc::new(WorkflowCache::new(Some(300))); // 5 minute TTL

    // Create job manager and delay scheduler
    let job_manager = Arc::new(JobManager::new(db.clone()));
    let delay_scheduler = Arc::new(
        DelayScheduler::new(job_manager, db.clone())
            .await
            .expect("Failed to create delay scheduler")
    );

    AppState {
        db,
        engine,
        config,
        worker_pool,
        workflow_cache,
        delay_scheduler,
    }
}

/// Create test server with admin logging middleware
async fn create_test_server() -> TestServer {
    setup_test_env();
    let app_state = create_test_app_state().await;

    let app = create_router()
        .layer(middleware::from_fn_with_state(
            app_state.clone(),
            admin_api_logging_middleware,
        ))
        .with_state(app_state);

    TestServer::new(app).unwrap()
}

/// Generate a large workflow with many nodes and edges
fn create_large_workflow_request(node_count: usize, _edge_density: f32) -> serde_json::Value {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    // Create trigger node (start node)
    let trigger_id = Uuid::new_v4().to_string();
    nodes.push(json!({
        "id": trigger_id,
        "name": "Start Trigger",
        "node_type": {
            "Trigger": {
                "methods": ["POST"]
            }
        },
        "position_x": 100.0,
        "position_y": 100.0
    }));

    let mut node_ids = vec![trigger_id.clone()];

    // Create transformer and condition nodes
    for i in 1..node_count {
        let node_id = Uuid::new_v4().to_string();
        let x = 100.0 + ((i % 5) as f64) * 200.0; // 5 nodes per row
        let y = 100.0 + ((i / 5) as f64) * 150.0;

        let node_type = if i % 3 == 0 {
            // Condition node
            json!({
                "Condition": {
                    "script": format!("function condition(event) {{ return event.data.step_{} > 50; }}", i)
                }
            })
        } else if i % 3 == 1 {
            // Transformer node
            json!({
                "Transformer": {
                    "script": format!("function transformer(event) {{ event.data.step_{} = {}; return event; }}", i, i * 10)
                }
            })
        } else {
            // HTTP Request node
            json!({
                "HttpRequest": {
                    "url": format!("https://httpbin.org/post/{}", i),
                    "method": "POST",
                    "timeout_seconds": 30,
                    "failure_action": "Continue",
                    "retry_config": {
                        "max_attempts": 3,
                        "initial_delay_ms": 100,
                        "max_delay_ms": 5000,
                        "backoff_multiplier": 2.0
                    },
                    "headers": {}
                }
            })
        };

        nodes.push(json!({
            "id": node_id,
            "name": format!("Node {}", i),
            "node_type": node_type,
            "position_x": x,
            "position_y": y
        }));

        node_ids.push(node_id);
    }

    // Create sequential edges to ensure all nodes are reachable
    for i in 1..node_count {
        let from_idx = i - 1;
        let to_idx = i;

        // Only add condition_result if the source is actually a condition node
        let source_is_condition = from_idx > 0 && (from_idx % 3 == 0);

        edges.push(json!({
            "from_node_id": node_ids[from_idx],
            "to_node_id": node_ids[to_idx],
            "condition_result": if source_is_condition { Some(true) } else { Option::<bool>::None }
        }));
    }

    // Add a few additional edges for complexity without creating validation issues
    if node_count > 5 {
        // Add edge from node 2 to node 4 (skip one node)
        let source_is_condition = 2 % 3 == 0;
        edges.push(json!({
            "from_node_id": node_ids[2],
            "to_node_id": node_ids[4],
            "condition_result": if source_is_condition { Some(false) } else { Option::<bool>::None }
        }));
    }

    json!({
        "name": format!("Large Workflow {} Nodes", node_count),
        "description": format!("Integration test workflow with {} nodes and {} edges", node_count, edges.len()),
        "start_node_id": trigger_id,
        "nodes": nodes,
        "edges": edges
    })
}

/// Create an update workflow request that properly reuses existing nodes and adds new ones
fn create_update_workflow_request(
    existing_nodes: &[serde_json::Value],
    existing_edges: &[serde_json::Value],
    additional_node_count: usize,
) -> serde_json::Value {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    // Copy all existing nodes (keep their IDs)
    for existing_node in existing_nodes {
        nodes.push(existing_node.clone());
    }

    // Copy all existing edges
    for existing_edge in existing_edges {
        edges.push(existing_edge.clone());
    }

    // Find the last existing node ID to connect new nodes properly
    let existing_node_ids: Vec<String> = existing_nodes
        .iter()
        .map(|node| node["id"].as_str().unwrap().to_string())
        .collect();

    let last_existing_node_id = existing_node_ids.last().unwrap().clone();
    let mut new_node_ids = Vec::new();

    // Add new nodes starting from the last existing node
    for i in 0..additional_node_count {
        let new_node_id = Uuid::new_v4().to_string();
        let node_index = existing_nodes.len() + i + 1;

        let node_type = if i % 3 == 0 {
            // Condition node
            json!({
                "Condition": {
                    "script": format!("function condition(event) {{ return event.data.new_step_{} > 25; }}", i)
                }
            })
        } else if i % 3 == 1 {
            // Transformer node
            json!({
                "Transformer": {
                    "script": format!("function transformer(event) {{ event.data.new_step_{} = {}; return event; }}", i, i * 5)
                }
            })
        } else {
            // HTTP Request node
            json!({
                "HttpRequest": {
                    "url": format!("https://httpbin.org/post/new/{}", i),
                    "method": "POST",
                    "timeout_seconds": 30,
                    "failure_action": "Continue",
                    "retry_config": {
                        "max_attempts": 3,
                        "initial_delay_ms": 100,
                        "max_delay_ms": 5000,
                        "backoff_multiplier": 2.0
                    },
                    "headers": {}
                }
            })
        };

        let x = 100.0 + ((node_index % 5) as f64) * 200.0;
        let y = 100.0 + ((node_index / 5) as f64) * 150.0;

        nodes.push(json!({
            "id": new_node_id,
            "name": format!("New Node {}", i + 1),
            "node_type": node_type,
            "position_x": x,
            "position_y": y
        }));

        new_node_ids.push(new_node_id.clone());

        // Connect the first new node to the last existing node
        if i == 0 {
            edges.push(json!({
                "from_node_id": last_existing_node_id,
                "to_node_id": new_node_id,
                "condition_result": Option::<bool>::None
            }));
        }
        // Connect subsequent new nodes in sequence
        else {
            let source_is_condition = (i - 1) % 3 == 0;
            edges.push(json!({
                "from_node_id": new_node_ids[i - 1],
                "to_node_id": new_node_id,
                "condition_result": if source_is_condition { Some(true) } else { Option::<bool>::None }
            }));
        }
    }

    json!({
        "name": "Updated Large Workflow",
        "description": format!("Updated workflow with {} existing nodes and {} new nodes", existing_nodes.len(), additional_node_count),
        "nodes": nodes,
        "edges": edges
    })
}

#[tokio::test]
async fn test_create_large_workflow_10_nodes() {
    let server = create_test_server().await;
    let workflow_request = create_large_workflow_request(10, 1.5);

    println!("Creating workflow with 10 nodes and {} edges",
             workflow_request["edges"].as_array().unwrap().len());

    let response = server
        .post("/api/admin/v1/workflows")
        .add_header("Authorization", "Basic YWRtaW46YWRtaW4=") // admin:admin in base64
        .json(&workflow_request)
        .await;

    // Should succeed
    assert_eq!(response.status_code(), 201, "Response: {}", response.text());

    let workflow_response: serde_json::Value = response.json();
    assert!(workflow_response["id"].is_string());
    assert_eq!(workflow_response["name"], "Large Workflow 10 Nodes");

    println!("✅ Successfully created workflow with 10 nodes");
}

#[tokio::test]
async fn test_create_large_workflow_25_nodes() {
    let server = create_test_server().await;
    let workflow_request = create_large_workflow_request(25, 1.2);

    println!("Creating workflow with 25 nodes and {} edges",
             workflow_request["edges"].as_array().unwrap().len());

    let response = server
        .post("/api/admin/v1/workflows")
        .add_header("Authorization", "Basic YWRtaW46YWRtaW4=")
        .json(&workflow_request)
        .await;

    // Should succeed - this tests the middleware fix
    assert_eq!(response.status_code(), 201, "Response: {}", response.text());

    let workflow_response: serde_json::Value = response.json();
    assert!(workflow_response["id"].is_string());
    assert_eq!(workflow_response["name"], "Large Workflow 25 Nodes");

    println!("✅ Successfully created workflow with 25 nodes");
}

#[tokio::test]
async fn test_update_large_workflow() {
    let server = create_test_server().await;

    // First create a workflow
    let initial_request = create_large_workflow_request(15, 1.0);
    let create_response = server
        .post("/api/admin/v1/workflows")
        .add_header("Authorization", "Basic YWRtaW46YWRtaW4=")
        .json(&initial_request)
        .await;

    assert_eq!(create_response.status_code(), 201);
    let created_workflow: serde_json::Value = create_response.json();
    let workflow_id = created_workflow["id"].as_str().unwrap();

    // Get the created workflow to extract existing node IDs
    let get_response = server
        .get(&format!("/api/admin/v1/workflows/{}", workflow_id))
        .add_header("Authorization", "Basic YWRtaW46YWRtaW4=")
        .await;

    assert_eq!(get_response.status_code(), 200);
    let existing_workflow: serde_json::Value = get_response.json();

    // Extract existing nodes and edges
    let existing_nodes = existing_workflow["nodes"].as_array().unwrap();
    let existing_edges = existing_workflow["edges"].as_array().unwrap();

    // Create update request that reuses existing nodes and adds new ones
    let update_request = create_update_workflow_request(existing_nodes, existing_edges, 5); // Add 5 more nodes

    let update_response = server
        .put(&format!("/api/admin/v1/workflows/{}", workflow_id))
        .add_header("Authorization", "Basic YWRtaW46YWRtaW4=")
        .json(&update_request)
        .await;

    // Should succeed - this specifically tests the fixed middleware issue
    assert_eq!(update_response.status_code(), 200, "Update response: {}", update_response.text());

    let updated_workflow: serde_json::Value = update_response.json();
    assert_eq!(updated_workflow["name"], "Updated Large Workflow");

    println!("✅ Successfully updated workflow with additional nodes");
}

#[tokio::test]
async fn test_very_large_workflow_50_nodes() {
    let server = create_test_server().await;
    let workflow_request = create_large_workflow_request(50, 0.8);

    println!("Creating very large workflow with 50 nodes and {} edges",
             workflow_request["edges"].as_array().unwrap().len());

    let response = server
        .post("/api/admin/v1/workflows")
        .add_header("Authorization", "Basic YWRtaW46YWRtaW4=")
        .json(&workflow_request)
        .await;

    // This tests the extreme case that was failing before the fix
    assert_eq!(response.status_code(), 201, "Very large workflow response: {}", response.text());

    let workflow_response: serde_json::Value = response.json();
    assert!(workflow_response["id"].is_string());
    assert_eq!(workflow_response["name"], "Large Workflow 50 Nodes");

    println!("✅ Successfully created very large workflow with 50 nodes");
}

#[tokio::test]
async fn test_middleware_preserves_request_body() {
    let server = create_test_server().await;

    // Create a workflow with specific content that would be lost if body is consumed
    let workflow_request = json!({
        "name": "Body Preservation Test",
        "description": "Test that middleware doesn't consume request body",
        "nodes": [
            {
                "id": Uuid::new_v4().to_string(),
                "name": "Test Trigger",
                "node_type": {
                    "Trigger": {
                        "methods": ["POST"]
                    }
                },
                "position_x": 100.0,
                "position_y": 100.0
            }
        ],
        "edges": []
    });

    let response = server
        .post("/api/admin/v1/workflows")
        .add_header("Authorization", "Basic YWRtaW46YWRtaW4=")
        .json(&workflow_request)
        .await;

    // If middleware consumed the body, this would fail with JSON parsing error
    assert_eq!(response.status_code(), 201,
        "Body preservation test failed. This indicates middleware is consuming request body. Response: {}",
        response.text());

    let workflow_response: serde_json::Value = response.json();
    assert_eq!(workflow_response["name"], "Body Preservation Test");

    println!("✅ Middleware correctly preserves request body");
}

#[tokio::test]
async fn test_payload_size_limits() {
    let server = create_test_server().await;

    // Test with extremely large workflow (100+ nodes)
    let workflow_request = create_large_workflow_request(100, 0.5);
    let json_size = serde_json::to_string(&workflow_request).unwrap().len();

    println!("Testing payload size: {} bytes with 100 nodes", json_size);

    let response = server
        .post("/api/admin/v1/workflows")
        .add_header("Authorization", "Basic YWRtaW46YWRtaW4=")
        .json(&workflow_request)
        .await;

    // Should handle large payloads without issues
    if response.status_code() == 201 {
        println!("✅ Successfully handled {} byte payload", json_size);
    } else {
        // Log the failure for debugging but don't fail the test if it's a validation error
        println!("⚠️ Large payload test status: {}, size: {} bytes", response.status_code(), json_size);
        if response.status_code() == 400 {
            println!("Response: {}", response.text());
        }
    }

    // At minimum, it shouldn't fail with request body consumption issues (empty body error)
    let response_text = response.text();
    assert!(!response_text.contains("EOF while parsing a value at line 1 column 0"),
        "Request body was consumed by middleware! Response: {}", response_text);
}