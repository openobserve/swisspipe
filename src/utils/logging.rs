/// Structured logging utilities for workflow execution
///
/// These macros provide consistent JSON-formatted logging with workflow context

/// Log an error with full workflow context (workflow_id, execution_id, node_id)
#[macro_export]
macro_rules! log_workflow_error {
    // Full context with error
    ($workflow_id:expr, $execution_id:expr, $node_id:expr, $msg:expr, $error:expr) => {
        tracing::error!(
            workflow_id = %$workflow_id,
            execution_id = %$execution_id,
            node_id = %$node_id,
            error = %$error,
            "{}",
            $msg
        )
    };
    // Without node_id
    ($workflow_id:expr, $execution_id:expr, $msg:expr, $error:expr) => {
        tracing::error!(
            workflow_id = %$workflow_id,
            execution_id = %$execution_id,
            error = %$error,
            "{}",
            $msg
        )
    };
    // Only workflow_id
    ($workflow_id:expr, $msg:expr, $error:expr) => {
        tracing::error!(
            workflow_id = %$workflow_id,
            error = %$error,
            "{}",
            $msg
        )
    };
}

/// Log a warning with full workflow context
#[macro_export]
macro_rules! log_workflow_warn {
    // Full context
    ($workflow_id:expr, $execution_id:expr, $node_id:expr, $msg:expr) => {
        tracing::warn!(
            workflow_id = %$workflow_id,
            execution_id = %$execution_id,
            node_id = %$node_id,
            "{}",
            $msg
        )
    };
    // Without node_id
    ($workflow_id:expr, $execution_id:expr, $msg:expr) => {
        tracing::warn!(
            workflow_id = %$workflow_id,
            execution_id = %$execution_id,
            "{}",
            $msg
        )
    };
    // Only workflow_id
    ($workflow_id:expr, $msg:expr) => {
        tracing::warn!(
            workflow_id = %$workflow_id,
            "{}",
            $msg
        )
    };
}

/// Log an error with optional workflow name
#[macro_export]
macro_rules! log_workflow_error_named {
    ($workflow_id:expr, $workflow_name:expr, $execution_id:expr, $node_id:expr, $msg:expr, $error:expr) => {
        tracing::error!(
            workflow_id = %$workflow_id,
            workflow_name = %$workflow_name,
            execution_id = %$execution_id,
            node_id = %$node_id,
            error = %$error,
            "{}",
            $msg
        )
    };
    ($workflow_id:expr, $workflow_name:expr, $execution_id:expr, $msg:expr, $error:expr) => {
        tracing::error!(
            workflow_id = %$workflow_id,
            workflow_name = %$workflow_name,
            execution_id = %$execution_id,
            error = %$error,
            "{}",
            $msg
        )
    };
}

/// Log a warning with optional workflow name
#[macro_export]
macro_rules! log_workflow_warn_named {
    ($workflow_id:expr, $workflow_name:expr, $execution_id:expr, $node_id:expr, $msg:expr) => {
        tracing::warn!(
            workflow_id = %$workflow_id,
            workflow_name = %$workflow_name,
            execution_id = %$execution_id,
            node_id = %$node_id,
            "{}",
            $msg
        )
    };
    ($workflow_id:expr, $workflow_name:expr, $execution_id:expr, $msg:expr) => {
        tracing::warn!(
            workflow_id = %$workflow_id,
            workflow_name = %$workflow_name,
            execution_id = %$execution_id,
            "{}",
            $msg
        )
    };
}