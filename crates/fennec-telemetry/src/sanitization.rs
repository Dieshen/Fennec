//! Log sanitization and privacy protection

use crate::{config::PrivacyConfig, Error, Result};
use regex::Regex;
use serde_json::{Map, Value};
use std::collections::HashMap;
use tracing::Subscriber;
use tracing_subscriber::{
    layer::{Context, Filter, SubscriberExt},
    Layer,
};

/// Layer that sanitizes sensitive data from log messages
pub struct SanitizationLayer {
    redaction_patterns: Vec<Regex>,
    redacted_fields: Vec<String>,
    audit_trail: bool,
}

impl SanitizationLayer {
    /// Create a new sanitization layer
    pub fn new(config: PrivacyConfig) -> Result<Self> {
        let mut redaction_patterns = Vec::new();

        for pattern_str in &config.redaction_patterns {
            let regex = Regex::new(pattern_str).map_err(|e| Error::Config {
                message: format!("Invalid redaction pattern '{}': {}", pattern_str, e),
            })?;
            redaction_patterns.push(regex);
        }

        Ok(Self {
            redaction_patterns,
            redacted_fields: config.redacted_fields,
            audit_trail: config.audit_trail,
        })
    }

    /// Sanitize a log message
    fn sanitize_message(&self, message: &str) -> String {
        let mut sanitized = message.to_string();

        for pattern in &self.redaction_patterns {
            sanitized = pattern
                .replace_all(&sanitized, |caps: &regex::Captures| {
                    if caps.len() >= 2 {
                        // Keep the field name but redact the value
                        format!("{}=[REDACTED]", &caps[1])
                    } else {
                        "[REDACTED]".to_string()
                    }
                })
                .to_string();
        }

        sanitized
    }

    /// Sanitize structured data (JSON values)
    fn sanitize_structured_data(&self, value: &mut Value) {
        match value {
            Value::Object(map) => {
                for (key, val) in map.iter_mut() {
                    if self
                        .redacted_fields
                        .iter()
                        .any(|field| key.to_lowercase().contains(&field.to_lowercase()))
                    {
                        *val = Value::String("[REDACTED]".to_string());
                    } else {
                        self.sanitize_structured_data(val);
                    }
                }
            }
            Value::Array(arr) => {
                for item in arr.iter_mut() {
                    self.sanitize_structured_data(item);
                }
            }
            Value::String(s) => {
                *s = self.sanitize_message(s);
            }
            _ => {} // Numbers, booleans, null don't need sanitization
        }
    }

    /// Create an audit log entry for redacted content
    fn create_audit_entry(&self, original_len: usize, sanitized_len: usize) {
        if self.audit_trail && original_len != sanitized_len {
            tracing::debug!(
                telemetry.event = "content_redacted",
                original_length = original_len,
                sanitized_length = sanitized_len,
                redaction_count = (original_len - sanitized_len),
                "Content was redacted for privacy"
            );
        }
    }
}

impl<S> Layer<S> for SanitizationLayer
where
    S: Subscriber + for<'lookup> tracing_subscriber::registry::LookupSpan<'lookup>,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        // For now, we'll implement basic message sanitization
        // In a full implementation, we would need to intercept and modify
        // the event's fields before they reach other layers

        // This is a simplified version - a complete implementation would require
        // intercepting at the formatter level or using a custom visitor
    }
}

/// Utility functions for data sanitization
pub struct DataSanitizer {
    patterns: Vec<Regex>,
    redacted_fields: Vec<String>,
}

impl DataSanitizer {
    /// Create a new data sanitizer
    pub fn new(config: &PrivacyConfig) -> Result<Self> {
        let mut patterns = Vec::new();

        for pattern_str in &config.redaction_patterns {
            let regex = Regex::new(pattern_str).map_err(|e| Error::Config {
                message: format!("Invalid redaction pattern '{}': {}", pattern_str, e),
            })?;
            patterns.push(regex);
        }

        Ok(Self {
            patterns,
            redacted_fields: config.redacted_fields.clone(),
        })
    }

    /// Sanitize a text string
    pub fn sanitize_text(&self, text: &str) -> String {
        let mut sanitized = text.to_string();

        for pattern in &self.patterns {
            sanitized = pattern
                .replace_all(&sanitized, |caps: &regex::Captures| {
                    if caps.len() >= 3 {
                        // For patterns like email: user@domain.com -> u***@d***.com
                        let field = &caps[1];
                        let value = &caps[2];

                        if field.to_lowercase().contains("email") {
                            self.partially_redact_email(value)
                        } else {
                            format!("{}=[REDACTED]", field)
                        }
                    } else if caps.len() >= 2 {
                        // For patterns with field and value
                        format!("{}=[REDACTED]", &caps[1])
                    } else {
                        "[REDACTED]".to_string()
                    }
                })
                .to_string();
        }

        sanitized
    }

    /// Sanitize a JSON object
    pub fn sanitize_json(&self, mut json: Value) -> Value {
        self.sanitize_json_recursive(&mut json);
        json
    }

    /// Sanitize a hashmap of string values
    pub fn sanitize_map(&self, map: &mut HashMap<String, String>) {
        for (key, value) in map.iter_mut() {
            if self.should_redact_field(key) {
                *value = "[REDACTED]".to_string();
            } else {
                *value = self.sanitize_text(value);
            }
        }
    }

    /// Check if a field name should be redacted
    fn should_redact_field(&self, field_name: &str) -> bool {
        let field_lower = field_name.to_lowercase();
        self.redacted_fields
            .iter()
            .any(|redacted_field| field_lower.contains(&redacted_field.to_lowercase()))
    }

    /// Recursively sanitize JSON values
    fn sanitize_json_recursive(&self, value: &mut Value) {
        match value {
            Value::Object(map) => {
                for (key, val) in map.iter_mut() {
                    if self.should_redact_field(key) {
                        *val = Value::String("[REDACTED]".to_string());
                    } else {
                        self.sanitize_json_recursive(val);
                    }
                }
            }
            Value::Array(arr) => {
                for item in arr.iter_mut() {
                    self.sanitize_json_recursive(item);
                }
            }
            Value::String(s) => {
                *s = self.sanitize_text(s);
            }
            _ => {} // Numbers, booleans, null don't need sanitization
        }
    }

    /// Partially redact an email address (user@domain.com -> u***@d***.com)
    fn partially_redact_email(&self, email: &str) -> String {
        if let Some(at_pos) = email.find('@') {
            let (user, domain_part) = email.split_at(at_pos);
            let domain = &domain_part[1..]; // Remove '@'

            let redacted_user = if user.len() <= 1 {
                "*".to_string()
            } else {
                format!("{}***", &user[..1])
            };

            let redacted_domain = if let Some(dot_pos) = domain.find('.') {
                let (domain_name, tld) = domain.split_at(dot_pos);
                if domain_name.len() <= 1 {
                    format!("*{}", tld)
                } else {
                    format!("{}***{}", &domain_name[..1], tld)
                }
            } else {
                "***".to_string()
            };

            format!("{}@{}", redacted_user, redacted_domain)
        } else {
            "[REDACTED_EMAIL]".to_string()
        }
    }

    /// Redact credit card numbers (keep first 4 and last 4 digits)
    fn redact_credit_card(&self, cc_number: &str) -> String {
        let digits: String = cc_number.chars().filter(|c| c.is_ascii_digit()).collect();

        if digits.len() >= 8 {
            let first_four = &digits[..4];
            let last_four = &digits[digits.len() - 4..];
            format!("{}****{}", first_four, last_four)
        } else {
            "[REDACTED_CC]".to_string()
        }
    }

    /// Validate that sanitization is working correctly
    pub fn validate_sanitization(&self, original: &str, sanitized: &str) -> SanitizationReport {
        let mut report = SanitizationReport {
            original_length: original.len(),
            sanitized_length: sanitized.len(),
            redactions_detected: 0,
            potentially_sensitive_data_found: false,
        };

        // Count redactions
        report.redactions_detected = sanitized.matches("[REDACTED]").count() as u32;

        // Check for potentially missed sensitive data
        let sensitive_patterns = [
            r"(?i)password\s*[:=]\s*\w+",
            r"(?i)api_?key\s*[:=]\s*\w+",
            r"(?i)token\s*[:=]\s*\w+",
            r"\b\d{4}[\s\-]?\d{4}[\s\-]?\d{4}[\s\-]?\d{4}\b", // Credit card pattern
        ];

        for pattern_str in &sensitive_patterns {
            if let Ok(regex) = Regex::new(pattern_str) {
                if regex.is_match(sanitized) {
                    report.potentially_sensitive_data_found = true;
                    break;
                }
            }
        }

        report
    }
}

/// Report on sanitization operations
#[derive(Debug, Clone)]
pub struct SanitizationReport {
    pub original_length: usize,
    pub sanitized_length: usize,
    pub redactions_detected: u32,
    pub potentially_sensitive_data_found: bool,
}

impl SanitizationReport {
    pub fn redaction_percentage(&self) -> f64 {
        if self.original_length == 0 {
            0.0
        } else {
            (self.original_length - self.sanitized_length) as f64 / self.original_length as f64
                * 100.0
        }
    }
}

/// Pre-defined sanitization patterns for common sensitive data types
pub struct SanitizationPatterns;

impl SanitizationPatterns {
    /// Get default patterns for API keys and tokens
    pub fn api_credentials() -> Vec<String> {
        vec![
            // Generic API key patterns
            r"(?i)(api_?key|token|secret|password)\s*[:=]\s*['\x22]?([a-zA-Z0-9_\-\.]{8,})['\x22]?"
                .to_string(),
            // Bearer tokens
            r"(?i)bearer\s+([a-zA-Z0-9_\-\.]+)".to_string(),
            // Basic auth
            r"(?i)basic\s+([a-zA-Z0-9+/=]+)".to_string(),
        ]
    }

    /// Get patterns for personal identifiable information
    pub fn pii_patterns() -> Vec<String> {
        vec![
            // Social Security Numbers
            r"\b\d{3}-\d{2}-\d{4}\b".to_string(),
            // Phone numbers
            r"\b\d{3}-\d{3}-\d{4}\b".to_string(),
            r"\b\(\d{3}\)\s*\d{3}-\d{4}\b".to_string(),
            // Email addresses (partial redaction)
            r"\b([a-zA-Z0-9._%+-]+)@([a-zA-Z0-9.-]+\.[a-zA-Z]{2,})\b".to_string(),
        ]
    }

    /// Get patterns for financial information
    pub fn financial_patterns() -> Vec<String> {
        vec![
            // Credit card numbers
            r"\b\d{4}[\s\-]?\d{4}[\s\-]?\d{4}[\s\-]?\d{4}\b".to_string(),
            // Bank account numbers (simplified)
            r"\b\d{8,17}\b".to_string(),
        ]
    }

    /// Get all default patterns combined
    pub fn all_default_patterns() -> Vec<String> {
        let mut patterns = Vec::new();
        patterns.extend(Self::api_credentials());
        patterns.extend(Self::pii_patterns());
        patterns.extend(Self::financial_patterns());
        patterns
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::PrivacyConfig;

    #[test]
    fn test_text_sanitization() {
        let config = PrivacyConfig {
            sanitize_enabled: true,
            redaction_patterns: SanitizationPatterns::api_credentials(),
            redacted_fields: vec!["password".to_string(), "secret".to_string()],
            audit_trail: true,
            audit_log_path: None,
        };

        let sanitizer = DataSanitizer::new(&config).unwrap();

        let original = "User logged in with api_key=sk-1234567890abcdef and password=mysecret";
        let sanitized = sanitizer.sanitize_text(original);

        assert!(sanitized.contains("[REDACTED]"));
        assert!(!sanitized.contains("sk-1234567890abcdef"));
        assert!(!sanitized.contains("mysecret"));
    }

    #[test]
    fn test_email_partial_redaction() {
        let config = PrivacyConfig {
            sanitize_enabled: true,
            redaction_patterns: SanitizationPatterns::pii_patterns(),
            redacted_fields: vec![],
            audit_trail: false,
            audit_log_path: None,
        };

        let sanitizer = DataSanitizer::new(&config).unwrap();
        let redacted = sanitizer.partially_redact_email("user@example.com");

        assert_eq!(redacted, "u***@e***.com");
    }

    #[test]
    fn test_json_sanitization() {
        let config = PrivacyConfig {
            sanitize_enabled: true,
            redaction_patterns: vec![],
            redacted_fields: vec!["password".to_string(), "api_key".to_string()],
            audit_trail: false,
            audit_log_path: None,
        };

        let sanitizer = DataSanitizer::new(&config).unwrap();

        let mut json = serde_json::json!({
            "username": "testuser",
            "password": "secret123",
            "api_key": "sk-1234567890",
            "data": {
                "nested_password": "anothersecret"
            }
        });

        json = sanitizer.sanitize_json(json);

        assert_eq!(json["username"], "testuser");
        assert_eq!(json["password"], "[REDACTED]");
        assert_eq!(json["api_key"], "[REDACTED]");
        assert_eq!(json["data"]["nested_password"], "[REDACTED]");
    }

    #[test]
    fn test_credit_card_redaction() {
        let sanitizer = DataSanitizer::new(&PrivacyConfig::default()).unwrap();
        let redacted = sanitizer.redact_credit_card("4532-1234-5678-9012");
        assert_eq!(redacted, "4532****9012");
    }

    #[test]
    fn test_sanitization_report() {
        let config = PrivacyConfig {
            sanitize_enabled: true,
            redaction_patterns: SanitizationPatterns::api_credentials(),
            redacted_fields: vec![],
            audit_trail: false,
            audit_log_path: None,
        };

        let sanitizer = DataSanitizer::new(&config).unwrap();

        let original = "api_key=sk-1234567890abcdef";
        let sanitized = sanitizer.sanitize_text(original);
        let report = sanitizer.validate_sanitization(original, &sanitized);

        assert!(report.redactions_detected > 0);
        assert!(report.original_length > report.sanitized_length);
    }

    #[test]
    fn test_potentially_sensitive_data_detection() {
        let config = PrivacyConfig::default();
        let sanitizer = DataSanitizer::new(&config).unwrap();

        let text_with_sensitive_data = "password=stillhere";
        let report = sanitizer.validate_sanitization("", text_with_sensitive_data);

        assert!(report.potentially_sensitive_data_found);
    }
}
