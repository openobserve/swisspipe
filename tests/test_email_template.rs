use swisspipe::email::template::TemplateEngine;
use swisspipe::email::{EmailConfig, EmailAddress};
use swisspipe::workflow::models::WorkflowEvent;
use swisspipe::email::models::SmtpConfig;
use serde_json::json;
use std::collections::HashMap;

/// Helper to create a basic SMTP config for testing
fn create_test_smtp_config() -> SmtpConfig {
    SmtpConfig {
        host: "localhost".to_string(),
        port: 587,
        username: Some("test".to_string()),
        password: Some("test".to_string()),
        from_email: "test@example.com".to_string(),
        from_name: Some("Test Sender".to_string()),
        security: swisspipe::email::models::SmtpSecurity::Tls,
        timeout_seconds: 30,
        max_retries: 3,
        retry_delay_seconds: 1,
    }
}

/// Helper to create a basic workflow event for testing
fn create_test_event(data: serde_json::Value) -> WorkflowEvent {
    WorkflowEvent {
        data,
        metadata: HashMap::new(),
        headers: HashMap::new(),
        condition_results: HashMap::new(),
        hil_task: None,
        sources: Vec::new(),
    }
}

#[test]
fn test_json_helper_in_html_template() {
    let engine = TemplateEngine::new().expect("Failed to create template engine");

    let event_data = json!({
        "name": "John Doe",
        "age": 30,
        "email": "john@example.com"
    });

    let event = create_test_event(event_data);

    let email_config = EmailConfig {
        to: vec![EmailAddress {
            email: "recipient@example.com".to_string(),
            name: None,
        }],
        cc: None,
        bcc: None,
        subject: "Test Subject".to_string(),
        template_type: "html".to_string(),
        body_template: r#"<!DOCTYPE html><html>
<body>
<h1>User Data</h1>
<pre>{{json event.data}}</pre>
</body>
</html>"#.to_string(),
        text_body_template: None,
        attachments: None,
    };

    let smtp_config = create_test_smtp_config();

    let result = engine.render_email(
        &email_config,
        &event,
        "test-execution-id",
        "test-node-id",
        &smtp_config,
    );

    assert!(result.is_ok(), "Email rendering should succeed");
    let email = result.unwrap();
    assert!(email.html_body.is_some());

    let html_body = email.html_body.unwrap();
    assert!(html_body.contains("John Doe"), "HTML should contain the name");
    assert!(html_body.contains("john@example.com"), "HTML should contain the email");
    assert!(html_body.contains("30"), "HTML should contain the age");
}

#[test]
fn test_json_helper_with_nested_data() {
    let engine = TemplateEngine::new().expect("Failed to create template engine");

    let event_data = json!({
        "user": {
            "profile": {
                "name": "Jane Smith",
                "location": "New York"
            },
            "preferences": {
                "theme": "dark",
                "notifications": true
            }
        }
    });

    let event = create_test_event(event_data);

    let email_config = EmailConfig {
        to: vec![EmailAddress {
            email: "recipient@example.com".to_string(),
            name: None,
        }],
        cc: None,
        bcc: None,
        subject: "Nested Data Test".to_string(),
        template_type: "html".to_string(),
        body_template: r#"<!DOCTYPE html><html>
<body>
<h1>Full Data</h1>
<pre>{{json event.data}}</pre>
<h2>User Profile</h2>
<pre>{{json event.data.user.profile}}</pre>
</body>
</html>"#.to_string(),
        text_body_template: None,
        attachments: None,
    };

    let smtp_config = create_test_smtp_config();

    let result = engine.render_email(
        &email_config,
        &event,
        "test-execution-id",
        "test-node-id",
        &smtp_config,
    );

    assert!(result.is_ok(), "Email rendering with nested data should succeed");
    let email = result.unwrap();
    let html_body = email.html_body.unwrap();

    assert!(html_body.contains("Jane Smith"), "HTML should contain nested name");
    assert!(html_body.contains("New York"), "HTML should contain nested location");
    assert!(html_body.contains("dark"), "HTML should contain preferences");
}

#[test]
fn test_json_helper_with_array_data() {
    let engine = TemplateEngine::new().expect("Failed to create template engine");

    let event_data = json!({
        "items": [
            {"id": 1, "name": "Item 1"},
            {"id": 2, "name": "Item 2"},
            {"id": 3, "name": "Item 3"}
        ]
    });

    let event = create_test_event(event_data);

    let email_config = EmailConfig {
        to: vec![EmailAddress {
            email: "recipient@example.com".to_string(),
            name: None,
        }],
        cc: None,
        bcc: None,
        subject: "Array Data Test".to_string(),
        template_type: "html".to_string(),
        body_template: r#"<!DOCTYPE html><html>
<body>
<h1>Items List</h1>
<pre>{{json event.data.items}}</pre>
</body>
</html>"#.to_string(),
        text_body_template: None,
        attachments: None,
    };

    let smtp_config = create_test_smtp_config();

    let result = engine.render_email(
        &email_config,
        &event,
        "test-execution-id",
        "test-node-id",
        &smtp_config,
    );

    assert!(result.is_ok(), "Email rendering with array data should succeed");
    let email = result.unwrap();
    let html_body = email.html_body.unwrap();

    assert!(html_body.contains("Item 1"), "HTML should contain first item");
    assert!(html_body.contains("Item 2"), "HTML should contain second item");
    assert!(html_body.contains("Item 3"), "HTML should contain third item");
}

#[test]
fn test_json_helper_in_text_template() {
    let engine = TemplateEngine::new().expect("Failed to create template engine");

    let event_data = json!({
        "message": "Hello World",
        "status": "success"
    });

    let event = create_test_event(event_data);

    let email_config = EmailConfig {
        to: vec![EmailAddress {
            email: "recipient@example.com".to_string(),
            name: None,
        }],
        cc: None,
        bcc: None,
        subject: "Text Template Test".to_string(),
        template_type: "text".to_string(),
        body_template: "Event Data:\n{{json event.data}}".to_string(),
        text_body_template: None,
        attachments: None,
    };

    let smtp_config = create_test_smtp_config();

    let result = engine.render_email(
        &email_config,
        &event,
        "test-execution-id",
        "test-node-id",
        &smtp_config,
    );

    assert!(result.is_ok(), "Email rendering with text template should succeed");
    let email = result.unwrap();
    assert!(email.text_body.is_some());

    let text_body = email.text_body.unwrap();
    assert!(text_body.contains("Hello World"), "Text should contain message");
    assert!(text_body.contains("success"), "Text should contain status");
}

#[test]
fn test_json_helper_with_metadata() {
    let engine = TemplateEngine::new().expect("Failed to create template engine");

    let event_data = json!({
        "user_id": "123"
    });

    let mut event = create_test_event(event_data);
    event.metadata.insert("source".to_string(), "api".to_string());
    event.metadata.insert("version".to_string(), "1.0".to_string());

    let email_config = EmailConfig {
        to: vec![EmailAddress {
            email: "recipient@example.com".to_string(),
            name: None,
        }],
        cc: None,
        bcc: None,
        subject: "Metadata Test".to_string(),
        template_type: "html".to_string(),
        body_template: r#"<!DOCTYPE html><html>
<body>
<h1>Event Metadata</h1>
<pre>{{json event.metadata}}</pre>
<h2>Event Data</h2>
<pre>{{json event.data}}</pre>
</body>
</html>"#.to_string(),
        text_body_template: None,
        attachments: None,
    };

    let smtp_config = create_test_smtp_config();

    let result = engine.render_email(
        &email_config,
        &event,
        "test-execution-id",
        "test-node-id",
        &smtp_config,
    );

    assert!(result.is_ok(), "Email rendering with metadata should succeed");
    let email = result.unwrap();
    let html_body = email.html_body.unwrap();

    assert!(html_body.contains("api"), "HTML should contain metadata source");
    assert!(html_body.contains("1.0"), "HTML should contain metadata version");
}

#[test]
fn test_json_helper_with_special_characters() {
    let engine = TemplateEngine::new().expect("Failed to create template engine");

    let event_data = json!({
        "message": "Test with \"quotes\" and <html> tags",
        "path": "/api/v1/test?param=value&other=123"
    });

    let event = create_test_event(event_data);

    let email_config = EmailConfig {
        to: vec![EmailAddress {
            email: "recipient@example.com".to_string(),
            name: None,
        }],
        cc: None,
        bcc: None,
        subject: "Special Characters Test".to_string(),
        template_type: "html".to_string(),
        body_template: r#"<!DOCTYPE html><html>
<body>
<h1>Data with Special Characters</h1>
<pre>{{json event.data}}</pre>
</body>
</html>"#.to_string(),
        text_body_template: None,
        attachments: None,
    };

    let smtp_config = create_test_smtp_config();

    let result = engine.render_email(
        &email_config,
        &event,
        "test-execution-id",
        "test-node-id",
        &smtp_config,
    );

    assert!(result.is_ok(), "Email rendering with special characters should succeed");
    let email = result.unwrap();
    let html_body = email.html_body.unwrap();

    // JSON should properly escape special characters
    assert!(html_body.contains("\\\"quotes\\\"") || html_body.contains("\"quotes\""),
            "HTML should contain escaped quotes");
}

#[test]
fn test_combined_json_and_regular_variables() {
    let engine = TemplateEngine::new().expect("Failed to create template engine");

    let event_data = json!({
        "name": "Alice",
        "details": {
            "age": 25,
            "city": "Boston"
        }
    });

    let event = create_test_event(event_data);

    let email_config = EmailConfig {
        to: vec![EmailAddress {
            email: "recipient@example.com".to_string(),
            name: None,
        }],
        cc: None,
        bcc: None,
        subject: "Combined Variables Test".to_string(),
        template_type: "html".to_string(),
        body_template: r#"<!DOCTYPE html><html>
<body>
<h1>Hello {{name}}!</h1>
<p>Your age is: {{details.age}}</p>
<h2>Full Details (JSON)</h2>
<pre>{{json event.data.details}}</pre>
</body>
</html>"#.to_string(),
        text_body_template: None,
        attachments: None,
    };

    let smtp_config = create_test_smtp_config();

    let result = engine.render_email(
        &email_config,
        &event,
        "test-execution-id",
        "test-node-id",
        &smtp_config,
    );

    assert!(result.is_ok(), "Email rendering with combined variables should succeed");
    let email = result.unwrap();
    let html_body = email.html_body.unwrap();

    assert!(html_body.contains("Hello Alice!"), "HTML should use flattened variable");
    assert!(html_body.contains("Your age is: 25"), "HTML should use nested variable");
    assert!(html_body.contains("Boston"), "HTML should contain JSON output");
}

#[test]
fn test_json_helper_empty_data() {
    let engine = TemplateEngine::new().expect("Failed to create template engine");

    let event_data = json!({});
    let event = create_test_event(event_data);

    let email_config = EmailConfig {
        to: vec![EmailAddress {
            email: "recipient@example.com".to_string(),
            name: None,
        }],
        cc: None,
        bcc: None,
        subject: "Empty Data Test".to_string(),
        template_type: "html".to_string(),
        body_template: r#"<!DOCTYPE html><html>
<body>
<h1>Empty Event Data</h1>
<pre>{{json event.data}}</pre>
</body>
</html>"#.to_string(),
        text_body_template: None,
        attachments: None,
    };

    let smtp_config = create_test_smtp_config();

    let result = engine.render_email(
        &email_config,
        &event,
        "test-execution-id",
        "test-node-id",
        &smtp_config,
    );

    assert!(result.is_ok(), "Email rendering with empty data should succeed");
    let email = result.unwrap();
    let html_body = email.html_body.unwrap();

    // Empty JSON object should be rendered as {}
    assert!(html_body.contains("{}"), "HTML should contain empty JSON object");
}
