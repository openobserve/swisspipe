use crate::workflow::{
    errors::Result,
    models::{InputMergeStrategy, Workflow, WorkflowEvent},
    input_sync::{InputSyncService, InputSyncResult},
};
use std::sync::Arc;

/// Trait for input coordination functionality shared between WorkerPool and WorkerPoolForBranch
#[allow(async_fn_in_trait)]
pub trait InputCoordination {
    fn get_input_sync_service(&self) -> &Arc<InputSyncService>;

    /// Count incoming edges for a node
    fn count_incoming_edges(&self, workflow: &Workflow, node_name: &str) -> usize {
        workflow.edges
            .iter()
            .filter(|edge| edge.to_node_name == node_name)
            .count()
    }

    /// Handle input synchronization for nodes with multiple inputs
    async fn handle_input_synchronization(
        &self,
        execution_id: &str,
        node_name: &str,
        event: &WorkflowEvent,
        expected_input_count: i32,
        merge_strategy: &InputMergeStrategy,
    ) -> Result<InputSyncResult> {
        let input_sync_service = self.get_input_sync_service();
        
        // Try to initialize the sync record if it doesn't exist
        match input_sync_service
            .initialize_node_sync(execution_id, node_name, expected_input_count, merge_strategy)
            .await
        {
            Ok(_) => {
                tracing::debug!("Initialized input sync for node '{}' in execution '{}'", node_name, execution_id);
            }
            Err(e) => {
                // Check if this is a database constraint violation (record already exists)
                match &e {
                    crate::workflow::errors::SwissPipeError::Database(db_err) => {
                        let error_msg = db_err.to_string().to_lowercase();
                        if error_msg.contains("unique constraint") || 
                           error_msg.contains("duplicate") ||
                           error_msg.contains("primary key") {
                            tracing::debug!("Input sync record for node '{}' already exists, continuing", node_name);
                        } else {
                            // This is a different database error we should propagate
                            tracing::error!("Database error initializing input sync for node '{}': {}", node_name, db_err);
                            return Err(e);
                        }
                    }
                    _ => {
                        // Non-database errors should always be propagated
                        tracing::error!("Failed to initialize input sync for node '{}': {}", node_name, e);
                        return Err(e);
                    }
                }
            }
        }

        // Add this input to the synchronization
        let result = input_sync_service
            .add_input(execution_id, node_name, event.clone())
            .await?;

        // Mark as completed if ready
        if let InputSyncResult::Ready(_) = result {
            input_sync_service
                .mark_completed(execution_id, node_name)
                .await?;
        }

        Ok(result)
    }

    /// Check if a node needs input coordination and handle it
    async fn coordinate_node_inputs(
        &self,
        workflow: &Workflow,
        execution_id: &str,
        node_name: &str,
        event: &WorkflowEvent,
        input_merge_strategy: Option<&InputMergeStrategy>,
    ) -> Result<(bool, WorkflowEvent)> {
        let incoming_edge_count = self.count_incoming_edges(workflow, node_name);
        let merge_strategy = input_merge_strategy.unwrap_or(&InputMergeStrategy::FirstWins);
        
        if incoming_edge_count > 1 {
            // Multiple inputs - check synchronization
            match self.handle_input_synchronization(
                execution_id,
                node_name,
                event,
                incoming_edge_count as i32,
                merge_strategy,
            ).await? {
                InputSyncResult::Ready(merged_inputs) => {
                    // Merge the inputs based on strategy
                    let merged_event = InputSyncService::merge_inputs(merged_inputs, merge_strategy)?;
                    Ok((true, merged_event))
                }
                InputSyncResult::Waiting => {
                    // Not all inputs received yet, skip this node for now
                    tracing::debug!("Node '{}' waiting for more inputs, skipping execution", node_name);
                    Ok((false, event.clone()))
                }
                InputSyncResult::AlreadyCompleted => {
                    // Node was already executed, skip
                    tracing::debug!("Node '{}' already completed, skipping", node_name);
                    Ok((false, event.clone()))
                }
                InputSyncResult::TimedOut(partial_inputs) => {
                    // Timeout exceeded, execute with partial inputs
                    tracing::warn!("Node '{}' timed out, executing with {} partial inputs", node_name, partial_inputs.len());
                    if partial_inputs.is_empty() {
                        // No inputs received, use original event
                        Ok((true, event.clone()))
                    } else {
                        // Merge partial inputs
                        let merged_event = InputSyncService::merge_inputs(partial_inputs, merge_strategy)?;
                        Ok((true, merged_event))
                    }
                }
            }
        } else {
            // Single input, execute immediately
            Ok((true, event.clone()))
        }
    }
}