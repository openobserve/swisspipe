use crate::database::workflow_versions;
use crate::workflow::errors::{Result, SwissPipeError};
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect, ColumnTrait, Set, PaginatorTrait};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct VersionService {
    db: Arc<DatabaseConnection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVersionRequest {
    pub workflow_snapshot: String, // Complete workflow JSON as string
    pub commit_message: String,
    pub commit_description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionResponse {
    pub id: String,
    pub workflow_id: String,
    pub version_number: i32,
    pub commit_message: String,
    pub commit_description: Option<String>,
    pub changed_by: String,
    pub created_at: i64,
    pub workflow_name: String, // Extracted from snapshot for display
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionDetailResponse {
    pub id: String,
    pub workflow_id: String,
    pub version_number: i32,
    pub workflow_snapshot: serde_json::Value, // Parsed JSON
    pub commit_message: String,
    pub commit_description: Option<String>,
    pub changed_by: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionHistoryResponse {
    pub versions: Vec<VersionResponse>,
    pub total: u64,
    pub limit: u64,
    pub offset: u64,
}

impl VersionService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Create a new version when workflow is saved
    pub async fn create_version(
        &self,
        workflow_id: &str,
        workflow_snapshot: &str,
        commit_message: &str,
        commit_description: Option<&str>,
        changed_by: &str,
    ) -> Result<VersionResponse> {
        // Validate commit message length
        if commit_message.is_empty() {
            return Err(SwissPipeError::ValidationError(
                "Commit message cannot be empty".to_string(),
            ));
        }
        if commit_message.len() > 100 {
            return Err(SwissPipeError::ValidationError(
                "Commit message must be 100 characters or less".to_string(),
            ));
        }
        if let Some(desc) = commit_description {
            if desc.len() > 1000 {
                return Err(SwissPipeError::ValidationError(
                    "Commit description must be 1000 characters or less".to_string(),
                ));
            }
        }

        // Parse workflow snapshot to extract workflow name
        let snapshot_json: serde_json::Value = serde_json::from_str(workflow_snapshot)
            .map_err(|e| SwissPipeError::ValidationError(format!("Invalid workflow JSON: {e}")))?;

        let workflow_name = snapshot_json
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unnamed Workflow")
            .to_string();

        // Get latest version number for this workflow
        let latest_version = workflow_versions::Entity::find()
            .filter(workflow_versions::Column::WorkflowId.eq(workflow_id))
            .order_by_desc(workflow_versions::Column::VersionNumber)
            .one(self.db.as_ref())
            .await?;

        let new_version_number = latest_version
            .map(|v| v.version_number + 1)
            .unwrap_or(1);

        // Create new version
        let now = chrono::Utc::now().timestamp_micros();
        let version_id = Uuid::now_v7().to_string();

        let new_version = workflow_versions::ActiveModel {
            id: Set(version_id.clone()),
            workflow_id: Set(workflow_id.to_string()),
            version_number: Set(new_version_number),
            workflow_snapshot: Set(workflow_snapshot.to_string()),
            commit_message: Set(commit_message.to_string()),
            commit_description: Set(commit_description.map(|s| s.to_string())),
            changed_by: Set(changed_by.to_string()),
            created_at: Set(now),
        };

        new_version.insert(self.db.as_ref()).await?;

        tracing::info!(
            "Created version {} for workflow {} by {}",
            new_version_number,
            workflow_id,
            changed_by
        );

        Ok(VersionResponse {
            id: version_id,
            workflow_id: workflow_id.to_string(),
            version_number: new_version_number,
            commit_message: commit_message.to_string(),
            commit_description: commit_description.map(|s| s.to_string()),
            changed_by: changed_by.to_string(),
            created_at: now,
            workflow_name,
        })
    }

    /// Get version history with pagination
    pub async fn get_versions(
        &self,
        workflow_id: &str,
        limit: u64,
        offset: u64,
    ) -> Result<VersionHistoryResponse> {
        // Get total count
        let total = workflow_versions::Entity::find()
            .filter(workflow_versions::Column::WorkflowId.eq(workflow_id))
            .count(self.db.as_ref())
            .await?;

        // Get versions with pagination
        let versions = workflow_versions::Entity::find()
            .filter(workflow_versions::Column::WorkflowId.eq(workflow_id))
            .order_by_desc(workflow_versions::Column::VersionNumber)
            .limit(limit)
            .offset(offset)
            .all(self.db.as_ref())
            .await?;

        // Convert to response format
        let version_responses: Vec<VersionResponse> = versions
            .into_iter()
            .map(|v| {
                // Extract workflow name from snapshot
                let workflow_name = serde_json::from_str::<serde_json::Value>(&v.workflow_snapshot)
                    .ok()
                    .and_then(|json| json.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
                    .unwrap_or_else(|| "Unnamed Workflow".to_string());

                VersionResponse {
                    id: v.id,
                    workflow_id: v.workflow_id,
                    version_number: v.version_number,
                    commit_message: v.commit_message,
                    commit_description: v.commit_description,
                    changed_by: v.changed_by,
                    created_at: v.created_at,
                    workflow_name,
                }
            })
            .collect();

        Ok(VersionHistoryResponse {
            versions: version_responses,
            total,
            limit,
            offset,
        })
    }

    /// Get specific version details with full workflow snapshot
    pub async fn get_version_detail(
        &self,
        workflow_id: &str,
        version_id: &str,
    ) -> Result<VersionDetailResponse> {
        let version = workflow_versions::Entity::find_by_id(version_id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| SwissPipeError::NotFound(format!("Version not found: {version_id}")))?;

        // Verify it belongs to the correct workflow
        if version.workflow_id != workflow_id {
            return Err(SwissPipeError::NotFound("Version not found for this workflow".to_string()));
        }

        // Parse workflow snapshot
        let snapshot_json: serde_json::Value = serde_json::from_str(&version.workflow_snapshot)
            .map_err(|e| SwissPipeError::Generic(format!("Failed to parse workflow snapshot: {e}")))?;

        Ok(VersionDetailResponse {
            id: version.id,
            workflow_id: version.workflow_id,
            version_number: version.version_number,
            workflow_snapshot: snapshot_json,
            commit_message: version.commit_message,
            commit_description: version.commit_description,
            changed_by: version.changed_by,
            created_at: version.created_at,
        })
    }
}
