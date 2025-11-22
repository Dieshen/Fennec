use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Represents a compiler error or warning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilerMessage {
    pub level: MessageLevel,
    pub message: String,
    pub code: Option<String>,
    pub spans: Vec<CodeSpan>,
    pub children: Vec<CompilerMessage>,
    pub rendered: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MessageLevel {
    Error,
    Warning,
    Note,
    Help,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSpan {
    pub file_name: PathBuf,
    pub line_start: usize,
    pub line_end: usize,
    pub column_start: usize,
    pub column_end: usize,
    pub text: Vec<SpanText>,
    pub label: Option<String>,
    pub suggested_replacement: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanText {
    pub text: String,
    pub highlight_start: usize,
    pub highlight_end: usize,
}

/// Represents a suggested fix from the compiler
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedFix {
    pub message: String,
    pub file_path: PathBuf,
    pub line: usize,
    pub column: usize,
    pub original_code: String,
    pub suggested_code: String,
    pub confidence: FixConfidence,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FixConfidence {
    High,    // Compiler explicitly suggests this fix
    Medium,  // Inferred from error message
    Low,     // Heuristic-based suggestion
}

impl SuggestedFix {
    pub fn new(
        message: String,
        file_path: PathBuf,
        line: usize,
        column: usize,
        original_code: String,
        suggested_code: String,
        confidence: FixConfidence,
    ) -> Self {
        Self {
            message,
            file_path,
            line,
            column,
            original_code,
            suggested_code,
            confidence,
        }
    }

    /// Format the fix for display
    pub fn format(&self) -> String {
        format!(
            "[{}] {} @ {}:{}:{}\n  - {}\n  + {}",
            match self.confidence {
                FixConfidence::High => "HIGH",
                FixConfidence::Medium => "MED",
                FixConfidence::Low => "LOW",
            },
            self.message,
            self.file_path.display(),
            self.line,
            self.column,
            self.original_code.trim(),
            self.suggested_code.trim()
        )
    }
}

/// Parse cargo JSON output to extract compiler messages
pub fn parse_cargo_json(json_line: &str) -> Option<CompilerMessage> {
    #[derive(Deserialize)]
    struct CargoMessage {
        message: Option<CompilerMessageJson>,
        reason: String,
    }

    #[derive(Deserialize)]
    struct CompilerMessageJson {
        level: String,
        message: String,
        code: Option<CodeJson>,
        spans: Vec<SpanJson>,
        children: Vec<CompilerMessageJson>,
        rendered: Option<String>,
    }

    #[derive(Deserialize)]
    struct CodeJson {
        code: String,
    }

    #[derive(Deserialize)]
    struct SpanJson {
        file_name: String,
        line_start: usize,
        line_end: usize,
        column_start: usize,
        column_end: usize,
        text: Vec<SpanTextJson>,
        label: Option<String>,
        suggested_replacement: Option<String>,
    }

    #[derive(Deserialize)]
    struct SpanTextJson {
        text: String,
        highlight_start: usize,
        highlight_end: usize,
    }

    let cargo_msg: CargoMessage = serde_json::from_str(json_line).ok()?;

    if cargo_msg.reason != "compiler-message" {
        return None;
    }

    let msg = cargo_msg.message?;

    let level = match msg.level.as_str() {
        "error" => MessageLevel::Error,
        "warning" => MessageLevel::Warning,
        "note" => MessageLevel::Note,
        "help" => MessageLevel::Help,
        _ => return None,
    };

    let spans = msg
        .spans
        .into_iter()
        .map(|s| CodeSpan {
            file_name: PathBuf::from(s.file_name),
            line_start: s.line_start,
            line_end: s.line_end,
            column_start: s.column_start,
            column_end: s.column_end,
            text: s
                .text
                .into_iter()
                .map(|t| SpanText {
                    text: t.text,
                    highlight_start: t.highlight_start,
                    highlight_end: t.highlight_end,
                })
                .collect(),
            label: s.label,
            suggested_replacement: s.suggested_replacement,
        })
        .collect();

    fn convert_children(children: Vec<CompilerMessageJson>) -> Vec<CompilerMessage> {
        children
            .into_iter()
            .filter_map(|child| {
                let level = match child.level.as_str() {
                    "error" => MessageLevel::Error,
                    "warning" => MessageLevel::Warning,
                    "note" => MessageLevel::Note,
                    "help" => MessageLevel::Help,
                    _ => return None,
                };

                Some(CompilerMessage {
                    level,
                    message: child.message,
                    code: child.code.map(|c| c.code),
                    spans: child
                        .spans
                        .into_iter()
                        .map(|s| CodeSpan {
                            file_name: PathBuf::from(s.file_name),
                            line_start: s.line_start,
                            line_end: s.line_end,
                            column_start: s.column_start,
                            column_end: s.column_end,
                            text: s
                                .text
                                .into_iter()
                                .map(|t| SpanText {
                                    text: t.text,
                                    highlight_start: t.highlight_start,
                                    highlight_end: t.highlight_end,
                                })
                                .collect(),
                            label: s.label,
                            suggested_replacement: s.suggested_replacement,
                        })
                        .collect(),
                    children: convert_children(child.children),
                    rendered: child.rendered,
                })
            })
            .collect()
    }

    Some(CompilerMessage {
        level,
        message: msg.message,
        code: msg.code.map(|c| c.code),
        spans,
        children: convert_children(msg.children),
        rendered: msg.rendered,
    })
}

/// Extract suggested fixes from a compiler message
pub fn extract_fixes(message: &CompilerMessage) -> Vec<SuggestedFix> {
    let mut fixes = Vec::new();

    // Extract fixes from spans with suggested replacements
    for span in &message.spans {
        if let Some(ref replacement) = span.suggested_replacement {
            if let Some(original) = span.text.first() {
                let fix = SuggestedFix::new(
                    message.message.clone(),
                    span.file_name.clone(),
                    span.line_start,
                    span.column_start,
                    original.text.clone(),
                    replacement.clone(),
                    FixConfidence::High,
                );
                fixes.push(fix);
            }
        }
    }

    // Recursively extract fixes from child messages
    for child in &message.children {
        fixes.extend(extract_fixes(child));
    }

    fixes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cargo_json_error() {
        let json = r#"{
            "reason": "compiler-message",
            "message": {
                "level": "error",
                "message": "cannot find value `x` in this scope",
                "code": {
                    "code": "E0425"
                },
                "spans": [
                    {
                        "file_name": "src/main.rs",
                        "line_start": 2,
                        "line_end": 2,
                        "column_start": 14,
                        "column_end": 15,
                        "text": [
                            {
                                "text": "    println!(\"{}\", x);",
                                "highlight_start": 13,
                                "highlight_end": 14
                            }
                        ],
                        "label": "not found in this scope",
                        "suggested_replacement": null
                    }
                ],
                "children": [],
                "rendered": null
            }
        }"#;

        let message = parse_cargo_json(json).unwrap();
        assert_eq!(message.level, MessageLevel::Error);
        assert_eq!(message.message, "cannot find value `x` in this scope");
        assert_eq!(message.code, Some("E0425".to_string()));
        assert_eq!(message.spans.len(), 1);
        assert_eq!(message.spans[0].line_start, 2);
    }

    #[test]
    fn test_parse_cargo_json_with_suggestion() {
        let json = r#"{
            "reason": "compiler-message",
            "message": {
                "level": "warning",
                "message": "unused variable: `y`",
                "code": null,
                "spans": [
                    {
                        "file_name": "src/main.rs",
                        "line_start": 3,
                        "line_end": 3,
                        "column_start": 9,
                        "column_end": 10,
                        "text": [
                            {
                                "text": "    let y = 5;",
                                "highlight_start": 8,
                                "highlight_end": 9
                            }
                        ],
                        "label": null,
                        "suggested_replacement": "_y"
                    }
                ],
                "children": [],
                "rendered": null
            }
        }"#;

        let message = parse_cargo_json(json).unwrap();
        assert_eq!(message.level, MessageLevel::Warning);
        assert_eq!(message.spans[0].suggested_replacement, Some("_y".to_string()));
    }

    #[test]
    fn test_extract_fixes() {
        let message = CompilerMessage {
            level: MessageLevel::Warning,
            message: "unused variable: `y`".to_string(),
            code: None,
            spans: vec![CodeSpan {
                file_name: PathBuf::from("src/main.rs"),
                line_start: 3,
                line_end: 3,
                column_start: 9,
                column_end: 10,
                text: vec![SpanText {
                    text: "    let y = 5;".to_string(),
                    highlight_start: 8,
                    highlight_end: 9,
                }],
                label: None,
                suggested_replacement: Some("_y".to_string()),
            }],
            children: vec![],
            rendered: None,
        };

        let fixes = extract_fixes(&message);
        assert_eq!(fixes.len(), 1);
        assert_eq!(fixes[0].suggested_code, "_y");
        assert_eq!(fixes[0].confidence, FixConfidence::High);
    }

    #[test]
    fn test_suggested_fix_format() {
        let fix = SuggestedFix::new(
            "unused variable: `y`".to_string(),
            PathBuf::from("src/main.rs"),
            3,
            9,
            "y".to_string(),
            "_y".to_string(),
            FixConfidence::High,
        );

        let formatted = fix.format();
        assert!(formatted.contains("[HIGH]"));
        assert!(formatted.contains("src/main.rs:3:9"));
        assert!(formatted.contains("- y"));
        assert!(formatted.contains("+ _y"));
    }
}
