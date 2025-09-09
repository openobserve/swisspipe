use crate::email::{EmailError, EmailMessage, EmailAddress, EmailAttachment};
use crate::workflow::models::WorkflowEvent;
use handlebars::{Handlebars, Helper, HelperResult, Output, RenderContext, RenderError};
use serde_json::Value;

pub struct TemplateEngine {
    handlebars: Handlebars<'static>,
}

impl TemplateEngine {
    pub fn new() -> Result<Self, EmailError> {
        let mut handlebars = Handlebars::new();
        
        // Register helper functions
        handlebars.register_helper("json", Box::new(json_helper));
        handlebars.register_helper("date_format", Box::new(date_format_helper));
        handlebars.register_helper("escape_html", Box::new(escape_html_helper));
        handlebars.register_helper("upper", Box::new(upper_helper));
        handlebars.register_helper("lower", Box::new(lower_helper));
        
        // Configure handlebars for security
        handlebars.set_strict_mode(true);
        
        Ok(Self { handlebars })
    }
    
    pub fn render_email(
        &self,
        email_config: &crate::email::EmailConfig,
        workflow_event: &WorkflowEvent,
        execution_id: &str,
        node_id: &str,
    ) -> Result<EmailMessage, EmailError> {
        // Create template context
        let context = self.create_template_context(workflow_event, execution_id, node_id)?;
        
        // Render subject
        let subject = self.handlebars
            .render_template(&email_config.subject, &context)
            .map_err(|e| EmailError::template(format!("Failed to render subject: {e}")))?;
        
        // Render body templates
        let html_body = if email_config.template_type == "html" {
            Some(self.handlebars
                .render_template(&email_config.body_template, &context)
                .map_err(|e| EmailError::template(format!("Failed to render HTML body: {e}")))?)
        } else {
            None
        };
        
        let text_body = if email_config.template_type == "text" {
            Some(self.handlebars
                .render_template(&email_config.body_template, &context)
                .map_err(|e| EmailError::template(format!("Failed to render text body: {e}")))?)
        } else if let Some(ref text_template) = email_config.text_body_template {
            Some(self.handlebars
                .render_template(text_template, &context)
                .map_err(|e| EmailError::template(format!("Failed to render text body: {e}")))?)
        } else {
            None
        };
        
        // Render recipients
        let from = self.render_email_address(&email_config.from, &context)?;
        let to = self.render_email_addresses(&email_config.to, &context)?;
        let cc = if let Some(ref cc_addrs) = email_config.cc {
            self.render_email_addresses(cc_addrs, &context)?
        } else {
            vec![]
        };
        let bcc = if let Some(ref bcc_addrs) = email_config.bcc {
            self.render_email_addresses(bcc_addrs, &context)?
        } else {
            vec![]
        };
        
        // Process attachments
        let attachments = if let Some(ref attachment_configs) = email_config.attachments {
            self.render_attachments(attachment_configs, &context)?
        } else {
            vec![]
        };
        
        Ok(EmailMessage {
            from,
            to,
            cc,
            bcc,
            subject,
            html_body,
            text_body,
            attachments,
            priority: email_config.priority.clone(),
            delivery_receipt: email_config.delivery_receipt,
            read_receipt: email_config.read_receipt,
        })
    }
    
    fn create_template_context(
        &self,
        workflow_event: &WorkflowEvent,
        execution_id: &str,
        node_id: &str,
    ) -> Result<Value, EmailError> {
        let mut context = serde_json::Map::new();
        
        // Add event data (consistent with condition/transformer nodes)
        context.insert("event".to_string(), serde_json::json!({
            "data": workflow_event.data,
            "metadata": workflow_event.metadata,
            "headers": workflow_event.headers,
            "condition_results": workflow_event.condition_results,
        }));
        
        // Add workflow data (legacy support)
        context.insert("workflow".to_string(), serde_json::json!({
            "id": execution_id,
            "data": workflow_event.data,
            "metadata": workflow_event.metadata,
            "headers": workflow_event.headers,
            "condition_results": workflow_event.condition_results,
        }));
        
        // Add node data
        context.insert("node".to_string(), serde_json::json!({
            "id": node_id,
        }));
        
        // Add system data
        context.insert("system".to_string(), serde_json::json!({
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "hostname": std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown".to_string()),
        }));
        
        Ok(Value::Object(context))
    }
    
    fn render_email_address(
        &self,
        addr_config: &EmailAddress,
        context: &Value,
    ) -> Result<EmailAddress, EmailError> {
        let email = self.handlebars
            .render_template(&addr_config.email, context)
            .map_err(|e| EmailError::template(format!("Failed to render email address: {e}")))?;
        
        let name = if let Some(ref name_template) = addr_config.name {
            Some(self.handlebars
                .render_template(name_template, context)
                .map_err(|e| EmailError::template(format!("Failed to render email name: {e}")))?)
        } else {
            None
        };
        
        // Validate email
        if !validator::validate_email(&email) {
            return Err(EmailError::validation(format!("Invalid email address: {email}")));
        }
        
        Ok(EmailAddress { email, name })
    }
    
    fn render_email_addresses(
        &self,
        addr_configs: &[EmailAddress],
        context: &Value,
    ) -> Result<Vec<EmailAddress>, EmailError> {
        let mut addresses = Vec::new();
        
        for addr_config in addr_configs {
            addresses.push(self.render_email_address(addr_config, context)?);
        }
        
        Ok(addresses)
    }
    
    fn render_attachments(
        &self,
        attachment_configs: &[EmailAttachment],
        _context: &Value,
    ) -> Result<Vec<EmailAttachment>, EmailError> {
        let mut attachments = Vec::new();
        
        for attachment_config in attachment_configs {
            // For now, we'll assume attachment data is already provided
            // In a more complete implementation, we might render template data here
            attachments.push(attachment_config.clone());
        }
        
        Ok(attachments)
    }
}

// Handlebars helper functions

fn json_helper(
    h: &Helper,
    _: &Handlebars,
    _: &handlebars::Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let value = h.param(0)
        .ok_or_else(|| RenderError::new("json helper requires a parameter"))?;
    
    let json_str = serde_json::to_string_pretty(value.value())
        .map_err(|e| RenderError::new(format!("Failed to serialize to JSON: {e}")))?;
    
    out.write(&json_str)?;
    Ok(())
}

fn date_format_helper(
    h: &Helper,
    _: &Handlebars,
    _: &handlebars::Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let date_str = h.param(0)
        .ok_or_else(|| RenderError::new("date_format helper requires a date parameter"))?
        .value()
        .as_str()
        .ok_or_else(|| RenderError::new("date_format helper requires a string parameter"))?;
    
    let format = h.param(1)
        .and_then(|v| v.value().as_str())
        .unwrap_or("%Y-%m-%d %H:%M:%S");
    
    // Try to parse the date and format it
    if let Ok(datetime) = chrono::DateTime::parse_from_rfc3339(date_str) {
        let formatted = datetime.format(format).to_string();
        out.write(&formatted)?;
    } else if let Ok(datetime) = chrono::DateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S") {
        let formatted = datetime.format(format).to_string();
        out.write(&formatted)?;
    } else {
        // If parsing fails, just output the original string
        out.write(date_str)?;
    }
    
    Ok(())
}

fn escape_html_helper(
    h: &Helper,
    _: &Handlebars,
    _: &handlebars::Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let text = h.param(0)
        .ok_or_else(|| RenderError::new("escape_html helper requires a parameter"))?
        .value()
        .as_str()
        .ok_or_else(|| RenderError::new("escape_html helper requires a string parameter"))?;
    
    let escaped = html_escape::encode_text(text);
    out.write(&escaped)?;
    Ok(())
}

fn upper_helper(
    h: &Helper,
    _: &Handlebars,
    _: &handlebars::Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let text = h.param(0)
        .ok_or_else(|| RenderError::new("upper helper requires a parameter"))?
        .value()
        .as_str()
        .ok_or_else(|| RenderError::new("upper helper requires a string parameter"))?;
    
    out.write(&text.to_uppercase())?;
    Ok(())
}

fn lower_helper(
    h: &Helper,
    _: &Handlebars,
    _: &handlebars::Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let text = h.param(0)
        .ok_or_else(|| RenderError::new("lower helper requires a parameter"))?
        .value()
        .as_str()
        .ok_or_else(|| RenderError::new("lower helper requires a string parameter"))?;
    
    out.write(&text.to_lowercase())?;
    Ok(())
}