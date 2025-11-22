use crate::project_index::ProjectIndex;
use crate::registry::{CommandContext, CommandDescriptor, CommandExecutor};
use anyhow::Result;
use fennec_core::command::{Capability, CommandPreview, CommandResult};
use fennec_core::error::FennecError;
use fennec_security::SandboxLevel;
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexArgs {
    /// Type of analysis to perform
    #[serde(default = "default_analysis_type")]
    pub analysis_type: String,

    /// Specific file path for impact analysis
    #[serde(default)]
    pub file_path: Option<String>,

    /// Show detailed output
    #[serde(default)]
    pub detailed: bool,
}

fn default_analysis_type() -> String {
    "stats".to_string()
}

pub struct IndexCommand {
    descriptor: CommandDescriptor,
}

impl IndexCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor {
                name: "index".to_string(),
                description: "Analyze project structure, dependencies, and symbols".to_string(),
                version: "1.0.0".to_string(),
                author: Some("Fennec Contributors".to_string()),
                capabilities_required: vec![Capability::ReadFile],
                sandbox_level_required: SandboxLevel::ReadOnly,
                supports_preview: false,
                supports_dry_run: false,
            },
        }
    }

    async fn analyze_project(&self, args: &IndexArgs, context: &CommandContext) -> Result<String> {
        let workspace_path = context.workspace_path.as_ref().ok_or_else(|| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No workspace path set",
            )))
        })?;

        let workspace_path = Path::new(workspace_path);

        // Build project index
        let index = ProjectIndex::build(workspace_path).await.map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                e.kind(),
                format!("Failed to build project index: {}", e),
            )))
        })?;

        match args.analysis_type.as_str() {
            "stats" => self.format_statistics(&index, args.detailed),
            "deps" => self.format_dependencies(&index, args.detailed),
            "symbols" => self.format_symbols(&index, args.detailed),
            "impact" => {
                if let Some(ref file_path) = args.file_path {
                    let path = Path::new(file_path);
                    self.format_impact_analysis(&index, path)
                } else {
                    Ok("Impact analysis requires a file path (use --file-path <path>)".to_string())
                }
            }
            "modules" => self.format_module_hierarchy(&index, args.detailed),
            _ => Ok(format!(
                "Unknown analysis type: {}. Available types: stats, deps, symbols, impact, modules",
                args.analysis_type
            )),
        }
    }

    fn format_statistics(&self, index: &ProjectIndex, detailed: bool) -> Result<String> {
        let stats = index.get_statistics();
        let mut output = String::new();

        output.push_str("📊 Project Statistics\n\n");
        output.push_str(&format!("📦 Packages: {}\n", stats.total_packages));
        output.push_str(&format!("🔤 Symbols: {}\n", stats.total_symbols));
        output.push_str(&format!("📁 Modules: {}\n", stats.total_modules));
        output.push_str(&format!(
            "🔄 Circular Dependencies: {}\n\n",
            if stats.has_circular_deps {
                "⚠️  Yes"
            } else {
                "✅ No"
            }
        ));

        if detailed {
            output.push_str("Package List:\n");
            for (name, pkg) in &index.dependency_graph.packages {
                output.push_str(&format!(
                    "  - {} v{} ({} deps)\n",
                    name,
                    pkg.version,
                    pkg.dependencies.len()
                ));
            }
        }

        Ok(output)
    }

    fn format_dependencies(&self, index: &ProjectIndex, detailed: bool) -> Result<String> {
        let mut output = String::new();

        output.push_str("📦 Dependency Graph\n\n");

        for (name, pkg) in &index.dependency_graph.packages {
            output.push_str(&format!("Package: {} v{}\n", name, pkg.version));

            if !pkg.dependencies.is_empty() {
                output.push_str("  Dependencies:\n");
                for dep in &pkg.dependencies {
                    let version = dep
                        .version
                        .as_ref()
                        .map(|v| format!(" v{}", v))
                        .unwrap_or_default();
                    output.push_str(&format!("    - {}{}\n", dep.name, version));
                }
            }

            if detailed {
                let dependents = index.dependency_graph.get_dependents(name);
                if !dependents.is_empty() {
                    output.push_str("  Used by:\n");
                    for dependent in dependents {
                        output.push_str(&format!("    - {}\n", dependent.name));
                    }
                }
            }

            output.push('\n');
        }

        Ok(output)
    }

    fn format_symbols(&self, index: &ProjectIndex, detailed: bool) -> Result<String> {
        let mut output = String::new();

        output.push_str("🔤 Symbol Index\n\n");
        output.push_str(&format!("Total symbols: {}\n\n", index.symbol_index.len()));

        if detailed {
            use crate::symbols::SymbolType;

            for symbol_type in [
                SymbolType::Function,
                SymbolType::Struct,
                SymbolType::Enum,
                SymbolType::Trait,
            ] {
                let symbols = index.symbol_index.find_by_type(&symbol_type);
                if !symbols.is_empty() {
                    output.push_str(&format!("{:?}s ({}):\n", symbol_type, symbols.len()));
                    for symbol in symbols.iter().take(10) {
                        output.push_str(&format!("  - {}\n", symbol.name));
                    }
                    if symbols.len() > 10 {
                        output.push_str(&format!("  ... and {} more\n", symbols.len() - 10));
                    }
                    output.push('\n');
                }
            }
        }

        Ok(output)
    }

    fn format_impact_analysis(&self, index: &ProjectIndex, file_path: &Path) -> Result<String> {
        let analysis = index.analyze_impact(file_path);
        let mut output = String::new();

        output.push_str("🎯 Impact Analysis\n\n");
        output.push_str(&format!("File: {}\n\n", analysis.file_path.display()));

        output.push_str(&format!(
            "Affected Symbols ({}):  \n",
            analysis.affected_symbols.len()
        ));
        for symbol in &analysis.affected_symbols {
            output.push_str(&format!("  - {}\n", symbol));
        }
        output.push('\n');

        output.push_str(&format!(
            "Affected Packages ({}):\n",
            analysis.affected_packages.len()
        ));
        for package in &analysis.affected_packages {
            output.push_str(&format!("  - {}\n", package));
        }
        output.push('\n');

        if !analysis.estimated_test_files.is_empty() {
            output.push_str("Potentially Affected Tests:\n");
            for test_file in &analysis.estimated_test_files {
                output.push_str(&format!("  - {}\n", test_file.display()));
            }
        } else {
            output.push_str("No test files identified (analysis limited)\n");
        }

        Ok(output)
    }

    fn format_module_hierarchy(&self, index: &ProjectIndex, detailed: bool) -> Result<String> {
        let mut output = String::new();

        output.push_str("📁 Module Hierarchy\n\n");
        self.format_module_node(&index.module_hierarchy.root, 0, detailed, &mut output);

        Ok(output)
    }

    fn format_module_node(
        &self,
        node: &crate::project_index::ModuleNode,
        depth: usize,
        detailed: bool,
        output: &mut String,
    ) {
        let indent = "  ".repeat(depth);

        output.push_str(&format!("{}📁 {}", indent, node.name));
        if detailed && !node.symbols.is_empty() {
            output.push_str(&format!(" ({} symbols)", node.symbols.len()));
        }
        output.push('\n');

        if detailed && !node.symbols.is_empty() && depth < 3 {
            for symbol in node.symbols.iter().take(5) {
                output.push_str(&format!("{}  - {}\n", indent, symbol));
            }
            if node.symbols.len() > 5 {
                output.push_str(&format!(
                    "{}  ... and {} more\n",
                    indent,
                    node.symbols.len() - 5
                ));
            }
        }

        for child in &node.children {
            self.format_module_node(child, depth + 1, detailed, output);
        }
    }
}

impl Default for IndexCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl CommandExecutor for IndexCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn preview(
        &self,
        args: &serde_json::Value,
        _context: &CommandContext,
    ) -> Result<CommandPreview> {
        let args: IndexArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid index arguments: {}", e),
            )))
        })?;

        Ok(CommandPreview {
            command_id: Uuid::new_v4(),
            description: format!("Analyze project: {}", args.analysis_type),
            actions: vec![],
            requires_approval: false,
        })
    }

    async fn execute(
        &self,
        args: &serde_json::Value,
        context: &CommandContext,
    ) -> Result<CommandResult> {
        let args: IndexArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid index arguments: {}", e),
            )))
        })?;

        match self.analyze_project(&args, context).await {
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
        let args: IndexArgs = serde_json::from_value(args.clone()).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid index arguments: {}", e),
            )))
        })?;

        let valid_types = ["stats", "deps", "symbols", "impact", "modules"];
        if !valid_types.contains(&args.analysis_type.as_str()) {
            return Err(FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "Invalid analysis type: '{}'. Must be one of: {}",
                    args.analysis_type,
                    valid_types.join(", ")
                ),
            )))
            .into());
        }

        if args.analysis_type == "impact" && args.file_path.is_none() {
            return Err(FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Impact analysis requires file_path argument",
            )))
            .into());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dependency_graph::DependencyGraph;
    use crate::project_index::{ModuleHierarchy, ModuleNode, ProjectIndex};
    use crate::symbols::SymbolIndex;
    use tempfile::TempDir;
    use tokio_util::sync::CancellationToken;

    #[test]
    fn test_default_analysis_type() {
        assert_eq!(default_analysis_type(), "stats");
    }

    #[test]
    fn test_index_args_default() {
        let json = serde_json::json!({});
        let args: IndexArgs = serde_json::from_value(json).unwrap();
        assert_eq!(args.analysis_type, "stats");
        assert!(args.file_path.is_none());
        assert!(!args.detailed);
    }

    #[test]
    fn test_index_args_serialization() {
        let args = IndexArgs {
            analysis_type: "deps".to_string(),
            file_path: Some("src/main.rs".to_string()),
            detailed: true,
        };

        let json = serde_json::to_value(&args).unwrap();
        let deserialized: IndexArgs = serde_json::from_value(json).unwrap();

        assert_eq!(deserialized.analysis_type, "deps");
        assert_eq!(deserialized.file_path, Some("src/main.rs".to_string()));
        assert!(deserialized.detailed);
    }

    #[test]
    fn test_index_args_all_analysis_types() {
        for analysis_type in &["stats", "deps", "symbols", "impact", "modules"] {
            let json = serde_json::json!({
                "analysis_type": analysis_type
            });
            let args: IndexArgs = serde_json::from_value(json).unwrap();
            assert_eq!(args.analysis_type, *analysis_type);
        }
    }

    #[test]
    fn test_index_command_new() {
        let command = IndexCommand::new();
        assert_eq!(command.descriptor.name, "index");
        assert_eq!(command.descriptor.version, "1.0.0");
        assert!(!command.descriptor.supports_preview);
        assert!(!command.descriptor.supports_dry_run);
    }

    #[test]
    fn test_index_command_default() {
        let command = IndexCommand::default();
        assert_eq!(command.descriptor.name, "index");
    }

    #[test]
    fn test_index_command_descriptor() {
        let command = IndexCommand::new();
        let descriptor = command.descriptor();
        assert_eq!(descriptor.name, "index");
        assert_eq!(descriptor.capabilities_required, vec![Capability::ReadFile]);
        assert_eq!(descriptor.sandbox_level_required, SandboxLevel::ReadOnly);
    }

    #[tokio::test]
    async fn test_index_no_workspace() {
        let command = IndexCommand::new();
        let args = serde_json::json!({});

        let context = CommandContext {
            session_id: Uuid::new_v4(),
            user_id: None,
            workspace_path: None,
            sandbox_level: SandboxLevel::ReadOnly,
            dry_run: false,
            preview_only: false,
            cancellation_token: CancellationToken::new(),
            action_log: None,
        };

        let result = command.execute(&args, &context).await.unwrap();
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_validate_args() {
        let command = IndexCommand::new();

        // Valid args
        let valid_args = serde_json::json!({
            "analysis_type": "stats"
        });
        assert!(command.validate_args(&valid_args).is_ok());

        // Invalid analysis type
        let invalid_type = serde_json::json!({
            "analysis_type": "invalid"
        });
        assert!(command.validate_args(&invalid_type).is_err());

        // Impact without file_path
        let impact_no_file = serde_json::json!({
            "analysis_type": "impact"
        });
        assert!(command.validate_args(&impact_no_file).is_err());
    }

    #[test]
    fn test_validate_args_all_valid_types() {
        let command = IndexCommand::new();

        for analysis_type in &["stats", "deps", "symbols", "impact", "modules"] {
            let args = if *analysis_type == "impact" {
                serde_json::json!({
                    "analysis_type": analysis_type,
                    "file_path": "src/lib.rs"
                })
            } else {
                serde_json::json!({
                    "analysis_type": analysis_type
                })
            };

            assert!(command.validate_args(&args).is_ok());
        }
    }

    #[test]
    fn test_validate_args_invalid_json() {
        let command = IndexCommand::new();
        let invalid_json = serde_json::json!({
            "analysis_type": 123  // Should be string
        });

        let result = command.validate_args(&invalid_json);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_preview() {
        let command = IndexCommand::new();
        let args = serde_json::json!({
            "analysis_type": "stats"
        });

        let context = CommandContext {
            session_id: Uuid::new_v4(),
            user_id: None,
            workspace_path: Some("/test".to_string()),
            sandbox_level: SandboxLevel::ReadOnly,
            dry_run: false,
            preview_only: false,
            cancellation_token: CancellationToken::new(),
            action_log: None,
        };

        let preview = command.preview(&args, &context).await.unwrap();
        assert!(preview.description.contains("stats"));
        assert!(!preview.requires_approval);
    }

    #[tokio::test]
    async fn test_preview_with_invalid_args() {
        let command = IndexCommand::new();
        let args = serde_json::json!({
            "analysis_type": 123
        });

        let context = CommandContext {
            session_id: Uuid::new_v4(),
            user_id: None,
            workspace_path: Some("/test".to_string()),
            sandbox_level: SandboxLevel::ReadOnly,
            dry_run: false,
            preview_only: false,
            cancellation_token: CancellationToken::new(),
            action_log: None,
        };

        let result = command.preview(&args, &context).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_format_statistics_basic() {
        let command = IndexCommand::new();
        let temp_dir = TempDir::new().unwrap();

        let index = ProjectIndex {
            dependency_graph: DependencyGraph::new(),
            symbol_index: SymbolIndex::new(),
            module_hierarchy: ModuleHierarchy {
                root: ModuleNode {
                    name: "root".to_string(),
                    path: temp_dir.path().to_path_buf(),
                    children: vec![],
                    symbols: vec![],
                },
            },
            workspace_path: temp_dir.path().to_path_buf(),
            indexed_at: chrono::Utc::now(),
        };

        let output = command.format_statistics(&index, false).unwrap();
        assert!(output.contains("Project Statistics"));
        assert!(output.contains("Packages:"));
        assert!(output.contains("Symbols:"));
        assert!(output.contains("Modules:"));
    }

    #[test]
    fn test_format_statistics_detailed() {
        let command = IndexCommand::new();
        let temp_dir = TempDir::new().unwrap();

        let index = ProjectIndex {
            dependency_graph: DependencyGraph::new(),
            symbol_index: SymbolIndex::new(),
            module_hierarchy: ModuleHierarchy {
                root: ModuleNode {
                    name: "root".to_string(),
                    path: temp_dir.path().to_path_buf(),
                    children: vec![],
                    symbols: vec![],
                },
            },
            workspace_path: temp_dir.path().to_path_buf(),
            indexed_at: chrono::Utc::now(),
        };

        let output = command.format_statistics(&index, true).unwrap();
        assert!(output.contains("Project Statistics"));
        assert!(output.contains("Package List:"));
    }

    #[test]
    fn test_format_dependencies_basic() {
        let command = IndexCommand::new();
        let temp_dir = TempDir::new().unwrap();

        let index = ProjectIndex {
            dependency_graph: DependencyGraph::new(),
            symbol_index: SymbolIndex::new(),
            module_hierarchy: ModuleHierarchy {
                root: ModuleNode {
                    name: "root".to_string(),
                    path: temp_dir.path().to_path_buf(),
                    children: vec![],
                    symbols: vec![],
                },
            },
            workspace_path: temp_dir.path().to_path_buf(),
            indexed_at: chrono::Utc::now(),
        };

        let output = command.format_dependencies(&index, false).unwrap();
        assert!(output.contains("Dependency Graph"));
    }

    #[test]
    fn test_format_dependencies_detailed() {
        let command = IndexCommand::new();
        let temp_dir = TempDir::new().unwrap();

        let index = ProjectIndex {
            dependency_graph: DependencyGraph::new(),
            symbol_index: SymbolIndex::new(),
            module_hierarchy: ModuleHierarchy {
                root: ModuleNode {
                    name: "root".to_string(),
                    path: temp_dir.path().to_path_buf(),
                    children: vec![],
                    symbols: vec![],
                },
            },
            workspace_path: temp_dir.path().to_path_buf(),
            indexed_at: chrono::Utc::now(),
        };

        let output = command.format_dependencies(&index, true).unwrap();
        assert!(output.contains("Dependency Graph"));
    }

    #[test]
    fn test_format_symbols_basic() {
        let command = IndexCommand::new();
        let temp_dir = TempDir::new().unwrap();

        let index = ProjectIndex {
            dependency_graph: DependencyGraph::new(),
            symbol_index: SymbolIndex::new(),
            module_hierarchy: ModuleHierarchy {
                root: ModuleNode {
                    name: "root".to_string(),
                    path: temp_dir.path().to_path_buf(),
                    children: vec![],
                    symbols: vec![],
                },
            },
            workspace_path: temp_dir.path().to_path_buf(),
            indexed_at: chrono::Utc::now(),
        };

        let output = command.format_symbols(&index, false).unwrap();
        assert!(output.contains("Symbol Index"));
        assert!(output.contains("Total symbols:"));
    }

    #[test]
    fn test_format_symbols_detailed() {
        let command = IndexCommand::new();
        let temp_dir = TempDir::new().unwrap();

        let index = ProjectIndex {
            dependency_graph: DependencyGraph::new(),
            symbol_index: SymbolIndex::new(),
            module_hierarchy: ModuleHierarchy {
                root: ModuleNode {
                    name: "root".to_string(),
                    path: temp_dir.path().to_path_buf(),
                    children: vec![],
                    symbols: vec![],
                },
            },
            workspace_path: temp_dir.path().to_path_buf(),
            indexed_at: chrono::Utc::now(),
        };

        let output = command.format_symbols(&index, true).unwrap();
        assert!(output.contains("Symbol Index"));
    }

    #[test]
    fn test_format_impact_analysis() {
        let command = IndexCommand::new();
        let temp_dir = TempDir::new().unwrap();

        let index = ProjectIndex {
            dependency_graph: DependencyGraph::new(),
            symbol_index: SymbolIndex::new(),
            module_hierarchy: ModuleHierarchy {
                root: ModuleNode {
                    name: "root".to_string(),
                    path: temp_dir.path().to_path_buf(),
                    children: vec![],
                    symbols: vec![],
                },
            },
            workspace_path: temp_dir.path().to_path_buf(),
            indexed_at: chrono::Utc::now(),
        };

        let file_path = std::path::Path::new("src/lib.rs");
        let output = command.format_impact_analysis(&index, file_path).unwrap();
        assert!(output.contains("Impact Analysis"));
        assert!(output.contains("File:"));
        assert!(output.contains("Affected Symbols"));
        assert!(output.contains("Affected Packages"));
    }

    #[test]
    fn test_format_module_hierarchy_basic() {
        let command = IndexCommand::new();
        let temp_dir = TempDir::new().unwrap();

        let index = ProjectIndex {
            dependency_graph: DependencyGraph::new(),
            symbol_index: SymbolIndex::new(),
            module_hierarchy: ModuleHierarchy {
                root: ModuleNode {
                    name: "root".to_string(),
                    path: temp_dir.path().to_path_buf(),
                    children: vec![],
                    symbols: vec!["test_symbol".to_string()],
                },
            },
            workspace_path: temp_dir.path().to_path_buf(),
            indexed_at: chrono::Utc::now(),
        };

        let output = command.format_module_hierarchy(&index, false).unwrap();
        assert!(output.contains("Module Hierarchy"));
        assert!(output.contains("root"));
    }

    #[test]
    fn test_format_module_hierarchy_detailed() {
        let command = IndexCommand::new();
        let temp_dir = TempDir::new().unwrap();

        let index = ProjectIndex {
            dependency_graph: DependencyGraph::new(),
            symbol_index: SymbolIndex::new(),
            module_hierarchy: ModuleHierarchy {
                root: ModuleNode {
                    name: "root".to_string(),
                    path: temp_dir.path().to_path_buf(),
                    children: vec![],
                    symbols: vec!["symbol1".to_string(), "symbol2".to_string()],
                },
            },
            workspace_path: temp_dir.path().to_path_buf(),
            indexed_at: chrono::Utc::now(),
        };

        let output = command.format_module_hierarchy(&index, true).unwrap();
        assert!(output.contains("Module Hierarchy"));
        assert!(output.contains("symbols"));
    }

    #[test]
    fn test_format_module_node_basic() {
        let command = IndexCommand::new();
        let temp_dir = TempDir::new().unwrap();

        let node = ModuleNode {
            name: "test_module".to_string(),
            path: temp_dir.path().to_path_buf(),
            children: vec![],
            symbols: vec![],
        };

        let mut output = String::new();
        command.format_module_node(&node, 0, false, &mut output);
        assert!(output.contains("test_module"));
    }

    #[test]
    fn test_format_module_node_with_children() {
        let command = IndexCommand::new();
        let temp_dir = TempDir::new().unwrap();

        let child = ModuleNode {
            name: "child".to_string(),
            path: temp_dir.path().join("child"),
            children: vec![],
            symbols: vec![],
        };

        let parent = ModuleNode {
            name: "parent".to_string(),
            path: temp_dir.path().to_path_buf(),
            children: vec![child],
            symbols: vec!["parent_symbol".to_string()],
        };

        let mut output = String::new();
        command.format_module_node(&parent, 0, true, &mut output);
        assert!(output.contains("parent"));
        assert!(output.contains("child"));
    }

    #[test]
    fn test_format_module_node_depth() {
        let command = IndexCommand::new();
        let temp_dir = TempDir::new().unwrap();

        let node = ModuleNode {
            name: "test".to_string(),
            path: temp_dir.path().to_path_buf(),
            children: vec![],
            symbols: vec!["symbol".to_string()],
        };

        let mut output1 = String::new();
        command.format_module_node(&node, 0, true, &mut output1);

        let mut output2 = String::new();
        command.format_module_node(&node, 2, true, &mut output2);

        // Deeper nodes should have more indentation
        assert!(output2.len() > output1.len());
    }

    #[test]
    fn test_index_args_clone() {
        let args = IndexArgs {
            analysis_type: "stats".to_string(),
            file_path: Some("src/lib.rs".to_string()),
            detailed: true,
        };

        let cloned = args.clone();
        assert_eq!(cloned.analysis_type, args.analysis_type);
        assert_eq!(cloned.file_path, args.file_path);
        assert_eq!(cloned.detailed, args.detailed);
    }

    #[test]
    fn test_index_args_debug() {
        let args = IndexArgs {
            analysis_type: "deps".to_string(),
            file_path: None,
            detailed: false,
        };

        let debug = format!("{:?}", args);
        assert!(debug.contains("IndexArgs"));
        assert!(debug.contains("deps"));
    }
}
