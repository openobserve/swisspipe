use crate::database::environment_variables;
use crate::variables::encryption::EncryptionService;
use crate::workflow::errors::{Result, SwissPipeError};
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, Set};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct VariableService {
    db: Arc<DatabaseConnection>,
    encryption: EncryptionService,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVariableRequest {
    pub name: String,
    pub value_type: String, // "text" or "secret"
    pub value: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateVariableRequest {
    pub value: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableResponse {
    pub id: String,
    pub name: String,
    pub value_type: String,
    pub value: String, // Masked for secrets
    pub description: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl VariableService {
    pub fn new(db: Arc<DatabaseConnection>, encryption: EncryptionService) -> Self {
        Self { db, encryption }
    }

    /// Validate variable name format (A-Z, 0-9, _)
    pub fn validate_name(name: &str) -> Result<()> {
        if name.is_empty() {
            return Err(SwissPipeError::ValidationError(
                "Variable name cannot be empty".to_string(),
            ));
        }

        if !name.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_') {
            return Err(SwissPipeError::ValidationError(
                "Variable name must contain only uppercase letters, numbers, and underscores".to_string(),
            ));
        }

        Ok(())
    }

    /// Create a new variable
    pub async fn create_variable(&self, req: CreateVariableRequest) -> Result<VariableResponse> {
        // Validate name
        Self::validate_name(&req.name)?;

        // Check if variable already exists
        let existing = environment_variables::Entity::find()
            .filter(environment_variables::Column::Name.eq(&req.name))
            .one(self.db.as_ref())
            .await?;

        if existing.is_some() {
            return Err(SwissPipeError::ValidationError(format!(
                "Variable '{}' already exists",
                req.name
            )));
        }

        // Encrypt value if secret
        let stored_value = if req.value_type == "secret" {
            self.encryption
                .encrypt(&req.value)
                .map_err(|e| SwissPipeError::InternalError(format!("Encryption failed: {e}")))?
        } else {
            req.value.clone()
        };

        let now = chrono::Utc::now().timestamp_micros();
        let variable = environment_variables::ActiveModel {
            id: Set(uuid::Uuid::now_v7().to_string()),
            name: Set(req.name.clone()),
            value_type: Set(req.value_type.clone()),
            value: Set(stored_value),
            description: Set(req.description.clone()),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let inserted = variable.insert(self.db.as_ref()).await?;

        Ok(self.model_to_response(inserted))
    }

    /// Get all variables
    pub async fn get_all_variables(&self) -> Result<Vec<VariableResponse>> {
        let variables = environment_variables::Entity::find()
            .all(self.db.as_ref())
            .await?;

        Ok(variables.into_iter().map(|v| self.model_to_response(v)).collect())
    }

    /// Get variable by ID
    pub async fn get_variable(&self, id: &str) -> Result<VariableResponse> {
        let variable = environment_variables::Entity::find_by_id(id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| SwissPipeError::NotFound(format!("Variable '{id}' not found")))?;

        Ok(self.model_to_response(variable))
    }

    /// Update variable
    pub async fn update_variable(&self, id: &str, req: UpdateVariableRequest) -> Result<VariableResponse> {
        let variable = environment_variables::Entity::find_by_id(id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| SwissPipeError::NotFound(format!("Variable '{id}' not found")))?;

        // Encrypt value if secret
        let stored_value = if variable.value_type == "secret" {
            self.encryption
                .encrypt(&req.value)
                .map_err(|e| SwissPipeError::InternalError(format!("Encryption failed: {e}")))?
        } else {
            req.value.clone()
        };

        let now = chrono::Utc::now().timestamp_micros();
        let mut active: environment_variables::ActiveModel = variable.into();
        active.value = Set(stored_value);
        active.description = Set(req.description.clone());
        active.updated_at = Set(now);

        let updated = active.update(self.db.as_ref()).await?;

        Ok(self.model_to_response(updated))
    }

    /// Delete variable
    pub async fn delete_variable(&self, id: &str) -> Result<()> {
        let variable = environment_variables::Entity::find_by_id(id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| SwissPipeError::NotFound(format!("Variable '{id}' not found")))?;

        let active: environment_variables::ActiveModel = variable.into();
        active.delete(self.db.as_ref()).await?;

        Ok(())
    }

    /// Load all variables as a HashMap for template resolution
    /// Secrets are decrypted
    pub async fn load_variables_map(&self) -> Result<HashMap<String, String>> {
        let variables = environment_variables::Entity::find()
            .all(self.db.as_ref())
            .await?;

        let mut map = HashMap::new();
        for var in variables {
            let value = if var.value_type == "secret" {
                self.encryption
                    .decrypt(&var.value)
                    .map_err(|e| SwissPipeError::InternalError(format!("Decryption failed: {e}")))?
            } else {
                var.value.clone()
            };

            map.insert(var.name, value);
        }

        Ok(map)
    }

    /// Convert model to response (mask secrets)
    fn model_to_response(&self, model: environment_variables::Model) -> VariableResponse {
        let value = if model.value_type == "secret" {
            "••••••••".to_string()
        } else {
            model.value.clone()
        };

        VariableResponse {
            id: model.id,
            name: model.name,
            value_type: model.value_type,
            value,
            description: model.description,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}
