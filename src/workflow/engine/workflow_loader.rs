use crate::{
    database::{edges, entities, nodes},
    workflow::{
        errors::{Result, SwissPipeError},
        models::{Edge, Node, NodeType, Workflow},
    },
};
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait};
use std::sync::Arc;

pub struct WorkflowLoader {
    db: Arc<DatabaseConnection>,
}

impl WorkflowLoader {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Load a complete workflow with all nodes and edges from the database
    pub async fn load_workflow(&self, workflow_id: &str) -> Result<Workflow> {
        // Load workflow metadata
        let workflow_model = entities::Entity::find_by_id(workflow_id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| SwissPipeError::WorkflowNotFound(workflow_id.to_string()))?;

        // Load all nodes for this workflow
        let nodes = self.load_nodes(workflow_id).await?;

        // Load all edges for this workflow
        let edges = self.load_edges(workflow_id).await?;

        Ok(Workflow {
            id: workflow_model.id,
            name: workflow_model.name,
            description: workflow_model.description,
            start_node_id: workflow_model.start_node_id,
            enabled: workflow_model.enabled,
            nodes,
            edges,
        })
    }

    /// Get workflow, returning None if not found instead of error
    pub async fn get_workflow(&self, workflow_id: &str) -> Result<Option<Workflow>> {
        match self.load_workflow(workflow_id).await {
            Ok(workflow) => Ok(Some(workflow)),
            Err(SwissPipeError::WorkflowNotFound(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Load all nodes for a workflow
    async fn load_nodes(&self, workflow_id: &str) -> Result<Vec<Node>> {
        let node_models = nodes::Entity::find()
            .filter(nodes::Column::WorkflowId.eq(workflow_id))
            .all(self.db.as_ref())
            .await?;

        let mut nodes = Vec::new();
        for node_model in node_models {
            let node_type: NodeType = serde_json::from_str(&node_model.config)?;
            nodes.push(Node {
                id: node_model.id,
                workflow_id: node_model.workflow_id,
                name: node_model.name,
                node_type,
                input_merge_strategy: node_model.input_merge_strategy
                    .as_deref()
                    .and_then(|s| serde_json::from_str(s).ok()),
            });
        }

        Ok(nodes)
    }

    /// Load all edges for a workflow
    async fn load_edges(&self, workflow_id: &str) -> Result<Vec<Edge>> {
        let edge_models = edges::Entity::find()
            .filter(edges::Column::WorkflowId.eq(workflow_id))
            .all(self.db.as_ref())
            .await?;

        let edges = edge_models
            .into_iter()
            .map(|edge_model| Edge {
                id: edge_model.id,
                workflow_id: edge_model.workflow_id,
                from_node_id: edge_model.from_node_id,
                to_node_id: edge_model.to_node_id,
                condition_result: edge_model.condition_result,
            })
            .collect();

        Ok(edges)
    }
}