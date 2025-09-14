use crate::workflow::{errors::JavaScriptError, models::WorkflowEvent};
use rquickjs::{Context, Runtime};

pub struct JavaScriptExecutor;

impl JavaScriptExecutor {
    pub fn new() -> Result<Self, JavaScriptError> {
        Ok(JavaScriptExecutor)
    }
    
    pub async fn execute_condition(&self, script: &str, event: &WorkflowEvent) -> Result<bool, JavaScriptError> {
        let event_json = serde_json::to_string(event)
            .map_err(|e| JavaScriptError::SerializationError(e.to_string()))?;
        
        // Create a new runtime for each execution
        let runtime = Runtime::new()
            .map_err(|e| JavaScriptError::RuntimeError(e.to_string()))?;
        
        let context = Context::full(&runtime)
            .map_err(|e| JavaScriptError::RuntimeError(e.to_string()))?;
            
        let result = context.with(|ctx| {
            // User provides the complete function implementation
            let full_script = format!(
                r#"
                {script}
                condition({event_json});
                "#
            );
            
            tracing::info!("Executing JavaScript condition: {}", full_script);
            
            let result: rquickjs::Result<bool> = ctx.eval(full_script.as_bytes());
            match &result {
                Ok(val) => tracing::info!("JavaScript condition result: {}", val),
                Err(e) => tracing::error!("JavaScript condition error: {}", e),
            }
            result.map_err(|e| JavaScriptError::ExecutionError(e.to_string()))
        })?;
        
        Ok(result)
    }
    
    pub async fn execute_transformer(&self, script: &str, event: WorkflowEvent) -> Result<WorkflowEvent, JavaScriptError> {
        let event_json = serde_json::to_string(&event)
            .map_err(|e| JavaScriptError::SerializationError(e.to_string()))?;
        
        // Create a new runtime for each execution
        let runtime = Runtime::new()
            .map_err(|e| JavaScriptError::RuntimeError(e.to_string()))?;
        
        let context = Context::full(&runtime)
            .map_err(|e| JavaScriptError::RuntimeError(e.to_string()))?;
            
        let result = context.with(|ctx| {
            // User provides the complete function implementation
            let full_script = format!(
                r#"
                {script}
                JSON.stringify(transformer({event_json}));
                "#
            );
            
            let result: rquickjs::Result<String> = ctx.eval(full_script.as_bytes());
            result.map_err(|e| JavaScriptError::ExecutionError(e.to_string()))
        })?;
        
        // Handle null return (drop event case)
        if result == "null" {
            return Err(JavaScriptError::EventDropped);
        }

        // Try to parse as complete WorkflowEvent first
        match serde_json::from_str::<WorkflowEvent>(&result) {
            Ok(transformed_event) => Ok(transformed_event),
            Err(_) => {
                // If that fails, try to parse as just data and preserve original structure
                let data_value: serde_json::Value = serde_json::from_str(&result)
                    .map_err(|e| JavaScriptError::SerializationError(e.to_string()))?;

                Ok(WorkflowEvent {
                    data: data_value,
                    metadata: event.metadata,
                    headers: event.headers,
                    condition_results: event.condition_results,
                })
            }
        }
    }
}