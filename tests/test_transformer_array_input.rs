use swisspipe::utils::javascript::JavaScriptExecutor;
use swisspipe::workflow::models::WorkflowEvent;
use std::collections::HashMap;

#[tokio::test]
async fn test_transformer_with_array_input() {
    let js_executor = JavaScriptExecutor::new().expect("Failed to create JavaScript executor");
    
    // Create a WorkflowEvent with array data (simulating merged inputs)
    let event = WorkflowEvent {
        data: serde_json::json!([
            {"value": 10, "source": "branch_a"},
            {"value": 20, "source": "branch_b"}
        ]),
        metadata: HashMap::new(),
        headers: HashMap::new(),
        condition_results: HashMap::new(),
    };
    
    // Simple JavaScript transformer that processes array input
    let script = r#"
        function transformer(event) {
            if (Array.isArray(event.data)) {
                return {
                    data: {
                        merged: true,
                        input_count: event.data.length,
                        total_value: event.data[0].value + event.data[1].value
                    },
                    metadata: event.metadata,
                    headers: event.headers,
                    condition_results: event.condition_results
                };
            } else {
                return event;
            }
        }
    "#;
    
    // Execute the transformer
    let result = js_executor.execute_transformer(script, event).await
        .expect("Failed to execute transformer");
    
    // Verify the result
    println!("✅ Transformer result: {}", serde_json::to_string_pretty(&result.data).unwrap());
    
    // Check that the transformer processed the array correctly
    assert!(result.data.get("merged").unwrap().as_bool().unwrap());
    assert_eq!(result.data.get("total_value").unwrap().as_i64().unwrap(), 30);
    assert_eq!(result.data.get("input_count").unwrap().as_i64().unwrap(), 2);
    
    println!("✅ SUCCESS: Transformer correctly processed array input!");
}

#[tokio::test]
async fn test_transformer_with_single_input() {
    let js_executor = JavaScriptExecutor::new().expect("Failed to create JavaScript executor");
    
    // Create a WorkflowEvent with single object data
    let event = WorkflowEvent {
        data: serde_json::json!({"value": 42, "source": "single"}),
        metadata: HashMap::new(),
        headers: HashMap::new(),
        condition_results: HashMap::new(),
    };
    
    // Simple transformer that handles both single and array input
    let script = r#"
        function transformer(event) {
            if (Array.isArray(event.data)) {
                return {
                    data: { merged: true, total_value: 100 },
                    metadata: event.metadata,
                    headers: event.headers, 
                    condition_results: event.condition_results
                };
            } else {
                return {
                    data: {
                        single: true,
                        value: event.data.value,
                        processed: true
                    },
                    metadata: event.metadata,
                    headers: event.headers,
                    condition_results: event.condition_results
                };
            }
        }
    "#;
    
    // Execute the transformer
    let result = js_executor.execute_transformer(script, event).await
        .expect("Failed to execute transformer");
    
    // Verify the result
    println!("✅ Single input result: {}", serde_json::to_string_pretty(&result.data).unwrap());
    
    // Check that the transformer processed the single input correctly
    assert!(result.data.get("single").unwrap().as_bool().unwrap());
    assert_eq!(result.data.get("value").unwrap().as_i64().unwrap(), 42);
    assert!(result.data.get("processed").unwrap().as_bool().unwrap());
    
    println!("✅ SUCCESS: Transformer correctly handled single input!");
}

#[tokio::test]
async fn test_transformer_return_data_only() {
    let js_executor = JavaScriptExecutor::new().expect("Failed to create JavaScript executor");
    
    let event = WorkflowEvent {
        data: serde_json::json!({"message": "test", "value": 42}),
        metadata: {
            let mut map = HashMap::new();
            map.insert("source".to_string(), "test".to_string());
            map
        },
        headers: HashMap::new(),
        condition_results: HashMap::new(),
    };
    
    // Transformer that returns just data (not complete event)
    let script = r#"
        function transformer(event) {
            return {
                processed: true,
                original_value: event.data.value,
                doubled_value: event.data.value * 2
            };
        }
    "#;
    
    let result = js_executor.execute_transformer(script, event.clone()).await
        .expect("Failed to execute transformer");
    
    // Verify the result has complete WorkflowEvent structure
    assert!(result.data.get("processed").unwrap().as_bool().unwrap());
    assert_eq!(result.data.get("original_value").unwrap().as_i64().unwrap(), 42);
    assert_eq!(result.data.get("doubled_value").unwrap().as_i64().unwrap(), 84);
    
    // Verify metadata, headers, and condition_results are preserved from original
    assert_eq!(result.metadata.get("source").unwrap(), "test");
    assert!(result.headers.is_empty());
    assert!(result.condition_results.is_empty());
    
    println!("✅ SUCCESS: Transformer returning data-only works correctly!");
}

#[tokio::test]
async fn test_transformer_array_access_patterns() {
    let js_executor = JavaScriptExecutor::new().expect("Failed to create JavaScript executor");
    
    // Create a complex array input
    let event = WorkflowEvent {
        data: serde_json::json!([
            {"id": 1, "name": "Alice", "score": 95},
            {"id": 2, "name": "Bob", "score": 87},
            {"id": 3, "name": "Charlie", "score": 92}
        ]),
        metadata: HashMap::new(),
        headers: HashMap::new(),
        condition_results: HashMap::new(),
    };
    
    // Test various array access patterns
    let script = r#"
        function transformer(event) {
            if (!Array.isArray(event.data)) {
                return event;
            }
            
            var results = {
                first_item: event.data[0],
                last_item: event.data[event.data.length - 1],
                names: event.data.map(function(item) { return item.name; }),
                average_score: event.data.reduce(function(sum, item) { 
                    return sum + item.score; 
                }, 0) / event.data.length,
                high_scores: event.data.filter(function(item) { 
                    return item.score >= 90; 
                }),
                processed_at: new Date().toISOString()
            };
            
            return {
                data: results,
                metadata: event.metadata,
                headers: event.headers,
                condition_results: event.condition_results
            };
        }
    "#;
    
    // Execute the transformer
    let result = js_executor.execute_transformer(script, event).await
        .expect("Failed to execute transformer");
    
    // Verify the result
    println!("✅ Array access result: {}", serde_json::to_string_pretty(&result.data).unwrap());
    
    // Check first and last item access
    let first_item = result.data.get("first_item").unwrap();
    assert_eq!(first_item.get("name").unwrap().as_str().unwrap(), "Alice");
    
    let last_item = result.data.get("last_item").unwrap();
    assert_eq!(last_item.get("name").unwrap().as_str().unwrap(), "Charlie");
    
    // Check array methods (map, reduce, filter)
    let names = result.data.get("names").unwrap().as_array().unwrap();
    assert_eq!(names.len(), 3);
    assert_eq!(names[0].as_str().unwrap(), "Alice");
    
    let average_score = result.data.get("average_score").unwrap().as_f64().unwrap();
    assert!((average_score - 91.33).abs() < 0.1);
    
    let high_scores = result.data.get("high_scores").unwrap().as_array().unwrap();
    assert_eq!(high_scores.len(), 2); // Alice (95) and Charlie (92)
    
    println!("✅ SUCCESS: All array access patterns work correctly!");
}