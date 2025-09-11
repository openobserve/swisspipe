use sea_orm::entity::prelude::*;
use sea_orm::{ActiveModelTrait, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "nodes")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub workflow_id: String,
    pub name: String,
    pub node_type: String,
    pub config: String, // JSON configuration
    pub position_x: f64,
    pub position_y: f64,
    pub created_at: ChronoDateTimeUtc,
    pub input_merge_strategy: Option<String>, // JSON serialized InputMergeStrategy
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::entities::Entity",
        from = "Column::WorkflowId",
        to = "super::entities::Column::Id"
    )]
    Workflow,
}

impl Related<super::entities::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Workflow.def()
    }
}

impl ActiveModelBehavior for ActiveModel {
    fn new() -> Self {
        Self {
            id: Set(Uuid::new_v4().to_string()),
            created_at: Set(chrono::Utc::now()),
            ..ActiveModelTrait::default()
        }
    }
}