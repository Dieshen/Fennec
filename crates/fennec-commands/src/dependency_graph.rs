use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use tokio::fs;

/// Represents a Cargo package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CargoPackage {
    pub name: String,
    pub version: String,
    pub path: PathBuf,
    pub dependencies: Vec<Dependency>,
    pub dev_dependencies: Vec<Dependency>,
    pub is_workspace_member: bool,
}

/// Represents a dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub version: Option<String>,
    pub path: Option<PathBuf>,
    pub features: Vec<String>,
    pub optional: bool,
}

/// Dependency graph for the project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyGraph {
    pub packages: HashMap<String, CargoPackage>,
    pub edges: HashMap<String, Vec<String>>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            packages: HashMap::new(),
            edges: HashMap::new(),
        }
    }

    /// Add a package to the graph
    pub fn add_package(&mut self, package: CargoPackage) {
        let name = package.name.clone();
        let deps: Vec<String> = package
            .dependencies
            .iter()
            .map(|d| d.name.clone())
            .collect();

        self.packages.insert(name.clone(), package);
        self.edges.insert(name, deps);
    }

    /// Get all packages that depend on a given package
    pub fn get_dependents(&self, package_name: &str) -> Vec<&CargoPackage> {
        self.edges
            .iter()
            .filter_map(|(name, deps)| {
                if deps.contains(&package_name.to_string()) {
                    self.packages.get(name)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get all dependencies of a package (transitive)
    pub fn get_all_dependencies(&self, package_name: &str) -> HashSet<String> {
        let mut result = HashSet::new();
        let mut queue = vec![package_name.to_string()];

        while let Some(pkg) = queue.pop() {
            if let Some(deps) = self.edges.get(&pkg) {
                for dep in deps {
                    if result.insert(dep.clone()) {
                        queue.push(dep.clone());
                    }
                }
            }
        }

        result
    }

    /// Check if there are circular dependencies
    pub fn has_cycles(&self) -> bool {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for package in self.packages.keys() {
            if self.has_cycle_util(package, &mut visited, &mut rec_stack) {
                return true;
            }
        }

        false
    }

    fn has_cycle_util(
        &self,
        node: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
    ) -> bool {
        if !visited.contains(node) {
            visited.insert(node.to_string());
            rec_stack.insert(node.to_string());

            if let Some(deps) = self.edges.get(node) {
                for dep in deps {
                    if !visited.contains(dep) {
                        if self.has_cycle_util(dep, visited, rec_stack) {
                            return true;
                        }
                    } else if rec_stack.contains(dep) {
                        return true;
                    }
                }
            }
        }

        rec_stack.remove(node);
        false
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse Cargo.toml to extract package information
pub async fn parse_cargo_toml(path: &Path) -> Result<CargoPackage, std::io::Error> {
    let content = fs::read_to_string(path).await?;

    // Simple TOML parsing (in real implementation, use toml crate)
    let package_name = extract_value(&content, "name");
    let package_version = extract_value(&content, "version");

    let dependencies = extract_dependencies(&content, "[dependencies]");
    let dev_dependencies = extract_dependencies(&content, "[dev-dependencies]");

    Ok(CargoPackage {
        name: package_name.unwrap_or_else(|| "unknown".to_string()),
        version: package_version.unwrap_or_else(|| "0.0.0".to_string()),
        path: path.parent().unwrap_or(Path::new(".")).to_path_buf(),
        dependencies,
        dev_dependencies,
        is_workspace_member: false,
    })
}

fn extract_value(content: &str, key: &str) -> Option<String> {
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with(key) {
            if let Some(value) = line.split('=').nth(1) {
                return Some(value.trim().trim_matches('"').to_string());
            }
        }
    }
    None
}

fn extract_dependencies(content: &str, section: &str) -> Vec<Dependency> {
    let mut deps = Vec::new();
    let mut in_section = false;

    for line in content.lines() {
        let line = line.trim();

        if line.starts_with(section) {
            in_section = true;
            continue;
        }

        if in_section && line.starts_with('[') {
            break;
        }

        if in_section && !line.is_empty() && !line.starts_with('#') {
            if let Some(name) = line.split('=').next() {
                let name = name.trim().to_string();
                let version = line
                    .split('=')
                    .nth(1)
                    .map(|v| v.trim().trim_matches('"').to_string());

                deps.push(Dependency {
                    name,
                    version,
                    path: None,
                    features: Vec::new(),
                    optional: false,
                });
            }
        }
    }

    deps
}

/// Build dependency graph from workspace
pub async fn build_dependency_graph(
    workspace_path: &Path,
) -> Result<DependencyGraph, std::io::Error> {
    let mut graph = DependencyGraph::new();

    // Find all Cargo.toml files
    let cargo_files = find_cargo_files(workspace_path.to_path_buf()).await?;

    for cargo_file in cargo_files {
        match parse_cargo_toml(&cargo_file).await {
            Ok(package) => {
                graph.add_package(package);
            }
            Err(_) => {
                // Skip files that can't be parsed
                continue;
            }
        }
    }

    Ok(graph)
}

fn find_cargo_files(
    path: PathBuf,
) -> futures::future::BoxFuture<'static, Result<Vec<PathBuf>, std::io::Error>> {
    Box::pin(async move {
        let mut result = Vec::new();
        let mut entries = fs::read_dir(&path).await?;

        while let Some(entry) = entries.next_entry().await? {
            let entry_path = entry.path();

            if entry_path.is_file()
                && entry_path.file_name() == Some(std::ffi::OsStr::new("Cargo.toml"))
            {
                result.push(entry_path);
            } else if entry_path.is_dir() {
                let dir_name = entry_path.file_name().and_then(|n| n.to_str());
                if dir_name != Some("target")
                    && dir_name != Some("node_modules")
                    && !dir_name.map(|n| n.starts_with('.')).unwrap_or(false)
                {
                    let sub_files = find_cargo_files(entry_path).await?;
                    result.extend(sub_files);
                }
            }
        }

        Ok(result)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dependency_graph_creation() {
        let mut graph = DependencyGraph::new();

        let pkg = CargoPackage {
            name: "test-pkg".to_string(),
            version: "1.0.0".to_string(),
            path: PathBuf::from("/test"),
            dependencies: vec![Dependency {
                name: "serde".to_string(),
                version: Some("1.0".to_string()),
                path: None,
                features: vec![],
                optional: false,
            }],
            dev_dependencies: vec![],
            is_workspace_member: false,
        };

        graph.add_package(pkg);
        assert_eq!(graph.packages.len(), 1);
        assert!(graph.packages.contains_key("test-pkg"));
    }

    #[test]
    fn test_get_dependents() {
        let mut graph = DependencyGraph::new();

        let pkg1 = CargoPackage {
            name: "pkg1".to_string(),
            version: "1.0.0".to_string(),
            path: PathBuf::from("/pkg1"),
            dependencies: vec![Dependency {
                name: "base".to_string(),
                version: None,
                path: None,
                features: vec![],
                optional: false,
            }],
            dev_dependencies: vec![],
            is_workspace_member: false,
        };

        graph.add_package(pkg1);

        let dependents = graph.get_dependents("base");
        assert_eq!(dependents.len(), 1);
        assert_eq!(dependents[0].name, "pkg1");
    }

    #[test]
    fn test_transitive_dependencies() {
        let mut graph = DependencyGraph::new();

        let pkg1 = CargoPackage {
            name: "pkg1".to_string(),
            version: "1.0.0".to_string(),
            path: PathBuf::from("/pkg1"),
            dependencies: vec![Dependency {
                name: "pkg2".to_string(),
                version: None,
                path: None,
                features: vec![],
                optional: false,
            }],
            dev_dependencies: vec![],
            is_workspace_member: false,
        };

        let pkg2 = CargoPackage {
            name: "pkg2".to_string(),
            version: "1.0.0".to_string(),
            path: PathBuf::from("/pkg2"),
            dependencies: vec![Dependency {
                name: "pkg3".to_string(),
                version: None,
                path: None,
                features: vec![],
                optional: false,
            }],
            dev_dependencies: vec![],
            is_workspace_member: false,
        };

        graph.add_package(pkg1);
        graph.add_package(pkg2);

        let all_deps = graph.get_all_dependencies("pkg1");
        assert!(all_deps.contains("pkg2"));
        assert!(all_deps.contains("pkg3"));
    }

    #[test]
    fn test_extract_value() {
        let content = r#"
[package]
name = "test-crate"
version = "0.1.0"
        "#;

        assert_eq!(
            extract_value(content, "name"),
            Some("test-crate".to_string())
        );
        assert_eq!(extract_value(content, "version"), Some("0.1.0".to_string()));
    }
}
