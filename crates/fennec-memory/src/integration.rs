//! # Context Injection Integration Points
//!
//! This module provides integration points for injecting context into various
//! parts of the Fennec system, including AI providers and command execution.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::{
    context::{ContextBundle, ContextEngine, ContextRequest, ContextUseCase},
    service::{ConversationContext, MemoryService},
};

/// Trait for systems that can receive context injection
pub trait ContextReceiver {
    /// Inject context into the system
    fn inject_context(&mut self, context: &ContextBundle) -> Result<()>;

    /// Get the preferred context use case for this receiver
    fn preferred_use_case(&self) -> ContextUseCase;

    /// Get any specific context requirements
    fn context_requirements(&self) -> Option<ContextRequirements>;
}

/// Requirements for context injection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextRequirements {
    /// Maximum tokens that can be accepted
    pub max_tokens: Option<usize>,
    /// Preferred memory types
    pub preferred_memory_types: Vec<crate::service::MemoryType>,
    /// Minimum relevance score threshold
    pub min_relevance: Option<f64>,
    /// Whether to include full content or just previews
    pub include_full_content: bool,
}

impl Default for ContextRequirements {
    fn default() -> Self {
        Self {
            max_tokens: Some(2000),
            preferred_memory_types: vec![
                crate::service::MemoryType::Guidance,
                crate::service::MemoryType::Transcripts,
                crate::service::MemoryType::MemoryFiles,
            ],
            min_relevance: Some(0.3),
            include_full_content: false,
        }
    }
}

/// Context injection service that provides integration points
pub struct ContextInjectionService {
    context_engine: ContextEngine,
    memory_service: std::sync::Arc<MemoryService>,
}

impl ContextInjectionService {
    /// Create a new context injection service
    pub fn new(
        context_engine: ContextEngine,
        memory_service: std::sync::Arc<MemoryService>,
    ) -> Self {
        Self {
            context_engine,
            memory_service,
        }
    }

    /// Inject context for AI provider
    pub async fn inject_for_provider(
        &self,
        session_id: Uuid,
        conversation_context: ConversationContext,
        query: Option<String>,
    ) -> Result<ProviderContextInjection> {
        let request = ContextRequest {
            session_id,
            conversation_context,
            recent_messages: self.get_recent_messages(session_id).await?,
            explicit_query: query,
            preferred_types: vec![
                crate::service::MemoryType::Guidance,
                crate::service::MemoryType::Transcripts,
            ],
            use_case: ContextUseCase::AIPrompt,
            size_constraints: None,
        };

        let context_bundle = self.context_engine.inject_context(request).await?;

        Ok(ProviderContextInjection {
            system_prompt_enhancement: self.format_system_prompt_context(&context_bundle),
            user_context: self.format_user_context(&context_bundle),
            metadata: context_bundle.metadata,
            estimated_tokens: context_bundle.size_info.total_tokens,
        })
    }

    /// Inject context for command preview
    pub async fn inject_for_command_preview(
        &self,
        session_id: Uuid,
        command_name: &str,
        command_args: &serde_json::Value,
    ) -> Result<CommandContextInjection> {
        let conversation_context = self.get_session_context(session_id).await?;

        let request = ContextRequest {
            session_id,
            conversation_context,
            recent_messages: self.get_recent_messages(session_id).await?,
            explicit_query: Some(format!("{} command", command_name)),
            preferred_types: vec![
                crate::service::MemoryType::Guidance,
                crate::service::MemoryType::MemoryFiles,
            ],
            use_case: ContextUseCase::CommandPreview,
            size_constraints: None,
        };

        let context_bundle = self.context_engine.inject_context(request).await?;

        Ok(CommandContextInjection {
            relevant_guidance: self.extract_guidance_for_command(&context_bundle, command_name),
            similar_executions: self.extract_similar_executions(&context_bundle, command_name),
            warnings: self.generate_command_warnings(&context_bundle, command_args),
            suggestions: self.generate_command_suggestions(&context_bundle, command_name),
        })
    }

    /// Inject context for session initialization
    pub async fn inject_for_session_init(
        &self,
        session_id: Uuid,
    ) -> Result<SessionInitContextInjection> {
        let conversation_context = ConversationContext::default();

        let request = ContextRequest {
            session_id,
            conversation_context,
            recent_messages: Vec::new(),
            explicit_query: None,
            preferred_types: vec![
                crate::service::MemoryType::Guidance,
                crate::service::MemoryType::MemoryFiles,
            ],
            use_case: ContextUseCase::SessionInit,
            size_constraints: None,
        };

        let context_bundle = self.context_engine.inject_context(request).await?;

        Ok(SessionInitContextInjection {
            project_context: self.extract_project_context(&context_bundle),
            available_commands: self.extract_available_commands(&context_bundle),
            recent_patterns: self.extract_recent_patterns(&context_bundle),
            suggestions: self.generate_session_suggestions(&context_bundle),
        })
    }

    /// Get enhanced memory injection with context engine
    pub async fn get_enhanced_memory_injection(
        &self,
        session_id: Uuid,
        query: Option<&str>,
        use_case: ContextUseCase,
    ) -> Result<EnhancedMemoryInjection> {
        let conversation_context = self.get_session_context(session_id).await?;

        let request = ContextRequest {
            session_id,
            conversation_context: conversation_context.clone(),
            recent_messages: self.get_recent_messages(session_id).await?,
            explicit_query: query.map(|q| q.to_string()),
            preferred_types: vec![
                crate::service::MemoryType::Guidance,
                crate::service::MemoryType::Transcripts,
                crate::service::MemoryType::MemoryFiles,
            ],
            use_case,
            size_constraints: None,
        };

        let context_bundle = self.context_engine.inject_context(request).await?;

        // Also get the traditional memory injection for compatibility
        let traditional_injection = self
            .memory_service
            .get_memory_injection(session_id, query)
            .await?;

        let formatted_context = self.format_enhanced_context(&context_bundle);

        Ok(EnhancedMemoryInjection {
            traditional: traditional_injection,
            context_bundle,
            formatted_context,
        })
    }

    /// Helper to get recent messages for a session
    async fn get_recent_messages(
        &self,
        session_id: Uuid,
    ) -> Result<Vec<fennec_core::transcript::Message>> {
        if let Some(session_memory) = self.memory_service.get_session_memory(session_id).await {
            Ok(session_memory
                .transcript
                .messages
                .into_iter()
                .rev()
                .take(10)
                .collect())
        } else {
            Ok(Vec::new())
        }
    }

    /// Helper to get session context
    async fn get_session_context(&self, session_id: Uuid) -> Result<ConversationContext> {
        if let Some(session_memory) = self.memory_service.get_session_memory(session_id).await {
            Ok(session_memory.context)
        } else {
            Ok(ConversationContext::default())
        }
    }

    /// Format context for system prompt enhancement
    fn format_system_prompt_context(&self, context_bundle: &ContextBundle) -> String {
        let mut parts = Vec::new();

        if !context_bundle.items.is_empty() {
            parts.push("## Relevant Context".to_string());

            for item in &context_bundle.items {
                if item.relevance_score > 0.7 {
                    parts.push(format!("**{}**: {}", item.title, item.content));
                }
            }
        }

        if !context_bundle.summary.key_topics.is_empty() {
            parts.push(format!(
                "## Current Session Topics: {}",
                context_bundle.summary.key_topics.join(", ")
            ));
        }

        parts.join("\n\n")
    }

    /// Format context for user visibility
    fn format_user_context(&self, context_bundle: &ContextBundle) -> String {
        if context_bundle.items.is_empty() {
            return "No relevant context found.".to_string();
        }

        let mut formatted = String::new();
        formatted.push_str(&format!(
            "Found {} relevant context items:\n",
            context_bundle.items.len()
        ));

        for (i, item) in context_bundle.items.iter().take(3).enumerate() {
            formatted.push_str(&format!(
                "{}. {} (score: {:.2})\n",
                i + 1,
                item.title,
                item.relevance_score
            ));
        }

        if context_bundle.items.len() > 3 {
            formatted.push_str(&format!(
                "... and {} more items\n",
                context_bundle.items.len() - 3
            ));
        }

        formatted
    }

    /// Extract guidance relevant to a specific command
    fn extract_guidance_for_command(
        &self,
        context_bundle: &ContextBundle,
        command_name: &str,
    ) -> Vec<String> {
        context_bundle
            .items
            .iter()
            .filter(|item| {
                item.source_type == crate::service::MemoryType::Guidance
                    && item
                        .content
                        .to_lowercase()
                        .contains(&command_name.to_lowercase())
            })
            .map(|item| item.content.clone())
            .collect()
    }

    /// Extract similar command executions
    fn extract_similar_executions(
        &self,
        context_bundle: &ContextBundle,
        command_name: &str,
    ) -> Vec<String> {
        context_bundle
            .items
            .iter()
            .filter(|item| {
                item.source_type == crate::service::MemoryType::Transcripts
                    && item
                        .content
                        .to_lowercase()
                        .contains(&command_name.to_lowercase())
            })
            .map(|item| format!("Previous execution: {}", item.content))
            .collect()
    }

    /// Generate warnings for command execution
    fn generate_command_warnings(
        &self,
        _context_bundle: &ContextBundle,
        _command_args: &serde_json::Value,
    ) -> Vec<String> {
        // This would analyze the context for potential issues
        // For now, return empty - would be implemented based on specific command patterns
        Vec::new()
    }

    /// Generate suggestions for command execution
    fn generate_command_suggestions(
        &self,
        context_bundle: &ContextBundle,
        command_name: &str,
    ) -> Vec<String> {
        let mut suggestions = Vec::new();

        // Look for related guidance or patterns
        for item in &context_bundle.items {
            if item.source_type == crate::service::MemoryType::Guidance
                && item.content.to_lowercase().contains("best practice")
                && item
                    .content
                    .to_lowercase()
                    .contains(&command_name.to_lowercase())
            {
                suggestions.push(format!("Best practice: {}", item.title));
            }
        }

        suggestions
    }

    /// Extract project context for session initialization
    fn extract_project_context(&self, context_bundle: &ContextBundle) -> HashMap<String, String> {
        let mut context = HashMap::new();

        for item in &context_bundle.items {
            if item.source_type == crate::service::MemoryType::MemoryFiles {
                context.insert(item.title.clone(), item.content.clone());
            }
        }

        context
    }

    /// Extract available commands from guidance
    fn extract_available_commands(&self, context_bundle: &ContextBundle) -> Vec<String> {
        let mut commands = Vec::new();

        for item in &context_bundle.items {
            if item.source_type == crate::service::MemoryType::Guidance {
                // Simple pattern matching for command mentions
                for line in item.content.lines() {
                    if line.contains("Command:") || line.contains("Usage:") {
                        commands.push(line.to_string());
                    }
                }
            }
        }

        commands
    }

    /// Extract recent patterns from transcripts
    fn extract_recent_patterns(&self, context_bundle: &ContextBundle) -> Vec<String> {
        context_bundle
            .items
            .iter()
            .filter(|item| item.source_type == crate::service::MemoryType::Transcripts)
            .map(|item| format!("Recent activity: {}", item.title))
            .collect()
    }

    /// Generate suggestions for session initialization
    fn generate_session_suggestions(&self, context_bundle: &ContextBundle) -> Vec<String> {
        let mut suggestions = Vec::new();

        if !context_bundle.summary.key_topics.is_empty() {
            suggestions.push(format!(
                "Continue working on: {}",
                context_bundle.summary.key_topics.join(", ")
            ));
        }

        if context_bundle.items.iter().any(|item| {
            item.content.to_lowercase().contains("error")
                || item.content.to_lowercase().contains("failed")
        }) {
            suggestions.push("Review recent errors and issues".to_string());
        }

        suggestions
    }

    /// Format enhanced context for general use
    fn format_enhanced_context(&self, context_bundle: &ContextBundle) -> String {
        let mut formatted = String::new();

        formatted.push_str("# Enhanced Context\n\n");
        formatted.push_str(&format!(
            "**Quality Score**: {:.2}\n",
            context_bundle.quality_metrics.avg_relevance
        ));
        formatted.push_str(&format!(
            "**Items Found**: {}\n",
            context_bundle.items.len()
        ));
        formatted.push_str(&format!(
            "**Total Tokens**: {}\n\n",
            context_bundle.size_info.total_tokens
        ));

        if !context_bundle.summary.key_topics.is_empty() {
            formatted.push_str("## Key Topics\n");
            for topic in &context_bundle.summary.key_topics {
                formatted.push_str(&format!("- {}\n", topic));
            }
            formatted.push('\n');
        }

        formatted.push_str("## Context Items\n");
        for (i, item) in context_bundle.items.iter().enumerate() {
            formatted.push_str(&format!(
                "{}. **{}** (Score: {:.2}, Type: {:?})\n   {}\n\n",
                i + 1,
                item.title,
                item.relevance_score,
                item.source_type,
                item.content
            ));
        }

        formatted
    }
}

/// Context injection for AI providers
#[derive(Debug, Clone)]
pub struct ProviderContextInjection {
    /// Enhanced system prompt with relevant context
    pub system_prompt_enhancement: String,
    /// User-visible context information
    pub user_context: String,
    /// Context metadata
    pub metadata: crate::context::ContextBundleMetadata,
    /// Estimated token count
    pub estimated_tokens: usize,
}

/// Context injection for command previews
#[derive(Debug, Clone)]
pub struct CommandContextInjection {
    /// Relevant guidance for the command
    pub relevant_guidance: Vec<String>,
    /// Similar previous executions
    pub similar_executions: Vec<String>,
    /// Warnings about potential issues
    pub warnings: Vec<String>,
    /// Suggestions for better execution
    pub suggestions: Vec<String>,
}

/// Context injection for session initialization
#[derive(Debug, Clone)]
pub struct SessionInitContextInjection {
    /// Project-specific context
    pub project_context: HashMap<String, String>,
    /// Available commands from guidance
    pub available_commands: Vec<String>,
    /// Recent activity patterns
    pub recent_patterns: Vec<String>,
    /// Suggestions for the session
    pub suggestions: Vec<String>,
}

/// Enhanced memory injection combining traditional and context engine approaches
#[derive(Debug, Clone)]
pub struct EnhancedMemoryInjection {
    /// Traditional memory injection for backward compatibility
    pub traditional: crate::service::MemoryInjection,
    /// Context bundle from context engine
    pub context_bundle: crate::context::ContextBundle,
    /// Formatted context for easy consumption
    pub formatted_context: String,
}

/// Simple provider integration trait for demonstration
pub trait SimpleProviderIntegration {
    /// Enhance a prompt with context
    fn enhance_prompt(&mut self, base_prompt: &str, context: &ProviderContextInjection) -> String;
}

/// Simple command integration trait for demonstration
pub trait SimpleCommandIntegration {
    /// Enhance command preview with context
    fn enhance_preview(&mut self, base_preview: &str, context: &CommandContextInjection) -> String;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{
        CacheStatus, ContextBundleMetadata, ContextDiscoveryStrategy, ContextImportance, ContextItem,
        ContextItemMetadata, ContextQualityMetrics, ContextSizeInfo, ContextSummary, ContentClassification,
    };

    #[test]
    fn test_context_requirements_default() {
        let requirements = ContextRequirements::default();
        assert_eq!(requirements.max_tokens, Some(2000));
        assert!(!requirements.include_full_content);
        assert_eq!(requirements.preferred_memory_types.len(), 3);
        assert_eq!(requirements.min_relevance, Some(0.3));
    }

    #[test]
    fn test_context_requirements_custom() {
        let requirements = ContextRequirements {
            max_tokens: Some(5000),
            preferred_memory_types: vec![crate::service::MemoryType::Guidance],
            min_relevance: Some(0.5),
            include_full_content: true,
        };

        assert_eq!(requirements.max_tokens, Some(5000));
        assert_eq!(requirements.preferred_memory_types.len(), 1);
        assert_eq!(requirements.min_relevance, Some(0.5));
        assert!(requirements.include_full_content);
    }

    #[test]
    fn test_context_requirements_serialization() {
        let requirements = ContextRequirements::default();
        let json = serde_json::to_string(&requirements).unwrap();
        let deserialized: ContextRequirements = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.max_tokens, requirements.max_tokens);
        assert_eq!(deserialized.include_full_content, requirements.include_full_content);
    }

    #[test]
    fn test_context_requirements_no_max_tokens() {
        let requirements = ContextRequirements {
            max_tokens: None,
            preferred_memory_types: vec![],
            min_relevance: None,
            include_full_content: false,
        };

        assert!(requirements.max_tokens.is_none());
        assert!(requirements.min_relevance.is_none());
    }

    #[tokio::test]
    async fn test_context_injection_service_new() {
        let memory_service = std::sync::Arc::new(MemoryService::new().await.unwrap());
        let context_engine = crate::context::ContextEngine::new(memory_service.clone());

        let service = ContextInjectionService::new(context_engine, memory_service);
        // Service created successfully - just verify it exists
        assert!(std::mem::size_of_val(&service) > 0);
    }

    #[test]
    fn test_provider_context_injection_creation() {
        let injection = ProviderContextInjection {
            system_prompt_enhancement: "Enhanced prompt".to_string(),
            user_context: "User context".to_string(),
            metadata: create_test_metadata(),
            estimated_tokens: 500,
        };

        assert_eq!(injection.estimated_tokens, 500);
        assert_eq!(injection.system_prompt_enhancement, "Enhanced prompt");
        assert_eq!(injection.user_context, "User context");
    }

    #[test]
    fn test_provider_context_injection_empty() {
        let injection = ProviderContextInjection {
            system_prompt_enhancement: String::new(),
            user_context: String::new(),
            metadata: create_test_metadata(),
            estimated_tokens: 0,
        };

        assert_eq!(injection.estimated_tokens, 0);
        assert!(injection.system_prompt_enhancement.is_empty());
    }

    #[test]
    fn test_command_context_injection_creation() {
        let injection = CommandContextInjection {
            relevant_guidance: vec!["guidance1".to_string(), "guidance2".to_string()],
            similar_executions: vec!["exec1".to_string()],
            warnings: vec!["warning1".to_string()],
            suggestions: vec!["suggestion1".to_string()],
        };

        assert_eq!(injection.relevant_guidance.len(), 2);
        assert_eq!(injection.similar_executions.len(), 1);
        assert_eq!(injection.warnings.len(), 1);
        assert_eq!(injection.suggestions.len(), 1);
    }

    #[test]
    fn test_command_context_injection_empty() {
        let injection = CommandContextInjection {
            relevant_guidance: vec![],
            similar_executions: vec![],
            warnings: vec![],
            suggestions: vec![],
        };

        assert!(injection.relevant_guidance.is_empty());
        assert!(injection.similar_executions.is_empty());
        assert!(injection.warnings.is_empty());
        assert!(injection.suggestions.is_empty());
    }

    #[test]
    fn test_session_init_context_injection_creation() {
        let mut project_context = HashMap::new();
        project_context.insert("file1".to_string(), "content1".to_string());

        let injection = SessionInitContextInjection {
            project_context,
            available_commands: vec!["command1".to_string()],
            recent_patterns: vec!["pattern1".to_string()],
            suggestions: vec!["suggestion1".to_string()],
        };

        assert_eq!(injection.project_context.len(), 1);
        assert_eq!(injection.available_commands.len(), 1);
        assert_eq!(injection.recent_patterns.len(), 1);
        assert_eq!(injection.suggestions.len(), 1);
    }

    #[test]
    fn test_session_init_context_injection_empty() {
        let injection = SessionInitContextInjection {
            project_context: HashMap::new(),
            available_commands: vec![],
            recent_patterns: vec![],
            suggestions: vec![],
        };

        assert!(injection.project_context.is_empty());
        assert!(injection.available_commands.is_empty());
        assert!(injection.recent_patterns.is_empty());
    }

    #[test]
    fn test_enhanced_memory_injection_creation() {
        let traditional = crate::service::MemoryInjection {
            guidance: vec![],
            conversation_history: vec![],
            session_context: ConversationContext::default(),
            estimated_tokens: 0,
        };

        let injection = EnhancedMemoryInjection {
            traditional,
            context_bundle: create_test_context_bundle(),
            formatted_context: "Formatted context".to_string(),
        };

        assert!(!injection.formatted_context.is_empty());
        assert_eq!(injection.context_bundle.items.len(), 1);
    }

    #[tokio::test]
    async fn test_format_system_prompt_context_empty() {
        let service = create_test_service().await;
        let bundle = create_empty_context_bundle();

        let formatted = service.format_system_prompt_context(&bundle);
        assert_eq!(formatted, "");
    }

    #[tokio::test]
    async fn test_format_system_prompt_context_with_items() {
        let service = create_test_service().await;
        let bundle = create_test_context_bundle();

        let formatted = service.format_system_prompt_context(&bundle);
        assert!(formatted.contains("Relevant Context"));
    }

    #[tokio::test]
    async fn test_format_system_prompt_context_high_score_only() {
        let service = create_test_service().await;
        let mut bundle = create_test_context_bundle();
        bundle.items[0].relevance_score = 0.5; // Below 0.7 threshold

        let formatted = service.format_system_prompt_context(&bundle);
        // Should not contain the item since score is below threshold
        assert!(!formatted.contains("Test"));
    }

    #[tokio::test]
    async fn test_format_system_prompt_context_with_topics() {
        let service = create_test_service().await;
        let mut bundle = create_test_context_bundle();
        bundle.summary.key_topics = vec!["rust".to_string(), "testing".to_string()];

        let formatted = service.format_system_prompt_context(&bundle);
        assert!(formatted.contains("Current Session Topics"));
        assert!(formatted.contains("rust"));
        assert!(formatted.contains("testing"));
    }

    #[tokio::test]
    async fn test_format_user_context_empty() {
        let service = create_test_service().await;
        let bundle = create_empty_context_bundle();

        let formatted = service.format_user_context(&bundle);
        assert!(formatted.contains("No relevant context found"));
    }

    #[tokio::test]
    async fn test_format_user_context_with_items() {
        let service = create_test_service().await;
        let bundle = create_test_context_bundle();

        let formatted = service.format_user_context(&bundle);
        assert!(formatted.contains("Found 1 relevant context items"));
        assert!(formatted.contains("Test"));
    }

    #[tokio::test]
    async fn test_format_user_context_many_items() {
        let service = create_test_service().await;
        let mut bundle = create_test_context_bundle();

        // Add more items
        for i in 0..5 {
            bundle.items.push(create_test_context_item(&format!("item-{}", i), 0.7));
        }

        let formatted = service.format_user_context(&bundle);
        assert!(formatted.contains("... and"));
        assert!(formatted.contains("more items"));
    }

    #[tokio::test]
    async fn test_extract_guidance_for_command() {
        let service = create_test_service().await;
        let mut bundle = create_test_context_bundle();

        bundle.items[0].source_type = crate::service::MemoryType::Guidance;
        bundle.items[0].content = "This is guidance for the test command".to_string();

        let guidance = service.extract_guidance_for_command(&bundle, "test");
        assert_eq!(guidance.len(), 1);
        assert!(guidance[0].contains("test command"));
    }

    #[tokio::test]
    async fn test_extract_guidance_for_command_no_match() {
        let service = create_test_service().await;
        let bundle = create_test_context_bundle();

        let guidance = service.extract_guidance_for_command(&bundle, "nonexistent");
        assert_eq!(guidance.len(), 0);
    }

    #[tokio::test]
    async fn test_extract_guidance_for_command_case_insensitive() {
        let service = create_test_service().await;
        let mut bundle = create_test_context_bundle();

        bundle.items[0].source_type = crate::service::MemoryType::Guidance;
        bundle.items[0].content = "This is guidance for the TEST command".to_string();

        let guidance = service.extract_guidance_for_command(&bundle, "test");
        assert_eq!(guidance.len(), 1);
    }

    #[tokio::test]
    async fn test_extract_similar_executions() {
        let service = create_test_service().await;
        let mut bundle = create_test_context_bundle();

        bundle.items[0].source_type = crate::service::MemoryType::Transcripts;
        bundle.items[0].content = "Previously ran test command successfully".to_string();

        let executions = service.extract_similar_executions(&bundle, "test");
        assert_eq!(executions.len(), 1);
        assert!(executions[0].contains("Previous execution"));
    }

    #[tokio::test]
    async fn test_extract_similar_executions_no_match() {
        let service = create_test_service().await;
        let bundle = create_test_context_bundle();

        let executions = service.extract_similar_executions(&bundle, "nonexistent");
        assert_eq!(executions.len(), 0);
    }

    #[tokio::test]
    async fn test_generate_command_warnings() {
        let service = create_test_service().await;
        let bundle = create_test_context_bundle();
        let args = serde_json::json!({});

        let warnings = service.generate_command_warnings(&bundle, &args);
        // Currently returns empty - testing the signature
        assert!(warnings.is_empty());
    }

    #[tokio::test]
    async fn test_generate_command_suggestions() {
        let service = create_test_service().await;
        let mut bundle = create_test_context_bundle();

        bundle.items[0].source_type = crate::service::MemoryType::Guidance;
        bundle.items[0].content = "Best practice for test command: use --verbose".to_string();
        bundle.items[0].title = "Test best practices".to_string();

        let suggestions = service.generate_command_suggestions(&bundle, "test");
        assert_eq!(suggestions.len(), 1);
        assert!(suggestions[0].contains("Best practice"));
    }

    #[tokio::test]
    async fn test_generate_command_suggestions_no_match() {
        let service = create_test_service().await;
        let bundle = create_test_context_bundle();

        let suggestions = service.generate_command_suggestions(&bundle, "test");
        assert_eq!(suggestions.len(), 0);
    }

    #[tokio::test]
    async fn test_extract_project_context() {
        let service = create_test_service().await;
        let mut bundle = create_test_context_bundle();

        bundle.items[0].source_type = crate::service::MemoryType::MemoryFiles;
        bundle.items[0].title = "project.md".to_string();
        bundle.items[0].content = "Project documentation".to_string();

        let context = service.extract_project_context(&bundle);
        assert_eq!(context.len(), 1);
        assert!(context.contains_key("project.md"));
    }

    #[tokio::test]
    async fn test_extract_project_context_empty() {
        let service = create_test_service().await;
        let bundle = create_test_context_bundle();

        let context = service.extract_project_context(&bundle);
        assert_eq!(context.len(), 0);
    }

    #[tokio::test]
    async fn test_extract_available_commands() {
        let service = create_test_service().await;
        let mut bundle = create_test_context_bundle();

        bundle.items[0].source_type = crate::service::MemoryType::Guidance;
        bundle.items[0].content = "Command: fennec test\nUsage: fennec test <args>".to_string();

        let commands = service.extract_available_commands(&bundle);
        assert_eq!(commands.len(), 2);
        assert!(commands[0].contains("Command:"));
        assert!(commands[1].contains("Usage:"));
    }

    #[tokio::test]
    async fn test_extract_available_commands_no_match() {
        let service = create_test_service().await;
        let bundle = create_test_context_bundle();

        let commands = service.extract_available_commands(&bundle);
        assert_eq!(commands.len(), 0);
    }

    #[tokio::test]
    async fn test_extract_recent_patterns() {
        let service = create_test_service().await;
        let mut bundle = create_test_context_bundle();

        bundle.items[0].source_type = crate::service::MemoryType::Transcripts;
        bundle.items[0].title = "Recent test execution".to_string();

        let patterns = service.extract_recent_patterns(&bundle);
        assert_eq!(patterns.len(), 1);
        assert!(patterns[0].contains("Recent activity"));
    }

    #[tokio::test]
    async fn test_extract_recent_patterns_empty() {
        let service = create_test_service().await;
        let bundle = create_empty_context_bundle();

        let patterns = service.extract_recent_patterns(&bundle);
        assert_eq!(patterns.len(), 0);
    }

    #[tokio::test]
    async fn test_generate_session_suggestions_empty() {
        let service = create_test_service().await;
        let bundle = create_empty_context_bundle();

        let suggestions = service.generate_session_suggestions(&bundle);
        assert_eq!(suggestions.len(), 0);
    }

    #[tokio::test]
    async fn test_generate_session_suggestions_with_topics() {
        let service = create_test_service().await;
        let mut bundle = create_test_context_bundle();
        bundle.summary.key_topics = vec!["rust".to_string(), "testing".to_string()];

        let suggestions = service.generate_session_suggestions(&bundle);
        assert!(suggestions.len() > 0);
        assert!(suggestions[0].contains("Continue working on"));
    }

    #[tokio::test]
    async fn test_generate_session_suggestions_with_errors() {
        let service = create_test_service().await;
        let mut bundle = create_test_context_bundle();
        bundle.items[0].content = "An error occurred during execution".to_string();

        let suggestions = service.generate_session_suggestions(&bundle);
        assert!(suggestions.iter().any(|s| s.contains("Review recent errors")));
    }

    #[tokio::test]
    async fn test_generate_session_suggestions_with_failures() {
        let service = create_test_service().await;
        let mut bundle = create_test_context_bundle();
        bundle.items[0].content = "Test failed with exception".to_string();

        let suggestions = service.generate_session_suggestions(&bundle);
        assert!(suggestions.iter().any(|s| s.contains("Review recent errors")));
    }

    #[tokio::test]
    async fn test_format_enhanced_context() {
        let service = create_test_service().await;
        let bundle = create_test_context_bundle();

        let formatted = service.format_enhanced_context(&bundle);
        assert!(formatted.contains("# Enhanced Context"));
        assert!(formatted.contains("Quality Score"));
        assert!(formatted.contains("Items Found"));
        assert!(formatted.contains("Total Tokens"));
    }

    #[tokio::test]
    async fn test_format_enhanced_context_with_topics() {
        let service = create_test_service().await;
        let mut bundle = create_test_context_bundle();
        bundle.summary.key_topics = vec!["rust".to_string(), "ai".to_string()];

        let formatted = service.format_enhanced_context(&bundle);
        assert!(formatted.contains("## Key Topics"));
        assert!(formatted.contains("- rust"));
        assert!(formatted.contains("- ai"));
    }

    #[tokio::test]
    async fn test_format_enhanced_context_items() {
        let service = create_test_service().await;
        let bundle = create_test_context_bundle();

        let formatted = service.format_enhanced_context(&bundle);
        assert!(formatted.contains("## Context Items"));
        assert!(formatted.contains("1. **Test**"));
        assert!(formatted.contains("Score: 0.80"));
    }

    #[tokio::test]
    async fn test_format_enhanced_context_empty() {
        let service = create_test_service().await;
        let bundle = create_empty_context_bundle();

        let formatted = service.format_enhanced_context(&bundle);
        assert!(formatted.contains("# Enhanced Context"));
        assert!(formatted.contains("**Items Found**: 0"));
    }

    // Helper functions for tests
    fn create_test_metadata() -> ContextBundleMetadata {
        ContextBundleMetadata {
            created_at: chrono::Utc::now(),
            request_id: "test-req".to_string(),
            strategies_used: vec![ContextDiscoveryStrategy::KeywordExtraction],
            execution_time_ms: 100,
            cache_status: CacheStatus::Miss,
        }
    }

    fn create_test_context_item(id: &str, score: f64) -> ContextItem {
        ContextItem {
            id: id.to_string(),
            source_type: crate::service::MemoryType::Transcripts,
            title: "Test".to_string(),
            content: "Test content".to_string(),
            relevance_score: score,
            importance: ContextImportance::High,
            timestamp: chrono::Utc::now(),
            session_id: Some(Uuid::new_v4()),
            metadata: ContextItemMetadata {
                estimated_tokens: 100,
                discovery_strategy: "test".to_string(),
                matching_keywords: vec![],
                content_classification: ContentClassification::Technical,
                freshness_score: 0.9,
            },
        }
    }

    fn create_test_context_bundle() -> crate::context::ContextBundle {
        crate::context::ContextBundle {
            items: vec![create_test_context_item("test-1", 0.8)],
            summary: ContextSummary {
                description: "Test bundle".to_string(),
                key_topics: vec![],
                time_range: None,
                memory_types: vec![crate::service::MemoryType::Transcripts],
            },
            size_info: ContextSizeInfo {
                total_tokens: 100,
                item_count: 1,
                tokens_by_type: HashMap::new(),
                truncated: false,
            },
            quality_metrics: ContextQualityMetrics {
                avg_relevance: 0.8,
                topic_coverage: 0.7,
                freshness: 0.9,
                diversity: 0.6,
            },
            metadata: create_test_metadata(),
        }
    }

    fn create_empty_context_bundle() -> crate::context::ContextBundle {
        crate::context::ContextBundle {
            items: vec![],
            summary: ContextSummary {
                description: "Empty bundle".to_string(),
                key_topics: vec![],
                time_range: None,
                memory_types: vec![],
            },
            size_info: ContextSizeInfo {
                total_tokens: 0,
                item_count: 0,
                tokens_by_type: HashMap::new(),
                truncated: false,
            },
            quality_metrics: ContextQualityMetrics {
                avg_relevance: 0.0,
                topic_coverage: 0.0,
                freshness: 0.0,
                diversity: 0.0,
            },
            metadata: create_test_metadata(),
        }
    }

    async fn create_test_service() -> ContextInjectionService {
        let memory_service = std::sync::Arc::new(MemoryService::new().await.unwrap());
        let context_engine = crate::context::ContextEngine::new(memory_service.clone());
        ContextInjectionService::new(context_engine, memory_service)
    }
}
