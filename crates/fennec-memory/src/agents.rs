use anyhow::{Context, Result};
use directories::ProjectDirs;
use notify::{Event, EventKind, RecursiveMode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::sync::watch;
use tracing::{debug, error, info, warn};

/// Represents a parsed AGENTS.md file with structured content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentsConfig {
    /// Raw content of the AGENTS.md file
    pub raw_content: String,
    /// Structured sections parsed from the content
    pub sections: HashMap<String, AgentSection>,
    /// File path where this config was loaded from
    pub source_path: PathBuf,
    /// Last modified time
    pub last_modified: chrono::DateTime<chrono::Utc>,
}

/// Represents a section within the AGENTS.md file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSection {
    /// Section title (e.g., "Project Structure & Module Organization")
    pub title: String,
    /// Section content
    pub content: String,
    /// Subsections if any
    pub subsections: Vec<AgentSection>,
}

/// Service for loading and managing AGENTS.md files
#[derive(Debug)]
pub struct AgentsService {
    /// Current loaded configuration
    config: watch::Receiver<Option<AgentsConfig>>,
    /// Sender for configuration updates
    config_sender: watch::Sender<Option<AgentsConfig>>,
    /// File watcher for automatic reloading
    _watcher: Option<notify::RecommendedWatcher>,
}

impl AgentsService {
    /// Create a new AgentsService and load initial configuration
    pub async fn new() -> Result<Self> {
        let (config_sender, config) = watch::channel(None);

        let mut service = Self {
            config,
            config_sender,
            _watcher: None,
        };

        // Load initial configuration
        if let Err(e) = service.load_config().await {
            warn!("Failed to load initial AGENTS.md configuration: {}", e);
        }

        // Set up file watching
        if let Err(e) = service.setup_file_watching().await {
            warn!("Failed to set up file watching: {}", e);
        }

        Ok(service)
    }

    /// Get the current configuration
    pub fn get_config(&self) -> Option<AgentsConfig> {
        self.config.borrow().clone()
    }

    /// Subscribe to configuration changes
    pub fn subscribe(&self) -> watch::Receiver<Option<AgentsConfig>> {
        self.config.clone()
    }

    /// Load configuration from AGENTS.md files
    async fn load_config(&self) -> Result<()> {
        let config_paths = self.get_config_paths();

        for path in config_paths {
            if path.exists() {
                debug!("Loading AGENTS.md from: {}", path.display());

                match self.load_config_from_path(&path).await {
                    Ok(config) => {
                        info!("Successfully loaded AGENTS.md from: {}", path.display());
                        self.config_sender
                            .send(Some(config))
                            .map_err(|_| anyhow::anyhow!("Failed to send config update"))?;
                        return Ok(());
                    }
                    Err(e) => {
                        warn!("Failed to load AGENTS.md from {}: {}", path.display(), e);
                        continue;
                    }
                }
            }
        }

        debug!("No AGENTS.md files found in any of the expected locations");
        self.config_sender
            .send(None)
            .map_err(|_| anyhow::anyhow!("Failed to send config update"))?;

        Ok(())
    }

    /// Get ordered list of configuration file paths to check
    fn get_config_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // First priority: repo root ./AGENTS.md
        if let Ok(current_dir) = std::env::current_dir() {
            paths.push(current_dir.join("AGENTS.md"));
        }

        // Second priority: global ~/.fennec/AGENTS.md
        if let Some(proj_dirs) = ProjectDirs::from("", "", "fennec") {
            paths.push(proj_dirs.config_dir().join("AGENTS.md"));
        }

        paths
    }

    /// Load configuration from a specific path
    async fn load_config_from_path(&self, path: &Path) -> Result<AgentsConfig> {
        let content = fs::read_to_string(path)
            .await
            .with_context(|| format!("Failed to read AGENTS.md from {}", path.display()))?;

        let metadata = fs::metadata(path)
            .await
            .with_context(|| format!("Failed to get metadata for {}", path.display()))?;

        let last_modified = metadata
            .modified()
            .map(|time| chrono::DateTime::<chrono::Utc>::from(time))
            .unwrap_or_else(|_| chrono::Utc::now());

        let sections = self.parse_markdown(&content)?;

        Ok(AgentsConfig {
            raw_content: content,
            sections,
            source_path: path.to_owned(),
            last_modified,
        })
    }

    /// Parse markdown content into structured sections
    fn parse_markdown(&self, content: &str) -> Result<HashMap<String, AgentSection>> {
        let mut sections = HashMap::new();
        let mut current_section: Option<AgentSection> = None;
        let mut current_content = Vec::new();

        for line in content.lines() {
            if line.starts_with("## ") {
                // Save previous section if exists
                if let Some(section) = current_section.take() {
                    sections.insert(section.title.clone(), section);
                }

                // Start new section
                let title = line.trim_start_matches("## ").trim().to_string();
                current_section = Some(AgentSection {
                    title: title.clone(),
                    content: String::new(),
                    subsections: Vec::new(),
                });
                current_content.clear();
            } else if line.starts_with("### ") && current_section.is_some() {
                // Handle subsection
                let subsection_title = line.trim_start_matches("### ").trim().to_string();
                let mut _subsection_content: Vec<String> = Vec::new();

                // This is a simplified parser - in a real implementation you'd want
                // to handle nested subsections more robustly
                if let Some(ref mut section) = current_section {
                    section.subsections.push(AgentSection {
                        title: subsection_title,
                        content: String::new(), // Would collect content until next heading
                        subsections: Vec::new(),
                    });
                }
                current_content.push(line.to_string());
            } else {
                current_content.push(line.to_string());
            }
        }

        // Save final section
        if let Some(mut section) = current_section {
            section.content = current_content.join("\n");
            sections.insert(section.title.clone(), section);
        }

        Ok(sections)
    }

    /// Set up file watching for automatic reloading
    async fn setup_file_watching(&mut self) -> Result<()> {
        use notify::Watcher;

        let _config_sender = self.config_sender.clone();
        let paths = self.get_config_paths();

        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            match res {
                Ok(event) => {
                    if matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                        // Trigger reload (simplified - in production you'd want debouncing)
                        debug!("AGENTS.md file changed, triggering reload");
                        // Note: In a real implementation, you'd need a more sophisticated
                        // way to trigger reloads from the file watcher callback
                    }
                }
                Err(e) => error!("File watch error: {:?}", e),
            }
        })?;

        // Watch directories containing the config files
        for path in &paths {
            if let Some(parent) = path.parent() {
                if parent.exists() {
                    if let Err(e) = watcher.watch(parent, RecursiveMode::NonRecursive) {
                        warn!("Failed to watch directory {}: {}", parent.display(), e);
                    } else {
                        debug!(
                            "Watching directory for AGENTS.md changes: {}",
                            parent.display()
                        );
                    }
                }
            }
        }

        self._watcher = Some(watcher);
        Ok(())
    }

    /// Search for relevant guidance based on a query
    pub fn search_guidance(&self, query: &str) -> Vec<GuidanceMatch> {
        let config = match self.get_config() {
            Some(config) => config,
            None => return Vec::new(),
        };

        let mut matches = Vec::new();
        use fuzzy_matcher::FuzzyMatcher;
        let matcher = fuzzy_matcher::skim::SkimMatcherV2::default();

        // Search in section titles and content
        for (_, section) in &config.sections {
            // Check title match
            if let Some(score) = matcher.fuzzy_match(&section.title, query) {
                matches.push(GuidanceMatch {
                    section_title: section.title.clone(),
                    content: section.content.clone(),
                    score,
                    match_type: MatchType::Title,
                });
            }

            // Check content match
            if let Some(score) = matcher.fuzzy_match(&section.content, query) {
                matches.push(GuidanceMatch {
                    section_title: section.title.clone(),
                    content: section.content.clone(),
                    score,
                    match_type: MatchType::Content,
                });
            }

            // Check subsections
            for subsection in &section.subsections {
                if let Some(score) = matcher.fuzzy_match(&subsection.title, query) {
                    matches.push(GuidanceMatch {
                        section_title: format!("{} > {}", section.title, subsection.title),
                        content: subsection.content.clone(),
                        score,
                        match_type: MatchType::Subsection,
                    });
                }
            }
        }

        // Sort by score (highest first)
        matches.sort_by(|a, b| b.score.cmp(&a.score));
        matches
    }

    /// Get all available guidance sections
    pub fn get_all_guidance(&self) -> Vec<String> {
        let config = match self.get_config() {
            Some(config) => config,
            None => return Vec::new(),
        };

        config.sections.keys().cloned().collect()
    }

    /// Get specific guidance section by title
    pub fn get_guidance_section(&self, title: &str) -> Option<AgentSection> {
        let config = self.get_config()?;
        config.sections.get(title).cloned()
    }
}

/// Represents a search match for guidance
#[derive(Debug, Clone)]
pub struct GuidanceMatch {
    pub section_title: String,
    pub content: String,
    pub score: i64,
    pub match_type: MatchType,
}

#[derive(Debug, Clone)]
pub enum MatchType {
    Title,
    Content,
    Subsection,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parse_markdown() {
        let service = AgentsService::new().await.unwrap();
        let content = r#"# Repository Guidelines

## Project Structure & Module Organization
- `src/cli/` holds the TUI front end
- `src/agents/` contains reusable agent behaviors

## Build, Test, and Development Commands  
- `cargo fmt` formats Rust code
- `cargo clippy` enforces lint rules
"#;

        let sections = service.parse_markdown(content).unwrap();
        assert_eq!(sections.len(), 2);
        assert!(sections.contains_key("Project Structure & Module Organization"));
        assert!(sections.contains_key("Build, Test, and Development Commands"));
    }

    #[tokio::test]
    async fn test_search_guidance() {
        let service = AgentsService::new().await.unwrap();
        // This test would need a mock config loaded for full testing
        let matches = service.search_guidance("cargo");
        // Without loaded config, should return empty
        assert!(matches.is_empty());
    }
}
