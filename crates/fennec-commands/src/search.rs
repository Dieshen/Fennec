use anyhow::Result;
use crate::registry::{CommandContext, CommandDescriptor, CommandExecutor};
use fennec_core::{command::{Capability, CommandPreview, CommandResult}, error::FennecError};
use fennec_security::SandboxLevel;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchArgs {
    pub query: String,
    #[serde(default)]
    pub pattern: Option<String>,
    #[serde(default)]
    pub case_insensitive: bool,
    #[serde(default)]
    pub regex: bool,
    #[serde(default = "default_max_results")]
    pub max_results: usize,
    #[serde(default)]
    pub context_lines: usize,
    #[serde(default)]
    pub filename_only: bool,
}

fn default_max_results() -> usize {
    100
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub file_path: PathBuf,
    pub line_number: usize,
    pub line_content: String,
    pub match_count: usize,
}

pub struct SearchCommand {
    descriptor: CommandDescriptor,
}

impl SearchCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor {
                name: "search".to_string(),
                description: "Search for text across project files with optional regex and filtering".to_string(),
                version: "1.0.0".to_string(),
                author: Some("Fennec Contributors".to_string()),
                capabilities_required: vec![Capability::ReadFile],
                sandbox_level_required: SandboxLevel::ReadOnly,
                supports_preview: true,
                supports_dry_run: false,
            },
        }
    }

    fn should_search_file(path: &Path, pattern: &Option<String>) -> bool {
        if let Some(pattern) = pattern {
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if pattern.contains('*') {
                    let pattern = pattern.replace("*.", ".");
                    return file_name.ends_with(&pattern);
                } else {
                    return file_name.contains(pattern);
                }
            }
            return false;
        }
        true
    }

    fn is_text_file(path: &Path) -> bool {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            matches!(
                ext.to_lowercase().as_str(),
                "rs" | "toml" | "md" | "txt" | "json" | "yaml" | "yml" | "sh" | "py" | "js"
                    | "ts" | "html" | "css" | "xml" | "c" | "cpp" | "h" | "hpp" | "go" | "java"
                    | "kt" | "swift" | "rb" | "php" | "sql" | "lock" | "gitignore" | "env"
            )
        } else {
            true
        }
    }

    fn search_in_file(
        path: &Path,
        query: &str,
        case_insensitive: bool,
        use_regex: bool,
        _context_lines: usize,
    ) -> Result<Vec<SearchResult>> {
        let content = std::fs::read_to_string(path)?;
        let lines: Vec<&str> = content.lines().collect();
        let mut results = Vec::new();

        let regex_pattern = if use_regex {
            let pattern_str = if case_insensitive {
                format!("(?i){}", query)
            } else {
                query.to_string()
            };
            Some(regex::Regex::new(&pattern_str).map_err(|e| {
                FennecError::Command(Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Invalid regex: {}", e)
                )))
            })?)
        } else {
            None
        };

        for (idx, line) in lines.iter().enumerate() {
            let line_matches = if let Some(ref regex) = regex_pattern {
                regex.is_match(line)
            } else if case_insensitive {
                line.to_lowercase().contains(&query.to_lowercase())
            } else {
                line.contains(query)
            };

            if line_matches {
                let match_count = if let Some(ref regex) = regex_pattern {
                    regex.find_iter(line).count()
                } else {
                    let search_line = if case_insensitive {
                        line.to_lowercase()
                    } else {
                        line.to_string()
                    };
                    let search_query = if case_insensitive {
                        query.to_lowercase()
                    } else {
                        query.to_string()
                    };
                    search_line.matches(&search_query).count()
                };

                results.push(SearchResult {
                    file_path: path.to_path_buf(),
                    line_number: idx + 1,
                    line_content: line.to_string(),
                    match_count,
                });
            }
        }

        Ok(results)
    }

    fn search_filenames(
        workspace_path: &Path,
        query: &str,
        case_insensitive: bool,
        max_results: usize,
    ) -> Result<Vec<PathBuf>> {
        let mut results = Vec::new();
        let search_query = if case_insensitive {
            query.to_lowercase()
        } else {
            query.to_string()
        };

        for entry in WalkDir::new(workspace_path)
            .max_depth(10)
            .into_iter()
            .filter_entry(|e| {
                !e.file_name().to_str().map(|s| s.starts_with('.') || s == "target" || s == "node_modules").unwrap_or(false)
            })
            .filter_map(|e| e.ok())
        {
            if let Some(filename) = entry.file_name().to_str() {
                let compare_name = if case_insensitive {
                    filename.to_lowercase()
                } else {
                    filename.to_string()
                };

                if compare_name.contains(&search_query) {
                    results.push(entry.path().to_path_buf());
                    if results.len() >= max_results {
                        break;
                    }
                }
            }
        }

        Ok(results)
    }

    async fn perform_search(&self, args: &SearchArgs, context: &CommandContext) -> Result<String> {
        let workspace_path_str = context.workspace_path.as_ref().ok_or_else(|| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No workspace path set"
            )))
        })?;
        let workspace_path = Path::new(workspace_path_str);

        if context.cancellation_token.is_cancelled() {
            return Err(FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "Search cancelled"
            ))).into());
        }

        if args.filename_only {
            let results = Self::search_filenames(workspace_path, &args.query, args.case_insensitive, args.max_results)?;

            if results.is_empty() {
                Ok("No files found matching query".to_string())
            } else {
                let mut output = format!("Found {} files matching '{}':\n\n", results.len(), args.query);
                for path in &results {
                    if let Ok(rel_path) = path.strip_prefix(workspace_path) {
                        output.push_str(&format!("  {}\n", rel_path.display()));
                    }
                }
                Ok(output)
            }
        } else {
            let mut all_results = Vec::new();
            let mut files_searched = 0;

            for entry in WalkDir::new(workspace_path)
                .max_depth(10)
                .into_iter()
                .filter_entry(|e| !e.file_name().to_str().map(|s| s.starts_with('.') || s == "target" || s == "node_modules").unwrap_or(false))
                .filter_map(|e| e.ok())
            {
                if files_searched % 50 == 0 && context.cancellation_token.is_cancelled() {
                    return Err(FennecError::Command(Box::new(std::io::Error::new(
                        std::io::ErrorKind::Interrupted,
                        "Search cancelled"
                    ))).into());
                }

                let path = entry.path();
                if !path.is_file() || !Self::should_search_file(path, &args.pattern) || !Self::is_text_file(path) {
                    continue;
                }

                files_searched += 1;

                if let Ok(results) = Self::search_in_file(path, &args.query, args.case_insensitive, args.regex, args.context_lines) {
                    all_results.extend(results);
                    if all_results.len() >= args.max_results {
                        break;
                    }
                }
            }

            if all_results.is_empty() {
                Ok(format!("No matches found for '{}' in {} files", args.query, files_searched))
            } else {
                let mut output = format!("Found {} matches for '{}' in {} files:\n\n", all_results.len(), args.query, files_searched);
                for result in &all_results {
                    if let Ok(rel_path) = result.file_path.strip_prefix(workspace_path) {
                        output.push_str(&format!("{}:{} ({} matches)\n  > {}\n\n", 
                            rel_path.display(), result.line_number, result.match_count, result.line_content.trim()));
                    }
                }
                Ok(output)
            }
        }
    }
}

impl Default for SearchCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl CommandExecutor for SearchCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn preview(&self, args: &serde_json::Value, _context: &CommandContext) -> Result<CommandPreview> {
        let args: SearchArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid search arguments: {}", e)
            )))
        })?;

        let description = if args.filename_only {
            format!("Search for files matching '{}'", args.query)
        } else {
            format!("Search for '{}' in {} files{}", args.query, args.pattern.as_deref().unwrap_or("all"),
                if args.case_insensitive { " (case-insensitive)" } else { "" })
        };

        Ok(CommandPreview {
            command_id: Uuid::new_v4(),
            description,
            actions: vec![],
            requires_approval: false,
        })
    }

    async fn execute(&self, args: &serde_json::Value, context: &CommandContext) -> Result<CommandResult> {
        let args: SearchArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid search arguments: {}", e)
            )))
        })?;

        match self.perform_search(&args, context).await {
            Ok(output) => Ok(CommandResult {
                command_id: Uuid::new_v4(),
                success: true,
                output,
                error: None,
            }),
            Err(e) => Ok(CommandResult {
                command_id: Uuid::new_v4(),
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
            }),
        }
    }

    fn validate_args(&self, args: &serde_json::Value) -> Result<()> {
        let args: SearchArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid search arguments: {}", e)
            )))
        })?;

        if args.query.trim().is_empty() {
            return Err(FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Search query cannot be empty"
            ))).into());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio_util::sync::CancellationToken;

    #[test]
    fn test_should_search_file() {
        let path = Path::new("src/main.rs");
        assert!(SearchCommand::should_search_file(path, &Some("*.rs".to_string())));
        assert!(!SearchCommand::should_search_file(path, &Some("*.toml".to_string())));
        assert!(SearchCommand::should_search_file(path, &None));
    }

    #[test]
    fn test_is_text_file() {
        assert!(SearchCommand::is_text_file(Path::new("file.rs")));
        assert!(SearchCommand::is_text_file(Path::new("Cargo.toml")));
        assert!(SearchCommand::is_text_file(Path::new("README.md")));
        assert!(!SearchCommand::is_text_file(Path::new("image.png")));
    }

    #[tokio::test]
    async fn test_search_in_files() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.rs");
        std::fs::write(&test_file, "fn main() {\n    println!(\"Hello\");\n}\n").unwrap();

        let results = SearchCommand::search_in_file(&test_file, "main", false, false, 0).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].line_number, 1);
        assert!(results[0].line_content.contains("main"));
    }

    #[tokio::test]
    async fn test_search_command() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        std::fs::write(&test_file, "Hello World\nTest Line\n").unwrap();

        let command = SearchCommand::new();
        let args = serde_json::json!({
            "query": "Hello",
            "case_insensitive": false,
            "regex": false,
            "max_results": 10,
            "context_lines": 0,
            "filename_only": false
        });

        let context = CommandContext {
            session_id: Uuid::new_v4(),
            user_id: None,
            workspace_path: Some(temp_dir.path().to_string_lossy().to_string()),
            sandbox_level: SandboxLevel::ReadOnly,
            dry_run: false,
            preview_only: false,
            cancellation_token: CancellationToken::new(),
            action_log: None,
        };

        let result = command.execute(&args, &context).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("Hello"));
    }
}
