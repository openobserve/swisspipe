use thiserror::Error;

#[derive(Debug, Error)]
pub enum EmailError {
    #[error("SMTP configuration error: {message}")]
    ConfigError { message: String },

    #[error("SMTP connection error: {message}")]
    ConnectionError { message: String },

    #[error("Email template error: {message}")]
    TemplateError { message: String },

    #[error("Email validation error: {message}")]
    ValidationError { message: String },

    #[error("Email sending error: {message}")]
    SendError { message: String },

    #[error("Database error: {0}")]
    DatabaseError(#[from] sea_orm::DbErr),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Queue full")]
    QueueFull,

    #[error("Email expired in queue")]
    QueueExpired,
}

impl EmailError {
    pub fn config(message: impl Into<String>) -> Self {
        Self::ConfigError {
            message: message.into(),
        }
    }

    pub fn connection(message: impl Into<String>) -> Self {
        Self::ConnectionError {
            message: message.into(),
        }
    }

    pub fn template(message: impl Into<String>) -> Self {
        Self::TemplateError {
            message: message.into(),
        }
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::ValidationError {
            message: message.into(),
        }
    }

    pub fn send(message: impl Into<String>) -> Self {
        Self::SendError {
            message: message.into(),
        }
    }
}