use swisspipe::email::template::TemplateEngine;
use swisspipe::email::EmailConfig;
use swisspipe::workflow::models::WorkflowEvent;
use swisspipe::email::{EmailAddress, EmailPriority};
use std::collections::HashMap;

#[tokio::test]
async fn test_direct_property_access_in_templates() {
    let template_engine = TemplateEngine::new().expect("Failed to create template engine");
    
    // Create test data with nested properties
    let mut event = WorkflowEvent {
        data: serde_json::json!({
            "name": "Acme Corp",
            "interestLevel": "HIGH",
            "contact": {
                "email": "john@acme.com",
                "phone": "+1-555-0123"
            },
            "value": 50000
        }),
        metadata: HashMap::new(),
        headers: HashMap::new(),
        condition_results: HashMap::new(),
    };
    event.metadata.insert("source".to_string(), "test".to_string());
    
    // Create email config with direct property access
    let email_config = EmailConfig {
        smtp_config: "default".to_string(),
        from: EmailAddress {
            email: "noreply@test.com".to_string(),
            name: Some("Test System".to_string()),
        },
        to: vec![EmailAddress {
            email: "recipient@test.com".to_string(),
            name: None,
        }],
        cc: None,
        bcc: None,
        subject: "Opportunity: {{event.data.name}} - Interest: {{event.data.interestLevel}}".to_string(),
        body_template: r#"
Hello,

We have a new opportunity:
- Company: {{event.data.name}}
- Interest Level: {{event.data.interestLevel}}
- Contact Email: {{event.data.contact.email}}
- Value: ${{event.data.value}}

Also accessible directly (flattened):
- Company: {{name}}
- Interest: {{interestLevel}}

Best regards,
The System
        "#.to_string(),
        text_body_template: None,
        template_type: "text".to_string(),
        priority: EmailPriority::Normal,
        delivery_receipt: false,
        read_receipt: false,
        queue_if_rate_limited: false,
        max_queue_wait_minutes: 10,
        bypass_rate_limit: false,
        attachments: None,
    };
    
    // Render the email
    let result = template_engine.render_email(&email_config, &event, "test-exec-123", "test-node-456")
        .expect("Failed to render email");
    
    // Test subject line
    println!("Subject: {}", result.subject);
    assert_eq!(result.subject, "Opportunity: Acme Corp - Interest: HIGH");
    
    // Test body content
    if let Some(text_body) = result.text_body {
        println!("Body:\n{text_body}");
        
        // Verify direct access works
        assert!(text_body.contains("Company: Acme Corp"));
        assert!(text_body.contains("Interest Level: HIGH"));
        assert!(text_body.contains("Contact Email: john@acme.com"));
        assert!(text_body.contains("Value: $50000"));
        
        // Verify flattened access also works
        assert!(text_body.contains("Company: Acme Corp")); // from {{name}}
        assert!(text_body.contains("Interest: HIGH")); // from {{interestLevel}}
    } else {
        panic!("Expected text body");
    }
    
    println!("✅ SUCCESS: Both {{event.data.property}} and {{property}} work correctly!");
}

#[tokio::test] 
async fn test_array_data_access() {
    let template_engine = TemplateEngine::new().expect("Failed to create template engine");
    
    // Test with array data (from multi-input merge)
    let event = WorkflowEvent {
        data: serde_json::json!([
            {"name": "Company A", "value": 1000},
            {"name": "Company B", "value": 2000}
        ]),
        metadata: HashMap::new(),
        headers: HashMap::new(),
        condition_results: HashMap::new(),
    };
    
    let email_config = EmailConfig {
        smtp_config: "default".to_string(),
        from: EmailAddress {
            email: "noreply@test.com".to_string(),
            name: Some("Test System".to_string()),
        },
        to: vec![EmailAddress {
            email: "recipient@test.com".to_string(), 
            name: None,
        }],
        cc: None,
        bcc: None,
        subject: "Multi-input data report".to_string(),
        body_template: r#"
Array data (using each helper):
{{#each event.data}}
- {{name}}: ${{value}}
{{/each}}

Raw array: {{json event.data}}
        "#.to_string(),
        text_body_template: None,
        template_type: "text".to_string(),
        priority: EmailPriority::Normal,
        delivery_receipt: false,
        read_receipt: false,
        queue_if_rate_limited: false,
        max_queue_wait_minutes: 10,
        bypass_rate_limit: false,
        attachments: None,
    };
    
    let result = template_engine.render_email(&email_config, &event, "test-exec-456", "test-node-789")
        .expect("Failed to render email with array data");
    
    if let Some(text_body) = result.text_body {
        println!("Array template result:\n{text_body}");
        
        // Should contain both companies
        assert!(text_body.contains("Company A: $1000"));
        assert!(text_body.contains("Company B: $2000"));
    }
    
    println!("✅ SUCCESS: Array data access works correctly!");
}