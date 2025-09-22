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

    #[test]
    fn test_context_requirements_default() {
        let requirements = ContextRequirements::default();
        assert_eq!(requirements.max_tokens, Some(2000));
        assert!(!requirements.include_full_content);
    }

    #[test]
    fn test_provider_context_injection() {
        let injection = ProviderContextInjection {
            system_prompt_enhancement: "Enhanced prompt".to_string(),
            user_context: "User context".to_string(),
            metadata: crate::context::ContextBundleMetadata {
                created_at: chrono::Utc::now(),
                request_id: "test".to_string(),
                strategies_used: Vec::new(),
                execution_time_ms: 100,
                cache_status: crate::context::CacheStatus::Miss,
            },
            estimated_tokens: 500,
        };

        assert_eq!(injection.estimated_tokens, 500);
        assert!(!injection.system_prompt_enhancement.is_empty());
    }
}
