use swisspipe::email::template::TemplateEngine;
use swisspipe::email::{EmailConfig, EmailAddress, EmailMessage};
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
fn test_reply_to_header_is_set_with_static_email() {
    let engine = TemplateEngine::new().expect("Failed to create template engine");

    let event_data = json!({
        "message": "Test message"
    });

    let event = create_test_event(event_data);

    let email_config = EmailConfig {
        to: vec![EmailAddress {
            email: "recipient@example.com".to_string(),
            name: Some("Recipient Name".to_string()),
        }],
        cc: None,
        bcc: None,
        reply_to: Some(EmailAddress {
            email: "replyto@example.com".to_string(),
            name: Some("Reply To Name".to_string()),
        }),
        subject: "Test Subject".to_string(),
        template_type: "html".to_string(),
        body_template: "<p>{{message}}</p>".to_string(),
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
    let email_message = result.unwrap();

    // Verify reply_to is present in the EmailMessage
    assert!(email_message.reply_to.is_some(), "Reply-to should be present in EmailMessage");
    let reply_to = email_message.reply_to.as_ref().unwrap();
    assert_eq!(reply_to.email, "replyto@example.com", "Reply-to email should match");
    assert_eq!(reply_to.name, Some("Reply To Name".to_string()), "Reply-to name should match");

    // Now build the actual lettre message to verify the header
    let message = build_test_message(&email_message);
    assert!(message.is_ok(), "Building lettre message should succeed");

    // Get the message as string to check headers
    let message = message.unwrap();
    let message_string = format!("{:?}", message);

    // Verify Reply-To header is present in the message
    assert!(message_string.contains("Reply-To") || message_string.contains("reply-to"),
            "Reply-To header should be present in the message. Message debug: {}", message_string);
}

#[test]
fn test_reply_to_header_with_template_variables() {
    let engine = TemplateEngine::new().expect("Failed to create template engine");

    let event_data = json!({
        "customer_email": "customer@example.com",
        "customer_name": "John Customer",
        "ticket_id": "12345"
    });

    let event = create_test_event(event_data);

    let email_config = EmailConfig {
        to: vec![EmailAddress {
            email: "support@example.com".to_string(),
            name: Some("Support Team".to_string()),
        }],
        cc: None,
        bcc: None,
        reply_to: Some(EmailAddress {
            email: "{{customer_email}}".to_string(),
            name: Some("{{customer_name}}".to_string()),
        }),
        subject: "Support Ticket {{ticket_id}}".to_string(),
        template_type: "html".to_string(),
        body_template: "<p>Ticket: {{ticket_id}}</p>".to_string(),
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

    assert!(result.is_ok(), "Email rendering with template variables should succeed");
    let email_message = result.unwrap();

    // Verify reply_to was rendered correctly with template variables
    assert!(email_message.reply_to.is_some(), "Reply-to should be present in EmailMessage");
    let reply_to = email_message.reply_to.as_ref().unwrap();
    assert_eq!(reply_to.email, "customer@example.com", "Reply-to email should be rendered from template");
    assert_eq!(reply_to.name, Some("John Customer".to_string()), "Reply-to name should be rendered from template");

    // Build the lettre message
    let message = build_test_message(&email_message);
    assert!(message.is_ok(), "Building lettre message should succeed");

    let message = message.unwrap();
    let message_string = format!("{:?}", message);

    // Verify Reply-To header contains the rendered email
    assert!(message_string.contains("customer@example.com"),
            "Reply-To header should contain rendered customer email. Message: {}", message_string);
}

#[test]
fn test_reply_to_header_with_event_data_nested() {
    let engine = TemplateEngine::new().expect("Failed to create template engine");

    let event_data = json!({
        "user": {
            "email": "user@example.com",
            "name": "Jane Doe"
        }
    });

    let event = create_test_event(event_data);

    let email_config = EmailConfig {
        to: vec![EmailAddress {
            email: "admin@example.com".to_string(),
            name: None,
        }],
        cc: None,
        bcc: None,
        reply_to: Some(EmailAddress {
            email: "{{event.data.user.email}}".to_string(),
            name: Some("{{event.data.user.name}}".to_string()),
        }),
        subject: "User Request".to_string(),
        template_type: "html".to_string(),
        body_template: "<p>Request from {{event.data.user.name}}</p>".to_string(),
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

    assert!(result.is_ok(), "Email rendering with nested event data should succeed");
    let email_message = result.unwrap();

    // Verify nested template variables were resolved
    assert!(email_message.reply_to.is_some(), "Reply-to should be present");
    let reply_to = email_message.reply_to.as_ref().unwrap();
    assert_eq!(reply_to.email, "user@example.com", "Nested email template should be resolved");
    assert_eq!(reply_to.name, Some("Jane Doe".to_string()), "Nested name template should be resolved");

    // Build and verify the message
    let message = build_test_message(&email_message);
    assert!(message.is_ok(), "Building lettre message should succeed");
}

#[test]
fn test_no_reply_to_header_when_not_specified() {
    let engine = TemplateEngine::new().expect("Failed to create template engine");

    let event_data = json!({
        "message": "Test"
    });

    let event = create_test_event(event_data);

    let email_config = EmailConfig {
        to: vec![EmailAddress {
            email: "recipient@example.com".to_string(),
            name: None,
        }],
        cc: None,
        bcc: None,
        reply_to: None, // No reply-to specified
        subject: "Test".to_string(),
        template_type: "html".to_string(),
        body_template: "<p>Test</p>".to_string(),
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

    assert!(result.is_ok(), "Email rendering without reply-to should succeed");
    let email_message = result.unwrap();

    // Verify reply_to is None
    assert!(email_message.reply_to.is_none(), "Reply-to should not be present when not specified");

    // Build the message and verify it still works
    let message = build_test_message(&email_message);
    assert!(message.is_ok(), "Building message without reply-to should succeed");
}

/// Helper function to build a lettre Message from an EmailMessage
/// This simulates what EmailService::build_lettre_message does
fn build_test_message(email_message: &EmailMessage) -> Result<lettre::Message, String> {
    use lettre::{Message, message::{header::ContentType, Mailbox, MultiPart, SinglePart}, Address};

    // Build from mailbox
    let from_address: Address = email_message.from.email.parse()
        .map_err(|e| format!("Invalid from address: {}", e))?;
    let from_mailbox = if let Some(ref name) = email_message.from.name {
        Mailbox::new(Some(name.clone()), from_address)
    } else {
        Mailbox::new(None, from_address)
    };

    let mut builder = Message::builder().from(from_mailbox);

    // Add Reply-To header if specified
    if let Some(ref reply_to_addr) = email_message.reply_to {
        let reply_to_address: Address = reply_to_addr.email.parse()
            .map_err(|e| format!("Invalid reply-to address: {}", e))?;
        let reply_to_mailbox = if let Some(ref name) = reply_to_addr.name {
            Mailbox::new(Some(name.clone()), reply_to_address)
        } else {
            Mailbox::new(None, reply_to_address)
        };
        builder = builder.reply_to(reply_to_mailbox);
    }

    // Add recipients
    for to_addr in &email_message.to {
        let address: Address = to_addr.email.parse()
            .map_err(|e| format!("Invalid to address: {}", e))?;
        let mailbox = if let Some(ref name) = to_addr.name {
            Mailbox::new(Some(name.clone()), address)
        } else {
            Mailbox::new(None, address)
        };
        builder = builder.to(mailbox);
    }

    builder = builder.subject(&email_message.subject);

    // Build message body
    let body = if let Some(ref html_body) = email_message.html_body {
        MultiPart::alternative()
            .singlepart(
                SinglePart::builder()
                    .header(ContentType::TEXT_HTML)
                    .body(html_body.clone())
            )
    } else if let Some(ref text_body) = email_message.text_body {
        MultiPart::alternative()
            .singlepart(
                SinglePart::builder()
                    .header(ContentType::TEXT_PLAIN)
                    .body(text_body.clone())
            )
    } else {
        return Err("Email must have either HTML or text body".to_string());
    };

    let message = builder.multipart(body)
        .map_err(|e| format!("Failed to build message: {}", e))?;

    Ok(message)
}

#[test]
fn test_reply_to_header_verification_with_raw_message() {
    let engine = TemplateEngine::new().expect("Failed to create template engine");

    let event_data = json!({
        "reply_email": "dynamic-reply@example.com"
    });

    let event = create_test_event(event_data);

    let email_config = EmailConfig {
        to: vec![EmailAddress {
            email: "to@example.com".to_string(),
            name: None,
        }],
        cc: None,
        bcc: None,
        reply_to: Some(EmailAddress {
            email: "{{reply_email}}".to_string(),
            name: None,
        }),
        subject: "Test".to_string(),
        template_type: "text".to_string(),
        body_template: "Test body".to_string(),
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
    let email_message = result.unwrap();

    // Verify the EmailMessage has the correct reply_to
    let reply_to = email_message.reply_to.as_ref()
        .expect("Reply-to should be present");
    assert_eq!(reply_to.email, "dynamic-reply@example.com",
               "Reply-to should have resolved template variable");

    // Build the actual SMTP message
    let message = build_test_message(&email_message)
        .expect("Should build message successfully");

    // Convert to bytes to inspect raw headers
    let message_bytes = format!("{:?}", message);

    // Verify the Reply-To header is actually in the message
    println!("Message debug output:\n{}", message_bytes);

    assert!(
        message_bytes.contains("reply-to") ||
        message_bytes.contains("Reply-To") ||
        message_bytes.contains("dynamic-reply@example.com"),
        "Reply-To header with dynamic email should be present in the message"
    );
}
