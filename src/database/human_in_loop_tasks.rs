use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "human_in_loop_tasks")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: String,
    pub execution_id: String,
    pub node_id: String,
    pub node_execution_id: String,
    pub workflow_id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub timeout_at: Option<i64>,
    pub timeout_action: Option<String>,
    pub required_fields: Option<Json>,
    pub metadata: Option<Json>,
    pub response_data: Option<Json>,
    pub response_received_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    // Relations removed to avoid foreign key validation issues
    // HIL tasks are now standalone entities without relational constraints
}

impl ActiveModelBehavior for ActiveModel {}