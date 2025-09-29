use axum::http::StatusCode;
use sea_orm::{ActiveModelTrait, EntityTrait, Set, ColumnTrait, QueryFilter, TransactionTrait};
use uuid::Uuid;

use crate::{
    database::{edges, entities, nodes},
    workflow::{
        models::{Edge, Node, NodeType},
        validation::WorkflowValidator,
    },
    AppState,
};

use super::{
    types::{CreateWorkflowRequest, UpdateContext, PlannedOperations, UpdateResult, WorkflowResponse},
    validation::validate_workflow_update_request,
    operations::{categorize_node_changes, categorize_edge_changes, node_type_to_string, build_workflow_response},
};


/// Service for handling workflow updates with proper separation of concerns
pub struct UpdateWorkflowService<'a> {
    pub state: &'a AppState,
    pub workflow_id: String,
    pub request: CreateWorkflowRequest,
    pub update_start: std::time::Instant,
}

impl<'a> UpdateWorkflowService<'a> {
    pub fn new(state: &'a AppState, workflow_id: String, request: CreateWorkflowRequest) -> Self {
        Self {
            state,
            workflow_id,
            request,
            update_start: std::time::Instant::now(),
        }
    }

    /// Main entry point for workflow updates
    pub async fn update_workflow(self) -> Result<WorkflowResponse, StatusCode> {
        tracing::info!(
            "Workflow update initiated: workflow_id={}, nodes_count={}, edges_count={}",
            self.workflow_id, self.request.nodes.len(), self.request.edges.len()
        );

        let context = self.prepare_update_context().await?;
        let operations = self.plan_operations(&context).await?;
        let result = self.execute_operations(operations).await?;
        Ok(self.build_response(result))
    }

    /// Phase 1: Prepare all context needed for the update
    async fn prepare_update_context(&self) -> Result<UpdateContext, StatusCode> {
        // Fetch existing workflow
        let workflow = entities::Entity::find_by_id(&self.workflow_id)
            .one(&*self.state.db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to fetch workflow for update: workflow_id={}, error={:?}", self.workflow_id, e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
            .ok_or_else(|| {
                tracing::warn!("Workflow not found for update: workflow_id={}", self.workflow_id);
                StatusCode::NOT_FOUND
            })?;
            
        tracing::info!(
            "Workflow update: found existing workflow '{}' (description: {:?})", 
            workflow.name, workflow.description
        );

        let existing_start_node_id = workflow.start_node_id.clone()
            .ok_or_else(|| {
                tracing::error!("Workflow {} has no start_node_id", self.workflow_id);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        // Get existing nodes and edges
        tracing::debug!("Workflow update: fetching existing nodes for workflow_id={}", self.workflow_id);
        let existing_nodes = nodes::Entity::find()
            .filter(nodes::Column::WorkflowId.eq(&self.workflow_id))
            .all(&*self.state.db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to fetch existing nodes for workflow update: workflow_id={}, error={:?}", self.workflow_id, e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        tracing::debug!("Workflow update: fetching existing edges for workflow_id={}", self.workflow_id);
        let existing_edges = edges::Entity::find()
            .filter(edges::Column::WorkflowId.eq(&self.workflow_id))
            .all(&*self.state.db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to fetch existing edges for workflow update: workflow_id={}, error={:?}", self.workflow_id, e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        // Get existing start node to preserve it
        let existing_start_node = existing_nodes.iter()
            .find(|n| n.id == existing_start_node_id)
            .cloned()
            .ok_or_else(|| {
                tracing::error!("Start node {} not found for workflow {}", existing_start_node_id, self.workflow_id);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        // Convert existing start node to internal model
        let start_node_config: NodeType = serde_json::from_str(&existing_start_node.config)
            .map_err(|e| {
                tracing::error!(
                    "Failed to parse start node config: workflow_id={}, start_node_id={}, config='{}', error={:?}", 
                    self.workflow_id, existing_start_node_id, existing_start_node.config, e
                );
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        
        let start_node = Node {
            id: existing_start_node_id.clone(),
            workflow_id: self.workflow_id.clone(),
            name: existing_start_node.name.clone(),
            node_type: start_node_config,
            input_merge_strategy: None,
        };

        // Convert request nodes to internal models and add start node
        let mut internal_nodes: Vec<Node> = vec![start_node]; // Start with the preserved start node
        internal_nodes.extend(self.request.nodes.iter().map(|n| Node {
            id: n.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string()),
            workflow_id: self.workflow_id.clone(),
            name: n.name.clone(),
            node_type: n.node_type.clone(),
            input_merge_strategy: None,
        }));
        
        let internal_edges: Vec<Edge> = self.request.edges.iter().map(|e| Edge {
            id: Uuid::new_v4().to_string(),
            workflow_id: self.workflow_id.clone(),
            from_node_id: e.from_node_id.clone(),
            to_node_id: e.to_node_id.clone(),
            condition_result: e.condition_result,
            source_handle_id: e.source_handle_id.clone(),
        }).collect();

        Ok(UpdateContext {
            workflow,
            existing_nodes,
            existing_edges,
            existing_start_node_id,
            internal_nodes,
            internal_edges,
        })
    }

    /// Phase 2: Plan all operations based on the context
    async fn plan_operations<'b>(&self, context: &'b UpdateContext) -> Result<PlannedOperations<'b>, StatusCode> {
        // Validate input request
        tracing::info!("Workflow update: validating request for workflow_id={}", self.workflow_id);
        if let Err(validation_error) = validate_workflow_update_request(&self.request, &context.existing_start_node_id, &context.existing_nodes) {
            tracing::warn!(
                "Workflow update validation failed: workflow_id={}, nodes_count={}, edges_count={}, error='{}'",
                self.workflow_id, self.request.nodes.len(), self.request.edges.len(), validation_error
            );
            // For now, return BAD_REQUEST. In the future, we could enhance this to return detailed error info
            return Err(StatusCode::BAD_REQUEST);
        }
        tracing::info!("Workflow update: validation passed for workflow_id={}", self.workflow_id);


        // Validate workflow structure
        if let Err(validation_error) = WorkflowValidator::validate_workflow(
            &self.request.name,
            &context.existing_start_node_id,
            &context.internal_nodes,
            &context.internal_edges,
        ) {
            tracing::warn!(
                "Workflow structure validation failed: workflow_id={}, workflow_name='{}', error='{}'", 
                self.workflow_id, self.request.name, validation_error
            );
            return Err(StatusCode::BAD_REQUEST);
        }

        // Check for warnings and log them
        let warnings = WorkflowValidator::validate_condition_completeness(&context.internal_nodes, &context.internal_edges);
        for warning in warnings {
            tracing::warn!("Workflow update warning: workflow_id={}, warning='{}'", self.workflow_id, warning);
        }

        // Update workflow metadata
        let mut updated_workflow: entities::ActiveModel = context.workflow.clone().into();
        updated_workflow.name = Set(self.request.name.clone());
        updated_workflow.description = Set(self.request.description.clone());
        // Keep existing start_node_id - don't update it

        let updated_workflow = updated_workflow
            .update(&*self.state.db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to update workflow metadata: workflow_id={}, error={:?}", self.workflow_id, e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        // No need to check for active executions since we use workflow caching:
        // - Ongoing executions use cached workflow data and are unaffected by DB updates
        // - Cache invalidation is instant (in-memory HashMap removal)
        // - New executions will get the updated workflow from DB after cache invalidation
        tracing::info!("Workflow update: proceeding with safe cache-aware update for workflow_id={}", self.workflow_id);

        // Categorize changes
        let node_ops = categorize_node_changes(&context.existing_nodes, &self.request.nodes, &context.existing_start_node_id);
        let edge_ops = categorize_edge_changes(&context.existing_edges, &self.request.edges);

        tracing::info!(
            "Workflow {} differential update: nodes(create={}, update={}, delete={}), edges(create={}, delete={})",
            self.workflow_id, node_ops.to_create.len(), node_ops.to_update.len(), node_ops.to_delete.len(),
            edge_ops.to_create.len(), edge_ops.to_delete.len()
        );

        Ok(PlannedOperations {
            node_ops,
            edge_ops,
            updated_workflow,
        })
    }

    /// Phase 3: Execute all planned operations in a transaction
    async fn execute_operations(&self, operations: PlannedOperations<'_>) -> Result<UpdateResult, StatusCode> {
        // Start transaction for atomic updates
        tracing::info!("Workflow update: starting database transaction for workflow_id={}", self.workflow_id);
        let txn = self.state.db.begin().await.map_err(|e| {
            tracing::error!("Failed to start transaction: workflow_id={}, error={:?}", self.workflow_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        // Execute node operations
        self.execute_node_operations(&txn, &operations.node_ops).await?;

        // Execute edge operations  
        self.execute_edge_operations(&txn, &operations.edge_ops).await?;

        // Commit transaction
        tracing::info!("Workflow update: committing transaction for workflow_id={}", self.workflow_id);
        let commit_start = std::time::Instant::now();
        txn.commit().await.map_err(|e| {
            tracing::error!("Failed to commit transaction: workflow_id={}, error={:?}", self.workflow_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        let commit_duration = commit_start.elapsed();
        tracing::info!("Workflow update: transaction committed successfully for workflow_id={}, duration={:?}", self.workflow_id, commit_duration);

        // Fetch updated data for response
        let (nodes, edges) = self.fetch_updated_data().await?;

        let total_duration = self.update_start.elapsed();

        Ok(UpdateResult {
            workflow: operations.updated_workflow,
            nodes,
            edges,
            start_node_id: self.workflow_id.clone(), // This should be the existing_start_node_id
            total_duration,
        })
    }

    /// Execute all node operations within the transaction
    async fn execute_node_operations(&self, txn: &sea_orm::DatabaseTransaction, node_ops: &super::types::NodeOperations<'_>) -> Result<(), StatusCode> {
        // 1. Update existing nodes
        tracing::info!("Workflow update: updating {} existing nodes for workflow_id={}", node_ops.to_update.len(), self.workflow_id);
        for (node_id, node_data, existing_node) in &node_ops.to_update {
            tracing::debug!("Updating node: workflow_id={}, node_id={}, name='{}'", self.workflow_id, node_id, node_data.name);
            
            let node_config = serde_json::to_string(&node_data.node_type)
                .map_err(|e| {
                    tracing::error!("Failed to serialize node config: workflow_id={}, node_id={}, error={:?}", self.workflow_id, node_id, e);
                    StatusCode::BAD_REQUEST
                })?;

            let position_x = node_data.position_x.unwrap_or(100.0);
            let position_y = node_data.position_y.unwrap_or(100.0);

            let node_type_str = node_type_to_string(&node_data.node_type);

            // Update existing node using the reference we already have
            let mut node_model: nodes::ActiveModel = (*existing_node).clone().into();
            node_model.name = Set(node_data.name.clone());
            node_model.node_type = Set(node_type_str);
            node_model.config = Set(node_config);
            node_model.position_x = Set(position_x);
            node_model.position_y = Set(position_y);

            node_model.update(txn).await.map_err(|e| {
                tracing::error!("Failed to update node: workflow_id={}, node_id={}, error={:?}", self.workflow_id, node_id, e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
            
            tracing::debug!("Successfully updated node: workflow_id={}, node_id={}", self.workflow_id, node_id);
        }

        // 2. Create new nodes
        tracing::info!("Workflow update: creating {} new nodes for workflow_id={}", node_ops.to_create.len(), self.workflow_id);
        for node_data in &node_ops.to_create {
            let node_id = node_data.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
            tracing::debug!("Creating node: workflow_id={}, node_id={}, name='{}'", self.workflow_id, node_id, node_data.name);
            
            let node_config = serde_json::to_string(&node_data.node_type)
                .map_err(|e| {
                    tracing::error!("Failed to serialize node config for creation: workflow_id={}, node_id={}, error={:?}", self.workflow_id, node_id, e);
                    StatusCode::BAD_REQUEST
                })?;

            let position_x = node_data.position_x.unwrap_or(100.0);
            let position_y = node_data.position_y.unwrap_or(100.0);

            let node_type_str = node_type_to_string(&node_data.node_type);

            let node_model = nodes::ActiveModel {
                id: Set(node_id.clone()),
                workflow_id: Set(self.workflow_id.clone()),
                name: Set(node_data.name.clone()),
                node_type: Set(node_type_str),
                config: Set(node_config),
                position_x: Set(position_x),
                position_y: Set(position_y),
                ..Default::default()
            };

            node_model.insert(txn).await.map_err(|e| {
                tracing::error!("Failed to create node: workflow_id={}, node_id={}, error={:?}", self.workflow_id, node_id, e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
            
            tracing::debug!("Successfully created node: workflow_id={}, node_id={}", self.workflow_id, node_id);
        }

        // 3. Delete unused nodes (after edges to avoid FK violations)
        if !node_ops.to_delete.is_empty() {
            tracing::info!("Workflow update: deleting {} unused nodes for workflow_id={}", node_ops.to_delete.len(), self.workflow_id);
            tracing::debug!("Deleting nodes: workflow_id={}, node_ids={:?}", self.workflow_id, node_ops.to_delete);
            
            nodes::Entity::delete_many()
                .filter(nodes::Column::Id.is_in(&node_ops.to_delete))
                .exec(txn)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to delete nodes: workflow_id={}, error={:?}", self.workflow_id, e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
                
            tracing::info!("Workflow update: successfully deleted {} nodes for workflow_id={}", node_ops.to_delete.len(), self.workflow_id);
        }

        Ok(())
    }

    /// Execute all edge operations within the transaction
    async fn execute_edge_operations(&self, txn: &sea_orm::DatabaseTransaction, edge_ops: &super::types::EdgeOperations) -> Result<(), StatusCode> {
        // 1. Delete edges first (before nodes to avoid FK violations)
        if !edge_ops.to_delete.is_empty() {
            tracing::info!("Workflow update: deleting {} edges for workflow_id={}", edge_ops.to_delete.len(), self.workflow_id);
            tracing::debug!("Deleting edges: workflow_id={}, edge_ids={:?}", self.workflow_id, edge_ops.to_delete);
            
            edges::Entity::delete_many()
                .filter(edges::Column::Id.is_in(&edge_ops.to_delete))
                .exec(txn)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to delete edges: workflow_id={}, error={:?}", self.workflow_id, e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
                
            tracing::info!("Workflow update: successfully deleted {} edges for workflow_id={}", edge_ops.to_delete.len(), self.workflow_id);
        }

        // 2. Create new edges 
        tracing::info!("Workflow update: creating {} new edges for workflow_id={}", edge_ops.to_create.len(), self.workflow_id);
        for edge_data in &edge_ops.to_create {
            let edge_id = Uuid::new_v4().to_string();
            tracing::debug!("Creating edge: workflow_id={}, edge_id={}, from_node={}, to_node={}", 
                           self.workflow_id, edge_id, edge_data.from_node_id, edge_data.to_node_id);
            
            let edge_model = edges::ActiveModel {
                id: Set(edge_id.clone()),
                workflow_id: Set(self.workflow_id.clone()),
                from_node_id: Set(edge_data.from_node_id.clone()),
                to_node_id: Set(edge_data.to_node_id.clone()),
                condition_result: Set(edge_data.condition_result),
                source_handle_id: Set(edge_data.source_handle_id.clone()),
                ..Default::default()
            };

            edge_model.insert(txn).await.map_err(|e| {
                tracing::error!("Failed to create edge: workflow_id={}, edge_id={}, error={:?}", self.workflow_id, edge_id, e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
            
            tracing::debug!("Successfully created edge: workflow_id={}, edge_id={}", self.workflow_id, edge_id);
        }

        Ok(())
    }

    /// Fetch updated data after transaction completion
    async fn fetch_updated_data(&self) -> Result<(Vec<nodes::Model>, Vec<edges::Model>), StatusCode> {
        // Fetch nodes
        tracing::debug!("Workflow update: fetching updated nodes for response for workflow_id={}", self.workflow_id);
        let nodes = nodes::Entity::find()
            .filter(nodes::Column::WorkflowId.eq(&self.workflow_id))
            .all(&*self.state.db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to fetch updated nodes: workflow_id={}, error={:?}", self.workflow_id, e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        // Fetch edges
        tracing::debug!("Workflow update: fetching updated edges for response for workflow_id={}", self.workflow_id);
        let edges = edges::Entity::find()
            .filter(edges::Column::WorkflowId.eq(&self.workflow_id))
            .all(&*self.state.db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to fetch updated edges: workflow_id={}, error={:?}", self.workflow_id, e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        Ok((nodes, edges))
    }

    /// Phase 4: Build final response
    fn build_response(&self, result: UpdateResult) -> WorkflowResponse {
        // Invalidate cache since workflow was updated, then cache new version
        tracing::debug!("Workflow update: invalidating cache for workflow_id={}", self.workflow_id);
        // Note: We should invalidate cache here, but we can't call async methods in this sync method
        // Cache invalidation will be handled in the async handler after this returns

        tracing::info!(
            "Workflow update completed successfully: workflow_id={}, total_nodes={}, total_edges={}, total_duration={:?}",
            self.workflow_id, result.nodes.len(), result.edges.len(), result.total_duration
        );

        build_workflow_response(
            result.workflow,
            result.nodes,
            result.edges,
            result.start_node_id,
        )
    }
}