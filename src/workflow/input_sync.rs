use crate::{
    database::node_input_sync::{self, SyncStatus},
    workflow::{
        errors::{Result, SwissPipeError},
        models::{InputMergeStrategy, WorkflowEvent},
    },
};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde_json;
use std::{collections::HashMap, sync::Arc};
use uuid::Uuid;

pub struct InputSyncService {
    db: Arc<DatabaseConnection>,
}

impl InputSyncService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Initialize input synchronization for a node that expects multiple inputs
    pub async fn initialize_node_sync(
        &self,
        execution_id: &str,
        node_id: &str,
        expected_input_count: i32,
        merge_strategy: &InputMergeStrategy,
    ) -> Result<()> {
        let timeout_at = match merge_strategy {
            InputMergeStrategy::TimeoutBased(seconds) => {
                Some((chrono::Utc::now() + chrono::Duration::seconds(*seconds as i64)).timestamp_micros())
            }
            _ => None,
        };

        let sync_record = node_input_sync::ActiveModel {
            id: Set(Uuid::new_v4().to_string()),
            execution_id: Set(execution_id.to_string()),
            node_id: Set(node_id.to_string()),
            expected_input_count: Set(expected_input_count),
            received_inputs: Set("[]".to_string()),
            timeout_at: Set(timeout_at),
            status: Set(SyncStatus::Waiting.to_string()),
            ..Default::default()
        };

        sync_record.insert(self.db.as_ref()).await?;
        tracing::debug!(
            "Initialized input sync for node '{}' in execution '{}', expecting {} inputs",
            node_id, execution_id, expected_input_count
        );

        Ok(())
    }

    /// Add an input to a node's synchronization record using atomic operations
    pub async fn add_input(
        &self,
        execution_id: &str,
        node_id: &str,
        event: WorkflowEvent,
    ) -> Result<InputSyncResult> {
        use sea_orm::{TransactionTrait, QuerySelect};

        // Use database transaction to ensure atomic read-modify-write
        let txn = self.db.begin().await?;

        // Lock the record for update to prevent race conditions
        let sync_record = node_input_sync::Entity::find()
            .filter(node_input_sync::Column::ExecutionId.eq(execution_id))
            .filter(node_input_sync::Column::NodeId.eq(node_id))
            .lock_exclusive()  // This prevents other transactions from modifying
            .one(&txn)
            .await?
            .ok_or_else(|| {
                SwissPipeError::Generic(format!(
                    "No input sync record found for node '{node_id}' in execution '{execution_id}'"
                ))
            })?;

        // Check if already completed or timed out
        let current_status = SyncStatus::from(sync_record.status.clone());
        match current_status {
            SyncStatus::Completed => {
                txn.rollback().await?;
                return Ok(InputSyncResult::AlreadyCompleted);
            }
            SyncStatus::Timeout => {
                // Return partial inputs if timed out
                let received_inputs: Vec<WorkflowEvent> = 
                    serde_json::from_str(&sync_record.received_inputs)
                        .map_err(|e| SwissPipeError::Generic(format!("Invalid JSON in sync record: {e}")))?;
                txn.rollback().await?;
                return Ok(InputSyncResult::TimedOut(received_inputs));
            }
            _ => {}
        }

        // Parse existing inputs with size validation
        let mut received_inputs: Vec<WorkflowEvent> = 
            serde_json::from_str(&sync_record.received_inputs)
                .map_err(|e| SwissPipeError::Generic(format!("Invalid JSON in sync record: {e}")))?;

        // Validate input count bounds to prevent overflow
        if received_inputs.len() >= sync_record.expected_input_count as usize {
            txn.rollback().await?;
            return Err(SwissPipeError::Generic(
                "Input count already at maximum, possible race condition detected".to_string()
            ));
        }

        // Add the new input
        received_inputs.push(event);

        let new_status = if received_inputs.len() >= sync_record.expected_input_count as usize {
            SyncStatus::Ready
        } else {
            SyncStatus::Waiting
        };

        // Update the record atomically within transaction
        let expected_count = sync_record.expected_input_count;
        let serialized_inputs = serde_json::to_string(&received_inputs)
            .map_err(|e| SwissPipeError::Generic(format!("Failed to serialize inputs: {e}")))?;
            
        let mut active_model: node_input_sync::ActiveModel = sync_record.into();
        active_model.received_inputs = Set(serialized_inputs);
        active_model.status = Set(new_status.to_string());
        active_model.save(&txn).await?;

        // Commit the transaction
        txn.commit().await?;

        tracing::debug!(
            "Added input to node '{}' in execution '{}'. Status: {:?}, Inputs: {}/{}",
            node_id, execution_id, new_status, received_inputs.len(), expected_count
        );

        match new_status {
            SyncStatus::Ready => Ok(InputSyncResult::Ready(received_inputs)),
            SyncStatus::Waiting => Ok(InputSyncResult::Waiting),
            _ => Ok(InputSyncResult::Waiting),
        }
    }

    /// Check for nodes that have timed out
    pub async fn check_timeouts(&self) -> Result<Vec<TimeoutResult>> {
        let now = chrono::Utc::now().timestamp_micros();
        let timed_out_records = node_input_sync::Entity::find()
            .filter(node_input_sync::Column::Status.eq("waiting"))
            .filter(node_input_sync::Column::TimeoutAt.lt(now))
            .all(self.db.as_ref())
            .await?;

        let mut results = Vec::new();
        for record in timed_out_records {
            // Parse the received inputs
            let received_inputs: Vec<WorkflowEvent> =
                serde_json::from_str(&record.received_inputs)?;

            // Store values before moving record
            let execution_id = record.execution_id.clone();
            let node_id = record.node_id.clone();
            let expected_count = record.expected_input_count;

            // Update status to timeout
            let mut active_model: node_input_sync::ActiveModel = record.into();
            active_model.status = Set(SyncStatus::Timeout.to_string());
            active_model.save(self.db.as_ref()).await?;

            results.push(TimeoutResult {
                execution_id: execution_id.clone(),
                node_id: node_id.clone(),
                received_inputs: received_inputs.clone(),
            });

            tracing::warn!(
                "Node '{}' in execution '{}' timed out with {}/{} inputs",
                node_id, execution_id, received_inputs.len(), expected_count
            );
        }

        Ok(results)
    }

    /// Mark a node's input synchronization as completed
    pub async fn mark_completed(&self, execution_id: &str, node_id: &str) -> Result<()> {
        let sync_record = node_input_sync::Entity::find()
            .filter(node_input_sync::Column::ExecutionId.eq(execution_id))
            .filter(node_input_sync::Column::NodeId.eq(node_id))
            .one(self.db.as_ref())
            .await?;

        if let Some(record) = sync_record {
            let mut active_model: node_input_sync::ActiveModel = record.into();
            active_model.status = Set(SyncStatus::Completed.to_string());
            active_model.save(self.db.as_ref()).await?;
        }

        Ok(())
    }

    /// Merge multiple inputs based on the merge strategy
    pub fn merge_inputs(
        inputs: Vec<WorkflowEvent>,
        strategy: &InputMergeStrategy,
    ) -> Result<WorkflowEvent> {
        if inputs.is_empty() {
            return Err(SwissPipeError::Generic("No inputs to merge".to_string()));
        }

        match strategy {
            InputMergeStrategy::FirstWins => Ok(inputs[0].clone()),
            InputMergeStrategy::WaitForAll => {
                // WaitForAll: Only merge when ALL expected inputs are received
                // This should only be called when all inputs are available
                Self::merge_all_inputs(inputs)
            }
            InputMergeStrategy::TimeoutBased(_) => {
                // TimeoutBased: Merge whatever inputs are available (partial or complete)
                Self::merge_all_inputs(inputs)
            }
        }
    }

    /// Helper method to merge all inputs as an array
    fn merge_all_inputs(inputs: Vec<WorkflowEvent>) -> Result<WorkflowEvent> {
        let mut merged_metadata = HashMap::new();
        let mut merged_headers = HashMap::new();
        let mut merged_condition_results = HashMap::new();

        // Add metadata about the merge operation for traceability  
        merged_metadata.insert("merge_info".to_string(), "multiple_inputs_merged".to_string());
        merged_metadata.insert("input_count".to_string(), inputs.len().to_string());
        merged_metadata.insert("merge_timestamp".to_string(), chrono::Utc::now().to_rfc3339());

        // Create array of input data values
        let input_data_array: Vec<serde_json::Value> = inputs.iter()
            .map(|input| input.data.clone())
            .collect();

        // Merge metadata, headers, and condition results with input prefix for traceability
        for (index, input) in inputs.iter().enumerate() {
            let input_key = format!("input_{index}");
            
            // Merge metadata with input prefix
            for (key, value) in &input.metadata {
                merged_metadata.insert(format!("{input_key}_{key}"), value.clone());
            }

            // Merge headers with input prefix  
            for (key, value) in &input.headers {
                merged_headers.insert(format!("{input_key}_{key}"), value.clone());
            }

            // Merge condition results with input prefix
            for (key, value) in &input.condition_results {
                merged_condition_results.insert(format!("{input_key}_{key}"), *value);
            }
        }

        Ok(WorkflowEvent {
            data: serde_json::Value::Array(input_data_array),
            metadata: merged_metadata,
            headers: merged_headers,
            condition_results: merged_condition_results,
        })
    }
}

#[derive(Debug, Clone)]
pub enum InputSyncResult {
    Waiting,                         // Still waiting for more inputs
    Ready(Vec<WorkflowEvent>),       // All inputs received, ready to execute
    AlreadyCompleted,                // Node already executed
    TimedOut(Vec<WorkflowEvent>),    // Timeout exceeded, contains partial inputs
}

#[derive(Debug, Clone)]
pub struct TimeoutResult {
    pub execution_id: String,
    pub node_id: String,
    pub received_inputs: Vec<WorkflowEvent>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_merge_strategies_logic() {
        // Test merge logic without database
        let event1 = WorkflowEvent {
            data: serde_json::json!({"key1": "value1"}),
            metadata: {
                let mut map = HashMap::new();
                map.insert("meta1".to_string(), "meta_value1".to_string());
                map
            },
            headers: HashMap::new(),
            condition_results: HashMap::new(),
        };
        
        let event2 = WorkflowEvent {
            data: serde_json::json!({"key2": "value2"}),
            metadata: {
                let mut map = HashMap::new();
                map.insert("meta2".to_string(), "meta_value2".to_string());
                map
            },
            headers: HashMap::new(),
            condition_results: HashMap::new(),
        };
        
        let inputs = vec![event1.clone(), event2];
        
        // Test FirstWins
        let first_wins_result = InputSyncService::merge_inputs(inputs.clone(), &InputMergeStrategy::FirstWins)
            .expect("FirstWins merge failed");
        assert_eq!(first_wins_result.data, event1.data);
        
        // Test WaitForAll
        let wait_all_result = InputSyncService::merge_inputs(inputs, &InputMergeStrategy::WaitForAll)
            .expect("WaitForAll merge failed");
        
        if let Some(array) = wait_all_result.data.as_array() {
            // With new array structure, check that we have 2 elements
            assert_eq!(array.len(), 2);
            
            // Verify the array elements contain the correct data
            if let Some(input_0_obj) = array[0].as_object() {
                assert!(input_0_obj.contains_key("key1"));
            }
            if let Some(input_1_obj) = array[1].as_object() {
                assert!(input_1_obj.contains_key("key2"));
            }
            println!("✅ WaitForAll merge created expected array structure");
        } else {
            panic!("WaitForAll should create array structure");
        }
        
        // Check metadata merging
        assert!(wait_all_result.metadata.contains_key("input_0_meta1"));
        assert!(wait_all_result.metadata.contains_key("input_1_meta2"));
        println!("✅ All merge strategy tests passed");
    }
}