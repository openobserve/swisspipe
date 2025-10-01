use handlebars::Handlebars;
use std::collections::HashMap;

/// Template engine for resolving variable references in workflow configurations
pub struct TemplateEngine {
    handlebars: Handlebars<'static>,
}

impl TemplateEngine {
    /// Create a new template engine
    pub fn new() -> Self {
        let mut handlebars = Handlebars::new();
        handlebars.set_strict_mode(true); // Fail on undefined variables
        Self { handlebars }
    }

    /// Resolve template with variables
    /// Template format: "https://{{ env.API_HOST }}/api"
    /// Variables: HashMap with variable names and values
    pub fn resolve(&self, template: &str, variables: &HashMap<String, String>) -> Result<String, String> {
        // Quick check: if no env variable references, return as-is
        // This allows other template engines (like email templates) to process their own helpers
        if !template.contains("{{ env.") && !template.contains("{{env.") {
            return Ok(template.to_string());
        }

        // Create context with env namespace
        let mut context = serde_json::Map::new();
        let env_map: serde_json::Map<String, serde_json::Value> = variables
            .iter()
            .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
            .collect();

        context.insert("env".to_string(), serde_json::Value::Object(env_map));

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
    fn test_non_env_templates_pass_through() {
        let engine = TemplateEngine::new();
        let vars = HashMap::new();

        // Templates with other helpers (like email templates) should pass through
        let result = engine.resolve("Hello {{json data}}", &vars).unwrap();
        assert_eq!(result, "Hello {{json data}}");

        let result = engine.resolve("Date: {{date_format timestamp}}", &vars).unwrap();
        assert_eq!(result, "Date: {{date_format timestamp}}");
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
