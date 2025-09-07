use crate::workflow::errors::SwissPipeError;
use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub username: String,
    pub password: String,
    pub database_url: String,
    pub port: u16,
}

impl Config {
    pub fn from_env() -> Result<Self, SwissPipeError> {
        let username = env::var("SP_USERNAME")
            .unwrap_or_else(|_| "admin".to_string());
        let password = env::var("SP_PASSWORD")
            .unwrap_or_else(|_| "admin".to_string());
        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "sqlite:data/swisspipe.db?mode=rwc".to_string());
        let port = env::var("PORT")
            .unwrap_or_else(|_| "3700".to_string())
            .parse()
            .map_err(|_| SwissPipeError::Config("Invalid PORT value".to_string()))?;

        // Ensure data directory exists
        if let Some(db_path_str) = database_url.strip_prefix("sqlite:") {
            if let Some(db_path) = db_path_str.split('?').next() {
                if let Some(parent) = std::path::Path::new(db_path).parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| SwissPipeError::Config(format!("Failed to create data directory: {}", e)))?;
                }
            }
        }

        Ok(Config {
            username,
            password,
            database_url,
            port,
        })
    }
}