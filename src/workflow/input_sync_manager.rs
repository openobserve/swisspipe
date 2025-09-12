use crate::workflow::{
    input_sync::InputSyncService,
    errors::Result,
};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use tokio::{task::JoinHandle, time::{interval, Duration}};
use sea_orm::DatabaseConnection;

pub struct InputSyncManager {
    input_sync_service: Arc<InputSyncService>,
    is_running: Arc<AtomicBool>,
    cleanup_handle: Option<JoinHandle<()>>,
}

impl InputSyncManager {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        let input_sync_service = Arc::new(InputSyncService::new(db));
        
        Self {
            input_sync_service,
            is_running: Arc::new(AtomicBool::new(false)),
            cleanup_handle: None,
        }
    }

    pub async fn start(&mut self, check_interval_seconds: u64) -> Result<()> {
        if self.is_running.load(Ordering::SeqCst) {
            return Ok(()); // Already running
        }

        self.is_running.store(true, Ordering::SeqCst);

        let input_sync_service = self.input_sync_service.clone();
        let is_running = self.is_running.clone();

        let handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(check_interval_seconds));

            while is_running.load(Ordering::SeqCst) {
                interval.tick().await;

                match input_sync_service.check_timeouts().await {
                    Ok(timed_out) => {
                        if !timed_out.is_empty() {
                            tracing::info!(
                                "Found {} nodes with timed out input synchronization",
                                timed_out.len()
                            );

                            // For each timed-out node, we could potentially create a job to resume execution
                            // For now, we just log it since the execution logic handles timeouts inline
                            for timeout_result in timed_out {
                                tracing::warn!(
                                    "Node '{}' in execution '{}' timed out with {} inputs",
                                    timeout_result.node_id,
                                    timeout_result.execution_id,
                                    timeout_result.received_inputs.len()
                                );

                                // TODO: Create continuation jobs for timed-out nodes if needed
                                // This would involve creating a job to resume execution at the timed-out node
                                // with the available inputs merged according to the timeout strategy
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Error checking input sync timeouts: {}", e);
                    }
                }
            }

            tracing::info!("Input sync manager cleanup task stopped");
        });

        self.cleanup_handle = Some(handle);

        tracing::info!("Input sync manager started with {}s check interval", check_interval_seconds);
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn stop(&mut self) -> Result<()> {
        if !self.is_running.load(Ordering::SeqCst) {
            return Ok(()); // Not running
        }

        self.is_running.store(false, Ordering::SeqCst);

        if let Some(handle) = self.cleanup_handle.take() {
            handle.abort();
            let _ = handle.await;
        }

        tracing::info!("Input sync manager stopped");
        Ok(())
    }

    #[allow(dead_code)]
    pub fn get_input_sync_service(&self) -> Arc<InputSyncService> {
        self.input_sync_service.clone()
    }
}

impl Drop for InputSyncManager {
    fn drop(&mut self) {
        self.is_running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.cleanup_handle.take() {
            handle.abort();
        }
    }
}