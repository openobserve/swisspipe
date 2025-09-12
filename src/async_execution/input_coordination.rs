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
    fn count_incoming_edges(&self, workflow: &Workflow, node_id: &str) -> usize {
        workflow.edges
            .iter()
            .filter(|edge| edge.to_node_id == node_id)
            .count()
    }

    /// Handle input synchronization for nodes with multiple inputs
    async fn handle_input_synchronization(
        &self,
        execution_id: &str,
        node_id: &str,
        event: &WorkflowEvent,
        expected_input_count: i32,
        merge_strategy: &InputMergeStrategy,
    ) -> Result<InputSyncResult> {
        let input_sync_service = self.get_input_sync_service();
        
        // Try to initialize the sync record if it doesn't exist
        match input_sync_service
            .initialize_node_sync(execution_id, node_id, expected_input_count, merge_strategy)
            .await
        {
            Ok(_) => {
                tracing::debug!("Initialized input sync for node with ID '{}' in execution '{}'", node_id, execution_id);
            }
            Err(e) => {
                // Check if this is a database constraint violation (record already exists)
                match &e {
                    crate::workflow::errors::SwissPipeError::Database(db_err) => {
                        let error_msg = db_err.to_string().to_lowercase();
                        if error_msg.contains("unique constraint") || 
                           error_msg.contains("duplicate") ||
                           error_msg.contains("primary key") {
                            tracing::debug!("Input sync record for node with ID '{}' already exists, continuing", node_id);
                        } else {
                            // This is a different database error we should propagate
                            tracing::error!("Database error initializing input sync for node with ID '{}': {}", node_id, db_err);
                            return Err(e);
                        }
                    }
                    _ => {
                        // Non-database errors should always be propagated
                        tracing::error!("Failed to initialize input sync for node with ID '{}': {}", node_id, e);
                        return Err(e);
                    }
                }
            }
        }

        // Add this input to the synchronization
        let result = input_sync_service
            .add_input(execution_id, node_id, event.clone())
            .await?;

        // Mark as completed if ready
        if let InputSyncResult::Ready(_) = result {
            input_sync_service
                .mark_completed(execution_id, node_id)
                .await?;
        }

        Ok(result)
    }

    /// Check if a node needs input coordination and handle it
    async fn coordinate_node_inputs(
        &self,
        workflow: &Workflow,
        execution_id: &str,
        node_id: &str,
        event: &WorkflowEvent,
        input_merge_strategy: Option<&InputMergeStrategy>,
    ) -> Result<(bool, WorkflowEvent)> {
        let incoming_edge_count = self.count_incoming_edges(workflow, node_id);
        let merge_strategy = input_merge_strategy.unwrap_or(&InputMergeStrategy::WaitForAll);
        
        // Get node name for logging
        let node_display_name = workflow.nodes.iter()
            .find(|n| n.id == node_id)
            .map(|n| n.name.as_str())
            .unwrap_or("unknown");
        
        if incoming_edge_count > 1 {
            // Multiple inputs - check synchronization
            match self.handle_input_synchronization(
                execution_id,
                node_id,
                event,
                incoming_edge_count as i32,
                merge_strategy,
            ).await? {
                InputSyncResult::Ready(merged_inputs) => {
                    // Log the inputs being merged for debugging
                    tracing::debug!(
                        "Node '{}' (id: {}) merging {} inputs using strategy {:?}",
                        node_display_name, node_id, merged_inputs.len(), merge_strategy
                    );
                    
                    // Merge the inputs based on strategy
                    let merged_event = InputSyncService::merge_inputs(merged_inputs, merge_strategy)?;
                    
                    tracing::debug!(
                        "Node '{}' (id: {}) merged inputs into event with data keys: {:?}",
                        node_display_name, node_id,
                        merged_event.data.as_object().map(|obj| obj.keys().collect::<Vec<_>>()).unwrap_or_default()
                    );
                    
                    Ok((true, merged_event))
                }
                InputSyncResult::Waiting => {
                    // Not all inputs received yet, skip this node for now
                    tracing::debug!("Node '{}' (id: {}) waiting for more inputs, skipping execution", node_display_name, node_id);
                    Ok((false, event.clone()))
                }
                InputSyncResult::AlreadyCompleted => {
                    // Node was already executed, skip
                    tracing::debug!("Node '{}' (id: {}) already completed, skipping", node_display_name, node_id);
                    Ok((false, event.clone()))
                }
                InputSyncResult::TimedOut(partial_inputs) => {
                    // Timeout exceeded, execute with partial inputs
                    tracing::warn!("Node '{}' (id: {}) timed out, executing with {} partial inputs", node_display_name, node_id, partial_inputs.len());
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