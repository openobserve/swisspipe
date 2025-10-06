use swisspipe::database::{establish_connection, environment_variables};
use swisspipe::variables::{EncryptionService, VariableService, CreateVariableRequest, UpdateVariableRequest, TemplateEngine};
use sea_orm::{EntityTrait, ColumnTrait, QueryFilter};
use std::sync::Arc;
use std::collections::HashMap;

/// Test encryption and decryption of secret values
#[tokio::test]
async fn test_encryption_roundtrip() {
    let key = [0u8; 32];
    let service = EncryptionService::new(&key);

    let plaintext = "my-super-secret-api-key-12345";
    let encrypted = service.encrypt(plaintext).unwrap();

    // Encrypted value should be different from plaintext
    assert_ne!(encrypted, plaintext);

    // Should decrypt back to original
    let decrypted = service.decrypt(&encrypted).unwrap();
    assert_eq!(decrypted, plaintext);
}

/// Test that encryption produces different ciphertexts for same plaintext
#[tokio::test]
async fn test_encryption_randomness() {
    let key = [1u8; 32];
    let service = EncryptionService::new(&key);

    let plaintext = "same-value";
    let encrypted1 = service.encrypt(plaintext).unwrap();
    let encrypted2 = service.encrypt(plaintext).unwrap();

    // Different nonces should produce different ciphertexts
    assert_ne!(encrypted1, encrypted2);

    // But both should decrypt to same value
    assert_eq!(service.decrypt(&encrypted1).unwrap(), plaintext);
    assert_eq!(service.decrypt(&encrypted2).unwrap(), plaintext);
}

/// Test variable name validation
#[tokio::test]
async fn test_variable_name_validation() {
    // Valid names
    assert!(VariableService::validate_name("API_KEY").is_ok());
    assert!(VariableService::validate_name("API_KEY_123").is_ok());
    assert!(VariableService::validate_name("MY_VAR").is_ok());
    assert!(VariableService::validate_name("A").is_ok());

    // Invalid names
    assert!(VariableService::validate_name("").is_err());
    assert!(VariableService::validate_name("lowercase").is_err());
    assert!(VariableService::validate_name("Has Space").is_err());
    assert!(VariableService::validate_name("has-dash").is_err());
    assert!(VariableService::validate_name("123_START_WITH_NUM").is_ok()); // Numbers are allowed
}

/// Test creating and retrieving text variables
#[tokio::test]
async fn test_create_text_variable() {
    let db = establish_connection("sqlite::memory:").await.unwrap();
    let db = Arc::new(db);

    let encryption = EncryptionService::new(&[0u8; 32]);
    let service = VariableService::new(db.clone(), encryption);

    let req = CreateVariableRequest {
        name: "API_HOST".to_string(),
        value_type: "text".to_string(),
        value: "https://api.example.com".to_string(),
        description: Some("Production API endpoint".to_string()),
    };

    let created = service.create_variable(req).await.unwrap();

    assert_eq!(created.name, "API_HOST");
    assert_eq!(created.value_type, "text");
    assert_eq!(created.value, "https://api.example.com"); // Text values not masked
    assert_eq!(created.description, Some("Production API endpoint".to_string()));
}

/// Test creating and retrieving secret variables (should be masked)
#[tokio::test]
async fn test_create_secret_variable() {
    let db = establish_connection("sqlite::memory:").await.unwrap();
    let db = Arc::new(db);

    let encryption = EncryptionService::new(&[0u8; 32]);
    let service = VariableService::new(db.clone(), encryption);

    let req = CreateVariableRequest {
        name: "API_TOKEN".to_string(),
        value_type: "secret".to_string(),
        value: "my-secret-token-12345".to_string(),
        description: Some("API authentication token".to_string()),
    };

    let created = service.create_variable(req).await.unwrap();

    assert_eq!(created.name, "API_TOKEN");
    assert_eq!(created.value_type, "secret");
    assert_eq!(created.value, "••••••••"); // Secrets should be masked

    // But the actual encrypted value should be stored in database
    let db_record = environment_variables::Entity::find()
        .filter(environment_variables::Column::Name.eq("API_TOKEN"))
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();

    assert_ne!(db_record.value, "my-secret-token-12345"); // Should be encrypted
    assert_ne!(db_record.value, "••••••••"); // Should not be the mask
}

/// Test duplicate variable name prevention
#[tokio::test]
async fn test_duplicate_variable_name() {
    let db = establish_connection("sqlite::memory:").await.unwrap();
    let db = Arc::new(db);

    let encryption = EncryptionService::new(&[0u8; 32]);
    let service = VariableService::new(db.clone(), encryption);

    let req = CreateVariableRequest {
        name: "DUPLICATE_VAR".to_string(),
        value_type: "text".to_string(),
        value: "value1".to_string(),
        description: None,
    };

    // First create should succeed
    service.create_variable(req.clone()).await.unwrap();

    // Second create with same name should fail
    let result = service.create_variable(req).await;
    assert!(result.is_err());
}

/// Test updating a variable
#[tokio::test]
async fn test_update_variable() {
    let db = establish_connection("sqlite::memory:").await.unwrap();
    let db = Arc::new(db);

    let encryption = EncryptionService::new(&[0u8; 32]);
    let service = VariableService::new(db.clone(), encryption);

    // Create initial variable
    let create_req = CreateVariableRequest {
        name: "UPDATE_TEST".to_string(),
        value_type: "text".to_string(),
        value: "original_value".to_string(),
        description: Some("Original description".to_string()),
    };
    let created = service.create_variable(create_req).await.unwrap();

    // Update it
    let update_req = UpdateVariableRequest {
        value: "new_value".to_string(),
        description: Some("Updated description".to_string()),
    };
    let updated = service.update_variable(&created.id, update_req).await.unwrap();

    assert_eq!(updated.name, "UPDATE_TEST"); // Name unchanged
    assert_eq!(updated.value, "new_value");
    assert_eq!(updated.description, Some("Updated description".to_string()));
}

/// Test deleting a variable
#[tokio::test]
async fn test_delete_variable() {
    let db = establish_connection("sqlite::memory:").await.unwrap();
    let db = Arc::new(db);

    let encryption = EncryptionService::new(&[0u8; 32]);
    let service = VariableService::new(db.clone(), encryption);

    // Create variable
    let req = CreateVariableRequest {
        name: "DELETE_TEST".to_string(),
        value_type: "text".to_string(),
        value: "value".to_string(),
        description: None,
    };
    let created = service.create_variable(req).await.unwrap();

    // Delete it
    service.delete_variable(&created.id).await.unwrap();

    // Verify it's gone
    let result = service.get_variable(&created.id).await;
    assert!(result.is_err());
}

/// Test loading variables as a map for template resolution
#[tokio::test]
async fn test_load_variables_map() {
    let db = establish_connection("sqlite::memory:").await.unwrap();
    let db = Arc::new(db);

    let encryption = EncryptionService::new(&[0u8; 32]);
    let service = VariableService::new(db.clone(), encryption);

    // Create text variable
    service.create_variable(CreateVariableRequest {
        name: "TEXT_VAR".to_string(),
        value_type: "text".to_string(),
        value: "text_value".to_string(),
        description: None,
    }).await.unwrap();

    // Create secret variable
    service.create_variable(CreateVariableRequest {
        name: "SECRET_VAR".to_string(),
        value_type: "secret".to_string(),
        value: "secret_value".to_string(),
        description: None,
    }).await.unwrap();

    // Load as map
    let map = service.load_variables_map().await.unwrap();

    assert_eq!(map.get("TEXT_VAR").unwrap(), "text_value");
    assert_eq!(map.get("SECRET_VAR").unwrap(), "secret_value"); // Secrets decrypted in map
}

/// Test template engine resolves simple variables
#[tokio::test]
async fn test_template_simple_resolution() {
    let engine = TemplateEngine::new();
    let mut vars = HashMap::new();
    vars.insert("API_HOST".to_string(), "https://api.example.com".to_string());

    let result = engine.resolve("{{ env.API_HOST }}/users", &vars).unwrap();
    assert_eq!(result, "https://api.example.com/users");
}

/// Test template engine resolves multiple variables
#[tokio::test]
async fn test_template_multiple_variables() {
    let engine = TemplateEngine::new();
    let mut vars = HashMap::new();
    vars.insert("HOST".to_string(), "api.example.com".to_string());
    vars.insert("PORT".to_string(), "8080".to_string());
    vars.insert("PATH".to_string(), "v1/users".to_string());

    let result = engine.resolve("https://{{ env.HOST }}:{{ env.PORT }}/{{ env.PATH }}", &vars).unwrap();
    assert_eq!(result, "https://api.example.com:8080/v1/users");
}

/// Test template engine fails on undefined variables (strict mode)
#[tokio::test]
async fn test_template_undefined_variable_fails() {
    let engine = TemplateEngine::new();
    let vars = HashMap::new();

    let result = engine.resolve("{{ env.UNDEFINED_VAR }}", &vars);
    assert!(result.is_err());
}

/// Test template engine behavior with undefined helpers
#[tokio::test]
async fn test_template_passes_through_non_env() {
    let engine = TemplateEngine::new();
    let vars = HashMap::new();

    // Templates with undefined helpers should fail in strict mode
    // This is correct behavior - email templates should NOT go through the variables template engine
    let result = engine.resolve("Hello {{json data}}", &vars);
    assert!(result.is_err(), "Templates with undefined helpers should fail");
    assert!(result.unwrap_err().contains("Helper not defined"));

    // Template without any {{ should pass through
    let result = engine.resolve("https://api.example.com/users", &vars).unwrap();
    assert_eq!(result, "https://api.example.com/users");
}

/// Test template engine with actual workflow scenario
#[tokio::test]
async fn test_template_workflow_scenario() {
    let engine = TemplateEngine::new();
    let mut vars = HashMap::new();
    vars.insert("API_BASE".to_string(), "https://api.example.com".to_string());
    vars.insert("API_KEY".to_string(), "secret-key-12345".to_string());
    vars.insert("VERSION".to_string(), "v2".to_string());

    // HTTP URL resolution
    let url = engine.resolve("{{ env.API_BASE }}/{{ env.VERSION }}/users", &vars).unwrap();
    assert_eq!(url, "https://api.example.com/v2/users");

    // HTTP Header resolution
    let auth_header = engine.resolve("Bearer {{ env.API_KEY }}", &vars).unwrap();
    assert_eq!(auth_header, "Bearer secret-key-12345");
}

/// Integration test: Create variables and use them in template resolution
#[tokio::test]
async fn test_end_to_end_variable_workflow() {
    let db = establish_connection("sqlite::memory:").await.unwrap();
    let db = Arc::new(db);

    let encryption = EncryptionService::new(&[0u8; 32]);
    let service = VariableService::new(db.clone(), encryption);
    let engine = TemplateEngine::new();

    // Create variables
    service.create_variable(CreateVariableRequest {
        name: "API_HOST".to_string(),
        value_type: "text".to_string(),
        value: "https://httpbin.org".to_string(),
        description: Some("Test API host".to_string()),
    }).await.unwrap();

    service.create_variable(CreateVariableRequest {
        name: "API_TOKEN".to_string(),
        value_type: "secret".to_string(),
        value: "my-secret-token".to_string(),
        description: Some("API authentication token".to_string()),
    }).await.unwrap();

    // Load variables
    let vars = service.load_variables_map().await.unwrap();

    // Resolve templates (simulating workflow execution)
    let url = engine.resolve("{{ env.API_HOST }}/get", &vars).unwrap();
    let auth = engine.resolve("Bearer {{ env.API_TOKEN }}", &vars).unwrap();

    assert_eq!(url, "https://httpbin.org/get");
    assert_eq!(auth, "Bearer my-secret-token");
}

/// Test that text variables are stored as plaintext
#[tokio::test]
async fn test_text_variable_stored_plaintext() {
    let db = establish_connection("sqlite::memory:").await.unwrap();
    let db = Arc::new(db);

    let encryption = EncryptionService::new(&[0u8; 32]);
    let service = VariableService::new(db.clone(), encryption);

    service.create_variable(CreateVariableRequest {
        name: "TEXT_VAR".to_string(),
        value_type: "text".to_string(),
        value: "plaintext_value".to_string(),
        description: None,
    }).await.unwrap();

    // Check database directly
    let db_record = environment_variables::Entity::find()
        .filter(environment_variables::Column::Name.eq("TEXT_VAR"))
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();

    // Text values should be stored as-is, not encrypted
    assert_eq!(db_record.value, "plaintext_value");
}

/// Test that secret variables are encrypted in database
#[tokio::test]
async fn test_secret_variable_stored_encrypted() {
    let db = establish_connection("sqlite::memory:").await.unwrap();
    let db = Arc::new(db);

    let encryption = EncryptionService::new(&[0u8; 32]);
    let service = VariableService::new(db.clone(), encryption.clone());

    let secret_value = "super-secret-password";
    service.create_variable(CreateVariableRequest {
        name: "SECRET_VAR".to_string(),
        value_type: "secret".to_string(),
        value: secret_value.to_string(),
        description: None,
    }).await.unwrap();

    // Check database directly
    let db_record = environment_variables::Entity::find()
        .filter(environment_variables::Column::Name.eq("SECRET_VAR"))
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();

    // Value in DB should be encrypted (not plaintext)
    assert_ne!(db_record.value, secret_value);

    // Should be able to decrypt it
    let decrypted = encryption.decrypt(&db_record.value).unwrap();
    assert_eq!(decrypted, secret_value);
}

/// Test get_all_variables returns masked secrets
#[tokio::test]
async fn test_get_all_variables_masks_secrets() {
    let db = establish_connection("sqlite::memory:").await.unwrap();
    let db = Arc::new(db);

    let encryption = EncryptionService::new(&[0u8; 32]);
    let service = VariableService::new(db.clone(), encryption);

    service.create_variable(CreateVariableRequest {
        name: "TEXT_1".to_string(),
        value_type: "text".to_string(),
        value: "visible_value".to_string(),
        description: None,
    }).await.unwrap();

    service.create_variable(CreateVariableRequest {
        name: "SECRET_1".to_string(),
        value_type: "secret".to_string(),
        value: "hidden_value".to_string(),
        description: None,
    }).await.unwrap();

    let all_vars = service.get_all_variables().await.unwrap();

    assert_eq!(all_vars.len(), 2);

    let text_var = all_vars.iter().find(|v| v.name == "TEXT_1").unwrap();
    assert_eq!(text_var.value, "visible_value");

    let secret_var = all_vars.iter().find(|v| v.name == "SECRET_1").unwrap();
    assert_eq!(secret_var.value, "••••••••"); // Should be masked
}
