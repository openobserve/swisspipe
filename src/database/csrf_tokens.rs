use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "csrf_tokens")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    #[sea_orm(unique)]
    pub token: String,
    pub created_at: i64,
    pub expires_at: i64,
    pub used: bool,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}


impl Model {
    /// Check if the CSRF token is expired
    pub fn is_expired(&self) -> bool {
        let now = chrono::Utc::now().timestamp();
        self.expires_at < now
    }

    /// Check if the CSRF token is valid for use
    pub fn is_valid(&self) -> bool {
        !self.used && !self.is_expired()
    }

    /// Create a new CSRF token
    pub fn new(
        token: String,
        expires_in_seconds: i64,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Self {
        let now = chrono::Utc::now().timestamp();

        Self {
            id: Uuid::new_v4().to_string(),
            token,
            created_at: now,
            expires_at: now + expires_in_seconds,
            used: false,
            ip_address,
            user_agent,
        }
    }

}