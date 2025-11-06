use crate::dependency_graph::{build_dependency_graph, DependencyGraph};
use crate::symbols::{extract_symbols, SymbolIndex};
use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use walkdir::WalkDir;

/// Comprehensive project index combining dependencies and symbols
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectIndex {
    pub dependency_graph: DependencyGraph,
    pub symbol_index: SymbolIndex,
    pub module_hierarchy: ModuleHierarchy,
    pub workspace_path: PathBuf,
    pub indexed_at: chrono::DateTime<chrono::Utc>,
}

/// Module hierarchy representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleHierarchy {
    pub root: ModuleNode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleNode {
    pub name: String,
    pub path: PathBuf,
    pub children: Vec<ModuleNode>,
    pub symbols: Vec<String>, // Symbol names defined in this module
}

impl ProjectIndex {
    /// Build a complete project index
    pub async fn build(workspace_path: &Path) -> Result<Self, std::io::Error> {
        let dependency_graph = build_dependency_graph(workspace_path).await?;
        let symbol_index = Self::build_symbol_index(workspace_path).await?;
        let module_hierarchy = Self::build_module_hierarchy(workspace_path).await?;

        Ok(Self {
            dependency_graph,
            symbol_index,
            module_hierarchy,
            workspace_path: workspace_path.to_path_buf(),
            indexed_at: chrono::Utc::now(),
        })
    }

    async fn build_symbol_index(workspace_path: &Path) -> Result<SymbolIndex, std::io::Error> {
        let mut index = SymbolIndex::new();

        for entry in WalkDir::new(workspace_path)
            .max_depth(10)
            .into_iter()
            .filter_entry(|e| {
                if e.path() == workspace_path {
                    return true;
                }
                !e.file_name()
                    .to_str()
                    .map(|s| s.starts_with('.') || s == "target" || s == "node_modules")
                    .unwrap_or(false)
            })
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            if let Some(ext) = path.extension() {
                if ext != "rs" {
                    continue;
                }
            } else {
                continue;
            }

            match fs::read_to_string(path).await {
                Ok(content) => {
                    if let Ok(symbols) = extract_symbols(path, &content) {
                        index.add_symbols(symbols);
                    }
                }
                Err(_) => continue,
            }
        }

        Ok(index)
    }

    async fn build_module_hierarchy(workspace_path: &Path) -> Result<ModuleHierarchy, std::io::Error> {
        let root = Self::scan_directory(workspace_path.to_path_buf(), "root".to_string()).await?;
        Ok(ModuleHierarchy { root })
    }

    fn scan_directory(
        path: PathBuf,
        name: String,
    ) -> BoxFuture<'static, Result<ModuleNode, std::io::Error>> {
        Box::pin(async move {
            let mut children = Vec::new();
            let mut symbols = Vec::new();

            if let Ok(mut entries) = fs::read_dir(&path).await {
                while let Some(entry) = entries.next_entry().await? {
                    let entry_path = entry.path();
                    let file_name = entry_path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string();

                    if file_name.starts_with('.') || file_name == "target" {
                        continue;
                    }

                    if entry_path.is_dir() {
                        if let Ok(child) = Self::scan_directory(entry_path.clone(), file_name).await {
                            children.push(child);
                        }
                    } else if file_name.ends_with(".rs") {
                        if let Ok(content) = fs::read_to_string(&entry_path).await {
                            if let Ok(file_symbols) = extract_symbols(&entry_path, &content) {
                                symbols.extend(file_symbols.iter().map(|s| s.name.clone()));
                            }
                        }
                    }
                }
            }

            Ok(ModuleNode {
                name,
                path,
                children,
                symbols,
            })
        })
    }

    /// Get impact analysis for a file change
    pub fn analyze_impact(&self, file_path: &Path) -> ImpactAnalysis {
        let mut affected_symbols = Vec::new();
        let mut affected_packages = Vec::new();

        // Find symbols in the changed file
        let file_symbols = self.symbol_index.find_in_file(file_path);
        affected_symbols.extend(file_symbols.iter().map(|s| s.name.clone()));

        // Find packages that might be affected (very simplified)
        for (pkg_name, pkg) in &self.dependency_graph.packages {
            if file_path.starts_with(&pkg.path) {
                affected_packages.push(pkg_name.clone());
            }
        }

        ImpactAnalysis {
            file_path: file_path.to_path_buf(),
            affected_symbols,
            affected_packages,
            estimated_test_files: self.estimate_affected_tests(file_path),
        }
    }

    fn estimate_affected_tests(&self, _file_path: &Path) -> Vec<PathBuf> {
        // Simplified: would need more sophisticated analysis
        Vec::new()
    }

    /// Get statistics about the project
    pub fn get_statistics(&self) -> ProjectStatistics {
        ProjectStatistics {
            total_packages: self.dependency_graph.packages.len(),
            total_symbols: self.symbol_index.len(),
            total_modules: self.count_modules(&self.module_hierarchy.root),
            has_circular_deps: self.dependency_graph.has_cycles(),
        }
    }

    fn count_modules(&self, node: &ModuleNode) -> usize {
        1 + node.children.iter().map(|c| self.count_modules(c)).sum::<usize>()
    }
}

/// Impact analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactAnalysis {
    pub file_path: PathBuf,
    pub affected_symbols: Vec<String>,
    pub affected_packages: Vec<String>,
    pub estimated_test_files: Vec<PathBuf>,
}

/// Project statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectStatistics {
    pub total_packages: usize,
    pub total_symbols: usize,
    pub total_modules: usize,
    pub has_circular_deps: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_node_creation() {
        let node = ModuleNode {
            name: "test".to_string(),
            path: PathBuf::from("/test"),
            children: vec![],
            symbols: vec!["Symbol1".to_string()],
        };

        assert_eq!(node.name, "test");
        assert_eq!(node.symbols.len(), 1);
    }

    #[test]
    fn test_impact_analysis_structure() {
        let analysis = ImpactAnalysis {
            file_path: PathBuf::from("src/main.rs"),
            affected_symbols: vec!["main".to_string()],
            affected_packages: vec!["my-app".to_string()],
            estimated_test_files: vec![],
        };

        assert_eq!(analysis.affected_symbols.len(), 1);
        assert_eq!(analysis.affected_packages.len(), 1);
    }
}
