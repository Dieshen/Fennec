use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use tokio::fs;
use tracing::{debug, info};
use uuid::Uuid;

/// Command plan for tracking planning sessions and execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandPlan {
    /// Unique identifier for this plan
    pub id: Uuid,
    /// Session this plan belongs to
    pub session_id: Uuid,
    /// Human-readable title for the plan
    pub title: String,
    /// Detailed description of what this plan aims to accomplish
    pub description: String,
    /// Ordered list of steps to execute
    pub steps: Vec<PlanStep>,
    /// Current status of the plan
    pub status: PlanStatus,
    /// When this plan was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When this plan was last updated
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Results from executing this plan
    pub execution_results: Vec<ExecutionResult>,
    /// Templates or patterns used to create this plan
    pub templates_used: Vec<String>,
    /// Tags for categorization and search
    pub tags: Vec<String>,
    /// User-defined priority level
    pub priority: PlanPriority,
    /// Estimated total effort for this plan
    pub estimated_effort: Option<String>,
    /// Actual time taken to complete (if completed)
    pub actual_duration: Option<Duration>,
}

/// Individual step within a command plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    /// Unique identifier for this step
    pub id: Uuid,
    /// Order of execution (0-based)
    pub order: u32,
    /// Description of what this step accomplishes
    pub description: String,
    /// Current status of this step
    pub status: StepStatus,
    /// Estimated effort for this step
    pub estimated_effort: Option<String>,
    /// Actual time taken to complete this step
    pub actual_duration: Option<Duration>,
    /// Other steps this step depends on
    pub dependencies: Vec<Uuid>,
    /// Commands associated with this step
    pub command_associations: Vec<CommandAssociation>,
    /// Notes specific to this step
    pub notes: Vec<String>,
    /// When this step was started
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    /// When this step was completed
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Status of a command plan
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PlanStatus {
    /// Plan is being drafted
    Draft,
    /// Plan is ready for execution
    Ready,
    /// Plan is currently being executed
    InProgress,
    /// Plan has been completed successfully
    Completed,
    /// Plan execution was cancelled
    Cancelled,
    /// Plan is temporarily on hold
    OnHold,
    /// Plan failed during execution
    Failed,
}

/// Status of an individual plan step
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StepStatus {
    /// Step is pending execution
    Pending,
    /// Step is currently being executed
    InProgress,
    /// Step completed successfully
    Completed,
    /// Step failed
    Failed,
    /// Step was skipped
    Skipped,
    /// Step is blocked by dependencies
    Blocked,
}

/// Priority level for plans
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PlanPriority {
    Low,
    Medium,
    High,
    Critical,
}

/// Association between a plan step and a command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandAssociation {
    /// The command that was executed
    pub command: String,
    /// When the command was executed
    pub executed_at: chrono::DateTime<chrono::Utc>,
    /// Result of the command execution
    pub result: ExecutionResult,
    /// Command output if successful
    pub output: Option<String>,
    /// Error message if failed
    pub error: Option<String>,
    /// How long the command took to execute
    pub duration: Option<Duration>,
    /// Exit code of the command
    pub exit_code: Option<i32>,
}

/// Result of executing a plan or command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Whether the execution was successful
    pub success: bool,
    /// Summary of what was accomplished
    pub summary: String,
    /// Detailed output or error information
    pub details: Option<String>,
    /// Metrics collected during execution
    pub metrics: HashMap<String, String>,
    /// Files or artifacts created
    pub artifacts: Vec<String>,
    /// Follow-up actions needed
    pub follow_up_actions: Vec<String>,
}

/// Storage service for managing command plans
#[derive(Debug)]
pub struct PlanStore {
    /// Base directory for storing plans
    storage_dir: PathBuf,
    /// In-memory cache of recent plans
    cache: HashMap<Uuid, CommandPlan>,
    /// Maximum cache size
    max_cache_size: usize,
}

impl PlanStore {
    /// Create a new plan store
    pub fn new() -> Result<Self> {
        let storage_dir = Self::get_storage_dir()?;

        // Ensure storage directory exists
        std::fs::create_dir_all(&storage_dir).with_context(|| {
            format!(
                "Failed to create plans directory: {}",
                storage_dir.display()
            )
        })?;

        Ok(Self {
            storage_dir,
            cache: HashMap::new(),
            max_cache_size: 100,
        })
    }

    /// Get the storage directory for plans
    fn get_storage_dir() -> Result<PathBuf> {
        let proj_dirs =
            ProjectDirs::from("", "", "fennec").context("Failed to get project directories")?;

        Ok(proj_dirs.data_dir().join("plans"))
    }

    /// Create a new command plan
    pub async fn create_plan(
        &mut self,
        session_id: Uuid,
        title: String,
        description: String,
    ) -> Result<Uuid> {
        let now = chrono::Utc::now();
        let plan_id = Uuid::new_v4();

        let plan = CommandPlan {
            id: plan_id,
            session_id,
            title: title.clone(),
            description,
            steps: Vec::new(),
            status: PlanStatus::Draft,
            created_at: now,
            updated_at: now,
            execution_results: Vec::new(),
            templates_used: Vec::new(),
            tags: Vec::new(),
            priority: PlanPriority::Medium,
            estimated_effort: None,
            actual_duration: None,
        };

        self.store_plan(&plan).await?;
        self.cache.insert(plan_id, plan);

        info!("Created plan: {} ({})", plan_id, title);
        Ok(plan_id)
    }

    /// Load a plan by ID
    pub async fn load_plan(&mut self, plan_id: Uuid) -> Result<Option<CommandPlan>> {
        // Check cache first
        if let Some(plan) = self.cache.get(&plan_id) {
            debug!("Found plan in cache: {}", plan_id);
            return Ok(Some(plan.clone()));
        }

        // Load from disk
        let file_path = self.get_plan_path(plan_id);
        if !file_path.exists() {
            return Ok(None);
        }

        let json = fs::read_to_string(&file_path)
            .await
            .with_context(|| format!("Failed to read plan from: {}", file_path.display()))?;

        let plan: CommandPlan = serde_json::from_str(&json)
            .with_context(|| format!("Failed to deserialize plan from: {}", file_path.display()))?;

        // Add to cache
        self.cache.insert(plan_id, plan.clone());
        self.manage_cache_size();

        debug!("Loaded plan from disk: {}", plan_id);
        Ok(Some(plan))
    }

    /// Update an existing plan
    pub async fn update_plan(&mut self, plan: CommandPlan) -> Result<()> {
        let plan_id = plan.id;
        let mut updated_plan = plan;
        updated_plan.updated_at = chrono::Utc::now();

        self.store_plan(&updated_plan).await?;
        self.cache.insert(updated_plan.id, updated_plan);

        debug!("Updated plan: {}", plan_id);
        Ok(())
    }

    /// Add a step to a plan
    pub async fn add_step(
        &mut self,
        plan_id: Uuid,
        description: String,
        dependencies: Vec<Uuid>,
    ) -> Result<Uuid> {
        let mut plan = self
            .load_plan(plan_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Plan not found: {}", plan_id))?;

        let step_id = Uuid::new_v4();
        let order = plan.steps.len() as u32;

        let step = PlanStep {
            id: step_id,
            order,
            description,
            status: StepStatus::Pending,
            estimated_effort: None,
            actual_duration: None,
            dependencies,
            command_associations: Vec::new(),
            notes: Vec::new(),
            started_at: None,
            completed_at: None,
        };

        plan.steps.push(step);
        self.update_plan(plan).await?;

        info!("Added step {} to plan {}", step_id, plan_id);
        Ok(step_id)
    }

    /// Update the status of a plan step
    pub async fn update_step_status(
        &mut self,
        plan_id: Uuid,
        step_id: Uuid,
        status: StepStatus,
    ) -> Result<()> {
        let mut plan = self
            .load_plan(plan_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Plan not found: {}", plan_id))?;

        let step = plan
            .steps
            .iter_mut()
            .find(|s| s.id == step_id)
            .ok_or_else(|| anyhow::anyhow!("Step not found: {}", step_id))?;

        let now = chrono::Utc::now();
        step.status = status.clone();

        match status {
            StepStatus::InProgress => {
                step.started_at = Some(now);
            }
            StepStatus::Completed | StepStatus::Failed | StepStatus::Skipped => {
                step.completed_at = Some(now);
                if let Some(started) = step.started_at {
                    step.actual_duration =
                        Some(Duration::from_secs((now - started).num_seconds() as u64));
                }
            }
            _ => {}
        }

        // Update plan status based on step statuses
        self.update_plan_status_from_steps(&mut plan);

        self.update_plan(plan).await?;

        debug!("Updated step {} status to {:?}", step_id, status);
        Ok(())
    }

    /// Associate a command execution with a plan step
    pub async fn associate_command(
        &mut self,
        plan_id: Uuid,
        step_id: Uuid,
        command: String,
        result: ExecutionResult,
    ) -> Result<()> {
        let mut plan = self
            .load_plan(plan_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Plan not found: {}", plan_id))?;

        let step = plan
            .steps
            .iter_mut()
            .find(|s| s.id == step_id)
            .ok_or_else(|| anyhow::anyhow!("Step not found: {}", step_id))?;

        let association = CommandAssociation {
            command,
            executed_at: chrono::Utc::now(),
            result,
            output: None,
            error: None,
            duration: None,
            exit_code: None,
        };

        step.command_associations.push(association);
        self.update_plan(plan).await?;

        debug!("Associated command with step {}", step_id);
        Ok(())
    }

    /// List all plans for a session
    pub async fn list_session_plans(&mut self, session_id: Uuid) -> Result<Vec<CommandPlan>> {
        let mut plans = Vec::new();

        let mut dir = fs::read_dir(&self.storage_dir).await.with_context(|| {
            format!(
                "Failed to read plans directory: {}",
                self.storage_dir.display()
            )
        })?;

        while let Some(entry) = dir.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(plan_id_str) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(plan_id) = Uuid::parse_str(plan_id_str) {
                        if let Ok(Some(plan)) = self.load_plan(plan_id).await {
                            if plan.session_id == session_id {
                                plans.push(plan);
                            }
                        }
                    }
                }
            }
        }

        // Sort by creation time (most recent first)
        plans.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(plans)
    }

    /// Search plans by title, description, or tags
    pub async fn search_plans(
        &mut self,
        query: &str,
        limit: Option<usize>,
    ) -> Result<Vec<PlanSearchResult>> {
        let mut results = Vec::new();
        use fuzzy_matcher::FuzzyMatcher;
        let matcher = fuzzy_matcher::skim::SkimMatcherV2::default();

        let mut dir = fs::read_dir(&self.storage_dir).await.with_context(|| {
            format!(
                "Failed to read plans directory: {}",
                self.storage_dir.display()
            )
        })?;

        while let Some(entry) = dir.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(plan_id_str) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(plan_id) = Uuid::parse_str(plan_id_str) {
                        if let Ok(Some(plan)) = self.load_plan(plan_id).await {
                            let mut best_score = 0i64;
                            let mut match_location = PlanMatchLocation::None;

                            // Search in title
                            if let Some(score) = matcher.fuzzy_match(&plan.title, query) {
                                if score > best_score {
                                    best_score = score;
                                    match_location = PlanMatchLocation::Title;
                                }
                            }

                            // Search in description
                            if let Some(score) = matcher.fuzzy_match(&plan.description, query) {
                                if score > best_score {
                                    best_score = score;
                                    match_location = PlanMatchLocation::Description;
                                }
                            }

                            // Search in tags
                            for tag in &plan.tags {
                                if let Some(score) = matcher.fuzzy_match(tag, query) {
                                    if score > best_score {
                                        best_score = score;
                                        match_location = PlanMatchLocation::Tags;
                                    }
                                }
                            }

                            // Search in step descriptions
                            for step in &plan.steps {
                                if let Some(score) = matcher.fuzzy_match(&step.description, query) {
                                    if score > best_score {
                                        best_score = score;
                                        match_location = PlanMatchLocation::Steps;
                                    }
                                }
                            }

                            if best_score > 0 {
                                results.push(PlanSearchResult {
                                    plan_id: plan.id,
                                    session_id: plan.session_id,
                                    title: plan.title,
                                    description: plan.description,
                                    status: plan.status,
                                    score: best_score,
                                    match_location,
                                    created_at: plan.created_at,
                                    updated_at: plan.updated_at,
                                });
                            }
                        }
                    }
                }
            }
        }

        // Sort by score (highest first)
        results.sort_by(|a, b| b.score.cmp(&a.score));

        // Apply limit if specified
        if let Some(limit) = limit {
            results.truncate(limit);
        }

        Ok(results)
    }

    /// Delete a plan
    pub async fn delete_plan(&mut self, plan_id: Uuid) -> Result<()> {
        // Remove from cache
        self.cache.remove(&plan_id);

        // Remove from disk
        let file_path = self.get_plan_path(plan_id);
        if file_path.exists() {
            fs::remove_file(&file_path)
                .await
                .with_context(|| format!("Failed to delete plan file: {}", file_path.display()))?;
            info!("Deleted plan: {}", plan_id);
        }

        Ok(())
    }

    /// Get plan template suggestions based on existing plans
    pub async fn get_plan_templates(&mut self, context: &str) -> Result<Vec<PlanTemplate>> {
        // This is a simplified implementation - in a real system you'd want
        // more sophisticated pattern recognition
        let mut templates = Vec::new();

        // Search for similar plans
        let search_results = self.search_plans(context, Some(5)).await?;

        for result in search_results {
            if let Ok(Some(plan)) = self.load_plan(result.plan_id).await {
                if plan.status == PlanStatus::Completed {
                    templates.push(PlanTemplate {
                        id: Uuid::new_v4(),
                        name: format!("Template: {}", plan.title),
                        description: format!("Based on successful plan: {}", plan.description),
                        steps: plan.steps.iter().map(|s| s.description.clone()).collect(),
                        estimated_effort: plan.estimated_effort,
                        tags: plan.tags,
                        usage_count: 1, // Would track actual usage in real implementation
                    });
                }
            }
        }

        Ok(templates)
    }

    /// Update plan status based on step statuses
    fn update_plan_status_from_steps(&self, plan: &mut CommandPlan) {
        if plan.steps.is_empty() {
            return;
        }

        let completed_steps = plan
            .steps
            .iter()
            .filter(|s| s.status == StepStatus::Completed)
            .count();
        let failed_steps = plan
            .steps
            .iter()
            .filter(|s| s.status == StepStatus::Failed)
            .count();
        let in_progress_steps = plan
            .steps
            .iter()
            .filter(|s| s.status == StepStatus::InProgress)
            .count();

        if failed_steps > 0 {
            plan.status = PlanStatus::Failed;
        } else if completed_steps == plan.steps.len() {
            plan.status = PlanStatus::Completed;
            plan.actual_duration = plan
                .steps
                .iter()
                .filter_map(|s| s.actual_duration)
                .fold(None, |acc, duration| {
                    Some(acc.unwrap_or(Duration::from_secs(0)) + duration)
                });
        } else if in_progress_steps > 0 {
            plan.status = PlanStatus::InProgress;
        }
    }

    /// Get the file path for a plan
    fn get_plan_path(&self, plan_id: Uuid) -> PathBuf {
        self.storage_dir.join(format!("{}.json", plan_id))
    }

    /// Store a plan to disk
    async fn store_plan(&self, plan: &CommandPlan) -> Result<()> {
        let file_path = self.get_plan_path(plan.id);
        let json = serde_json::to_string_pretty(plan).context("Failed to serialize plan")?;

        fs::write(&file_path, json)
            .await
            .with_context(|| format!("Failed to write plan to: {}", file_path.display()))?;

        Ok(())
    }

    /// Manage cache size by removing oldest entries
    fn manage_cache_size(&mut self) {
        while self.cache.len() > self.max_cache_size {
            if let Some((oldest_id, _)) = self
                .cache
                .iter()
                .min_by_key(|(_, plan)| plan.updated_at)
                .map(|(id, plan)| (*id, plan.clone()))
            {
                self.cache.remove(&oldest_id);
                debug!("Evicted plan from cache: {}", oldest_id);
            } else {
                break;
            }
        }
    }
}

/// Result of searching plans
#[derive(Debug, Clone)]
pub struct PlanSearchResult {
    pub plan_id: Uuid,
    pub session_id: Uuid,
    pub title: String,
    pub description: String,
    pub status: PlanStatus,
    pub score: i64,
    pub match_location: PlanMatchLocation,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Where a search match was found in a plan
#[derive(Debug, Clone)]
pub enum PlanMatchLocation {
    Title,
    Description,
    Tags,
    Steps,
    None,
}

/// Template for creating new plans based on existing patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanTemplate {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub steps: Vec<String>,
    pub estimated_effort: Option<String>,
    pub tags: Vec<String>,
    pub usage_count: u32,
}

impl Default for PlanStore {
    fn default() -> Self {
        Self::new().expect("Failed to create default PlanStore")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_create_and_load_plan() {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = temp_dir.path().to_owned();

        let mut store = PlanStore {
            storage_dir,
            cache: HashMap::new(),
            max_cache_size: 100,
        };

        let session_id = Uuid::new_v4();
        let plan_id = store
            .create_plan(
                session_id,
                "Test Plan".to_string(),
                "A plan for testing".to_string(),
            )
            .await
            .unwrap();

        let loaded = store.load_plan(plan_id).await.unwrap().unwrap();
        assert_eq!(loaded.title, "Test Plan");
        assert_eq!(loaded.session_id, session_id);
        assert_eq!(loaded.status, PlanStatus::Draft);
    }

    #[tokio::test]
    async fn test_add_step_to_plan() {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = temp_dir.path().to_owned();

        let mut store = PlanStore {
            storage_dir,
            cache: HashMap::new(),
            max_cache_size: 100,
        };

        let session_id = Uuid::new_v4();
        let plan_id = store
            .create_plan(
                session_id,
                "Test Plan".to_string(),
                "A plan for testing".to_string(),
            )
            .await
            .unwrap();

        let step_id = store
            .add_step(plan_id, "First step".to_string(), Vec::new())
            .await
            .unwrap();

        let plan = store.load_plan(plan_id).await.unwrap().unwrap();
        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].id, step_id);
        assert_eq!(plan.steps[0].description, "First step");
    }

    #[tokio::test]
    async fn test_update_step_status() {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = temp_dir.path().to_owned();

        let mut store = PlanStore {
            storage_dir,
            cache: HashMap::new(),
            max_cache_size: 100,
        };

        let session_id = Uuid::new_v4();
        let plan_id = store
            .create_plan(
                session_id,
                "Test Plan".to_string(),
                "A plan for testing".to_string(),
            )
            .await
            .unwrap();

        let step_id = store
            .add_step(plan_id, "First step".to_string(), Vec::new())
            .await
            .unwrap();

        store
            .update_step_status(plan_id, step_id, StepStatus::Completed)
            .await
            .unwrap();

        let plan = store.load_plan(plan_id).await.unwrap().unwrap();
        assert_eq!(plan.steps[0].status, StepStatus::Completed);
        assert_eq!(plan.status, PlanStatus::Completed);
    }

    #[tokio::test]
    async fn test_search_plans() {
        let temp_dir = TempDir::new().unwrap();
        let storage_dir = temp_dir.path().to_owned();

        let mut store = PlanStore {
            storage_dir,
            cache: HashMap::new(),
            max_cache_size: 100,
        };

        let session_id = Uuid::new_v4();
        store
            .create_plan(
                session_id,
                "Rust Implementation".to_string(),
                "Implement feature in Rust".to_string(),
            )
            .await
            .unwrap();

        let results = store.search_plans("rust", Some(10)).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Rust Implementation");
    }
}
