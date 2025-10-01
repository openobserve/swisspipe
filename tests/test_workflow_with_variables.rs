use swisspipe::database::establish_connection;
use swisspipe::variables::{EncryptionService, VariableService, CreateVariableRequest, TemplateEngine};
use std::sync::Arc;

/// Integration test: Create variables and verify they can be loaded for workflow use
#[tokio::test]
async fn test_create_variables_for_workflow_use() {
    let db = establish_connection("sqlite::memory:").await.unwrap();
    let db = Arc::new(db);

    let encryption = EncryptionService::new(&[0u8; 32]);
    let variable_service = Arc::new(VariableService::new(db.clone(), encryption));
    let template_engine = Arc::new(TemplateEngine::new());

    // Create variables
    variable_service.create_variable(CreateVariableRequest {
        name: "API_HOST".to_string(),
        value_type: "text".to_string(),
        value: "https://api.example.com".to_string(),
        description: Some("API host".to_string()),
    }).await.unwrap();

    variable_service.create_variable(CreateVariableRequest {
        name: "API_TOKEN".to_string(),
        value_type: "secret".to_string(),
        value: "secret-token-12345".to_string(),
        description: Some("API token".to_string()),
    }).await.unwrap();

    // Load variables as map (simulating workflow execution)
    let vars = variable_service.load_variables_map().await.unwrap();

    // Verify both variables are available
    assert_eq!(vars.get("API_HOST").unwrap(), "https://api.example.com");
    assert_eq!(vars.get("API_TOKEN").unwrap(), "secret-token-12345");

    // Test template resolution (simulating node execution)
    let url = template_engine.resolve("{{ env.API_HOST }}/v1/users", &vars).unwrap();
    assert_eq!(url, "https://api.example.com/v1/users");

    let auth = template_engine.resolve("Bearer {{ env.API_TOKEN }}", &vars).unwrap();
    assert_eq!(auth, "Bearer secret-token-12345");
}

/// Test variables persist across workflow engine restarts (database persistence)
#[tokio::test]
async fn test_variables_persist_across_restarts() {
    let db = establish_connection("sqlite::memory:").await.unwrap();
    let db = Arc::new(db);

    // First "session" - create variables
    {
        let encryption = EncryptionService::new(&[0u8; 32]);
        let variable_service = VariableService::new(db.clone(), encryption);

        variable_service.create_variable(CreateVariableRequest {
            name: "PERSISTENT_VAR".to_string(),
            value_type: "text".to_string(),
            value: "persistent_value".to_string(),
            description: None,
        }).await.unwrap();
    }

    // Second "session" - verify variables still exist
    {
        let encryption = EncryptionService::new(&[0u8; 32]);
        let variable_service = VariableService::new(db.clone(), encryption);

        let vars = variable_service.get_all_variables().await.unwrap();
        assert_eq!(vars.len(), 1);
        assert_eq!(vars[0].name, "PERSISTENT_VAR");
        assert_eq!(vars[0].value, "persistent_value");
    }
}

/// Test full variable setup and usage (without workflow engine initialization)
#[tokio::test]
async fn test_full_variable_setup_and_template_resolution() {
    let db = establish_connection("sqlite::memory:").await.unwrap();
    let db = Arc::new(db);

    // Setup all services (simulating what main.rs does)
    let encryption = EncryptionService::new(&[1u8; 32]);
    let variable_service = VariableService::new(db.clone(), encryption);
    let template_engine = TemplateEngine::new();

    // Create some test variables
    variable_service.create_variable(CreateVariableRequest {
        name: "TEST_VAR_1".to_string(),
        value_type: "text".to_string(),
        value: "value1".to_string(),
        description: None,
    }).await.unwrap();

    variable_service.create_variable(CreateVariableRequest {
        name: "TEST_VAR_2".to_string(),
        value_type: "secret".to_string(),
        value: "secret_value2".to_string(),
        description: None,
    }).await.unwrap();

    // Verify variables can be loaded through the service
    let vars = variable_service.load_variables_map().await.unwrap();
    assert_eq!(vars.len(), 2);
    assert!(vars.contains_key("TEST_VAR_1"));
    assert!(vars.contains_key("TEST_VAR_2"));

    // Verify templates can be resolved (this is what happens during node execution)
    let resolved = template_engine.resolve(
        "{{ env.TEST_VAR_1 }} and {{ env.TEST_VAR_2 }}",
        &vars
    ).unwrap();
    assert_eq!(resolved, "value1 and secret_value2");
}
