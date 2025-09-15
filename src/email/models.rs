use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SmtpSecurity {
    None,
    Tls,
    Ssl,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub security: SmtpSecurity,
    pub username: Option<String>,
    pub password: Option<String>,
    pub from_email: String,
    pub from_name: Option<String>,
    pub timeout_seconds: u64,
    pub max_retries: u32,
    pub retry_delay_seconds: u64,
}

impl Default for SmtpConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 587,
            security: SmtpSecurity::Tls,
            username: None,
            password: None,
            from_email: "noreply@localhost".to_string(),
            from_name: Some("SwissPipe".to_string()),
            timeout_seconds: 30,
            max_retries: 3,
            retry_delay_seconds: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct EmailAddress {
    #[validate(email)]
    pub email: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailAttachment {
    pub filename: String,
    pub content_type: String,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmailPriority {
    Critical,
    High,
    Normal,
    Low,
}

impl Default for EmailPriority {
    fn default() -> Self {
        Self::Normal
    }
}

impl EmailPriority {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Critical => "critical",
            Self::High => "high",
            Self::Normal => "normal",
            Self::Low => "low",
        }
    }

    pub fn from_priority_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "critical" => Some(Self::Critical),
            "high" => Some(Self::High),
            "normal" => Some(Self::Normal),
            "low" => Some(Self::Low),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct EmailConfig {
    pub smtp_config: String,
    #[validate]
    pub from: EmailAddress,
    #[validate]
    pub to: Vec<EmailAddress>,
    #[validate]
    pub cc: Option<Vec<EmailAddress>>,
    #[validate]
    pub bcc: Option<Vec<EmailAddress>>,
    pub subject: String,
    pub template_type: String, // "html" or "text"
    pub body_template: String,
    pub text_body_template: Option<String>,
    pub attachments: Option<Vec<EmailAttachment>>,
    pub priority: EmailPriority,
    pub delivery_receipt: bool,
    pub read_receipt: bool,
    pub queue_if_rate_limited: bool,
    pub max_queue_wait_minutes: u32,
    pub bypass_rate_limit: bool,
}

impl Default for EmailConfig {
    fn default() -> Self {
        Self {
            smtp_config: "default".to_string(),
            from: EmailAddress {
                email: "noreply@localhost".to_string(),
                name: Some("SwissPipe".to_string()),
            },
            to: vec![],
            cc: None,
            bcc: None,
            subject: "SwissPipe Workflow Notification".to_string(),
            template_type: "html".to_string(),
            body_template: "<p>Workflow completed successfully.</p>".to_string(),
            text_body_template: None,
            attachments: None,
            priority: EmailPriority::Normal,
            delivery_receipt: false,
            read_receipt: false,
            queue_if_rate_limited: true,
            max_queue_wait_minutes: 60,
            bypass_rate_limit: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailMessage {
    pub from: EmailAddress,
    pub to: Vec<EmailAddress>,
    pub cc: Vec<EmailAddress>,
    pub bcc: Vec<EmailAddress>,
    pub subject: String,
    pub html_body: Option<String>,
    pub text_body: Option<String>,
    pub attachments: Vec<EmailAttachment>,
    pub priority: EmailPriority,
    pub delivery_receipt: bool,
    pub read_receipt: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct QueuedEmail {
    pub id: String,
    pub execution_id: Option<String>,
    pub node_id: Option<String>,
    pub smtp_config: String,
    pub priority: EmailPriority,
    pub email_config: EmailConfig,
    pub template_context: serde_json::Value,
    pub status: String,
    pub queued_at: i64,
    pub scheduled_at: Option<i64>,
    pub processed_at: Option<i64>,
    pub sent_at: Option<i64>,
    pub max_wait_minutes: u32,
    pub retry_count: u32,
    pub max_retries: u32,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailSendResult {
    pub success: bool,
    pub message_id: Option<String>,
    pub error: Option<String>,
    pub partial_success: Option<PartialSuccessInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialSuccessInfo {
    pub successful_recipients: Vec<String>,
    pub failed_recipients: Vec<FailedRecipient>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedRecipient {
    pub email: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailQueueStats {
    pub queue_size: u32,
    pub rate_limit_per_minute: u32,
    pub tokens_available: u32,
    pub next_refill_seconds: u32,
    pub priority_breakdown: HashMap<String, u32>,
    pub average_wait_minutes: f64,
    pub emails_sent_last_minute: u32,
    pub emails_queued_last_minute: u32,
    pub processing_count: u32,
    pub failed_last_hour: u32,
    pub expired_last_hour: u32,
}