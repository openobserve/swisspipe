# Product Requirements Document: Cron/Schedule Trigger

## Document Information
- **Feature**: Cron/Schedule Trigger for Workflows
- **Created**: 2025-10-21
- **Status**: Draft - Awaiting Approval
- **Priority**: High

---

## 1. Executive Summary

Currently, SwissPipe workflows can only be triggered by HTTP endpoints (Native or Segment-compatible). This PRD proposes adding scheduled/cron-based triggers to enable workflows to execute automatically on a recurring schedule, enabling use cases such as:

- Periodic data synchronization and ETL jobs
- Regular report generation and distribution
- Scheduled maintenance and cleanup tasks
- Time-based notifications and reminders
- Periodic health checks and monitoring

---

## 2. Goals & Objectives

### Primary Goals
1. Enable workflows to run automatically on a defined schedule
2. Support standard cron expressions for maximum flexibility
3. Provide a user-friendly interface for schedule configuration
4. Maintain reliability and observability for scheduled executions

### Success Metrics
- Successfully execute scheduled workflows with 99.9% reliability
- Support at least 1000 concurrent scheduled workflows
- Schedule configuration completion time < 2 minutes
- Zero missed scheduled executions under normal operation

---

## 3. User Stories

### As a Data Engineer
- I want to schedule a workflow to run every hour to sync data from external APIs
- I want to schedule a daily workflow at 2 AM UTC to generate and email reports
- I want to pause/resume a scheduled workflow without deleting the schedule

### As a DevOps Engineer
- I want to schedule a workflow to run every 5 minutes to check service health
- I want to configure timezone-aware schedules for global operations
- I want to see execution history and identify missed or failed scheduled runs

### As a Business Analyst
- I want to schedule weekly summary reports to be generated and sent to stakeholders
- I want to configure a workflow to run on the last day of each month for billing
- I want to easily understand and modify existing schedules without technical knowledge

---

## 4. Functional Requirements

### 4.1 Trigger Configuration UI

#### 4.1.1 New "Cron/Schedule" Tab
- Add a fourth tab to TriggerConfig component: "Native Endpoints" | "Segment Endpoints" | "Test" | **"Cron/Schedule"**
- Tab should be accessible immediately when editing a Trigger node
- Should support multiple schedules per trigger (optional for MVP - start with single schedule)

#### 4.1.2 Schedule Configuration Fields

**Schedule Expression**
- Input field for cron expression (text input with validation)
- Visual cron builder (optional enhancement - dropdown selectors)
- Real-time validation with error messages
- Preview of next 5 execution times
- Support standard 5-field cron format: `* * * * *` (minute hour day month weekday)
- **Note**: Extended 6-field format with seconds is NOT supported in MVP (cron crate uses 5-field format)

**Timezone Configuration**
- Dropdown to select timezone (default: UTC)
- Show current time in selected timezone
- Support IANA timezone database (e.g., "America/New_York", "Europe/London", "UTC")

**Schedule Metadata**
- Schedule name/description (optional, for human readability)
- Enable/Disable toggle (to pause schedule without deletion)
- Start date/time (optional - schedule becomes active after this time)
- End date/time (optional - schedule becomes inactive after this time)

**Test Data Configuration**
- JSON payload to use for scheduled executions (similar to Test tab)
- Option to use empty payload `{}`
- Option to inject timestamp/metadata into payload automatically

#### 4.1.3 Quick Schedule Presets
Provide common schedule templates for easy configuration:
- Every minute: `* * * * *`
- Every 5 minutes: `*/5 * * * *`
- Every hour: `0 * * * *`
- Daily at midnight UTC: `0 0 * * *`
- Daily at 9 AM: `0 9 * * *`
- Weekly on Monday at 9 AM: `0 9 * * 1`
- Monthly on 1st at midnight: `0 0 1 * *`
- Weekdays at 8 AM: `0 8 * * 1-5`

#### 4.1.4 Schedule Status Display
- Current status (derived from database fields):
  - **Active**: `enabled = true` AND within start/end date range
  - **Paused**: `enabled = false`
  - **Expired**: `enabled = true` but past `end_date`
  - **Pending**: `enabled = true` but before `start_date`
- Last execution time and result (success/failure)
- Next scheduled execution time
- Total execution count and success rate (calculated from `execution_count` and `failure_count`)
- Quick link to view execution history

---

### 4.2 Backend Implementation

#### 4.2.1 Database Schema Changes (SeaORM)

**New Entity: `scheduled_triggers`**

Location: `src/database/entities/scheduled_triggers.rs`

```rust
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "scheduled_triggers")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,

    #[sea_orm(column_type = "Uuid")]
    pub workflow_id: Uuid,

    pub trigger_node_id: String,

    #[sea_orm(nullable)]
    pub schedule_name: Option<String>,

    pub cron_expression: String,

    pub timezone: String,

    #[sea_orm(column_type = "JsonBinary")]
    pub test_payload: Json,

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
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::workflows::Entity",
        from = "Column::WorkflowId",
        to = "super::workflows::Column::Id",
        on_delete = "Cascade"
    )]
    Workflow,
}

impl Related<super::workflows::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Workflow.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
```

**SeaORM Migration:**

Location: `src/database/migrations/mXXXX_create_scheduled_triggers.rs`

```rust
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ScheduledTriggers::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ScheduledTriggers::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ScheduledTriggers::WorkflowId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ScheduledTriggers::TriggerNodeId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ScheduledTriggers::ScheduleName)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ScheduledTriggers::CronExpression)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ScheduledTriggers::Timezone)
                            .string()
                            .not_null()
                            .default("UTC"),
                    )
                    .col(
                        ColumnDef::new(ScheduledTriggers::TestPayload)
                            .json_binary()
                            .not_null()
                            .default("{}"),
                    )
                    .col(
                        ColumnDef::new(ScheduledTriggers::Enabled)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(ScheduledTriggers::StartDate)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ScheduledTriggers::EndDate)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ScheduledTriggers::LastExecutionTime)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ScheduledTriggers::NextExecutionTime)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ScheduledTriggers::ExecutionCount)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(ScheduledTriggers::FailureCount)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(ScheduledTriggers::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(ScheduledTriggers::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_scheduled_triggers_workflow")
                            .from(ScheduledTriggers::Table, ScheduledTriggers::WorkflowId)
                            .to(Workflows::Table, Workflows::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index for efficient querying of due schedules
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_scheduled_triggers_next_execution")
                    .table(ScheduledTriggers::Table)
                    .col(ScheduledTriggers::NextExecutionTime)
                    .to_owned(),
            )
            .await?;

        // Create index for workflow lookups
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_scheduled_triggers_workflow")
                    .table(ScheduledTriggers::Table)
                    .col(ScheduledTriggers::WorkflowId)
                    .to_owned(),
            )
            .await?;

        // Create unique constraint on workflow_id + trigger_node_id
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_scheduled_triggers_unique_workflow_node")
                    .table(ScheduledTriggers::Table)
                    .col(ScheduledTriggers::WorkflowId)
                    .col(ScheduledTriggers::TriggerNodeId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ScheduledTriggers::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ScheduledTriggers {
    Table,
    Id,
    WorkflowId,
    TriggerNodeId,
    ScheduleName,
    CronExpression,
    Timezone,
    TestPayload,
    Enabled,
    StartDate,
    EndDate,
    LastExecutionTime,
    NextExecutionTime,
    ExecutionCount,
    FailureCount,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Workflows {
    Table,
    Id,
}
```

**Migration Registration:**

Add to `src/database/migrations/mod.rs`:
```rust
mod mXXXX_create_scheduled_triggers;

pub use mXXXX_create_scheduled_triggers::Migration as CreateScheduledTriggers;
```

And register in migrator:
```rust
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            // ... existing migrations
            Box::new(CreateScheduledTriggers),
        ]
    }
}
```

**Note**: The database schema will be managed through SeaORM's migration system, which provides:
- Type-safe schema definitions
- Automatic migration versioning
- Support for both SQLite and PostgreSQL
- Rollback capabilities via `down()` migrations

#### 4.2.2 Schedule Management Service

**Responsibilities:**
- Parse and validate cron expressions using `cron::Schedule`
- Calculate next execution times based on cron expression and timezone
- Manage schedule lifecycle (create, update, enable/disable, delete)
- Persist schedule configuration to database
- Handle timezone conversions correctly

**Key Functions:**
```rust
use cron::Schedule as CronSchedule;
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use std::str::FromStr;

pub struct ScheduleService {
    db: Arc<DatabaseConnection>,
}

impl ScheduleService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    // Create or update schedule for a trigger
    pub async fn upsert_schedule(&self, config: ScheduleConfig) -> Result<Schedule> {
        // Validate cron expression first
        self.validate_cron(&config.cron_expression)?;

        // Calculate next execution time
        let next_execution = self.calculate_next_execution(
            &config.cron_expression,
            &config.timezone,
            Utc::now()
        )?;

        // Save to database using SeaORM...
    }

    // Get schedule for a workflow trigger
    pub async fn get_schedule(&self, workflow_id: &str, node_id: &str) -> Result<Option<Schedule>>;

    // Enable/disable schedule
    pub async fn set_enabled(&self, schedule_id: &str, enabled: bool) -> Result<()>;

    // Delete schedule
    pub async fn delete_schedule(&self, schedule_id: &str) -> Result<()>;

    // Validate cron expression using cron::Schedule
    pub fn validate_cron(&self, expression: &str) -> Result<()> {
        CronSchedule::from_str(expression)
            .map(|_| ())
            .map_err(|e| SwissPipeError::InvalidCronExpression(e.to_string()))
    }

    // Preview next N execution times using cron::Schedule
    pub fn preview_executions(
        &self,
        expression: &str,
        timezone: &str,
        count: usize
    ) -> Result<Vec<DateTime<Tz>>> {
        let cron_schedule = CronSchedule::from_str(expression)
            .map_err(|e| SwissPipeError::InvalidCronExpression(e.to_string()))?;

        let tz: Tz = timezone.parse()
            .map_err(|e| SwissPipeError::InvalidTimezone(timezone.to_string()))?;

        let executions: Vec<DateTime<Tz>> = cron_schedule
            .upcoming(tz)
            .take(count)
            .collect();

        Ok(executions)
    }

    // Calculate next execution time using cron::Schedule
    pub fn calculate_next_execution(
        &self,
        expression: &str,
        timezone: &str,
        after: DateTime<Utc>
    ) -> Result<DateTime<Utc>> {
        let cron_schedule = CronSchedule::from_str(expression)
            .map_err(|e| SwissPipeError::InvalidCronExpression(e.to_string()))?;

        let tz: Tz = timezone.parse()
            .map_err(|e| SwissPipeError::InvalidTimezone(timezone.to_string()))?;

        let after_in_tz = after.with_timezone(&tz);
        let next = cron_schedule
            .after(&after_in_tz)
            .next()
            .ok_or_else(|| SwissPipeError::NoNextExecution)?;

        Ok(next.with_timezone(&Utc))
    }
}
```

**Implementation Notes:**
- **MUST** use `cron::Schedule::from_str()` for parsing and validation
- **MUST** use `schedule.upcoming()` or `schedule.after()` for calculating next execution times
- Store all calculated times in UTC in database
- Convert to user's timezone only for display purposes
- Handle timezone parsing errors gracefully

#### 4.2.3 Scheduler Service

**Responsibilities:**
- Run as a background task/service
- Poll database for schedules due for execution
- Trigger workflow execution when schedule fires
- Update execution statistics and next execution time
- Handle failure cases and retries

**Architecture Options:**

**Option A: In-Process Scheduler (Recommended for MVP)**
- Tokio interval-based polling (every 30 seconds)
- Query database for schedules where `next_execution_time <= NOW() AND enabled = TRUE`
- Execute workflows asynchronously via existing execution pipeline
- Simple to implement, no additional infrastructure
- Limitation: Single instance only (no horizontal scaling)

**Option B: Distributed Scheduler (Future Enhancement)**
- Use advisory locks or distributed locks (Redis/PostgreSQL)
- Support multiple scheduler instances for high availability
- Leader election for active scheduler
- More complex but supports horizontal scaling

**MVP Implementation (Option A) - Using Tokio Timers:**
```rust
use cron::Schedule as CronSchedule;
use tokio::time::{sleep_until, Duration, Instant, interval};
use std::collections::HashMap;
use tokio::sync::RwLock;

pub struct CronSchedulerService {
    db: Arc<DatabaseConnection>,
    engine: Arc<WorkflowEngine>,
    schedule_service: Arc<ScheduleService>,
    // Track running schedule tasks (similar to DelayScheduler)
    schedule_tasks: Arc<RwLock<HashMap<String, tokio::task::JoinHandle<()>>>>,
}

impl CronSchedulerService {
    pub fn new(
        db: Arc<DatabaseConnection>,
        engine: Arc<WorkflowEngine>,
        schedule_service: Arc<ScheduleService>,
    ) -> Self {
        Self {
            db,
            engine,
            schedule_service,
            schedule_tasks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start the scheduler service
    pub async fn start(&self) -> Result<()> {
        // Restore schedules from database on startup
        self.restore_from_database().await?;

        // Polling loop for checking new/updated schedules
        let mut check_interval = interval(Duration::from_secs(30));
        loop {
            check_interval.tick().await;
            if let Err(e) = self.check_and_sync_schedules().await {
                tracing::error!("Error checking schedules: {}", e);
            }
        }
    }

    /// Restore active schedules from database on startup
    async fn restore_from_database(&self) -> Result<usize> {
        tracing::info!("Restoring cron schedules from database...");

        let now = Utc::now();

        // Load all enabled schedules from database
        let enabled_schedules = scheduled_triggers::Entity::find()
            .filter(scheduled_triggers::Column::Enabled.eq(true))
            .all(&*self.db)
            .await?;

        let mut restored_count = 0;
        let mut executed_count = 0;

        for schedule_record in enabled_schedules {
            // Check if within active date range
            if let Some(start_date) = schedule_record.start_date {
                if now < start_date {
                    continue; // Not yet active
                }
            }
            if let Some(end_date) = schedule_record.end_date {
                if now > end_date {
                    continue; // Expired
                }
            }

            let next_exec = schedule_record.next_execution_time;

            if let Some(next_time) = next_exec {
                // Check if execution was missed during downtime
                if next_time <= now {
                    // Missed execution - handle based on strategy
                    // Strategy 1: Skip and calculate next (recommended)
                    tracing::warn!(
                        "Missed scheduled execution for schedule {} (was due at {}), calculating next execution",
                        schedule_record.id,
                        next_time
                    );

                    // Calculate NEW next execution time from NOW
                    let cron_schedule = CronSchedule::from_str(&schedule_record.cron_expression)?;
                    let tz: Tz = schedule_record.timezone.parse()?;
                    let new_next = cron_schedule.upcoming(tz).next()
                        .ok_or_else(|| SwissPipeError::NoNextExecution)?;

                    // Update database with new next_execution_time
                    let mut schedule_update: scheduled_triggers::ActiveModel = schedule_record.clone().into();
                    schedule_update.next_execution_time = Set(Some(new_next.with_timezone(&Utc)));
                    schedule_update.update(&*self.db).await?;

                    // Schedule for new next execution
                    self.schedule_cron_execution(
                        schedule_record.id.clone(),
                        schedule_record.cron_expression.clone(),
                        schedule_record.timezone.clone(),
                        schedule_record.workflow_id.to_string(),
                        schedule_record.test_payload.clone(),
                    ).await?;

                    restored_count += 1;
                } else {
                    // Future execution - restore as-is
                    self.schedule_cron_execution(
                        schedule_record.id.clone(),
                        schedule_record.cron_expression.clone(),
                        schedule_record.timezone.clone(),
                        schedule_record.workflow_id.to_string(),
                        schedule_record.test_payload.clone(),
                    ).await?;

                    restored_count += 1;
                }
            } else {
                // No next_execution_time set - calculate first one
                let cron_schedule = CronSchedule::from_str(&schedule_record.cron_expression)?;
                let tz: Tz = schedule_record.timezone.parse()?;
                let next = cron_schedule.upcoming(tz).next()
                    .ok_or_else(|| SwissPipeError::NoNextExecution)?;

                // Update database
                let mut schedule_update: scheduled_triggers::ActiveModel = schedule_record.clone().into();
                schedule_update.next_execution_time = Set(Some(next.with_timezone(&Utc)));
                schedule_update.update(&*self.db).await?;

                // Schedule
                self.schedule_cron_execution(
                    schedule_record.id.clone(),
                    schedule_record.cron_expression.clone(),
                    schedule_record.timezone.clone(),
                    schedule_record.workflow_id.to_string(),
                    schedule_record.test_payload.clone(),
                ).await?;

                restored_count += 1;
            }
        }

        tracing::info!(
            "Cron schedule restoration complete: {} schedules restored, {} missed executions skipped",
            restored_count,
            executed_count
        );

        Ok(restored_count)
    }

    /// Schedule a single cron trigger using tokio (similar to DelayScheduler)
    async fn schedule_cron_execution(
        &self,
        schedule_id: String,
        cron_expr: String,
        timezone: String,
        workflow_id: String,
        payload: serde_json::Value,
    ) -> Result<()> {
        let cron_schedule = CronSchedule::from_str(&cron_expr)?;
        let tz: Tz = timezone.parse()?;

        // Calculate next execution time
        let next_exec = cron_schedule
            .upcoming(tz)
            .next()
            .ok_or_else(|| SwissPipeError::NoNextExecution)?;

        let now = Utc::now();
        let time_diff = next_exec.with_timezone(&Utc) - now;
        let duration_secs = time_diff.num_seconds().clamp(1, 86400 * 365) as u64;
        let sleep_duration = Duration::from_secs(duration_secs);
        let wake_time = Instant::now().checked_add(sleep_duration)?;

        let db_clone = self.db.clone();
        let engine_clone = self.engine.clone();
        let schedule_id_clone = schedule_id.clone();
        let schedule_tasks_for_cleanup = self.schedule_tasks.clone();

        // Spawn tokio task (same pattern as DelayScheduler)
        let task = tokio::spawn(async move {
            sleep_until(wake_time).await;

            // Execute workflow
            if let Err(e) = Self::execute_scheduled_workflow(
                db_clone,
                engine_clone,
                schedule_id_clone.clone(),
                workflow_id,
                payload,
            ).await {
                tracing::error!("Failed to execute scheduled workflow: {}", e);
            }

            // Cleanup task handle
            schedule_tasks_for_cleanup.write().await.remove(&schedule_id_clone);
        });

        // Store task handle for cancellation
        self.schedule_tasks.write().await.insert(schedule_id, task);
        Ok(())
    }

    /// Execute scheduled workflow and automatically reschedule next execution
    async fn execute_scheduled_workflow(
        db: Arc<DatabaseConnection>,
        engine: Arc<WorkflowEngine>,
        schedule_id: String,
        workflow_id: String,
        payload: serde_json::Value,
        cron_expr: String,
        timezone: String,
        schedule_service: Arc<ScheduleService>,
    ) -> Result<()> {
        tracing::info!("Executing scheduled workflow for schedule {}", schedule_id);

        let now = Utc::now();

        // 1. Execute workflow via job queue
        let event = WorkflowEvent {
            data: payload,
            metadata: HashMap::from([
                ("triggered_by".to_string(), "cron_schedule".to_string()),
                ("schedule_id".to_string(), schedule_id.clone()),
                ("scheduled_time".to_string(), now.to_rfc3339()),
            ]),
            headers: HashMap::new(),
            condition_results: HashMap::new(),
        };

        match engine.execute_workflow(&workflow_id, event).await {
            Ok(_) => {
                // Success - increment execution_count
                let mut schedule = scheduled_triggers::Entity::find_by_id(&schedule_id)
                    .one(&*db)
                    .await?
                    .ok_or_else(|| SwissPipeError::Generic(format!("Schedule not found: {}", schedule_id)))?;

                let mut schedule_update: scheduled_triggers::ActiveModel = schedule.clone().into();
                schedule_update.execution_count = Set(schedule.execution_count + 1);
                schedule_update.last_execution_time = Set(Some(now));
                schedule_update.update(&*db).await?;
            }
            Err(e) => {
                // Failure - increment failure_count
                tracing::error!("Scheduled workflow execution failed: {}", e);

                let mut schedule = scheduled_triggers::Entity::find_by_id(&schedule_id)
                    .one(&*db)
                    .await?
                    .ok_or_else(|| SwissPipeError::Generic(format!("Schedule not found: {}", schedule_id)))?;

                let mut schedule_update: scheduled_triggers::ActiveModel = schedule.clone().into();
                schedule_update.failure_count = Set(schedule.failure_count + 1);
                schedule_update.last_execution_time = Set(Some(now));
                schedule_update.update(&*db).await?;
            }
        }

        // 2. Calculate NEXT execution time (CRITICAL for recurring schedules)
        let next_exec = schedule_service.calculate_next_execution(&cron_expr, &timezone, now)?;

        // 3. Update database with next execution time
        let mut schedule = scheduled_triggers::Entity::find_by_id(&schedule_id)
            .one(&*db)
            .await?
            .ok_or_else(|| SwissPipeError::Generic(format!("Schedule not found: {}", schedule_id)))?;

        let mut schedule_update: scheduled_triggers::ActiveModel = schedule.into();
        schedule_update.next_execution_time = Set(Some(next_exec));
        schedule_update.updated_at = Set(now);
        schedule_update.update(&*db).await?;

        // 4. Automatically spawn new tokio task for NEXT execution (recurring behavior)
        let time_diff = next_exec - now;
        let duration_secs = time_diff.num_seconds().clamp(1, 86400 * 365) as u64;
        let sleep_duration = Duration::from_secs(duration_secs);
        let wake_time = Instant::now().checked_add(sleep_duration)?;

        let db_clone = db.clone();
        let engine_clone = engine.clone();
        let schedule_id_clone = schedule_id.clone();
        let schedule_service_clone = schedule_service.clone();

        tokio::spawn(async move {
            sleep_until(wake_time).await;

            // Recursively execute and reschedule
            if let Err(e) = Self::execute_scheduled_workflow(
                db_clone,
                engine_clone,
                schedule_id_clone,
                workflow_id,
                payload,
                cron_expr,
                timezone,
                schedule_service_clone,
            ).await {
                tracing::error!("Failed to execute/reschedule workflow: {}", e);
            }
        });

        tracing::info!(
            "Scheduled workflow executed and rescheduled for next execution at {}",
            next_exec
        );

        Ok(())
    }

    /// Check database for new/updated schedules
    async fn check_and_sync_schedules(&self) -> Result<()> {
        // Query for schedules that need to be added/updated
        // Compare with currently running tasks
        // Spawn new tasks or cancel removed ones
    }

    /// Cancel a schedule
    pub async fn cancel_schedule(&self, schedule_id: &str) -> Result<()> {
        if let Some(handle) = self.schedule_tasks.write().await.remove(schedule_id) {
            handle.abort();
        }
        Ok(())
    }

    /// Shutdown scheduler
    pub async fn shutdown(&self) -> Result<()> {
        let mut tasks = self.schedule_tasks.write().await;
        for (schedule_id, handle) in tasks.drain() {
            handle.abort();
            tracing::debug!("Cancelled schedule task: {}", schedule_id);
        }
        Ok(())
    }
}
```

**Scheduler Architecture (Mirrors DelayScheduler):**
1. Use `tokio::time::interval()` for 30-second polling to check for new/updated schedules
2. For each active schedule:
   - Calculate next execution using `cron::Schedule`
   - Spawn tokio task with `sleep_until()` (same as Delay node)
   - Store task handle in HashMap for cancellation
3. On each wake:
   - Execute workflow via job queue
   - Update statistics in database
   - Calculate next execution time
   - Reschedule automatically by spawning new task
4. Handle startup restoration (similar to `DelayScheduler::restore_from_database()`)
5. Support cancellation via task handle abort

#### 4.2.4 API Endpoints

**Create/Update Schedule**
```
PUT /api/v1/workflows/{workflow_id}/triggers/{node_id}/schedule
{
  "schedule_name": "Daily Data Sync",
  "cron_expression": "0 2 * * *",
  "timezone": "UTC",
  "test_payload": {"source": "scheduled"},
  "enabled": true,
  "start_date": "2025-10-22T00:00:00Z",
  "end_date": null
}
```

**Get Schedule**
```
GET /api/v1/workflows/{workflow_id}/triggers/{node_id}/schedule
```

**Enable/Disable Schedule**
```
PATCH /api/v1/workflows/{workflow_id}/triggers/{node_id}/schedule
{
  "enabled": false
}
```

**Delete Schedule**
```
DELETE /api/v1/workflows/{workflow_id}/triggers/{node_id}/schedule
```

**Validate Cron Expression**
```
POST /api/v1/schedules/validate
{
  "cron_expression": "0 2 * * *",
  "timezone": "America/New_York"
}
Response:
{
  "valid": true,
  "next_executions": [
    "2025-10-22T02:00:00-04:00",
    "2025-10-23T02:00:00-04:00",
    ...
  ]
}
```

---

### 4.3 Frontend Implementation

#### 4.3.1 TriggerConfig Component Updates
- Add new tab "Cron/Schedule"
- Create `ScheduleConfig.vue` component for schedule configuration
- Integrate with existing TriggerConfig workflow

#### 4.3.2 ScheduleConfig Component Structure
```vue
<template>
  <div class="space-y-4">
    <!-- Enable/Disable Toggle -->
    <div class="flex items-center justify-between">
      <label>Schedule Enabled</label>
      <toggle v-model="config.enabled" />
    </div>

    <!-- Quick Presets -->
    <div>
      <label>Quick Presets</label>
      <select @change="applyPreset">
        <option>Every 5 minutes</option>
        <option>Hourly</option>
        <option>Daily at midnight</option>
        ...
      </select>
    </div>

    <!-- Cron Expression -->
    <div>
      <label>Cron Expression</label>
      <input v-model="config.cron_expression" @input="validateCron" />
      <p class="error" v-if="validationError">{{ validationError }}</p>
    </div>

    <!-- Timezone -->
    <div>
      <label>Timezone</label>
      <select v-model="config.timezone">
        <option value="UTC">UTC</option>
        <option value="America/New_York">America/New_York</option>
        ...
      </select>
    </div>

    <!-- Next Execution Preview -->
    <div v-if="nextExecutions.length">
      <label>Next Scheduled Executions</label>
      <ul>
        <li v-for="time in nextExecutions">{{ time }}</li>
      </ul>
    </div>

    <!-- Test Payload -->
    <div>
      <label>Scheduled Execution Payload</label>
      <CodeEditor v-model="config.test_payload" language="json" />
    </div>

    <!-- Optional Dates -->
    <div>
      <label>Start Date (Optional)</label>
      <input type="datetime-local" v-model="config.start_date" />
    </div>

    <div>
      <label>End Date (Optional)</label>
      <input type="datetime-local" v-model="config.end_date" />
    </div>

    <!-- Schedule Status -->
    <div v-if="existingSchedule">
      <h4>Schedule Status</h4>
      <p>Last Execution: {{ existingSchedule.last_execution_time }}</p>
      <p>Next Execution: {{ existingSchedule.next_execution_time }}</p>
      <p>Total Executions: {{ existingSchedule.execution_count }}</p>
      <p>Success Rate: {{ successRate }}%</p>
    </div>

    <!-- Save Button -->
    <button @click="saveSchedule">Save Schedule</button>
  </div>
</template>
```

#### 4.3.3 Type Definitions
```typescript
interface ScheduleConfig {
  schedule_name?: string
  cron_expression: string
  timezone: string
  test_payload: any
  enabled: boolean
  start_date?: string
  end_date?: string
}

interface Schedule extends ScheduleConfig {
  id: string
  workflow_id: string
  trigger_node_id: string
  last_execution_time?: string
  next_execution_time?: string
  execution_count: number
  failure_count: number
  created_at: string
  updated_at: string
}
```

---

## 5. Non-Functional Requirements

### 5.1 Performance
- Scheduler should check for due schedules every 30 seconds
- Execution latency: < 60 seconds from scheduled time
- Support 1000+ concurrent scheduled workflows
- Database queries for due schedules: < 100ms

### 5.2 Reliability
- No missed executions under normal operation
- Graceful handling of scheduler restarts
- Automatic recovery from transient failures
- At-least-once execution guarantee (not exactly-once initially)

### 5.3 Scalability
- MVP: Single scheduler instance
- Future: Support horizontal scaling with distributed locks
- Efficient database queries using indexes
- Async execution to prevent blocking

### 5.4 Security
- Schedule management requires authentication
- Only workflow owner can create/modify schedules
- Audit log for schedule changes
- Rate limiting on schedule creation

### 5.5 Observability
- Log all schedule executions
- Metrics: execution count, failure rate, latency
- Alerting for consecutive failures
- UI visibility into schedule health

---

## 6. Technical Considerations

### 6.1 Libraries and Dependencies
**Required: Use Same Libraries as Delay Node**

The cron scheduler MUST use the same library stack as the existing Delay node implementation:

**Cron Expression Parsing:**
- **`cron::Schedule`** (https://crates.io/crates/cron) - **REQUIRED** for parsing and calculating execution times
- Standard cron expression parsing (5-field format: `minute hour day month weekday`)
- Iterator-based API for calculating upcoming execution times

**Date/Time and Timezone Handling:**
- **`chrono`** - Already in use, for date/time operations
- **`chrono-tz`** - Already in use, for timezone support
- Store all times in UTC, convert to user timezone for display

**Async Scheduling (Same as Delay Node):**
- **`tokio::time::sleep_until()`** - For actual scheduling/waiting
- **`tokio::time::Instant`** - For calculating wake times
- **`tokio::time::Duration`** - For time duration calculations
- **`tokio::spawn()`** - For spawning async tasks

**Implementation Requirements:**
```rust
use cron::Schedule as CronSchedule;
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use tokio::time::{sleep_until, Duration, Instant};
use std::str::FromStr;

// Parse cron expression
let schedule = CronSchedule::from_str("0 2 * * *")?;

// Get next execution time in UTC
let next_utc = schedule.upcoming(Utc).next();

// Get next execution time in specific timezone
let tz: Tz = "America/New_York".parse()?;
let next_in_tz = schedule.upcoming(tz).next();

// Get next N execution times for preview
let next_5: Vec<DateTime<Utc>> = schedule
    .upcoming(Utc)
    .take(5)
    .collect();

// Schedule execution using tokio (same as Delay node)
let time_diff = next_utc - Utc::now();
let duration_secs = time_diff.num_seconds().clamp(1, 86400 * 365) as u64;
let sleep_duration = Duration::from_secs(duration_secs);
let wake_time = Instant::now().checked_add(sleep_duration)?;

tokio::spawn(async move {
    sleep_until(wake_time).await;
    // Execute workflow
});
```

**Cargo Dependencies:**
```toml
[dependencies]
cron = "0.12"           # Cron expression parsing (NEW - REQUIRED)
chrono = "0.4"          # Date/time handling (already in use)
chrono-tz = "0.8"       # Timezone support (already in use)
tokio = { version = "1", features = ["time", "rt-multi-thread"] }  # Async runtime & timers (already in use)
```

**Architecture Alignment:**
- Follow the same patterns as `src/async_execution/delay_scheduler.rs`
- Use tokio tasks with stored handles for cancellation support
- Persist state to database for recovery after restarts
- Handle overflow protection with `checked_add()` and duration clamping

**Validation:**
- Use `CronSchedule::from_str()` for validation - returns `Result<Schedule, Error>`
- Invalid expressions fail parsing with descriptive error messages
- Frontend should call validation API before saving

### 6.2 Timezone Handling
- Use `chrono-tz` crate for timezone support
- Store all times in UTC in database
- Convert to user's timezone for display
- Handle DST transitions correctly

### 6.3 Missed Execution Handling
**Scenarios:**
1. Scheduler was down during scheduled time
2. Execution took longer than interval
3. Database was unavailable

**Strategy:**
- Track `last_execution_time` to detect missed executions
- For missed executions: either skip or execute immediately (configurable)
- Log warning for missed executions
- Consider max_concurrent_executions limit per schedule

### 6.4 Concurrent Execution Prevention
- Use database transaction with SELECT FOR UPDATE
- Check if previous execution is still running
- Skip execution if previous one hasn't completed (configurable)

### 6.5 System Restart and Recovery Behavior

**Critical Difference from Delay Node:**
- Delay nodes are **one-time** executions (execute once, then done)
- Cron schedules are **recurring** executions (execute repeatedly forever)

**Restart Recovery Process:**

1. **On Startup** - `CronSchedulerService::restore_from_database()` runs automatically:
   - Query all enabled schedules from database
   - Check `next_execution_time` for each schedule
   - Handle three scenarios:
     - **Future execution**: Reschedule tokio task for future time
     - **Missed execution**: Skip missed run, calculate NEW next time from now, schedule for that
     - **No next_execution_time**: Calculate first execution time and schedule

2. **Missed Execution Strategy** (during downtime):
   - **Default**: Skip missed executions, schedule for next occurrence
   - Example: If daily 2 AM job was missed, don't run it at 10 AM, wait until next 2 AM
   - Rationale: Prevents avalanche of backlogged executions
   - Alternative strategy (Phase 2): Configurable "execute immediately if missed"

3. **Automatic Rescheduling** (key to persistence):
   - After EVERY execution, automatically:
     - Calculate next execution time using `cron::Schedule`
     - Update `next_execution_time` in database
     - Spawn new tokio task for next occurrence
   - This creates self-perpetuating cycle
   - Database always has next execution time for recovery

4. **State Persistence:**
   - All schedule config persisted in `scheduled_triggers` table
   - `next_execution_time` always up-to-date after each execution
   - Enables full recovery even after weeks of downtime
   - In-memory tokio tasks rebuilt from database on startup

**Example Scenario:**
```
1. Schedule created: "0 2 * * *" (daily at 2 AM)
2. Executes at 2 AM, updates next_execution_time to tomorrow 2 AM, spawns task
3. System restarts at 10 AM
4. restore_from_database() finds next_execution_time = tomorrow 2 AM
5. Spawns new tokio task to sleep until tomorrow 2 AM
6. Life continues normally
```

**Guarantees:**
- ✅ Schedules survive system restarts
- ✅ No duplicate executions
- ✅ No lost schedules
- ⚠️ Missed executions during downtime are skipped by default (configurable in Phase 2)

**Integration in `main.rs`:**
```rust
// In main.rs startup sequence
#[tokio::main]
async fn main() -> Result<()> {
    // ... existing initialization ...

    // Initialize scheduler services
    let schedule_service = Arc::new(ScheduleService::new(db.clone()));
    let cron_scheduler = Arc::new(CronSchedulerService::new(
        db.clone(),
        engine.clone(),
        schedule_service.clone(),
    ));

    // Spawn scheduler service as background task
    let scheduler_handle = tokio::spawn({
        let scheduler = cron_scheduler.clone();
        async move {
            if let Err(e) = scheduler.start().await {
                tracing::error!("Cron scheduler failed: {}", e);
            }
        }
    });

    // ... rest of application startup ...

    // Graceful shutdown
    tokio::select! {
        _ = signal::ctrl_c() => {
            tracing::info!("Shutting down...");
            cron_scheduler.shutdown().await?;
            scheduler_handle.abort();
        }
    }

    Ok(())
}
```

### 6.6 Data Migration
- SeaORM migration to add `scheduled_triggers` table
- No impact on existing workflows
- Opt-in feature (workflows without schedule continue as HTTP-only)

---

## 7. Risks & Mitigation

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| Scheduler crashes, missing executions | High | Medium | Implement health checks, restart mechanism, logging |
| Clock skew causing incorrect scheduling | High | Low | Use UTC, validate system time, add tolerance window |
| High volume causing performance degradation | Medium | Medium | Implement rate limiting, optimize queries, add indexing |
| Timezone calculation errors | Medium | Medium | Extensive testing, use well-tested library (`chrono-tz`) |
| Database lock contention | Medium | Low | Use advisory locks, optimize transaction scope |
| Concurrent modifications to schedule | Low | Low | Optimistic locking with version field |

---

## 8. Testing Strategy

### 8.1 Unit Tests
- Cron expression parsing and validation
- Next execution time calculation
- Timezone conversions
- Edge cases: leap years, DST transitions, end-of-month

### 8.2 Integration Tests
- Schedule creation/update/deletion via API
- Scheduled workflow execution end-to-end
- Database persistence and retrieval
- Scheduler service lifecycle

### 8.3 Load Tests
- 1000+ concurrent schedules
- High-frequency schedules (every minute)
- Database query performance under load

### 8.4 Scenario Tests
- Scheduler restart during execution
- Missed execution handling
- Concurrent execution prevention
- Start/end date boundaries
- Timezone edge cases

---

## 9. Implementation Phases

### Phase 1: MVP (2-3 weeks)
**Backend:**
- [ ] Database migration for `scheduled_triggers` table (SeaORM)
- [ ] Entity definition in `src/database/entities/scheduled_triggers.rs`
- [ ] `ScheduleService` implementation in `src/schedule/service.rs`
- [ ] `CronSchedulerService` implementation in `src/schedule/scheduler.rs`
- [ ] Integration in `main.rs` (spawn scheduler service on startup)
- [ ] API endpoints for schedule management in `src/api/schedules/`
- [ ] Add `cron` crate dependency to `Cargo.toml`

**Frontend:**
- [ ] "Cron/Schedule" tab in TriggerConfig
- [ ] Basic schedule configuration UI
- [ ] Cron expression input with validation
- [ ] Timezone selector
- [ ] Quick presets
- [ ] Next execution preview

**Testing:**
- [ ] Unit tests for schedule calculation
- [ ] Integration tests for schedule API
- [ ] Manual end-to-end testing

### Phase 2: Enhanced Features (1-2 weeks)
- [ ] Visual cron builder (dropdown-based)
- [ ] Schedule history and execution statistics
- [ ] Missed execution handling strategies
- [ ] Concurrent execution prevention
- [ ] Enhanced error handling and retry logic
- [ ] Comprehensive observability (metrics, logs)

### Phase 3: Production Hardening (1 week)
- [ ] Load testing and optimization
- [ ] High availability considerations
- [ ] Monitoring and alerting setup
- [ ] Documentation (user guide, API docs)
- [ ] Production deployment and rollout

### Phase 4: Advanced Features (Future)
- [ ] Distributed scheduler with leader election
- [ ] Advanced schedule patterns (e.g., "last Friday of month")
- [ ] Schedule dependency chains
- [ ] Jitter and randomization for load distribution
- [ ] Schedule templates and presets library

---

## 10. Success Criteria

### MVP Launch Criteria
- [ ] Users can create cron-based schedules via UI
- [ ] Schedules execute within 60 seconds of scheduled time
- [ ] Schedule enable/disable functionality works correctly
- [ ] Next execution time preview is accurate
- [ ] No data loss or corruption during schedule operations
- [ ] Documentation covers basic usage scenarios

### Post-Launch Success Metrics (30 days)
- 80% of scheduled workflows execute successfully
- < 1% missed executions under normal operation
- Average user setup time < 3 minutes
- Positive user feedback on usability
- Zero critical bugs related to scheduling

---

## 11. Open Questions & Decisions Needed

1. **Missed Execution Strategy**: Should we execute immediately or skip missed schedules?
   - **Recommendation**: Skip by default, add option to execute immediately

2. **Concurrent Execution**: Allow or prevent concurrent executions of same schedule?
   - **Recommendation**: Prevent by default, add option to allow

3. **Schedule Limits**: Maximum schedules per workflow? Per system?
   - **Recommendation**: No limit per workflow, monitor system-wide load

4. **Execution Timeout**: Should scheduled executions have different timeout than HTTP triggers?
   - **Recommendation**: Use same timeout settings from existing workflow configuration

5. **Payload Injection**: Auto-inject schedule metadata (e.g., `scheduled_time`) into payload?
   - **Recommendation**: Yes, add metadata to `event.metadata` with keys: `triggered_by`, `schedule_id`, `scheduled_time`
   - This follows existing pattern from HTTP triggers

6. **UI Location**: Should schedule config be in Trigger node or separate Schedule node type?
   - **Recommendation**: Keep in Trigger node with new tab (simpler, less breaking change)

7. **Multiple Schedules**: Support multiple schedules per trigger in MVP?
   - **Recommendation**: No, one schedule per trigger for MVP, enhance later

---

## 12. Alternatives Considered

### Alternative 1: External Scheduler (e.g., Kubernetes CronJob)
**Pros:**
- Battle-tested infrastructure
- No custom scheduler implementation
- Horizontal scaling built-in

**Cons:**
- Requires Kubernetes
- Less control over scheduling logic
- More complex deployment
- Difficult to manage many schedules dynamically

**Decision**: Not suitable for MVP, consider for enterprise deployment

### Alternative 2: Message Queue Based Scheduler (e.g., with Redis)
**Pros:**
- Better support for distributed systems
- Built-in retry and failure handling
- Horizontal scaling

**Cons:**
- Additional infrastructure dependency
- More complex to implement
- Potential queue management overhead

**Decision**: Good for future enhancement, too complex for MVP

### Alternative 3: Trigger as Separate Node Type
**Pros:**
- Cleaner separation of concerns
- Multiple trigger types per workflow

**Cons:**
- Breaking change to existing workflows
- More complex UI/UX
- Requires workflow model changes

**Decision**: Rejected, keep schedule as part of existing Trigger node

---

## 13. Documentation Requirements

### User Documentation
- Getting started guide for scheduled workflows
- Cron expression syntax reference
- Timezone configuration best practices
- Common scheduling patterns and examples
- Troubleshooting guide

### Developer Documentation
- Architecture overview of scheduler service
- API reference for schedule endpoints
- Database schema documentation
- Cron calculation logic explanation
- Testing guide for scheduled workflows

### Operational Documentation
- Deployment and configuration guide
- Monitoring and alerting setup
- Performance tuning recommendations
- Disaster recovery procedures

---

## 14. Appendix

### A. Cron Expression Format Reference
```
┌───────────── minute (0 - 59)
│ ┌───────────── hour (0 - 23)
│ │ ┌───────────── day of month (1 - 31)
│ │ │ ┌───────────── month (1 - 12)
│ │ │ │ ┌───────────── day of week (0 - 6) (Sunday to Saturday)
│ │ │ │ │
│ │ │ │ │
* * * * *
```

**Special Characters:**
- `*` : Any value
- `,` : Value list separator (e.g., `1,3,5`)
- `-` : Range of values (e.g., `1-5`)
- `/` : Step values (e.g., `*/5` = every 5 units)

**Examples:**
- `0 0 * * *` - Daily at midnight UTC
- `*/15 * * * *` - Every 15 minutes
- `0 9-17 * * 1-5` - Every hour 9 AM to 5 PM, Monday to Friday
- `0 0 1 * *` - First day of every month at midnight
- `0 0 * * 0` - Every Sunday at midnight

### B. Timezone Considerations
- Always store times in UTC in database
- Use IANA timezone database for all timezone operations
- Handle DST transitions automatically via `chrono-tz`
- Display times in user's preferred timezone
- Warn users about ambiguous times during DST transitions

### C. Similar Products Reference
- **GitHub Actions**: Cron-based workflow triggers
- **Apache Airflow**: DAG scheduling with cron
- **AWS EventBridge**: Scheduled events
- **Zapier**: Schedule trigger with simple UI
- **n8n**: Cron node for workflow automation

---

## Sign-off

**Prepared by**: Claude (AI Assistant)
**Stakeholders**: SwissPipe Product Team
**Next Steps**: Review PRD, gather feedback, finalize requirements, begin implementation planning

---

## Change Log

| Date | Version | Changes | Author |
|------|---------|---------|--------|
| 2025-10-21 | 1.0 | Initial draft | Claude |
