/// Diagnostic test to help troubleshoot Reply-To header issues
/// This test will print detailed information about how the reply_to field flows through the system

use swisspipe::email::template::TemplateEngine;
use swisspipe::email::{EmailConfig, EmailAddress};
use swisspipe::workflow::models::WorkflowEvent;
use swisspipe::email::models::SmtpConfig;
use serde_json::json;
use std::collections::HashMap;

fn create_test_smtp_config() -> SmtpConfig {
    SmtpConfig {
        host: "localhost".to_string(),
        port: 587,
        username: Some("test".to_string()),
        password: Some("test".to_string()),
        from_email: "from@example.com".to_string(),
        from_name: Some("From Name".to_string()),
        security: swisspipe::email::models::SmtpSecurity::Tls,
        timeout_seconds: 30,
        max_retries: 3,
        retry_delay_seconds: 1,
    }
}

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
fn diagnostic_reply_to_flow() {
    println!("\n=== DIAGNOSTIC: Reply-To Flow Test ===\n");

    let engine = TemplateEngine::new().expect("Failed to create template engine");

    // Step 1: Create event data with reply_to information
    let event_data = json!({
        "customer_email": "customer@company.com",
        "customer_name": "Alice Smith"
    });
    println!("Step 1: Event data created");
    println!("  customer_email: customer@company.com");
    println!("  customer_name: Alice Smith\n");

    let event = create_test_event(event_data);

    // Step 2: Create EmailConfig with template variables in reply_to
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
        subject: "Support Request".to_string(),
        template_type: "html".to_string(),
        body_template: "<p>Request from {{customer_name}} ({{customer_email}})</p>".to_string(),
        text_body_template: None,
        attachments: None,
    };
    println!("Step 2: EmailConfig created with reply_to template");
    println!("  reply_to.email (template): {{{{customer_email}}}}");
    println!("  reply_to.name (template): {{{{customer_name}}}}\n");

    let smtp_config = create_test_smtp_config();

    // Step 3: Render the email through the template engine
    println!("Step 3: Rendering email through template engine...");
    let result = engine.render_email(
        &email_config,
        &event,
        "diagnostic-execution-id",
        "diagnostic-node-id",
        &smtp_config,
    );

    assert!(result.is_ok(), "Email rendering should succeed");
    let email_message = result.unwrap();
    println!("  ✓ Email rendering succeeded\n");

    // Step 4: Verify reply_to was rendered correctly
    println!("Step 4: Checking rendered reply_to in EmailMessage");
    assert!(email_message.reply_to.is_some(), "Reply-to should be present");
    let reply_to = email_message.reply_to.as_ref().unwrap();
    println!("  ✓ reply_to is present");
    println!("  reply_to.email (rendered): {}", reply_to.email);
    println!("  reply_to.name (rendered): {:?}\n", reply_to.name);

    assert_eq!(reply_to.email, "customer@company.com", "Email should be rendered");
    assert_eq!(reply_to.name, Some("Alice Smith".to_string()), "Name should be rendered");
    println!("  ✓ Template variables were correctly resolved\n");

    // Step 5: Build the actual SMTP message
    println!("Step 5: Building SMTP message (lettre::Message)");
    let message = build_lettre_message(&email_message);
    assert!(message.is_ok(), "Building lettre message should succeed");
    let message = message.unwrap();
    println!("  ✓ SMTP message built successfully\n");

    // Step 6: Inspect the message headers
    println!("Step 6: Inspecting message headers");
    let message_debug = format!("{:?}", message);

    // Extract and display relevant headers
    if message_debug.contains("Reply-To") {
        println!("  ✓ Reply-To header IS PRESENT in the message");

        // Try to extract the Reply-To value
        if let Some(start) = message_debug.find("Reply-To") {
            if let Some(end) = message_debug[start..].find('}') {
                let reply_to_section = &message_debug[start..start + end + 1];
                println!("  Header details: {}", reply_to_section);
            }
        }
    } else {
        println!("  ✗ Reply-To header NOT FOUND in the message");
        println!("  This indicates a problem!");
    }
    println!();

    // Step 7: Check if the actual email address is in the message
    println!("Step 7: Verifying customer email is in the message");
    if message_debug.contains("customer@company.com") {
        println!("  ✓ Customer email 'customer@company.com' found in message");
    } else {
        println!("  ✗ Customer email NOT found in message (unexpected!)");
    }
    println!();

    // Final assertion
    assert!(message_debug.contains("Reply-To"), "Reply-To header must be present");
    assert!(message_debug.contains("customer@company.com"), "Customer email must be in message");

    println!("=== DIAGNOSTIC COMPLETE: All checks passed ===\n");
}

/// Build a lettre Message from an EmailMessage (same as in email service)
fn build_lettre_message(email_message: &swisspipe::email::EmailMessage) -> Result<lettre::Message, String> {
    use lettre::{Message, message::{header::ContentType, Mailbox, MultiPart, SinglePart}, Address};

    let from_address: Address = email_message.from.email.parse()
        .map_err(|e| format!("Invalid from address: {}", e))?;
    let from_mailbox = if let Some(ref name) = email_message.from.name {
        Mailbox::new(Some(name.clone()), from_address)
    } else {
        Mailbox::new(None, from_address)
    };

    let mut builder = Message::builder().from(from_mailbox);

    // Add Reply-To header if specified (THIS IS THE CRITICAL PART)
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
fn diagnostic_check_serialization() {
    println!("\n=== DIAGNOSTIC: Check JSON Serialization ===\n");

    // This test checks if reply_to survives JSON serialization (important for API calls)
    let email_config = EmailConfig {
        to: vec![EmailAddress {
            email: "to@example.com".to_string(),
            name: None,
        }],
        cc: None,
        bcc: None,
        reply_to: Some(EmailAddress {
            email: "reply@example.com".to_string(),
            name: Some("Reply Name".to_string()),
        }),
        subject: "Test".to_string(),
        template_type: "html".to_string(),
        body_template: "<p>Test</p>".to_string(),
        text_body_template: None,
        attachments: None,
    };

    println!("Step 1: Created EmailConfig with reply_to");
    println!("  reply_to.email: reply@example.com");
    println!("  reply_to.name: Reply Name\n");

    // Serialize to JSON (simulating API request)
    let json_string = serde_json::to_string_pretty(&email_config)
        .expect("Should serialize");
    println!("Step 2: Serialized to JSON:");
    println!("{}\n", json_string);

    // Check if reply_to is in the JSON
    assert!(json_string.contains("reply_to"), "reply_to should be in JSON");
    assert!(json_string.contains("reply@example.com"), "reply_to email should be in JSON");
    println!("  ✓ reply_to field is present in serialized JSON\n");

    // Deserialize back (simulating API receiving the request)
    let deserialized: EmailConfig = serde_json::from_str(&json_string)
        .expect("Should deserialize");
    println!("Step 3: Deserialized from JSON");

    assert!(deserialized.reply_to.is_some(), "reply_to should be present after deserialization");
    let reply_to = deserialized.reply_to.as_ref().unwrap();
    println!("  ✓ reply_to survived deserialization");
    println!("  reply_to.email: {}", reply_to.email);
    println!("  reply_to.name: {:?}\n", reply_to.name);

    assert_eq!(reply_to.email, "reply@example.com");
    assert_eq!(reply_to.name, Some("Reply Name".to_string()));

    println!("=== DIAGNOSTIC COMPLETE: Serialization works correctly ===\n");
}
