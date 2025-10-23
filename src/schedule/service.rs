use cron::Schedule as CronSchedule;
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use sea_orm::{entity::*, DatabaseConnection, QueryFilter};
use std::sync::Arc;
use std::str::FromStr;
use uuid::Uuid;

use crate::database::scheduled_cron_triggers;
use crate::workflow::errors::{SwissPipeError, Result};

pub struct ScheduleService {
    db: Arc<DatabaseConnection>,
}

impl ScheduleService {
    pub fn new(db: Arc<DatabaseConnection>) -> Result<Self> {
        Ok(Self { db })
    }

    /// Create or update schedule for a trigger
    pub async fn upsert_schedule(
        &self,
        workflow_id: Uuid,
        trigger_node_id: String,
        config: ScheduleConfig,
    ) -> Result<scheduled_cron_triggers::Model> {
        // Validate cron expression
        self.validate_cron(&config.cron_expression)?;

        // Validate timezone
        let _: Tz = config.timezone.parse()
            .map_err(|_| SwissPipeError::Generic(format!("Invalid timezone: {}", config.timezone)))?;

        // Validate date range if both start and end dates are provided
        if let (Some(start), Some(end)) = (config.start_date, config.end_date) {
            if end <= start {
                return Err(SwissPipeError::Generic(
                    "end_date must be after start_date".to_string()
                ));
            }
        }

        // Calculate next execution time
        let next_execution = self.calculate_next_execution(
            &config.cron_expression,
            &config.timezone,
            Utc::now(),
        )?;

        // Check if schedule already exists
        let existing = scheduled_cron_triggers::Entity::find()
            .filter(scheduled_cron_triggers::Column::WorkflowId.eq(workflow_id))
            .filter(scheduled_cron_triggers::Column::TriggerNodeId.eq(&trigger_node_id))
            .one(&*self.db)
            .await?;

        let now = Utc::now();

        if let Some(existing_schedule) = existing {
            // Update existing schedule
            let mut active_model: scheduled_cron_triggers::ActiveModel = existing_schedule.into();
            active_model.schedule_name = Set(config.schedule_name);
            active_model.cron_expression = Set(config.cron_expression);
            active_model.timezone = Set(config.timezone);
            active_model.test_payload = Set(config.test_payload);
            active_model.enabled = Set(config.enabled);
            active_model.start_date = Set(config.start_date);
            active_model.end_date = Set(config.end_date);
            active_model.next_execution_time = Set(Some(next_execution));
            active_model.updated_at = Set(now);

            let updated = active_model.update(&*self.db).await?;
            tracing::info!(
                workflow_id = %workflow_id,
                trigger_node_id = %trigger_node_id,
                "Updated schedule"
            );
            Ok(updated)
        } else {
            // Create new schedule
            let new_schedule = scheduled_cron_triggers::ActiveModel {
                id: Set(Uuid::now_v7()),
                workflow_id: Set(workflow_id),
                trigger_node_id: Set(trigger_node_id.clone()),
                schedule_name: Set(config.schedule_name),
                cron_expression: Set(config.cron_expression),
                timezone: Set(config.timezone),
                test_payload: Set(config.test_payload),
                enabled: Set(config.enabled),
                start_date: Set(config.start_date),
                end_date: Set(config.end_date),
                last_execution_time: Set(None),
                next_execution_time: Set(Some(next_execution)),
                execution_count: Set(0),
                failure_count: Set(0),
                created_at: Set(now),
                updated_at: Set(now),
            };

            let created = new_schedule.insert(&*self.db).await?;
            tracing::info!(
                workflow_id = %workflow_id,
                trigger_node_id = %trigger_node_id,
                "Created new schedule"
            );
            Ok(created)
        }
    }

    /// Get schedule for a workflow trigger
    pub async fn get_schedule(
        &self,
        workflow_id: Uuid,
        trigger_node_id: &str,
    ) -> Result<Option<scheduled_cron_triggers::Model>> {
        let schedule = scheduled_cron_triggers::Entity::find()
            .filter(scheduled_cron_triggers::Column::WorkflowId.eq(workflow_id))
            .filter(scheduled_cron_triggers::Column::TriggerNodeId.eq(trigger_node_id))
            .one(&*self.db)
            .await?;

        Ok(schedule)
    }

    /// Get schedule by ID
    pub async fn get_schedule_by_id(&self, schedule_id: Uuid) -> Result<Option<scheduled_cron_triggers::Model>> {
        let schedule = scheduled_cron_triggers::Entity::find_by_id(schedule_id)
            .one(&*self.db)
            .await?;

        Ok(schedule)
    }

    /// Get all enabled schedules
    pub async fn get_enabled_schedules(&self) -> Result<Vec<scheduled_cron_triggers::Model>> {
        let schedules = scheduled_cron_triggers::Entity::find()
            .filter(scheduled_cron_triggers::Column::Enabled.eq(true))
            .all(&*self.db)
            .await?;

        Ok(schedules)
    }

    /// Enable/disable schedule
    pub async fn set_enabled(&self, workflow_id: Uuid, trigger_node_id: &str, enabled: bool) -> Result<()> {
        let schedule = self.get_schedule(workflow_id, trigger_node_id).await?
            .ok_or_else(|| SwissPipeError::NotFound("Schedule not found".to_string()))?;

        let mut active_model: scheduled_cron_triggers::ActiveModel = schedule.into();
        active_model.enabled = Set(enabled);
        active_model.updated_at = Set(Utc::now());
        active_model.update(&*self.db).await?;

        tracing::info!(
            workflow_id = %workflow_id,
            trigger_node_id = %trigger_node_id,
            enabled = enabled,
            "Updated schedule enabled status"
        );

        Ok(())
    }

    /// Delete schedule
    pub async fn delete_schedule(&self, workflow_id: Uuid, trigger_node_id: &str) -> Result<()> {
        let schedule = self.get_schedule(workflow_id, trigger_node_id).await?
            .ok_or_else(|| SwissPipeError::NotFound("Schedule not found".to_string()))?;

        scheduled_cron_triggers::Entity::delete_by_id(schedule.id)
            .exec(&*self.db)
            .await?;

        tracing::info!(
            workflow_id = %workflow_id,
            trigger_node_id = %trigger_node_id,
            "Deleted schedule"
        );

        Ok(())
    }

    /// Validate cron expression using cron::Schedule
    pub fn validate_cron(&self, expression: &str) -> Result<()> {
        CronSchedule::from_str(expression)
            .map(|_| ())
            .map_err(|e| SwissPipeError::Generic(format!("Invalid cron expression: {e}")))
    }

    /// Preview next N execution times using cron::Schedule
    pub fn preview_executions(
        &self,
        expression: &str,
        timezone: &str,
        count: usize,
    ) -> Result<Vec<DateTime<Tz>>> {
        let cron_schedule = CronSchedule::from_str(expression)
            .map_err(|e| SwissPipeError::Generic(format!("Invalid cron expression: {e}")))?;

        let tz: Tz = timezone.parse()
            .map_err(|_| SwissPipeError::Generic(format!("Invalid timezone: {timezone}")))?;

        let executions: Vec<DateTime<Tz>> = cron_schedule
            .upcoming(tz)
            .take(count)
            .collect();

        Ok(executions)
    }

    /// Calculate next execution time using cron::Schedule
    pub fn calculate_next_execution(
        &self,
        expression: &str,
        timezone: &str,
        after: DateTime<Utc>,
    ) -> Result<DateTime<Utc>> {
        let cron_schedule = CronSchedule::from_str(expression)
            .map_err(|e| SwissPipeError::Generic(format!("Invalid cron expression: {e}")))?;

        let tz: Tz = timezone.parse()
            .map_err(|_| SwissPipeError::Generic(format!("Invalid timezone: {timezone}")))?;

        let after_in_tz = after.with_timezone(&tz);
        let next = cron_schedule
            .after(&after_in_tz)
            .next()
            .ok_or_else(|| SwissPipeError::Generic("No next execution time found".to_string()))?;

        Ok(next.with_timezone(&Utc))
    }

    /// Update schedule statistics after execution
    pub async fn update_after_execution(
        &self,
        schedule_id: Uuid,
        success: bool,
        next_execution: DateTime<Utc>,
    ) -> Result<()> {
        let schedule = self.get_schedule_by_id(schedule_id).await?
            .ok_or_else(|| SwissPipeError::NotFound("Schedule not found".to_string()))?;

        let mut active_model: scheduled_cron_triggers::ActiveModel = schedule.clone().into();
        active_model.last_execution_time = Set(Some(Utc::now()));
        active_model.next_execution_time = Set(Some(next_execution));

        if success {
            active_model.execution_count = Set(schedule.execution_count + 1);
        } else {
            active_model.failure_count = Set(schedule.failure_count + 1);
        }

        active_model.updated_at = Set(Utc::now());
        active_model.update(&*self.db).await?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ScheduleConfig {
    pub schedule_name: Option<String>,
    pub cron_expression: String,
    pub timezone: String,
    pub test_payload: serde_json::Value,
    pub enabled: bool,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
}
