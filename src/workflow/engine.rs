// Workflow engine - refactored for modularity and maintainability
// This file has been broken down into focused modules for better organization

// Import and re-export the modules
mod workflow_loader;
mod node_executor;
mod dag_executor;

use crate::{
    anthropic::AnthropicService,
    async_execution::StepTracker,
    email::service::EmailService,
    utils::{http_client::AppExecutor, javascript::JavaScriptExecutor},
    workflow::{
        errors::{Result, SwissPipeError},
        models::{Workflow, WorkflowEvent},
        input_sync::InputSyncService,
    },
};
use sea_orm::DatabaseConnection;
use std::sync::Arc;

pub use workflow_loader::WorkflowLoader;
pub use node_executor::NodeExecutor;
pub use dag_executor::DagExecutor;

/// Main WorkflowEngine struct - now acts as a coordinator for the modular components
pub struct WorkflowEngine {
    workflow_loader: Arc<WorkflowLoader>,
    node_executor: Arc<NodeExecutor>,
    dag_executor: Arc<DagExecutor>,

    // Keep public fields for backward compatibility with existing code
    pub js_executor: Arc<JavaScriptExecutor>,
    pub app_executor: Arc<AppExecutor>,
    pub email_service: Arc<EmailService>,
    pub anthropic_service: Arc<AnthropicService>,
    pub input_sync_service: Arc<InputSyncService>,
}

impl WorkflowEngine {
    /// Create a new workflow engine with all necessary services
    pub fn new(db: Arc<DatabaseConnection>) -> Result<Self> {
        // Initialize all services
        let js_executor = Arc::new(JavaScriptExecutor::new()?);
        let app_executor = Arc::new(AppExecutor::new());
        let email_service = Arc::new(EmailService::new(db.clone())
            .map_err(|e| SwissPipeError::Generic(e.to_string()))?);
        let anthropic_service = Arc::new(AnthropicService::new());
        let input_sync_service = Arc::new(InputSyncService::new(db.clone()));

        // Create modular components
        let workflow_loader = Arc::new(WorkflowLoader::new(db.clone()));

        let step_tracker = Arc::new(StepTracker::new(db.clone()));
        let node_executor = Arc::new(NodeExecutor::new(
            js_executor.clone(),
            app_executor.clone(),
            email_service.clone(),
            anthropic_service.clone(),
            db.clone(),
            step_tracker,
        ));

        let dag_executor = Arc::new(DagExecutor::new(
            node_executor.clone(),
            input_sync_service.clone(),
        ));

        Ok(Self {
            workflow_loader,
            node_executor,
            dag_executor,
            js_executor,
            app_executor,
            email_service,
            anthropic_service,
            input_sync_service,
        })
    }

    /// Load a workflow from the database
    pub async fn load_workflow(&self, workflow_id: &str) -> Result<Workflow> {
        self.workflow_loader.load_workflow(workflow_id).await
    }

    /// Get a workflow, returning None if not found
    pub async fn get_workflow(&self, workflow_id: &str) -> Result<Option<Workflow>> {
        self.workflow_loader.get_workflow(workflow_id).await
    }

    /// Execute a workflow using DAG traversal
    pub async fn execute_workflow(&self, workflow: &Workflow, event: WorkflowEvent, execution_id: &str) -> Result<WorkflowEvent> {
        self.dag_executor.execute_workflow(workflow, event, execution_id).await
    }

    /// Get direct access to the workflow loader
    pub fn workflow_loader(&self) -> &Arc<WorkflowLoader> {
        &self.workflow_loader
    }

    /// Get direct access to the node executor
    pub fn node_executor(&self) -> &Arc<NodeExecutor> {
        &self.node_executor
    }

    /// Set the HTTP loop scheduler for dependency injection
    pub fn set_http_loop_scheduler(&self, scheduler: Arc<crate::async_execution::HttpLoopScheduler>) -> Result<()> {
        self.node_executor.set_http_loop_scheduler(scheduler)
            .map_err(|_| SwissPipeError::Generic("HTTP loop scheduler already initialized".to_string()))?;
        Ok(())
    }

    /// Set the HIL service for dependency injection
    pub fn set_hil_service(&self, service: Arc<crate::hil::HilService>) -> Result<()> {
        self.node_executor.set_hil_service(service)
            .map_err(|_| SwissPipeError::Generic("HIL service already initialized".to_string()))?;
        Ok(())
    }

    /// Set the variable service for dependency injection
    pub fn set_variable_service(&self, service: Arc<crate::variables::VariableService>) -> Result<()> {
        self.node_executor.set_variable_service(service)
            .map_err(|_| SwissPipeError::Generic("Variable service already initialized".to_string()))?;
        Ok(())
    }

    /// Set the template engine for dependency injection
    pub fn set_template_engine(&self, engine: Arc<crate::variables::TemplateEngine>) -> Result<()> {
        self.node_executor.set_template_engine(engine)
            .map_err(|_| SwissPipeError::Generic("Template engine already initialized".to_string()))?;
        Ok(())
    }

    /// Get direct access to the DAG executor
    pub fn dag_executor(&self) -> &Arc<DagExecutor> {
        &self.dag_executor
    }
}