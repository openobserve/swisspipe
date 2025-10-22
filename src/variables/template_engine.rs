use handlebars::{Handlebars, Helper, HelperResult, Output, RenderContext, RenderError};
use std::collections::HashMap;
use serde_json::Value;

/// Template engine for resolving variable references in workflow configurations
pub struct TemplateEngine {
    handlebars: Handlebars<'static>,
}

impl TemplateEngine {
    /// Create a new template engine
    pub fn new() -> Self {
        let mut handlebars = Handlebars::new();

        // Register the json helper
        handlebars.register_helper("json", Box::new(json_helper));

        handlebars.set_strict_mode(true); // Fail on undefined variables
        Self { handlebars }
    }

    /// Resolve template with variables and optional event data
    ///
    /// Template formats:
    /// - Environment variables: "https://{{ env.API_HOST }}/api"
    /// - Event data: "https://api.com/users/{{ event.data.user_id }}"
    ///
    /// # Arguments
    /// - Variables: HashMap with variable names and values
    /// - Event data: Optional WorkflowEvent data (JSON value)
    pub fn resolve(&self, template: &str, variables: &HashMap<String, String>) -> Result<String, String> {
        self.resolve_with_event(template, variables, None)
    }

    /// Resolve template with variables and event data
    pub fn resolve_with_event(
        &self,
        template: &str,
        variables: &HashMap<String, String>,
        event_data: Option<&Value>
    ) -> Result<String, String> {
        // Quick check: if no template markers, return as-is
        if !template.contains("{{") {
            return Ok(template.to_string());
        }

        // Create context with env namespace
        let mut context = serde_json::Map::new();
        let env_map: serde_json::Map<String, serde_json::Value> = variables
            .iter()
            .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
            .collect();

        context.insert("env".to_string(), serde_json::Value::Object(env_map));

        // Add event data if provided
        if let Some(event) = event_data {
            context.insert("event".to_string(), event.clone());
        }

        self.handlebars
            .render_template(template, &context)
            .map_err(|e| format!("Template resolution failed: {e}"))
    }

    /// Resolve multiple templates
    pub fn resolve_map(&self, templates: &HashMap<String, String>, variables: &HashMap<String, String>) -> Result<HashMap<String, String>, String> {
        let mut resolved = HashMap::new();
        for (key, template) in templates {
            resolved.insert(key.clone(), self.resolve(template, variables)?);
        }
        Ok(resolved)
    }
}

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_variable_resolution() {
        let engine = TemplateEngine::new();
        let mut vars = HashMap::new();
        vars.insert("API_HOST".to_string(), "https://api.example.com".to_string());

        let result = engine.resolve("{{ env.API_HOST }}/users", &vars).unwrap();
        assert_eq!(result, "https://api.example.com/users");
    }

    #[test]
    fn test_multiple_variables() {
        let engine = TemplateEngine::new();
        let mut vars = HashMap::new();
        vars.insert("API_HOST".to_string(), "https://api.example.com".to_string());
        vars.insert("VERSION".to_string(), "v1".to_string());

        let result = engine.resolve("{{ env.API_HOST }}/{{ env.VERSION }}/users", &vars).unwrap();
        assert_eq!(result, "https://api.example.com/v1/users");
    }

    #[test]
    fn test_no_template_returns_as_is() {
        let engine = TemplateEngine::new();
        let vars = HashMap::new();

        let result = engine.resolve("https://api.example.com/users", &vars).unwrap();
        assert_eq!(result, "https://api.example.com/users");
    }

    #[test]
    fn test_event_data_resolution() {
        let engine = TemplateEngine::new();
        let vars = HashMap::new();
        let event_data = serde_json::json!({
            "data": {
                "user_id": "123",
                "name": "John Doe"
            },
            "metadata": {
                "source": "api"
            }
        });

        let result = engine.resolve_with_event(
            "https://api.com/users/{{ event.data.user_id }}",
            &vars,
            Some(&event_data)
        ).unwrap();
        assert_eq!(result, "https://api.com/users/123");
    }

    #[test]
    fn test_event_and_env_combined() {
        let engine = TemplateEngine::new();
        let mut vars = HashMap::new();
        vars.insert("API_HOST".to_string(), "https://api.example.com".to_string());

        let event_data = serde_json::json!({
            "data": {
                "user_id": "456"
            }
        });

        let result = engine.resolve_with_event(
            "{{ env.API_HOST }}/users/{{ event.data.user_id }}/profile",
            &vars,
            Some(&event_data)
        ).unwrap();
        assert_eq!(result, "https://api.example.com/users/456/profile");
    }

    #[test]
    fn test_event_metadata_access() {
        let engine = TemplateEngine::new();
        let vars = HashMap::new();
        let event_data = serde_json::json!({
            "data": {
                "id": "789"
            },
            "metadata": {
                "request_id": "abc-123"
            }
        });

        let result = engine.resolve_with_event(
            "https://api.com/resource?id={{ event.data.id }}&trace={{ event.metadata.request_id }}",
            &vars,
            Some(&event_data)
        ).unwrap();
        assert_eq!(result, "https://api.com/resource?id=789&trace=abc-123");
    }

    #[test]
    fn test_undefined_variable_fails() {
        let engine = TemplateEngine::new();
        let vars = HashMap::new();

        let result = engine.resolve("{{ env.UNDEFINED_VAR }}", &vars);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Template resolution failed"));
    }

    #[test]
    fn test_resolve_map() {
        let engine = TemplateEngine::new();
        let mut vars = HashMap::new();
        vars.insert("TOKEN".to_string(), "secret123".to_string());

        let mut templates = HashMap::new();
        templates.insert("auth".to_string(), "Bearer {{ env.TOKEN }}".to_string());
        templates.insert("key".to_string(), "Key {{ env.TOKEN }}".to_string());

        let result = engine.resolve_map(&templates, &vars).unwrap();
        assert_eq!(result.get("auth").unwrap(), "Bearer secret123");
        assert_eq!(result.get("key").unwrap(), "Key secret123");
    }
}

// Handlebars helper function for JSON serialization
fn json_helper(
    h: &Helper,
    _: &Handlebars,
    _: &handlebars::Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let value = h.param(0)
        .ok_or_else(|| RenderError::new("json helper requires a parameter"))?;

    let json_str = serde_json::to_string_pretty(value.value())
        .map_err(|e| RenderError::new(format!("Failed to serialize to JSON: {e}")))?;

    out.write(&json_str)?;
    Ok(())
}
