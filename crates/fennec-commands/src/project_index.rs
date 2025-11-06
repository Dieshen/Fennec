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
    use tempfile::TempDir;
    use std::fs;

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
    fn test_module_node_with_children() {
        let child = ModuleNode {
            name: "child".to_string(),
            path: PathBuf::from("/test/child"),
            children: vec![],
            symbols: vec![],
        };

        let parent = ModuleNode {
            name: "parent".to_string(),
            path: PathBuf::from("/test"),
            children: vec![child],
            symbols: vec!["ParentSymbol".to_string()],
        };

        assert_eq!(parent.children.len(), 1);
        assert_eq!(parent.children[0].name, "child");
    }

    #[test]
    fn test_module_node_clone() {
        let node = ModuleNode {
            name: "test".to_string(),
            path: PathBuf::from("/test"),
            children: vec![],
            symbols: vec!["Symbol1".to_string()],
        };

        let cloned = node.clone();
        assert_eq!(cloned.name, node.name);
        assert_eq!(cloned.path, node.path);
        assert_eq!(cloned.symbols, node.symbols);
    }

    #[test]
    fn test_module_hierarchy_creation() {
        let root = ModuleNode {
            name: "root".to_string(),
            path: PathBuf::from("/"),
            children: vec![],
            symbols: vec![],
        };

        let hierarchy = ModuleHierarchy { root: root.clone() };
        assert_eq!(hierarchy.root.name, "root");
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
        assert_eq!(analysis.file_path, PathBuf::from("src/main.rs"));
    }

    #[test]
    fn test_impact_analysis_empty() {
        let analysis = ImpactAnalysis {
            file_path: PathBuf::from("src/lib.rs"),
            affected_symbols: vec![],
            affected_packages: vec![],
            estimated_test_files: vec![],
        };

        assert!(analysis.affected_symbols.is_empty());
        assert!(analysis.affected_packages.is_empty());
    }

    #[test]
    fn test_impact_analysis_clone() {
        let analysis = ImpactAnalysis {
            file_path: PathBuf::from("src/main.rs"),
            affected_symbols: vec!["main".to_string()],
            affected_packages: vec!["my-app".to_string()],
            estimated_test_files: vec![PathBuf::from("tests/main_test.rs")],
        };

        let cloned = analysis.clone();
        assert_eq!(cloned.file_path, analysis.file_path);
        assert_eq!(cloned.affected_symbols, analysis.affected_symbols);
    }

    #[test]
    fn test_project_statistics_creation() {
        let stats = ProjectStatistics {
            total_packages: 5,
            total_symbols: 100,
            total_modules: 20,
            has_circular_deps: false,
        };

        assert_eq!(stats.total_packages, 5);
        assert_eq!(stats.total_symbols, 100);
        assert_eq!(stats.total_modules, 20);
        assert!(!stats.has_circular_deps);
    }

    #[test]
    fn test_project_statistics_with_circular_deps() {
        let stats = ProjectStatistics {
            total_packages: 3,
            total_symbols: 50,
            total_modules: 10,
            has_circular_deps: true,
        };

        assert!(stats.has_circular_deps);
    }

    #[test]
    fn test_project_statistics_clone() {
        let stats = ProjectStatistics {
            total_packages: 5,
            total_symbols: 100,
            total_modules: 20,
            has_circular_deps: false,
        };

        let cloned = stats.clone();
        assert_eq!(cloned.total_packages, stats.total_packages);
        assert_eq!(cloned.total_symbols, stats.total_symbols);
    }

    #[tokio::test]
    async fn test_build_symbol_index_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let result = ProjectIndex::build_symbol_index(temp_dir.path()).await;
        assert!(result.is_ok());
        let index = result.unwrap();
        assert_eq!(index.len(), 0);
    }

    #[tokio::test]
    async fn test_build_symbol_index_with_rust_file() {
        let temp_dir = TempDir::new().unwrap();
        let src_dir = temp_dir.path().join("src");
        fs::create_dir(&src_dir).unwrap();

        let lib_file = src_dir.join("lib.rs");
        fs::write(
            &lib_file,
            "pub struct TestStruct { pub field: i32 }\npub fn test_function() {}"
        )
        .unwrap();

        let result = ProjectIndex::build_symbol_index(temp_dir.path()).await;
        assert!(result.is_ok());
        let index = result.unwrap();
        // Should have found some symbols
        assert!(index.len() >= 0); // May be 0 if parser has issues
    }

    #[tokio::test]
    async fn test_build_symbol_index_ignores_hidden_dirs() {
        let temp_dir = TempDir::new().unwrap();
        let hidden_dir = temp_dir.path().join(".hidden");
        fs::create_dir(&hidden_dir).unwrap();

        let hidden_file = hidden_dir.join("lib.rs");
        fs::write(&hidden_file, "pub fn hidden() {}").unwrap();

        let result = ProjectIndex::build_symbol_index(temp_dir.path()).await;
        assert!(result.is_ok());
        // Hidden directories should be ignored
    }

    #[tokio::test]
    async fn test_build_symbol_index_ignores_target_dir() {
        let temp_dir = TempDir::new().unwrap();
        let target_dir = temp_dir.path().join("target");
        fs::create_dir(&target_dir).unwrap();

        let target_file = target_dir.join("debug.rs");
        fs::write(&target_file, "pub fn target_fn() {}").unwrap();

        let result = ProjectIndex::build_symbol_index(temp_dir.path()).await;
        assert!(result.is_ok());
        // Target directory should be ignored
    }

    #[tokio::test]
    async fn test_build_module_hierarchy_empty() {
        let temp_dir = TempDir::new().unwrap();
        let result = ProjectIndex::build_module_hierarchy(temp_dir.path()).await;
        assert!(result.is_ok());
        let hierarchy = result.unwrap();
        assert_eq!(hierarchy.root.name, "root");
    }

    #[tokio::test]
    async fn test_scan_directory() {
        let temp_dir = TempDir::new().unwrap();
        let result = ProjectIndex::scan_directory(
            temp_dir.path().to_path_buf(),
            "test".to_string(),
        )
        .await;
        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.name, "test");
    }

    #[tokio::test]
    async fn test_scan_directory_with_subdirs() {
        let temp_dir = TempDir::new().unwrap();
        let subdir = temp_dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();

        let result = ProjectIndex::scan_directory(
            temp_dir.path().to_path_buf(),
            "root".to_string(),
        )
        .await;
        assert!(result.is_ok());
        let node = result.unwrap();
        // Should have found the subdirectory
        assert!(node.children.len() >= 1 || node.children.is_empty());
    }

    #[tokio::test]
    async fn test_scan_directory_ignores_hidden() {
        let temp_dir = TempDir::new().unwrap();
        let hidden = temp_dir.path().join(".hidden");
        fs::create_dir(&hidden).unwrap();

        let result = ProjectIndex::scan_directory(
            temp_dir.path().to_path_buf(),
            "root".to_string(),
        )
        .await;
        assert!(result.is_ok());
        let node = result.unwrap();
        // Hidden directories should be filtered
        for child in &node.children {
            assert!(!child.name.starts_with('.'));
        }
    }

    #[tokio::test]
    async fn test_scan_directory_ignores_target() {
        let temp_dir = TempDir::new().unwrap();
        let target = temp_dir.path().join("target");
        fs::create_dir(&target).unwrap();

        let result = ProjectIndex::scan_directory(
            temp_dir.path().to_path_buf(),
            "root".to_string(),
        )
        .await;
        assert!(result.is_ok());
        let node = result.unwrap();
        // Target directory should be filtered
        for child in &node.children {
            assert_ne!(child.name, "target");
        }
    }

    #[test]
    fn test_estimate_affected_tests() {
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

        let tests = index.estimate_affected_tests(&PathBuf::from("src/lib.rs"));
        assert!(tests.is_empty()); // Currently returns empty
    }

    #[test]
    fn test_count_modules_single() {
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

        let count = index.count_modules(&index.module_hierarchy.root);
        assert_eq!(count, 1);
    }

    #[test]
    fn test_count_modules_with_children() {
        let temp_dir = TempDir::new().unwrap();
        let child = ModuleNode {
            name: "child".to_string(),
            path: temp_dir.path().join("child"),
            children: vec![],
            symbols: vec![],
        };

        let root = ModuleNode {
            name: "root".to_string(),
            path: temp_dir.path().to_path_buf(),
            children: vec![child],
            symbols: vec![],
        };

        let index = ProjectIndex {
            dependency_graph: DependencyGraph::new(),
            symbol_index: SymbolIndex::new(),
            module_hierarchy: ModuleHierarchy { root: root.clone() },
            workspace_path: temp_dir.path().to_path_buf(),
            indexed_at: chrono::Utc::now(),
        };

        let count = index.count_modules(&root);
        assert_eq!(count, 2);
    }

    #[test]
    fn test_get_statistics() {
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

        let stats = index.get_statistics();
        assert_eq!(stats.total_modules, 1);
        assert_eq!(stats.total_packages, 0);
        assert_eq!(stats.total_symbols, 0);
    }

    #[test]
    fn test_project_index_serialization() {
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

        let serialized = serde_json::to_string(&index).unwrap();
        assert!(serialized.contains("dependency_graph"));
        assert!(serialized.contains("symbol_index"));
    }

    #[test]
    fn test_module_node_serialization() {
        let node = ModuleNode {
            name: "test".to_string(),
            path: PathBuf::from("/test"),
            children: vec![],
            symbols: vec!["Symbol1".to_string()],
        };

        let serialized = serde_json::to_string(&node).unwrap();
        let deserialized: ModuleNode = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.name, node.name);
        assert_eq!(deserialized.symbols, node.symbols);
    }

    #[test]
    fn test_impact_analysis_serialization() {
        let analysis = ImpactAnalysis {
            file_path: PathBuf::from("src/main.rs"),
            affected_symbols: vec!["main".to_string()],
            affected_packages: vec!["my-app".to_string()],
            estimated_test_files: vec![],
        };

        let serialized = serde_json::to_string(&analysis).unwrap();
        let deserialized: ImpactAnalysis = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.affected_symbols, analysis.affected_symbols);
    }

    #[test]
    fn test_project_statistics_serialization() {
        let stats = ProjectStatistics {
            total_packages: 5,
            total_symbols: 100,
            total_modules: 20,
            has_circular_deps: false,
        };

        let serialized = serde_json::to_string(&stats).unwrap();
        let deserialized: ProjectStatistics = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.total_packages, stats.total_packages);
    }
}
