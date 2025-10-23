use sea_orm::entity::prelude::*;
use sea_orm::{Set, ActiveModelBehavior};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "scheduled_triggers")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,

    pub workflow_id: Uuid,

    pub trigger_node_id: String,

    #[sea_orm(nullable)]
    pub schedule_name: Option<String>,

    pub cron_expression: String,

    pub timezone: String,

    #[sea_orm(column_type = "JsonBinary")]
    pub test_payload: serde_json::Value,

    pub enabled: bool,

    #[sea_orm(nullable)]
    pub start_date: Option<DateTimeUtc>,

    #[sea_orm(nullable)]
    pub end_date: Option<DateTimeUtc>,

    #[sea_orm(nullable)]
    pub last_execution_time: Option<DateTimeUtc>,

    #[sea_orm(nullable)]
    pub next_execution_time: Option<DateTimeUtc>,

    pub execution_count: i64,

    pub failure_count: i64,

    pub created_at: DateTimeUtc,

    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {
    fn before_save<'life0, 'async_trait, C>(
        mut self,
        _db: &'life0 C,
        insert: bool,
    ) -> ::core::pin::Pin<Box<dyn ::core::future::Future<Output = Result<Self, DbErr>> + ::core::marker::Send + 'async_trait>>
    where
        C: 'async_trait + ConnectionTrait,
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            let now = chrono::Utc::now();
            if insert {
                self.created_at = Set(now);
            }
            self.updated_at = Set(now);
            Ok(self)
        })
    }
}
