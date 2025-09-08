pub mod models;
pub mod service;
pub mod template;
pub mod error;

pub use models::*;
pub use template::TemplateEngine;
pub use error::EmailError;