use cron::Schedule as CronSchedule;
use chrono::Utc;
use chrono_tz::Tz;
use sea_orm::{DatabaseConnection, Set, ActiveModelTrait};
use std::collections::HashMap;
use std::sync::Arc;
use std::str::FromStr;
use tokio::sync::RwLock;
use tokio::time::{sleep_until, Duration, Instant, interval};

use crate::database::scheduled_cron_triggers;
use crate::workflow::errors::{SwissPipeError, Result};
use crate::schedule::service::ScheduleService;
use crate::async_execution::execution_service::ExecutionService;

// Schedule validation constants
const SECONDS_PER_DAY: i64 = 86400;
const DAYS_PER_YEAR: i64 = 365;
const MAX_SCHEDULE_DURATION_SECS: i64 = SECONDS_PER_DAY * DAYS_PER_YEAR; // 1 year

// Default sync interval
const DEFAULT_SYNC_INTERVAL_SECS: u64 = 30;

// Circuit breaker constants
// TODO: Implement circuit breaker pattern to automatically disable schedules that fail repeatedly
// The circuit breaker should:
// 1. Track consecutive failures per schedule in failure_tracking HashMap
// 2. After MAX_CONSECUTIVE_FAILURES, automatically disable the schedule
// 3. Reset failure count after CIRCUIT_BREAKER_RESET_MINUTES of successful executions
// 4. Log warnings when approaching the failure threshold
// 5. Notify admins when a schedule is auto-disabled
#[allow(dead_code)]
const MAX_CONSECUTIVE_FAILURES: u32 = 5;
#[allow(dead_code)]
const CIRCUIT_BREAKER_RESET_MINUTES: u32 = 30;

// Type alias for circuit breaker tracking
type FailureTracker = Arc<RwLock<HashMap<String, (u32, chrono::DateTime<Utc>)>>>;

pub struct CronSchedulerService {
    db: Arc<DatabaseConnection>,
    schedule_service: Arc<ScheduleService>,
    // Track running schedule tasks (similar to DelayScheduler)
    schedule_tasks: Arc<RwLock<HashMap<String, tokio::task::JoinHandle<()>>>>,
    // Track schedule configurations to detect changes
    schedule_configs: Arc<RwLock<HashMap<String, (String, String)>>>, // schedule_id -> (cron_expr, timezone)
    // Circuit breaker: Track consecutive failures and last failure time
    #[allow(dead_code)] // Reserved for future circuit breaker implementation
    failure_tracking: FailureTracker, // schedule_id -> (consecutive_failures, last_failure_time)
}

impl CronSchedulerService {
    pub fn new(
        db: Arc<DatabaseConnection>,
        schedule_service: Arc<ScheduleService>,
    ) -> Result<Self> {
        Ok(Self {
            db,
            schedule_service,
            schedule_tasks: Arc::new(RwLock::new(HashMap::new())),
            schedule_configs: Arc::new(RwLock::new(HashMap::new())),
            failure_tracking: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Start the scheduler service
    pub async fn start(self: Arc<Self>) -> Result<()> {
        tracing::info!("Starting Cron Scheduler Service...");

        // Restore schedules from database on startup
        self.restore_from_database().await?;

        // Get sync interval from environment variable or use default
        let sync_interval_secs = std::env::var("SP_SCHEDULE_SYNC_INTERVAL_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(DEFAULT_SYNC_INTERVAL_SECS);

        tracing::info!("Using schedule sync interval: {} seconds", sync_interval_secs);

        // Polling loop for checking new/updated schedules
        let mut check_interval = interval(Duration::from_secs(sync_interval_secs));

        loop {
            // Periodic sync loop handles all schedule management
            check_interval.tick().await;
            if let Err(e) = self.check_and_sync_schedules().await {
                tracing::error!("Error checking schedules: {}", e);
            }
        }
    }

    /// Restore active schedules from database on startup
    pub async fn restore_from_database(&self) -> Result<usize> {
        tracing::info!("Restoring cron schedules from database...");

        let now = Utc::now();

        // Load all enabled schedules from database
        let enabled_schedules = self.schedule_service.get_enabled_schedules().await?;

        let mut restored_count = 0;

        for schedule_record in enabled_schedules {
            // Check if within active date range
            if let Some(start_date) = schedule_record.start_date {
                if now < start_date {
                    tracing::debug!("Schedule {} not yet active, skipping", schedule_record.id);
                    continue; // Not yet active
                }
            }
            if let Some(end_date) = schedule_record.end_date {
                if now > end_date {
                    tracing::debug!("Schedule {} expired, skipping", schedule_record.id);
                    continue; // Expired
                }
            }

            let next_exec = schedule_record.next_execution_time;

            if let Some(next_time) = next_exec {
                // Check if execution was missed during downtime
                if next_time <= now {
                    // Missed execution - skip and calculate next
                    tracing::warn!(
                        "Missed scheduled execution for schedule {} (was due at {}), calculating next execution",
                        schedule_record.id,
                        next_time
                    );

                    // Calculate NEW next execution time from NOW
                    match self.schedule_service.calculate_next_execution(
                        &schedule_record.cron_expression,
                        &schedule_record.timezone,
                        now,
                    ) {
                        Ok(new_next) => {
                            // Update database with new next_execution_time
                            let mut schedule_update: scheduled_cron_triggers::ActiveModel = schedule_record.clone().into();
                            schedule_update.next_execution_time = Set(Some(new_next));
                            if let Err(e) = schedule_update.update(&*self.db).await {
                                tracing::error!("Failed to update schedule {}: {}", schedule_record.id, e);
                                continue;
                            }

                            // Schedule for new next execution
                            if let Err(e) = self.schedule_cron_execution(
                                schedule_record.id.to_string(),
                                schedule_record.cron_expression.clone(),
                                schedule_record.timezone.clone(),
                                schedule_record.workflow_id,
                                schedule_record.test_payload.clone(),
                            ).await {
                                tracing::error!("Failed to schedule execution for {}: {}", schedule_record.id, e);
                                continue;
                            }

                            restored_count += 1;
                        }
                        Err(e) => {
                            tracing::error!("Failed to calculate next execution for {}: {}", schedule_record.id, e);
                            continue;
                        }
                    }
                } else {
                    // Future execution - restore as-is
                    if let Err(e) = self.schedule_cron_execution(
                        schedule_record.id.to_string(),
                        schedule_record.cron_expression.clone(),
                        schedule_record.timezone.clone(),
                        schedule_record.workflow_id,
                        schedule_record.test_payload.clone(),
                    ).await {
                        tracing::error!("Failed to restore schedule {}: {}", schedule_record.id, e);
                        continue;
                    }

                    restored_count += 1;
                }
            } else {
                // No next_execution_time set - calculate first one
                match self.schedule_service.calculate_next_execution(
                    &schedule_record.cron_expression,
                    &schedule_record.timezone,
                    now,
                ) {
                    Ok(next) => {
                        // Update database
                        let mut schedule_update: scheduled_cron_triggers::ActiveModel = schedule_record.clone().into();
                        schedule_update.next_execution_time = Set(Some(next));
                        if let Err(e) = schedule_update.update(&*self.db).await {
                            tracing::error!("Failed to update schedule {}: {}", schedule_record.id, e);
                            continue;
                        }

                        // Schedule
                        if let Err(e) = self.schedule_cron_execution(
                            schedule_record.id.to_string(),
                            schedule_record.cron_expression.clone(),
                            schedule_record.timezone.clone(),
                            schedule_record.workflow_id,
                            schedule_record.test_payload.clone(),
                        ).await {
                            tracing::error!("Failed to schedule execution for {}: {}", schedule_record.id, e);
                            continue;
                        }

                        restored_count += 1;
                    }
                    Err(e) => {
                        tracing::error!("Failed to calculate first execution for {}: {}", schedule_record.id, e);
                        continue;
                    }
                }
            }
        }

        tracing::info!(
            "Cron schedule restoration complete: {} schedules restored",
            restored_count
        );

        Ok(restored_count)
    }

    /// Schedule a single cron trigger using tokio (similar to DelayScheduler)
    /// Returns the schedule_id for tracking
    async fn schedule_cron_execution(
        &self,
        schedule_id: String,
        cron_expr: String,
        timezone: String,
        workflow_id: uuid::Uuid,
        payload: serde_json::Value,
    ) -> Result<String> {
        let cron_schedule = CronSchedule::from_str(&cron_expr)
            .map_err(|e| SwissPipeError::Generic(format!("Invalid cron expression: {e}")))?;

        let tz: Tz = timezone.parse()
            .map_err(|_| SwissPipeError::Generic(format!("Invalid timezone: {timezone}")))?;

        // Calculate next execution time
        let next_exec = cron_schedule
            .upcoming(tz)
            .next()
            .ok_or_else(|| SwissPipeError::Generic("No next execution time".to_string()))?;

        let now = Utc::now();
        let time_diff = next_exec.with_timezone(&Utc) - now;
        let duration_secs = time_diff.num_seconds();

        tracing::debug!(
            "Scheduling: schedule_id={}, cron={}, next_exec={}, now={}, duration_secs={}",
            schedule_id,
            cron_expr,
            next_exec.with_timezone(&Utc),
            now,
            duration_secs
        );

        // Validate duration is reasonable
        if duration_secs < 0 {
            tracing::error!(
                "Negative duration detected: schedule_id={}, duration_secs={}, next_exec={}, now={}",
                schedule_id,
                duration_secs,
                next_exec.with_timezone(&Utc),
                now
            );
            return Err(SwissPipeError::Generic(
                format!("Calculated negative duration: {duration_secs} seconds")
            ));
        }
        if duration_secs > MAX_SCHEDULE_DURATION_SECS {
            return Err(SwissPipeError::Generic(
                format!("Schedule too far in future: {duration_secs} seconds (max 1 year)")
            ));
        }

        let sleep_duration = Duration::from_secs(duration_secs as u64);

        let wake_time = Instant::now().checked_add(sleep_duration)
            .ok_or_else(|| SwissPipeError::Generic("Duration overflow".to_string()))?;

        let db_clone = self.db.clone();
        let schedule_service_clone = self.schedule_service.clone();
        let schedule_id_clone = schedule_id.clone();
        let schedule_tasks_for_cleanup = self.schedule_tasks.clone();

        // Clone for config tracking (needed before moving into async block)
        let cron_expr_for_config = cron_expr.clone();
        let timezone_for_config = timezone.clone();

        // Spawn tokio task (same pattern as DelayScheduler)
        let task = tokio::spawn(async move {
            sleep_until(wake_time).await;

            tracing::info!("Executing scheduled workflow for schedule {}", schedule_id_clone);

            // Execute workflow and reschedule
            if let Err(e) = Self::execute_scheduled_workflow(
                db_clone,
                schedule_id_clone.clone(),
                workflow_id,
                payload.clone(),
                cron_expr.clone(),
                timezone.clone(),
                schedule_service_clone.clone(),
            ).await {
                tracing::error!("Failed to queue scheduled workflow: {}", e);
            }

            // Cleanup task handle - the sync loop will detect the missing task and reschedule it
            schedule_tasks_for_cleanup.write().await.remove(&schedule_id_clone);

            // Note: We rely on the periodic sync loop (every 30 seconds) to detect that this
            // schedule no longer has a running task and reschedule it. This avoids race conditions
            // where both the channel-based reschedule and sync loop create duplicate tasks.
        });

        // Check if task already exists - if so, don't create a duplicate
        // IMPORTANT: Hold locks for both tasks AND configs to prevent race conditions
        let mut tasks = self.schedule_tasks.write().await;
        let mut configs = self.schedule_configs.write().await;

        if tasks.contains_key(&schedule_id) {
            tracing::debug!(
                "SKIP: Task already exists for schedule {}, not creating duplicate. Task count: {}",
                schedule_id,
                tasks.len()
            );
            // Abort the newly created task since we're not using it
            task.abort();
            drop(configs);
            drop(tasks);
            return Ok(schedule_id);
        }

        // Insert the new task and config atomically
        tasks.insert(schedule_id.clone(), task);
        configs.insert(schedule_id.clone(), (cron_expr_for_config, timezone_for_config));

        tracing::debug!(
            "NEW TASK: Created task for schedule {}. Task count: {}",
            schedule_id,
            tasks.len()
        );

        drop(configs);
        drop(tasks);

        Ok(schedule_id)
    }

    /// Execute scheduled workflow and automatically reschedule next execution
    async fn execute_scheduled_workflow(
        db: Arc<DatabaseConnection>,
        schedule_id: String,
        workflow_id: uuid::Uuid,
        payload: serde_json::Value,
        cron_expr: String,
        timezone: String,
        schedule_service: Arc<ScheduleService>,
    ) -> Result<()> {
        let now = Utc::now();

        // Parse schedule ID as UUID
        let schedule_uuid = uuid::Uuid::parse_str(&schedule_id)
            .map_err(|e| SwissPipeError::Generic(format!("Invalid schedule ID: {e}")))?;

        // Prepare headers with schedule metadata
        let mut event_headers = std::collections::HashMap::new();
        event_headers.insert("X-Triggered-By".to_string(), "cron-schedule".to_string());
        event_headers.insert("X-Schedule-ID".to_string(), schedule_id.clone());
        event_headers.insert("X-Scheduled-Time".to_string(), now.to_rfc3339());

        // Use ExecutionService to create execution and queue job (same path as webhook ingestion)
        let execution_service = ExecutionService::new(db.clone());

        let execution_id = match execution_service.create_execution(
            workflow_id.to_string(),
            payload,
            event_headers,
            None, // No priority
        ).await {
            Ok(id) => {
                tracing::info!("Created execution {} for scheduled workflow {}", id, workflow_id);
                id
            }
            Err(e) => {
                tracing::error!("Failed to create execution for scheduled workflow {}: {}", workflow_id, e);

                // Update schedule statistics with failure
                let next_exec = schedule_service.calculate_next_execution(&cron_expr, &timezone, now)?;
                schedule_service.update_after_execution(schedule_uuid, false, next_exec).await?;

                return Err(e);
            }
        };

        // The worker pool will handle the actual execution and status updates
        tracing::info!("Scheduled workflow {} queued successfully with execution_id {}", workflow_id, execution_id);

        // Calculate NEXT execution time (CRITICAL for recurring schedules)
        let next_exec = schedule_service.calculate_next_execution(&cron_expr, &timezone, now)?;

        // TODO: This currently tracks queueing success, not execution success
        // To properly track execution success/failure, the worker pool would need to
        // notify the scheduler service when executions complete. For now, we increment
        // execution_count on successful queueing, which is consistent with the
        // scheduler's responsibility (queueing jobs), not execution results.
        schedule_service.update_after_execution(schedule_uuid, true, next_exec).await?;

        // Note: Immediate rescheduling is triggered via channel notification to the sync loop.
        // The task handle cleanup and reschedule request happen in the spawned task.

        tracing::info!(
            "Scheduled workflow executed and rescheduled for next execution at {}",
            next_exec
        );

        Ok(())
    }

    /// Check database for new/updated schedules and detect deleted schedules
    async fn check_and_sync_schedules(&self) -> Result<()> {
        // Get all enabled schedules from database
        let enabled_schedules = self.schedule_service.get_enabled_schedules().await?;

        let current_tasks = self.schedule_tasks.read().await;
        let current_ids: std::collections::HashSet<String> = current_tasks.keys().cloned().collect();
        drop(current_tasks);

        let now = Utc::now();
        let mut active_schedule_ids = std::collections::HashSet::new();

        for schedule in enabled_schedules {
            let schedule_id = schedule.id.to_string();
            active_schedule_ids.insert(schedule_id.clone());

            // Check if within active date range
            let is_active = {
                let after_start = schedule.start_date.is_none_or(|start| now >= start);
                let before_end = schedule.end_date.is_none_or(|end| now <= end);
                after_start && before_end
            };

            if !is_active {
                // Schedule is not in active period - cancel if running
                if current_ids.contains(&schedule_id) {
                    self.cancel_schedule(&schedule_id).await?;
                }
                continue;
            }

            let current_config = (schedule.cron_expression.clone(), schedule.timezone.clone());

            // Check if schedule exists and if configuration has changed
            let needs_reschedule = if current_ids.contains(&schedule_id) {
                // Check if configuration changed (cron expression or timezone)
                let configs = self.schedule_configs.read().await;
                let config_changed = configs.get(&schedule_id)
                    .map(|old_config| old_config != &current_config)
                    .unwrap_or(true);
                drop(configs);

                if config_changed {
                    tracing::info!(
                        schedule_id = %schedule_id,
                        "Detected configuration change, rescheduling"
                    );
                    // Cancel existing schedule before rescheduling
                    self.cancel_schedule(&schedule_id).await?;
                    true
                } else {
                    false
                }
            } else {
                // New schedule
                true
            };

            if needs_reschedule {
                if let Err(e) = self.schedule_cron_execution(
                    schedule_id.clone(),
                    schedule.cron_expression.clone(),
                    schedule.timezone.clone(),
                    schedule.workflow_id,
                    schedule.test_payload,
                ).await {
                    tracing::error!("Failed to schedule new/updated schedule {}: {}", schedule_id, e);
                } else {
                    // Update stored configuration
                    self.schedule_configs.write().await.insert(schedule_id.clone(), current_config);
                    tracing::info!(
                        schedule_id = %schedule_id,
                        workflow_id = %schedule.workflow_id,
                        "Scheduled new/updated cron trigger"
                    );
                }
            }
        }

        // Cancel tasks for schedules that no longer exist (deleted or disabled)
        for current_id in current_ids {
            if !active_schedule_ids.contains(&current_id) {
                tracing::info!("Schedule {} no longer active, cancelling task", current_id);
                self.cancel_schedule(&current_id).await?;
            }
        }

        Ok(())
    }

    /// Cancel a schedule
    pub async fn cancel_schedule(&self, schedule_id: &str) -> Result<()> {
        if let Some(handle) = self.schedule_tasks.write().await.remove(schedule_id) {
            handle.abort();
            // Also remove from config tracking
            self.schedule_configs.write().await.remove(schedule_id);
            tracing::info!("Cancelled schedule task: {}", schedule_id);
        }
        Ok(())
    }

    /// Shutdown scheduler
    pub async fn shutdown(&self) -> Result<()> {
        tracing::info!("Shutting down Cron Scheduler Service...");

        let mut tasks = self.schedule_tasks.write().await;
        for (schedule_id, handle) in tasks.drain() {
            handle.abort();
            tracing::debug!("Cancelled schedule task: {}", schedule_id);
        }

        tracing::info!("Cron Scheduler Service shutdown complete");
        Ok(())
    }
}
