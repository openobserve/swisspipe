use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "sessions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub user_id: String,
    pub email: String,
    pub name: String,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub picture: Option<String>,
    pub locale: Option<String>,
    pub hosted_domain: Option<String>,
    pub verified_email: bool,
    pub created_at: i64,
    pub last_accessed_at: i64,
    pub expires_at: i64,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}


impl Model {
    /// Check if the session is expired
    pub fn is_expired(&self) -> bool {
        let now = chrono::Utc::now().timestamp();
        self.expires_at < now
    }


    /// Create a new session from Google user info
    pub fn from_google_user_info(
        session_id: String,
        user_info: &crate::auth::google::GoogleUserInfo,
        expires_in_seconds: i64,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Self {
        let now = chrono::Utc::now().timestamp();

        Self {
            id: session_id,
            user_id: user_info.id.clone(),
            email: user_info.email.clone(),
            name: user_info.name.clone(),
            given_name: user_info.given_name.clone(),
            family_name: user_info.family_name.clone(),
            picture: user_info.picture.clone(),
            locale: user_info.locale.clone(),
            hosted_domain: user_info.hd.clone(),
            verified_email: user_info.verified_email,
            created_at: now,
            last_accessed_at: now,
            expires_at: now + expires_in_seconds,
            ip_address,
            user_agent,
        }
    }
}