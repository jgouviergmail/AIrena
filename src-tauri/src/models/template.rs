//! Discussion templates (v1.20): a named, reusable setup (topic, mode, cast,
//! options). The configuration is an opaque JSON object owned by the frontend
//! (`TemplateConfig` in `src/lib/templates.ts`); the backend only stores it.

use serde::{Deserialize, Serialize};

use crate::constants;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscussionTemplate {
    pub id: String,
    pub name: String,
    /// JSON object (validated on save, never interpreted here)
    pub config_json: String,
    /// Seeded templates cannot be deleted (their name is translated by the frontend)
    #[serde(default)]
    pub builtin: bool,
    #[serde(default)]
    pub created_at: String,
}

impl DiscussionTemplate {
    /// A template needs a name and a JSON object as configuration.
    pub fn validate(&self) -> Result<(), String> {
        if self.id.trim().is_empty() {
            return Err("Template id must not be empty".to_string());
        }
        if self.name.trim().is_empty() {
            return Err("Template name must not be empty".to_string());
        }
        if self.name.chars().count() > constants::TEMPLATE_NAME_MAX_CHARS {
            return Err(format!("Template name is limited to {} characters", constants::TEMPLATE_NAME_MAX_CHARS));
        }
        if self.config_json.len() > constants::TEMPLATE_CONFIG_MAX_BYTES {
            return Err(format!("Template configuration is limited to {} bytes", constants::TEMPLATE_CONFIG_MAX_BYTES));
        }
        match serde_json::from_str::<serde_json::Value>(&self.config_json) {
            Ok(serde_json::Value::Object(_)) => Ok(()),
            _ => Err("Template configuration must be a JSON object".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_requires_a_name_and_a_json_object() {
        let ok = DiscussionTemplate { id: "t".into(), name: "Procès".into(), config_json: r#"{"topic":"x"}"#.into(), builtin: false, created_at: String::new() };
        assert!(ok.validate().is_ok());
        assert!(DiscussionTemplate { name: " ".into(), ..ok.clone() }.validate().is_err());
        assert!(DiscussionTemplate { config_json: "[1]".into(), ..ok.clone() }.validate().is_err());
        assert!(DiscussionTemplate { config_json: "nope".into(), ..ok.clone() }.validate().is_err());
        assert!(DiscussionTemplate { id: String::new(), ..ok.clone() }.validate().is_err());
        // Bounded: a name or a configuration past the limits is refused
        assert!(DiscussionTemplate { name: "n".repeat(crate::constants::TEMPLATE_NAME_MAX_CHARS + 1), ..ok.clone() }.validate().is_err());
        let huge = format!("{{\"topic\":\"{}\"}}", "x".repeat(crate::constants::TEMPLATE_CONFIG_MAX_BYTES));
        assert!(DiscussionTemplate { config_json: huge, ..ok }.validate().is_err());
    }
}
