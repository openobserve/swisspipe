use serde_json::json;
use swisspipe::workflow::models::{WorkflowEvent, NodeSource};

#[tokio::test]
async fn test_source_tracking_in_linear_workflow() {
    // Create initial event with no sources (as trigger would)
    let initial_event = WorkflowEvent {
        data: json!({"user_id": 123, "action": "login"}),
        metadata: std::collections::HashMap::new(),
        headers: std::collections::HashMap::new(),
        condition_results: std::collections::HashMap::new(),
        hil_task: None,
        sources: Vec::new(),
    };

    // Simulate Node 1 (HTTP Request) receiving the initial event
    let mut event_after_node1 = initial_event.clone();
    event_after_node1.sources.push(NodeSource {
        node_id: "node-1".to_string(),
        node_name: "Fetch User".to_string(),
        node_type: "HttpRequest".to_string(),
        data: initial_event.data.clone(),
        sequence: 0,
        timestamp: chrono::Utc::now(),
        metadata: None,
    });
    // Node 1 transforms data
    event_after_node1.data = json!({"user": {"id": 123, "name": "John Doe"}});

    // Verify source was added
    assert_eq!(event_after_node1.sources.len(), 1);
    assert_eq!(event_after_node1.sources[0].node_id, "node-1");
    assert_eq!(event_after_node1.sources[0].sequence, 0);
    assert_eq!(event_after_node1.sources[0].data, json!({"user_id": 123, "action": "login"}));

    // Simulate Node 2 (Transformer) receiving output from Node 1
    let mut event_after_node2 = event_after_node1.clone();
    event_after_node2.sources.push(NodeSource {
        node_id: "node-2".to_string(),
        node_name: "Transform User".to_string(),
        node_type: "Transformer".to_string(),
        data: event_after_node1.data.clone(), // Store what Node 2 received as input
        sequence: 1,
        timestamp: chrono::Utc::now(),
        metadata: None,
    });
    // Node 2 transforms data
    event_after_node2.data = json!({"user": {"id": 123, "name": "John Doe", "processed": true}});

    // Verify both sources exist
    assert_eq!(event_after_node2.sources.len(), 2);
    assert_eq!(event_after_node2.sources[0].node_id, "node-1");
    assert_eq!(event_after_node2.sources[1].node_id, "node-2");
    assert_eq!(event_after_node2.sources[1].sequence, 1);

    // Verify Node 2's source contains the data it received (Node 1's output)
    assert_eq!(event_after_node2.sources[1].data, json!({"user": {"id": 123, "name": "John Doe"}}));

    // Verify original data from Node 1 is still accessible
    assert_eq!(event_after_node2.sources[0].data, json!({"user_id": 123, "action": "login"}));
}

#[tokio::test]
async fn test_source_serialization() {
    let event = WorkflowEvent {
        data: json!({"final": "result"}),
        metadata: std::collections::HashMap::new(),
        headers: std::collections::HashMap::new(),
        condition_results: std::collections::HashMap::new(),
        hil_task: None,
        sources: vec![
            NodeSource {
                node_id: "node-1".to_string(),
                node_name: "First Node".to_string(),
                node_type: "HttpRequest".to_string(),
                data: json!({"initial": "data"}),
                sequence: 0,
                timestamp: chrono::DateTime::from_timestamp(1640000000, 0).unwrap(),
                metadata: None,
            },
            NodeSource {
                node_id: "node-2".to_string(),
                node_name: "Second Node".to_string(),
                node_type: "Transformer".to_string(),
                data: json!({"transformed": "data"}),
                sequence: 1,
                timestamp: chrono::DateTime::from_timestamp(1640000001, 0).unwrap(),
                metadata: Some([("iteration".to_string(), "1".to_string())].into_iter().collect()),
            },
        ],
    };

    // Serialize to JSON
    let json_str = serde_json::to_string(&event).expect("Failed to serialize");

    // Deserialize back
    let deserialized: WorkflowEvent = serde_json::from_str(&json_str).expect("Failed to deserialize");

    // Verify sources were preserved
    assert_eq!(deserialized.sources.len(), 2);
    assert_eq!(deserialized.sources[0].node_id, "node-1");
    assert_eq!(deserialized.sources[1].node_id, "node-2");
    assert_eq!(deserialized.sources[0].data, json!({"initial": "data"}));
    assert_eq!(deserialized.sources[1].data, json!({"transformed": "data"}));
    assert_eq!(deserialized.sources[1].metadata.as_ref().unwrap().get("iteration"), Some(&"1".to_string()));
}

#[test]
fn test_source_backward_compatibility() {
    // Test that events without sources can still be deserialized (backward compatibility)
    let json_without_sources = r#"{
        "data": {"value": 123},
        "metadata": {},
        "headers": {},
        "condition_results": {}
    }"#;

    let event: WorkflowEvent = serde_json::from_str(json_without_sources).expect("Failed to deserialize");

    // Should have empty sources array due to #[serde(default)]
    assert_eq!(event.sources.len(), 0);
    assert_eq!(event.data, json!({"value": 123}));
}
