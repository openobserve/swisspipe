use sea_orm::entity::prelude::*;
use sea_orm::{ActiveModelTrait, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "environment_variables")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    #[sea_orm(unique)]
    pub name: String,
    pub value_type: String, // 'text' or 'secret'
    pub value: String,      // Encrypted for secrets
    pub description: Option<String>,
    pub created_at: i64, // Unix epoch microseconds
    pub updated_at: i64, // Unix epoch microseconds
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {
    fn new() -> Self {
        let now = chrono::Utc::now().timestamp_micros();
        Self {
            id: Set(Uuid::now_v7().to_string()),
            created_at: Set(now),
            updated_at: Set(now),
            ..ActiveModelTrait::default()
        }
    }
}

// Value type enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ValueType {
    Text,
    Secret,
}

impl std::fmt::Display for ValueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValueType::Text => write!(f, "text"),
            ValueType::Secret => write!(f, "secret"),
        }
    }
}

impl From<String> for ValueType {
    fn from(s: String) -> Self {
        match s.as_str() {
            "secret" => ValueType::Secret,
            _ => ValueType::Text,
        }
    }
}

impl From<&str> for ValueType {
    fn from(s: &str) -> Self {
        match s {
            "secret" => ValueType::Secret,
            _ => ValueType::Text,
        }
    }
}
