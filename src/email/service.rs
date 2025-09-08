use crate::email::{
    EmailError, SmtpConfig, SmtpSecurity, EmailMessage, EmailAddress,
    EmailSendResult, EmailQueueStats, EmailConfig, TemplateEngine
};
use crate::workflow::models::WorkflowEvent;
use lettre::{
    Message, SmtpTransport, Transport, Address, message::{header::ContentType, Mailbox, MultiPart, SinglePart},
    transport::smtp::{authentication::Credentials, client::{Tls, TlsParameters}},
};
use sea_orm::{DatabaseConnection, EntityTrait, ActiveModelTrait, Set, QueryFilter, ColumnTrait, QueryOrder, PaginatorTrait};
use std::{collections::HashMap, sync::Arc, time::Duration};
use governor::{Quota, RateLimiter, state::NotKeyed, state::InMemoryState, clock::DefaultClock};
use uuid::Uuid;
use std::num::NonZeroU32;

pub struct EmailService {
    smtp_configs: HashMap<String, SmtpConfig>,
    template_engine: TemplateEngine,
    rate_limiter: Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>,
    db: Arc<DatabaseConnection>,
}

impl EmailService {
    pub fn new(db: Arc<DatabaseConnection>) -> Result<Self, EmailError> {
        let smtp_configs = Self::load_smtp_configs()?;
        let template_engine = TemplateEngine::new()?;
        
        // Create rate limiter from environment variables
        let rate_limit_per_minute = std::env::var("SMTP_RATE_LIMIT_PER_MINUTE")
            .unwrap_or_else(|_| "60".to_string())
            .parse::<u32>()
            .unwrap_or(60);
        
        let burst_limit = std::env::var("SMTP_BURST_LIMIT")
            .unwrap_or_else(|_| "10".to_string())
            .parse::<u32>()
            .unwrap_or(10);
        
        let quota = Quota::per_minute(NonZeroU32::new(rate_limit_per_minute).unwrap())
            .allow_burst(NonZeroU32::new(burst_limit).unwrap());
        let rate_limiter = Arc::new(RateLimiter::direct(quota));
        
        Ok(Self {
            smtp_configs,
            template_engine,
            rate_limiter,
            db,
        })
    }
    
    fn load_smtp_configs() -> Result<HashMap<String, SmtpConfig>, EmailError> {
        tracing::debug!("Loading SMTP configurations...");
        let mut configs = HashMap::new();
        
        // Load default SMTP configuration
        let smtp_host = std::env::var("SMTP_HOST")
            .map_err(|_| EmailError::config("SMTP_HOST environment variable not set"))?;
        let smtp_from_email = std::env::var("SMTP_FROM_EMAIL")
            .map_err(|_| EmailError::config("SMTP_FROM_EMAIL environment variable not set"))?;
            
        tracing::info!("SMTP config loaded: host={}, from_email={}", smtp_host, smtp_from_email);
        
        let default_config = SmtpConfig {
            host: smtp_host,
            port: std::env::var("SMTP_PORT")
                .unwrap_or_else(|_| "587".to_string())
                .parse()
                .map_err(|_| EmailError::config("Invalid SMTP_PORT"))?,
            security: match std::env::var("SMTP_SECURITY")
                .unwrap_or_else(|_| "tls".to_string())
                .to_lowercase()
                .as_str()
            {
                "none" => SmtpSecurity::None,
                "tls" => SmtpSecurity::Tls,
                "ssl" => SmtpSecurity::Ssl,
                _ => return Err(EmailError::config("Invalid SMTP_SECURITY value. Use: none, tls, or ssl")),
            },
            username: std::env::var("SMTP_USERNAME").ok(),
            password: std::env::var("SMTP_PASSWORD").ok(),
            from_email: smtp_from_email,
            from_name: std::env::var("SMTP_FROM_NAME").ok(),
            timeout_seconds: std::env::var("SMTP_TIMEOUT_SECONDS")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .unwrap_or(30),
            max_retries: std::env::var("SMTP_MAX_RETRIES")
                .unwrap_or_else(|_| "3".to_string())
                .parse()
                .unwrap_or(3),
            retry_delay_seconds: std::env::var("SMTP_RETRY_DELAY_SECONDS")
                .unwrap_or_else(|_| "5".to_string())
                .parse()
                .unwrap_or(5),
        };
        
        configs.insert("default".to_string(), default_config);
        
        // TODO: Load additional SMTP configurations (marketing, alerts, etc.)
        // This would involve checking for environment variables like:
        // SMTP_MARKETING_HOST, SMTP_ALERTS_HOST, etc.
        
        Ok(configs)
    }
    
    pub async fn send_email(
        &self,
        email_config: &EmailConfig,
        workflow_event: &WorkflowEvent,
        execution_id: &str,
        node_id: &str,
    ) -> Result<EmailSendResult, EmailError> {
        tracing::debug!("Starting email send for execution_id: {}, node_id: {}, smtp_config: {}", 
            execution_id, node_id, email_config.smtp_config);
        
        // Check rate limiter
        if self.rate_limiter.check().is_err() {
            tracing::debug!("Rate limit exceeded, email will be queued");
        } else {
            tracing::debug!("Rate limit check passed, proceeding with immediate send");
        }
        
        if self.rate_limiter.check().is_err() {
            if email_config.queue_if_rate_limited {
                // Queue the email
                let queue_id = self.enqueue_email(
                    email_config,
                    workflow_event,
                    Some(execution_id.to_string()),
                    Some(node_id.to_string()),
                ).await?;
                
                return Ok(EmailSendResult {
                    success: true,
                    message_id: Some(format!("queued:{}", queue_id)),
                    error: None,
                    partial_success: None,
                });
            } else {
                return Err(EmailError::RateLimitExceeded);
            }
        }
        
        // Render email template
        tracing::debug!("Rendering email template for execution_id: {}", execution_id);
        let email_message = self.template_engine.render_email(
            email_config,
            workflow_event,
            execution_id,
            node_id,
        )?;
        
        tracing::debug!("Email template rendered successfully. To: {:?}, Subject: {}", 
            email_message.to.iter().map(|addr| &addr.email).collect::<Vec<_>>(), 
            email_message.subject);
        
        // Send email immediately
        tracing::debug!("Sending email via SMTP config: {}", email_config.smtp_config);
        let result = self.send_email_message(&email_config.smtp_config, &email_message).await?;
        
        // Log to audit table
        self.log_email_audit(
            execution_id,
            node_id,
            &email_config.smtp_config,
            &email_message,
            &result,
        ).await?;
        
        Ok(result)
    }
    
    async fn send_email_message(
        &self,
        smtp_config_name: &str,
        email_message: &EmailMessage,
    ) -> Result<EmailSendResult, EmailError> {
        tracing::debug!("Looking up SMTP config: {}", smtp_config_name);
        let smtp_config = self.smtp_configs.get(smtp_config_name)
            .ok_or_else(|| EmailError::config(format!("SMTP configuration '{}' not found", smtp_config_name)))?;
        
        tracing::debug!("SMTP config found: {}:{} with security {:?}", 
            smtp_config.host, smtp_config.port, smtp_config.security);
        
        // Build the email message
        tracing::debug!("Building lettre message");
        let message = self.build_lettre_message(email_message)?;
        tracing::debug!("Message built successfully");
        
        // Create SMTP transport
        tracing::debug!("Creating SMTP transport");
        let transport = self.create_smtp_transport(smtp_config)?;
        tracing::debug!("SMTP transport created successfully");
        
        // Send the email
        tracing::debug!("Sending email via SMTP transport");
        match transport.send(&message) {
            Ok(response) => {
                let message_text = response.message().collect::<Vec<_>>().join(" ");
                tracing::info!("Email sent successfully: {}", message_text);
                Ok(EmailSendResult {
                    success: true,
                    message_id: Some(format!("Message sent: {}", message_text)),
                    error: None,
                    partial_success: None,
                })
            }
            Err(e) => {
                tracing::error!("SMTP send error: {}", e);
                Ok(EmailSendResult {
                    success: false,
                    message_id: None,
                    error: Some(format!("SMTP send error: {}", e)),
                    partial_success: None,
                })
            }
        }
    }
    
    fn build_lettre_message(&self, email_message: &EmailMessage) -> Result<Message, EmailError> {
        let from_mailbox = self.email_address_to_mailbox(&email_message.from)?;
        let mut builder = Message::builder().from(from_mailbox);
        
        // Add recipients
        for to_addr in &email_message.to {
            let mailbox = self.email_address_to_mailbox(to_addr)?;
            builder = builder.to(mailbox);
        }
        
        for cc_addr in &email_message.cc {
            let mailbox = self.email_address_to_mailbox(cc_addr)?;
            builder = builder.cc(mailbox);
        }
        
        for bcc_addr in &email_message.bcc {
            let mailbox = self.email_address_to_mailbox(bcc_addr)?;
            builder = builder.bcc(mailbox);
        }
        
        builder = builder.subject(&email_message.subject);
        
        // Build message body
        let body = if email_message.html_body.is_some() && email_message.text_body.is_some() {
            // Multipart message
            MultiPart::alternative()
                .singlepart(
                    SinglePart::builder()
                        .header(ContentType::TEXT_PLAIN)
                        .body(email_message.text_body.as_ref().unwrap().clone())
                )
                .singlepart(
                    SinglePart::builder()
                        .header(ContentType::TEXT_HTML)
                        .body(email_message.html_body.as_ref().unwrap().clone())
                )
        } else if let Some(ref html_body) = email_message.html_body {
            // HTML only
            MultiPart::alternative()
                .singlepart(
                    SinglePart::builder()
                        .header(ContentType::TEXT_HTML)
                        .body(html_body.clone())
                )
        } else if let Some(ref text_body) = email_message.text_body {
            // Text only
            MultiPart::alternative()
                .singlepart(
                    SinglePart::builder()
                        .header(ContentType::TEXT_PLAIN)
                        .body(text_body.clone())
                )
        } else {
            return Err(EmailError::validation("Email must have either HTML or text body"));
        };
        
        // TODO: Add attachment support
        // This would involve extending the MultiPart with attachment parts
        
        let message = builder.multipart(body)
            .map_err(|e| EmailError::send(format!("Failed to build email message: {}", e)))?;
        
        Ok(message)
    }
    
    fn email_address_to_mailbox(&self, email_addr: &EmailAddress) -> Result<Mailbox, EmailError> {
        let address: Address = email_addr.email.parse()
            .map_err(|e| EmailError::validation(format!("Invalid email address '{}': {}", email_addr.email, e)))?;
        
        if let Some(ref name) = email_addr.name {
            Ok(Mailbox::new(Some(name.clone()), address))
        } else {
            Ok(Mailbox::new(None, address))
        }
    }
    
    fn create_smtp_transport(&self, smtp_config: &SmtpConfig) -> Result<SmtpTransport, EmailError> {
        let mut builder = SmtpTransport::relay(&smtp_config.host)
            .map_err(|e| EmailError::connection(format!("Failed to create SMTP relay: {}", e)))?
            .port(smtp_config.port)
            .timeout(Some(Duration::from_secs(smtp_config.timeout_seconds)));
        
        // Configure security
        builder = match smtp_config.security {
            SmtpSecurity::None => builder.tls(Tls::None),
            SmtpSecurity::Tls => {
                let tls_params = TlsParameters::new(smtp_config.host.clone())
                    .map_err(|e| EmailError::connection(format!("TLS configuration error: {}", e)))?;
                builder.tls(Tls::Required(tls_params))
            },
            SmtpSecurity::Ssl => {
                let tls_params = TlsParameters::new(smtp_config.host.clone())
                    .map_err(|e| EmailError::connection(format!("TLS configuration error: {}", e)))?;
                builder.tls(Tls::Wrapper(tls_params))
            },
        };
        
        // Configure authentication
        if let (Some(username), Some(password)) = (&smtp_config.username, &smtp_config.password) {
            builder = builder.credentials(Credentials::new(username.clone(), password.clone()));
        }
        
        Ok(builder.build())
    }
    
    async fn enqueue_email(
        &self,
        email_config: &EmailConfig,
        workflow_event: &WorkflowEvent,
        execution_id: Option<String>,
        node_id: Option<String>,
    ) -> Result<String, EmailError> {
        use crate::database::email_queue;
        
        tracing::debug!("Enqueueing email with execution_id: {:?}, node_id: {:?}", execution_id, node_id);
        
        let queue_id = Uuid::now_v7().to_string();
        let now = chrono::Utc::now().timestamp_micros();
        
        let template_context = serde_json::json!({
            "workflow": {
                "data": workflow_event.data,
                "metadata": workflow_event.metadata,
                "headers": workflow_event.headers,
                "condition_results": workflow_event.condition_results,
            },
        });
        
        let active_model = email_queue::ActiveModel {
            id: Set(queue_id.clone()),
            execution_id: Set(execution_id.clone()),
            node_id: Set(node_id.clone()),
            smtp_config: Set(email_config.smtp_config.clone()),
            priority: Set(email_config.priority.as_str().to_string()),
            email_config: Set(serde_json::to_string(email_config)?),
            template_context: Set(serde_json::to_string(&template_context)?),
            status: Set("queued".to_string()),
            queued_at: Set(now),
            scheduled_at: Set(None),
            processed_at: Set(None),
            sent_at: Set(None),
            max_wait_minutes: Set(email_config.max_queue_wait_minutes as i32),
            retry_count: Set(0),
            max_retries: Set(3),
            error_message: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };
        
        tracing::debug!("About to insert email queue record with values - execution_id: {:?}, node_id: {:?}", 
            execution_id, node_id);
            
        match active_model.insert(&*self.db).await {
            Ok(_) => {
                tracing::debug!("Successfully inserted email queue record");
            }
            Err(e) => {
                tracing::error!("Failed to insert email queue record: {:?}", e);
                return Err(e.into());
            }
        }
        
        Ok(queue_id)
    }
    
    async fn log_email_audit(
        &self,
        execution_id: &str,
        node_id: &str,
        smtp_config: &str,
        email_message: &EmailMessage,
        result: &EmailSendResult,
    ) -> Result<(), EmailError> {
        use crate::database::email_audit_log;
        
        let audit_id = Uuid::now_v7().to_string();
        let now = chrono::Utc::now().timestamp_micros();
        
        let to_emails = email_message.to.iter()
            .map(|addr| &addr.email)
            .collect::<Vec<_>>();
        let cc_emails = email_message.cc.iter()
            .map(|addr| &addr.email)
            .collect::<Vec<_>>();
        let bcc_emails = email_message.bcc.iter()
            .map(|addr| &addr.email)
            .collect::<Vec<_>>();
        
        let email_size = email_message.html_body.as_ref().map(|s| s.len()).unwrap_or(0) +
            email_message.text_body.as_ref().map(|s| s.len()).unwrap_or(0);
        
        let status = if result.success { "sent" } else { "failed" };
        
        let active_model = email_audit_log::ActiveModel {
            id: Set(audit_id),
            execution_id: Set(execution_id.to_string()),
            node_id: Set(node_id.to_string()),
            smtp_config: Set(smtp_config.to_string()),
            from_email: Set(email_message.from.email.clone()),
            to_emails: Set(serde_json::to_string(&to_emails)?),
            cc_emails: Set(if cc_emails.is_empty() { None } else { Some(serde_json::to_string(&cc_emails)?) }),
            bcc_emails: Set(if bcc_emails.is_empty() { None } else { Some(serde_json::to_string(&bcc_emails)?) }),
            subject: Set(email_message.subject.clone()),
            email_size_bytes: Set(email_size as i32),
            attachment_count: Set(email_message.attachments.len() as i32),
            status: Set(status.to_string()),
            error_message: Set(result.error.clone()),
            smtp_message_id: Set(result.message_id.clone()),
            sent_at: Set(if result.success { Some(now) } else { None }),
            created_at: Set(now),
        };
        
        active_model.insert(&*self.db).await?;
        
        Ok(())
    }
    
    pub async fn process_email_queue(&self) -> Result<u32, EmailError> {
        use crate::database::{email_queue, email_queue::Entity as EmailQueue};
        
        // Get next email from queue (ordered by priority and queue time)
        let queued_email = EmailQueue::find()
            .filter(email_queue::Column::Status.eq("queued"))
            .order_by_desc(email_queue::Column::Priority)
            .order_by_asc(email_queue::Column::QueuedAt)
            .one(&*self.db)
            .await?;
        
        if let Some(email) = queued_email {
            // Check rate limiter
            if self.rate_limiter.check().is_err() {
                return Ok(0); // No emails processed due to rate limiting
            }
            
            // Mark as processing
            self.mark_email_processing(&email.id).await?;
            
            // Deserialize email config and template context
            let email_config: EmailConfig = serde_json::from_str(&email.email_config)?;
            let template_context: serde_json::Value = serde_json::from_str(&email.template_context)?;
            
            // Create a mock workflow event from the template context
            let workflow_event = WorkflowEvent {
                data: template_context.get("workflow")
                    .and_then(|w| w.get("data"))
                    .cloned()
                    .unwrap_or_default(),
                metadata: HashMap::new(),
                headers: HashMap::new(),
                condition_results: HashMap::new(),
            };
            
            // Send the email
            match self.send_email_message(&email.smtp_config, &self.template_engine.render_email(
                &email_config,
                &workflow_event,
                &email.execution_id.unwrap_or_default(),
                &email.node_id.unwrap_or_default(),
            )?).await {
                Ok(result) => {
                    if result.success {
                        self.mark_email_sent(&email.id, result.message_id.as_deref()).await?;
                    } else {
                        self.mark_email_failed(&email.id, &result.error.unwrap_or_default()).await?;
                    }
                }
                Err(e) => {
                    self.mark_email_failed(&email.id, &e.to_string()).await?;
                }
            }
            
            Ok(1)
        } else {
            Ok(0)
        }
    }
    
    async fn mark_email_processing(&self, email_id: &str) -> Result<(), EmailError> {
        use crate::database::{email_queue, email_queue::Entity as EmailQueue};
        
        let now = chrono::Utc::now().timestamp_micros();
        
        let email = EmailQueue::find_by_id(email_id.to_string())
            .one(&*self.db)
            .await?
            .ok_or_else(|| EmailError::validation("Email not found in queue"))?;
        
        let mut active_model: email_queue::ActiveModel = email.into();
        active_model.status = Set("processing".to_string());
        active_model.processed_at = Set(Some(now));
        active_model.updated_at = Set(now);
        
        active_model.update(&*self.db).await?;
        Ok(())
    }
    
    async fn mark_email_sent(&self, email_id: &str, _message_id: Option<&str>) -> Result<(), EmailError> {
        use crate::database::{email_queue, email_queue::Entity as EmailQueue};
        
        let now = chrono::Utc::now().timestamp_micros();
        
        let email = EmailQueue::find_by_id(email_id.to_string())
            .one(&*self.db)
            .await?
            .ok_or_else(|| EmailError::validation("Email not found in queue"))?;
        
        let mut active_model: email_queue::ActiveModel = email.into();
        active_model.status = Set("sent".to_string());
        active_model.sent_at = Set(Some(now));
        active_model.updated_at = Set(now);
        
        active_model.update(&*self.db).await?;
        Ok(())
    }
    
    async fn mark_email_failed(&self, email_id: &str, error: &str) -> Result<(), EmailError> {
        use crate::database::{email_queue, email_queue::Entity as EmailQueue};
        
        let now = chrono::Utc::now().timestamp_micros();
        
        let email = EmailQueue::find_by_id(email_id.to_string())
            .one(&*self.db)
            .await?
            .ok_or_else(|| EmailError::validation("Email not found in queue"))?;
        
        let retry_count = email.retry_count;
        let mut active_model: email_queue::ActiveModel = email.into();
        active_model.status = Set("failed".to_string());
        active_model.error_message = Set(Some(error.to_string()));
        active_model.retry_count = Set(retry_count + 1);
        active_model.updated_at = Set(now);
        
        active_model.update(&*self.db).await?;
        Ok(())
    }
    
    pub async fn cleanup_expired_emails(&self) -> Result<u32, EmailError> {
        use crate::database::{email_queue, email_queue::Entity as EmailQueue};
        use sea_orm::DeleteResult;
        
        let now = chrono::Utc::now().timestamp_micros();
        
        // Delete emails that have been in queue longer than their max wait time
        let result: DeleteResult = EmailQueue::delete_many()
            .filter(email_queue::Column::Status.eq("queued"))
            .filter(email_queue::Column::QueuedAt.lt(now - (60 * 1_000_000))) // 60 minutes ago in microseconds
            .exec(&*self.db)
            .await?;
        
        Ok(result.rows_affected as u32)
    }
    
    pub async fn get_queue_stats(&self) -> Result<EmailQueueStats, EmailError> {
        use crate::database::{email_queue, email_queue::Entity as EmailQueue};
        
        // This is a simplified version - in a real implementation you'd need more complex queries
        let queue_size = EmailQueue::find()
            .filter(email_queue::Column::Status.eq("queued"))
            .count(&*self.db)
            .await? as u32;
        
        let processing_count = EmailQueue::find()
            .filter(email_queue::Column::Status.eq("processing"))
            .count(&*self.db)
            .await? as u32;
        
        Ok(EmailQueueStats {
            queue_size,
            rate_limit_per_minute: 60, // From environment
            tokens_available: 10, // From rate limiter state
            next_refill_seconds: 30, // Calculated from rate limiter
            priority_breakdown: HashMap::new(), // Would need complex query
            average_wait_minutes: 0.0, // Would need complex query
            emails_sent_last_minute: 0, // Would need complex query
            emails_queued_last_minute: 0, // Would need complex query
            processing_count,
            failed_last_hour: 0, // Would need complex query
            expired_last_hour: 0, // Would need complex query
        })
    }
}