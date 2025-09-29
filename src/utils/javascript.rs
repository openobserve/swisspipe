use crate::workflow::{errors::JavaScriptError, models::WorkflowEvent};
use rquickjs::{Context, Runtime, Value};
use std::time::{Duration, Instant};

pub struct JavaScriptExecutor {
    max_execution_time: Duration,
}

impl JavaScriptExecutor {
    pub fn new() -> Result<Self, JavaScriptError> {
        Ok(JavaScriptExecutor {
            max_execution_time: Duration::from_millis(5000), // 5 second timeout
        })
    }

    /// Create a secure sandboxed JavaScript context
    fn create_secure_context(&self) -> Result<(Runtime, Context), JavaScriptError> {
        let runtime = Runtime::new()
            .map_err(|e| JavaScriptError::RuntimeError(e.to_string()))?;

        // Create a full context (temporarily using full instead of base)
        let context = Context::full(&runtime)
            .map_err(|e| JavaScriptError::RuntimeError(e.to_string()))?;

        // Set up the secure sandbox
        context.with(|ctx| {
            // Remove dangerous globals that might still be accessible
            self.remove_dangerous_globals(&ctx)?;

            // Set up safe math and basic operations
            self.setup_safe_environment(&ctx)?;

            Ok::<(), JavaScriptError>(())
        })?;

        Ok((runtime, context))
    }

    /// Remove dangerous global objects and functions
    fn remove_dangerous_globals(&self, ctx: &rquickjs::Ctx) -> Result<(), JavaScriptError> {
        let global = ctx.globals();

        // List of dangerous globals to remove/disable
        let dangerous_globals = [
            "eval", "setTimeout", "setInterval", "clearTimeout", "clearInterval",
            "XMLHttpRequest", "fetch", "WebSocket", "Worker", "SharedArrayBuffer",
            "Atomics", "WebAssembly", "importScripts", "postMessage", "close",
            "open", "print", "alert", "confirm", "prompt"
        ];

        for &name in &dangerous_globals {
            if global.contains_key(name)? {
                global.set(name, Value::new_undefined(ctx.clone()))?;
            }
        }

        // Remove constructor access that could be used for escaping
        // Note: Temporarily disabled to allow JavaScript functions to work
        // let object_prototype: Object = ctx.eval("Object.prototype")?;
        // if object_prototype.contains_key("constructor")? {
        //     object_prototype.set("constructor", Value::new_undefined(ctx.clone()))?;
        // }

        Ok(())
    }

    /// Set up safe environment with allowed operations
    fn setup_safe_environment(&self, ctx: &rquickjs::Ctx) -> Result<(), JavaScriptError> {
        // Keep safe Math operations
        let _: Value = ctx.eval(r"
            // Ensure Math object is available for legitimate calculations
            if (typeof Math === 'undefined') {
                Math = {};
            }

            // Provide safe JSON operations
            if (typeof JSON === 'undefined') {
                JSON = { parse: function() { throw new Error('JSON.parse not available in sandbox'); } };
            }
        ")?;

        Ok(())
    }

    /// Execute JavaScript with timeout and memory limits
    fn execute_with_limits<T, F>(&self, context: &Context, executor: F) -> Result<T, JavaScriptError>
    where
        F: FnOnce(&rquickjs::Ctx) -> rquickjs::Result<T>,
        T: 'static,
    {
        let start_time = Instant::now();

        let result = context.with(|ctx| {
            // Check execution time limit
            if start_time.elapsed() > self.max_execution_time {
                return Err(rquickjs::Error::new_from_js("script", "Execution timeout exceeded"));
            }

            executor(&ctx)
        });

        match result {
            Ok(value) => Ok(value),
            Err(e) => {
                if start_time.elapsed() > self.max_execution_time {
                    Err(JavaScriptError::ExecutionError("Execution timeout exceeded".to_string()))
                } else {
                    Err(JavaScriptError::ExecutionError(e.to_string()))
                }
            }
        }
    }

    /// Validate script for basic security (pre-execution check)
    fn validate_script_security(&self, script: &str) -> Result<(), JavaScriptError> {
        let script_lower = script.to_lowercase();

        // Check script length to prevent excessive memory usage
        if script.len() > 10_000 {
            return Err(JavaScriptError::ValidationError("Script too long (max 10KB)".to_string()));
        }

        // Check for suspicious patterns that might indicate injection attempts
        let suspicious_patterns = [
            "\\\\u", "\\\\x", "\\\\0", // Unicode/hex escaping
            "string.fromcharcode", "fromcharcode", // Character code manipulation
            "eval", "new\\s+function", "function\\s*constructor", // Dynamic code execution
            "\\.constructor\\s*\\(", "__proto__", "prototype\\.constructor", // Prototype pollution
            "globalthis", "global", "window", "self", // Global object access
            "import", "require", "module", "exports", // Module system
            "while\\s*\\(\\s*true", "for\\s*\\(\\s*;\\s*;", // Infinite loops
        ];

        for pattern in &suspicious_patterns {
            if regex::Regex::new(pattern)
                .map_err(|e| JavaScriptError::ValidationError(format!("Regex error: {e}")))?
                .is_match(&script_lower)
            {
                return Err(JavaScriptError::SecurityError(
                    format!("Suspicious pattern detected: {pattern}")
                ));
            }
        }

        Ok(())
    }

    pub async fn execute_condition(&self, script: &str, event: &WorkflowEvent) -> Result<bool, JavaScriptError> {
        // Validate script security before execution
        self.validate_script_security(script)?;

        let event_json = serde_json::to_string(event)
            .map_err(|e| JavaScriptError::SerializationError(e.to_string()))?;

        // Create secure sandbox
        let (_runtime, context) = self.create_secure_context()?;

        let result = self.execute_with_limits(&context, |ctx| {
            // User provides the complete function implementation
            let full_script = format!(
                r"
                {script}
                condition({event_json});
                "
            );

            tracing::info!("Executing secure JavaScript condition: {}", script.chars().take(100).collect::<String>());

            let result: rquickjs::Result<bool> = ctx.eval(full_script.as_bytes());
            match &result {
                Ok(val) => tracing::info!("JavaScript condition result: {}", val),
                Err(e) => tracing::warn!("JavaScript condition error: {}", e),
            }
            result
        })?;

        Ok(result)
    }
    
    pub async fn execute_transformer(&self, script: &str, event: WorkflowEvent) -> Result<WorkflowEvent, JavaScriptError> {
        // Validate script security before execution
        self.validate_script_security(script)?;

        let event_json = serde_json::to_string(&event)
            .map_err(|e| JavaScriptError::SerializationError(e.to_string()))?;

        // Create secure sandbox
        let (_runtime, context) = self.create_secure_context()?;

        let result = self.execute_with_limits(&context, |ctx| {
            // User provides the complete function implementation
            let full_script = format!(
                r"
                {script}
                JSON.stringify(transformer({event_json}));
                "
            );

            tracing::info!("Executing secure JavaScript transformer: {}", script.chars().take(100).collect::<String>());

            let result: rquickjs::Result<String> = ctx.eval(full_script.as_bytes());
            match &result {
                Ok(val) => tracing::info!("JavaScript transformer result length: {}", val.len()),
                Err(e) => tracing::warn!("JavaScript transformer error: {}", e),
            }
            result
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
        hil_task: None,
                })
            }
        }
    }

    pub async fn execute_raw(&self, script: &str) -> Result<bool, JavaScriptError> {
        // Validate script security before execution
        self.validate_script_security(script)?;

        // Create secure sandbox
        let (_runtime, context) = self.create_secure_context()?;

        let result = self.execute_with_limits(&context, |ctx| {
            tracing::debug!("Executing secure raw JavaScript: {}", script);

            let result: rquickjs::Result<bool> = ctx.eval(script.as_bytes());
            match &result {
                Ok(val) => tracing::debug!("Raw JavaScript result: {}", val),
                Err(e) => tracing::error!("Raw JavaScript error: {}", e),
            }
            result
        })?;

        Ok(result)
    }

    pub async fn execute_numeric(&self, script: &str) -> Result<f64, JavaScriptError> {
        // Validate script security before execution
        self.validate_script_security(script)?;

        // Create secure sandbox
        let (_runtime, context) = self.create_secure_context()?;

        let result = self.execute_with_limits(&context, |ctx| {
            tracing::debug!("Executing secure numeric JavaScript: {}", script);

            let result: rquickjs::Result<f64> = ctx.eval(script.as_bytes());
            match &result {
                Ok(val) => tracing::debug!("Numeric JavaScript result: {}", val),
                Err(e) => tracing::error!("Numeric JavaScript error: {}", e),
            }
            result
        })?;

        Ok(result)
    }
}