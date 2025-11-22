use anyhow::Result;
use crate::registry::{CommandContext, CommandDescriptor, CommandExecutor};
use crate::symbols::{extract_symbols, Symbol, SymbolIndex, SymbolType};
use fennec_core::{command::{Capability, CommandPreview, CommandResult}, error::FennecError};
use fennec_security::SandboxLevel;
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;
use walkdir::WalkDir;
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindSymbolArgs {
    pub query: String,
    #[serde(default)]
    pub symbol_type: Option<String>,
    #[serde(default)]
    pub exact_match: bool,
    #[serde(default = "default_max_results")]
    pub max_results: usize,
}

fn default_max_results() -> usize {
    50
}

pub struct FindSymbolCommand {
    descriptor: CommandDescriptor,
}

impl FindSymbolCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor {
                name: "find-symbol".to_string(),
                description: "Find Rust symbols (functions, structs, traits, etc.) in the workspace".to_string(),
                version: "1.0.0".to_string(),
                author: Some("Fennec Contributors".to_string()),
                capabilities_required: vec![Capability::ReadFile],
                sandbox_level_required: SandboxLevel::ReadOnly,
                supports_preview: false,
                supports_dry_run: false,
            },
        }
    }

    fn parse_symbol_type(type_str: &str) -> Option<SymbolType> {
        match type_str.to_lowercase().as_str() {
            "function" | "fn" => Some(SymbolType::Function),
            "struct" => Some(SymbolType::Struct),
            "enum" => Some(SymbolType::Enum),
            "trait" => Some(SymbolType::Trait),
            "type" => Some(SymbolType::Type),
            "const" => Some(SymbolType::Const),
            "module" | "mod" => Some(SymbolType::Module),
            "impl" => Some(SymbolType::Impl),
            _ => None,
        }
    }

    async fn build_index(&self, workspace_path: &Path, context: &CommandContext) -> Result<SymbolIndex> {
        let mut index = SymbolIndex::new();
        let mut files_indexed = 0;

        for entry in WalkDir::new(workspace_path)
            .max_depth(10)
            .into_iter()
            .filter_entry(|e| {
                // Don't filter the root directory itself
                if e.path() == workspace_path {
                    return true;
                }
                // Filter out hidden directories, target, and node_modules
                !e.file_name()
                    .to_str()
                    .map(|s| s.starts_with('.') || s == "target" || s == "node_modules")
                    .unwrap_or(false)
            })
            .filter_map(|e| e.ok())
        {
            if files_indexed % 10 == 0 && context.cancellation_token.is_cancelled() {
                return Err(FennecError::Command(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Interrupted,
                    "Indexing cancelled"
                ))).into());
            }

            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            // Only index Rust files
            if let Some(ext) = path.extension() {
                if ext != "rs" {
                    continue;
                }
            } else {
                continue;
            }

            files_indexed += 1;

            // Read file content
            match fs::read_to_string(path).await {
                Ok(content) => {
                    match extract_symbols(path, &content) {
                        Ok(symbols) => {
                            index.add_symbols(symbols);
                        }
                        Err(_) => {
                            // Skip files that fail to parse
                            continue;
                        }
                    }
                }
                Err(_) => {
                    // Skip files that can't be read
                    continue;
                }
            }
        }

        Ok(index)
    }

    async fn perform_search(&self, args: &FindSymbolArgs, context: &CommandContext) -> Result<String> {
        let workspace_path_str = context.workspace_path.as_ref().ok_or_else(|| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No workspace path set"
            )))
        })?;
        let workspace_path = Path::new(workspace_path_str);

        // Build symbol index
        let index = self.build_index(workspace_path, context).await?;

        if index.is_empty() {
            return Ok("No symbols found in workspace".to_string());
        }

        // Search for symbols
        let mut results: Vec<&Symbol> = if args.exact_match {
            index.find_by_name(&args.query)
        } else {
            index.find_by_name_partial(&args.query)
        };

        // Filter by symbol type if specified
        if let Some(ref type_str) = args.symbol_type {
            if let Some(symbol_type) = Self::parse_symbol_type(type_str) {
                results.retain(|s| s.symbol_type == symbol_type);
            }
        }

        // Limit results
        results.truncate(args.max_results);

        if results.is_empty() {
            return Ok(format!("No symbols found matching '{}'", args.query));
        }

        // Format output
        let mut output = format!("Found {} symbols matching '{}':\n\n", results.len(), args.query);

        for symbol in &results {
            let relative_path = symbol.path.strip_prefix(workspace_path)
                .unwrap_or(&symbol.path);

            let type_str = match symbol.symbol_type {
                SymbolType::Function => "fn",
                SymbolType::Struct => "struct",
                SymbolType::Enum => "enum",
                SymbolType::Trait => "trait",
                SymbolType::Type => "type",
                SymbolType::Const => "const",
                SymbolType::Module => "mod",
                SymbolType::Impl => "impl",
            };

            let vis_str = match symbol.visibility {
                crate::symbols::Visibility::Public => "pub ",
                crate::symbols::Visibility::Crate => "pub(crate) ",
                crate::symbols::Visibility::Super => "pub(super) ",
                crate::symbols::Visibility::Private => "",
            };

            output.push_str(&format!(
                "  {} {}{} {} @ {}:{}\n",
                type_str,
                vis_str,
                symbol.name,
                "".to_string(), // Could add signature info here
                relative_path.display(),
                symbol.line
            ));

            if let Some(ref doc) = symbol.doc_comment {
                let doc_preview = doc.lines().next().unwrap_or("");
                if !doc_preview.is_empty() {
                    output.push_str(&format!("    // {}\n", doc_preview));
                }
            }
        }

        output.push_str(&format!("\nIndexed {} total symbols\n", index.len()));

        Ok(output)
    }
}

impl Default for FindSymbolCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl CommandExecutor for FindSymbolCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn preview(&self, args: &serde_json::Value, _context: &CommandContext) -> Result<CommandPreview> {
        let args: FindSymbolArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid find-symbol arguments: {}", e)
            )))
        })?;

        let description = if let Some(ref sym_type) = args.symbol_type {
            format!("Find {} symbols matching '{}'", sym_type, args.query)
        } else {
            format!("Find symbols matching '{}'", args.query)
        };

        Ok(CommandPreview {
            command_id: Uuid::new_v4(),
            description,
            actions: vec![],
            requires_approval: false,
        })
    }

    async fn execute(&self, args: &serde_json::Value, context: &CommandContext) -> Result<CommandResult> {
        let args: FindSymbolArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid find-symbol arguments: {}", e)
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
        let args: FindSymbolArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid find-symbol arguments: {}", e)
            )))
        })?;

        if args.query.trim().is_empty() {
            return Err(FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Query cannot be empty"
            ))).into());
        }

        if let Some(ref type_str) = args.symbol_type {
            if Self::parse_symbol_type(type_str).is_none() {
                return Err(FennecError::Command(Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Invalid symbol type: {}. Valid types: function, struct, enum, trait, type, const, module, impl", type_str)
                ))).into());
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio_util::sync::CancellationToken;

    #[tokio::test]
    async fn test_find_symbol_in_temp_workspace() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("lib.rs");

        let code = r#"
            pub fn hello_world() {
                println!("Hello!");
            }

            pub struct MyStruct {
                value: i32,
            }
        "#;

        std::fs::write(&test_file, code).unwrap();

        let command = FindSymbolCommand::new();
        let args = serde_json::json!({
            "query": "hello",
            "exact_match": false,
            "max_results": 10
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
        assert!(result.success, "Command should succeed");
        assert!(result.output.contains("hello_world"), "Output should contain 'hello_world'");
    }

    #[tokio::test]
    async fn test_find_symbol_by_type() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("lib.rs");

        let code = r#"
            pub fn my_function() {}
            pub struct MyStruct {}
        "#;

        std::fs::write(&test_file, code).unwrap();

        let command = FindSymbolCommand::new();
        let args = serde_json::json!({
            "query": "My",
            "symbol_type": "struct",
            "exact_match": false,
            "max_results": 10
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
        assert!(result.output.contains("MyStruct"));
        assert!(!result.output.contains("my_function"));
    }
}
