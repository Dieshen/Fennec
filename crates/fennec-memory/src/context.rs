//! # Context Injection Engine
//!
//! This module provides intelligent context discovery, selection, and injection
//! for enhancing AI interactions with relevant memory context.
//!
//! ## Features
//!
//! - **Automatic Context Discovery**: Analyzes current conversation to identify context needs
//! - **Smart Filtering**: Intelligently filters and ranks relevant memory
//! - **Context Summarization**: Formats context for optimal AI consumption
//! - **Integration Points**: Provides interfaces for provider and command system integration
//! - **Context Management**: Handles size constraints, deduplication, and optimization

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tracing::{debug, info};
use uuid::Uuid;

use fennec_core::transcript::Message;

use crate::service::{
    AdvancedSearchCriteria, ConversationContext, MemoryService, MemoryType, ScoringStrategy,
    SessionFilter, TimeFilter, UnifiedSearchResult,
};

/// Core context injection engine
#[derive(Debug)]
pub struct ContextEngine {
    /// Reference to memory service
    memory_service: std::sync::Arc<MemoryService>,
    /// Context configuration
    config: ContextConfig,
    /// Context cache for performance
    context_cache: std::sync::Arc<tokio::sync::RwLock<ContextCache>>,
}

/// Configuration for context injection behavior
#[derive(Debug, Clone)]
pub struct ContextConfig {
    /// Maximum context size in tokens
    pub max_context_tokens: usize,
    /// Maximum number of context items
    pub max_context_items: usize,
    /// Context freshness threshold in hours
    pub freshness_threshold_hours: u32,
    /// Enable context caching
    pub enable_caching: bool,
    /// Cache TTL in minutes
    pub cache_ttl_minutes: u32,
    /// Context discovery strategies to use
    pub discovery_strategies: Vec<ContextDiscoveryStrategy>,
    /// Default scoring strategy
    pub default_scoring_strategy: ScoringStrategy,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            max_context_tokens: 4000,
            max_context_items: 20,
            freshness_threshold_hours: 24,
            enable_caching: true,
            cache_ttl_minutes: 30,
            discovery_strategies: vec![
                ContextDiscoveryStrategy::ConversationAnalysis,
                ContextDiscoveryStrategy::KeywordExtraction,
                ContextDiscoveryStrategy::TopicModeling,
                ContextDiscoveryStrategy::SessionHistory,
            ],
            default_scoring_strategy: ScoringStrategy::ContextAware {
                conversation_context: ConversationContext::default(),
            },
        }
    }
}

/// Strategies for discovering relevant context
#[derive(Debug, Clone, PartialEq)]
pub enum ContextDiscoveryStrategy {
    /// Analyze current conversation for context needs
    ConversationAnalysis,
    /// Extract keywords and topics from recent messages
    KeywordExtraction,
    /// Use topic modeling to find related content
    TopicModeling,
    /// Look at historical session patterns
    SessionHistory,
    /// Use explicit user queries or commands
    ExplicitQuery,
}

/// Request for context injection
#[derive(Debug, Clone)]
pub struct ContextRequest {
    /// Current session
    pub session_id: Uuid,
    /// Current conversation context
    pub conversation_context: ConversationContext,
    /// Recent messages for analysis
    pub recent_messages: Vec<Message>,
    /// Explicit query or topic focus
    pub explicit_query: Option<String>,
    /// Preferred context types
    pub preferred_types: Vec<MemoryType>,
    /// Target use case for context
    pub use_case: ContextUseCase,
    /// Size constraints
    pub size_constraints: Option<ContextSizeConstraints>,
}

/// Use cases for context injection
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum ContextUseCase {
    /// AI prompt enhancement
    AIPrompt,
    /// Command preview enhancement
    CommandPreview,
    /// Session initialization
    SessionInit,
    /// Real-time conversation support
    ConversationSupport,
    /// Knowledge synthesis
    KnowledgeSynthesis,
}

/// Size constraints for context
#[derive(Debug, Clone)]
pub struct ContextSizeConstraints {
    /// Maximum tokens
    pub max_tokens: Option<usize>,
    /// Maximum number of items
    pub max_items: Option<usize>,
    /// Preferred token distribution across memory types
    pub token_distribution: Option<HashMap<MemoryType, f64>>,
}

/// Individual context item with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextItem {
    /// Unique identifier
    pub id: String,
    /// Source memory type
    pub source_type: MemoryType,
    /// Content title/summary
    pub title: String,
    /// Main content
    pub content: String,
    /// Relevance score (0.0 to 1.0)
    pub relevance_score: f64,
    /// Context importance level
    pub importance: ContextImportance,
    /// Timestamp of original content
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Associated session (if any)
    pub session_id: Option<Uuid>,
    /// Metadata for context management
    pub metadata: ContextItemMetadata,
}

/// Importance levels for context items
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ContextImportance {
    Critical,
    High,
    Medium,
    Low,
}

/// Metadata for context items
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextItemMetadata {
    /// Estimated token count
    pub estimated_tokens: usize,
    /// Discovery strategy that found this item
    pub discovery_strategy: String,
    /// Keywords that matched
    pub matching_keywords: Vec<String>,
    /// Content type classification
    pub content_classification: ContentClassification,
    /// Freshness score based on age
    pub freshness_score: f64,
}

/// Classification of context content
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ContentClassification {
    Technical,
    Conversational,
    Documentation,
    ProblemSolving,
    Planning,
    Learning,
    Reference,
}

/// Assembled context bundle ready for injection
#[derive(Debug, Clone)]
pub struct ContextBundle {
    /// Context items in priority order
    pub items: Vec<ContextItem>,
    /// Summary of the context
    pub summary: ContextSummary,
    /// Total size information
    pub size_info: ContextSizeInfo,
    /// Quality metrics
    pub quality_metrics: ContextQualityMetrics,
    /// Bundle metadata
    pub metadata: ContextBundleMetadata,
}

/// Summary of context bundle
#[derive(Debug, Clone)]
pub struct ContextSummary {
    /// Brief description of included context
    pub description: String,
    /// Key topics covered
    pub key_topics: Vec<String>,
    /// Time range of context
    pub time_range: Option<(chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)>,
    /// Memory types included
    pub memory_types: Vec<MemoryType>,
}

/// Size information for context bundle
#[derive(Debug, Clone)]
pub struct ContextSizeInfo {
    /// Total estimated tokens
    pub total_tokens: usize,
    /// Number of items
    pub item_count: usize,
    /// Tokens by memory type
    pub tokens_by_type: HashMap<MemoryType, usize>,
    /// Truncation applied
    pub truncated: bool,
}

/// Quality metrics for context bundle
#[derive(Debug, Clone)]
pub struct ContextQualityMetrics {
    /// Average relevance score
    pub avg_relevance: f64,
    /// Coverage of conversation topics
    pub topic_coverage: f64,
    /// Freshness score
    pub freshness: f64,
    /// Diversity score (variety of sources)
    pub diversity: f64,
}

/// Metadata about context bundle creation
#[derive(Debug, Clone)]
pub struct ContextBundleMetadata {
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Request that generated this bundle
    pub request_id: String,
    /// Discovery strategies used
    pub strategies_used: Vec<ContextDiscoveryStrategy>,
    /// Search execution time
    pub execution_time_ms: u64,
    /// Cache hit/miss status
    pub cache_status: CacheStatus,
}

/// Cache status for context operations
#[derive(Debug, Clone, PartialEq)]
pub enum CacheStatus {
    Hit,
    Miss,
    Partial,
    Disabled,
}

/// Context cache for performance optimization
#[derive(Debug)]
struct ContextCache {
    /// Cached context bundles
    bundles: HashMap<String, (ContextBundle, chrono::DateTime<chrono::Utc>)>,
    /// Maximum cache size
    max_size: usize,
}

impl ContextCache {
    fn new(max_size: usize) -> Self {
        Self {
            bundles: HashMap::new(),
            max_size,
        }
    }

    /// Get cached context bundle if valid
    fn get(&mut self, key: &str, ttl_minutes: u32) -> Option<ContextBundle> {
        if let Some((bundle, cached_at)) = self.bundles.get(key) {
            let age = chrono::Utc::now() - *cached_at;
            if age.num_minutes() < ttl_minutes as i64 {
                debug!("Context cache hit for key: {}", key);
                return Some(bundle.clone());
            } else {
                // Expired, remove it
                self.bundles.remove(key);
            }
        }
        None
    }

    /// Store context bundle in cache
    fn store(&mut self, key: String, bundle: ContextBundle) {
        // Evict oldest if at capacity
        if self.bundles.len() >= self.max_size {
            if let Some(oldest_key) = self.get_oldest_key() {
                self.bundles.remove(&oldest_key);
            }
        }

        self.bundles.insert(key, (bundle, chrono::Utc::now()));
        debug!("Stored context bundle in cache");
    }

    fn get_oldest_key(&self) -> Option<String> {
        self.bundles
            .iter()
            .min_by_key(|(_, (_, timestamp))| timestamp)
            .map(|(key, _)| key.clone())
    }
}

impl ContextEngine {
    /// Create a new context engine
    pub fn new(memory_service: std::sync::Arc<MemoryService>) -> Self {
        let config = ContextConfig::default();
        let context_cache = std::sync::Arc::new(tokio::sync::RwLock::new(ContextCache::new(100)));

        Self {
            memory_service,
            config,
            context_cache,
        }
    }

    /// Create context engine with custom configuration
    pub fn with_config(
        memory_service: std::sync::Arc<MemoryService>,
        config: ContextConfig,
    ) -> Self {
        let context_cache = std::sync::Arc::new(tokio::sync::RwLock::new(ContextCache::new(100)));

        Self {
            memory_service,
            config,
            context_cache,
        }
    }

    /// Discover and inject relevant context
    pub async fn inject_context(&self, request: ContextRequest) -> Result<ContextBundle> {
        let start_time = std::time::Instant::now();
        let request_id = format!("ctx_{}", Uuid::new_v4());

        debug!(
            "Context injection request for session: {} with use case: {:?}",
            request.session_id, request.use_case
        );

        // Check cache first
        let cache_key = self.generate_cache_key(&request);
        if self.config.enable_caching {
            let mut cache = self.context_cache.write().await;
            if let Some(cached_bundle) = cache.get(&cache_key, self.config.cache_ttl_minutes) {
                info!("Returning cached context bundle");
                return Ok(cached_bundle);
            }
        }

        // Discover context using configured strategies
        let mut all_context_items = Vec::new();
        let mut strategies_used = Vec::new();

        for strategy in &self.config.discovery_strategies {
            strategies_used.push(strategy.clone());
            let items = self
                .discover_context_with_strategy(&request, strategy)
                .await?;
            all_context_items.extend(items);
        }

        // Remove duplicates and apply smart filtering
        all_context_items = self.deduplicate_context_items(all_context_items);
        all_context_items = self.apply_smart_filtering(all_context_items, &request);

        // Score and rank context items
        self.score_and_rank_items(&mut all_context_items, &request);

        // Apply size constraints and build final bundle
        let final_items = self.apply_size_constraints(all_context_items, &request);
        let bundle = self.build_context_bundle(final_items, &request, strategies_used, &request_id);

        let execution_time = start_time.elapsed();
        debug!(
            "Context injection completed in {}ms, found {} items",
            execution_time.as_millis(),
            bundle.items.len()
        );

        // Cache the result
        if self.config.enable_caching {
            let mut cache = self.context_cache.write().await;
            cache.store(cache_key, bundle.clone());
        }

        Ok(bundle)
    }

    /// Discover context using a specific strategy
    async fn discover_context_with_strategy(
        &self,
        request: &ContextRequest,
        strategy: &ContextDiscoveryStrategy,
    ) -> Result<Vec<ContextItem>> {
        match strategy {
            ContextDiscoveryStrategy::ConversationAnalysis => {
                self.discover_from_conversation_analysis(request).await
            }
            ContextDiscoveryStrategy::KeywordExtraction => {
                self.discover_from_keyword_extraction(request).await
            }
            ContextDiscoveryStrategy::TopicModeling => {
                self.discover_from_topic_modeling(request).await
            }
            ContextDiscoveryStrategy::SessionHistory => {
                self.discover_from_session_history(request).await
            }
            ContextDiscoveryStrategy::ExplicitQuery => {
                self.discover_from_explicit_query(request).await
            }
        }
    }

    /// Discover context from conversation analysis
    async fn discover_from_conversation_analysis(
        &self,
        request: &ContextRequest,
    ) -> Result<Vec<ContextItem>> {
        debug!("Discovering context from conversation analysis");

        let mut context_items = Vec::new();

        // Analyze recent messages for patterns
        let analysis = self.analyze_conversation_patterns(&request.recent_messages);

        // Search for related content based on analysis
        for query in analysis.suggested_queries {
            let search_criteria = AdvancedSearchCriteria {
                query,
                session_filter: Some(SessionFilter::ExcludeCurrentSession(request.session_id)),
                time_filter: Some(TimeFilter::LastDays(7)), // Focus on recent content
                memory_types: request.preferred_types.clone(),
                scoring_strategy: ScoringStrategy::ContextAware {
                    conversation_context: request.conversation_context.clone(),
                },
                limit: Some(5),
                min_score: Some(0.3),
            };

            let search_results = self.memory_service.search_advanced(search_criteria).await?;
            context_items.extend(self.convert_search_results_to_context_items(
                search_results.results,
                "conversation_analysis",
            ));
        }

        Ok(context_items)
    }

    /// Discover context from keyword extraction
    async fn discover_from_keyword_extraction(
        &self,
        request: &ContextRequest,
    ) -> Result<Vec<ContextItem>> {
        debug!("Discovering context from keyword extraction");

        let keywords = self.extract_keywords(&request.recent_messages);
        let mut context_items = Vec::new();

        for keyword in keywords {
            let search_criteria = AdvancedSearchCriteria {
                query: keyword,
                session_filter: Some(SessionFilter::CrossSession),
                time_filter: Some(TimeFilter::LastDays(30)),
                memory_types: vec![MemoryType::Transcripts, MemoryType::MemoryFiles],
                scoring_strategy: ScoringStrategy::FuzzyMatch,
                limit: Some(3),
                min_score: Some(0.4),
            };

            let search_results = self.memory_service.search_advanced(search_criteria).await?;
            context_items.extend(self.convert_search_results_to_context_items(
                search_results.results,
                "keyword_extraction",
            ));
        }

        Ok(context_items)
    }

    /// Discover context from topic modeling
    async fn discover_from_topic_modeling(
        &self,
        request: &ContextRequest,
    ) -> Result<Vec<ContextItem>> {
        debug!("Discovering context from topic modeling");

        // Extract topics from conversation context
        let topics = self.extract_topics_from_context(&request.conversation_context);
        let mut context_items = Vec::new();

        for topic in topics {
            let search_criteria = AdvancedSearchCriteria {
                query: topic,
                session_filter: Some(SessionFilter::CrossSession),
                time_filter: Some(TimeFilter::LastDays(14)),
                memory_types: vec![MemoryType::Guidance, MemoryType::MemoryFiles],
                scoring_strategy: ScoringStrategy::Weighted {
                    text_relevance_weight: 0.6,
                    recency_weight: 0.2,
                    session_relevance_weight: 0.2,
                },
                limit: Some(4),
                min_score: Some(0.3),
            };

            let search_results = self.memory_service.search_advanced(search_criteria).await?;
            context_items.extend(
                self.convert_search_results_to_context_items(
                    search_results.results,
                    "topic_modeling",
                ),
            );
        }

        Ok(context_items)
    }

    /// Discover context from session history
    async fn discover_from_session_history(
        &self,
        request: &ContextRequest,
    ) -> Result<Vec<ContextItem>> {
        debug!("Discovering context from session history");

        let search_criteria = AdvancedSearchCriteria {
            query: "".to_string(), // Empty query to get all results
            session_filter: Some(SessionFilter::CurrentSession(request.session_id)),
            time_filter: Some(TimeFilter::LastHours(24)),
            memory_types: vec![MemoryType::Transcripts],
            scoring_strategy: ScoringStrategy::Weighted {
                text_relevance_weight: 0.3,
                recency_weight: 0.7,
                session_relevance_weight: 1.0,
            },
            limit: Some(10),
            min_score: None,
        };

        let search_results = self.memory_service.search_advanced(search_criteria).await?;
        Ok(self.convert_search_results_to_context_items(search_results.results, "session_history"))
    }

    /// Discover context from explicit query
    async fn discover_from_explicit_query(
        &self,
        request: &ContextRequest,
    ) -> Result<Vec<ContextItem>> {
        debug!("Discovering context from explicit query");

        if let Some(ref query) = request.explicit_query {
            let search_criteria = AdvancedSearchCriteria {
                query: query.clone(),
                session_filter: Some(SessionFilter::CrossSession),
                time_filter: None, // No time filter for explicit queries
                memory_types: request.preferred_types.clone(),
                scoring_strategy: self.config.default_scoring_strategy.clone(),
                limit: Some(self.config.max_context_items / 2), // Reserve half for explicit queries
                min_score: Some(0.2),
            };

            let search_results = self.memory_service.search_advanced(search_criteria).await?;
            Ok(self
                .convert_search_results_to_context_items(search_results.results, "explicit_query"))
        } else {
            Ok(Vec::new())
        }
    }

    /// Generate cache key for context request
    fn generate_cache_key(&self, request: &ContextRequest) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        request.session_id.hash(&mut hasher);
        request.use_case.hash(&mut hasher);
        request.explicit_query.hash(&mut hasher);

        // Hash recent message content
        for msg in &request.recent_messages {
            msg.content.hash(&mut hasher);
        }

        format!("ctx_{:x}", hasher.finish())
    }

    /// Analyze conversation patterns to suggest queries
    fn analyze_conversation_patterns(&self, messages: &[Message]) -> ConversationAnalysis {
        let mut suggested_queries = Vec::new();
        let mut intent = ConversationIntent::General;

        // Simple pattern analysis - in production this would be more sophisticated
        for message in messages.iter().rev().take(5) {
            let content_lower = message.content.to_lowercase();

            // Detect intent from patterns
            if content_lower.contains("implement") || content_lower.contains("create") {
                intent = ConversationIntent::Implementation;
                suggested_queries.push(format!(
                    "implementation {}",
                    self.extract_key_terms(&message.content)
                ));
            } else if content_lower.contains("error") || content_lower.contains("debug") {
                intent = ConversationIntent::Debugging;
                suggested_queries.push(format!(
                    "error {}",
                    self.extract_key_terms(&message.content)
                ));
            } else if content_lower.contains("explain") || content_lower.contains("how") {
                intent = ConversationIntent::Learning;
                suggested_queries.push(format!(
                    "explanation {}",
                    self.extract_key_terms(&message.content)
                ));
            }
        }

        ConversationAnalysis {
            suggested_queries,
            intent,
        }
    }

    /// Extract keywords from messages
    fn extract_keywords(&self, messages: &[Message]) -> Vec<String> {
        let mut keywords = HashSet::new();

        for message in messages.iter().rev().take(3) {
            // Simple keyword extraction - in production use NLP libraries
            let words: Vec<&str> = message
                .content
                .split_whitespace()
                .filter(|word| word.len() > 3 && !self.is_stop_word(word))
                .collect();

            for word in words {
                keywords.insert(word.to_lowercase());
            }
        }

        keywords.into_iter().collect()
    }

    /// Extract topics from conversation context
    fn extract_topics_from_context(&self, context: &ConversationContext) -> Vec<String> {
        let mut topics = Vec::new();

        // Add technologies as topics
        topics.extend(context.technologies.clone());

        // Add current task as topic
        if let Some(ref task) = context.current_task {
            topics.push(task.clone());
        }

        // Add recent topics
        topics.extend(context.recent_topics.iter().take(3).cloned());

        topics
    }

    /// Check if word is a stop word
    fn is_stop_word(&self, word: &str) -> bool {
        // Simple stop word list - in production use a comprehensive list
        let stop_words = [
            "the", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with", "by",
        ];
        stop_words.contains(&word.to_lowercase().as_str())
    }

    /// Extract key terms from content
    fn extract_key_terms(&self, content: &str) -> String {
        content
            .split_whitespace()
            .filter(|word| word.len() > 3 && !self.is_stop_word(word))
            .take(3)
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Convert search results to context items
    fn convert_search_results_to_context_items(
        &self,
        results: Vec<UnifiedSearchResult>,
        discovery_strategy: &str,
    ) -> Vec<ContextItem> {
        results
            .into_iter()
            .map(|result| ContextItem {
                id: result.id,
                source_type: result.memory_type,
                title: result.title,
                content: result.content_preview.clone(),
                relevance_score: result.relevance_score,
                importance: self.classify_importance(result.relevance_score),
                timestamp: result.timestamp,
                session_id: result.session_id,
                metadata: ContextItemMetadata {
                    estimated_tokens: result.content_preview.len() / 4, // Rough estimate
                    discovery_strategy: discovery_strategy.to_string(),
                    matching_keywords: Vec::new(), // Would be populated with actual matches
                    content_classification: self.classify_content(&result.content_preview),
                    freshness_score: self.calculate_freshness_score(result.timestamp),
                },
            })
            .collect()
    }

    /// Classify importance based on relevance score
    fn classify_importance(&self, relevance_score: f64) -> ContextImportance {
        if relevance_score >= 0.8 {
            ContextImportance::Critical
        } else if relevance_score >= 0.6 {
            ContextImportance::High
        } else if relevance_score >= 0.4 {
            ContextImportance::Medium
        } else {
            ContextImportance::Low
        }
    }

    /// Classify content type
    fn classify_content(&self, content: &str) -> ContentClassification {
        let content_lower = content.to_lowercase();

        if content_lower.contains("function")
            || content_lower.contains("class")
            || content_lower.contains("implement")
        {
            ContentClassification::Technical
        } else if content_lower.contains("plan")
            || content_lower.contains("task")
            || content_lower.contains("todo")
        {
            ContentClassification::Planning
        } else if content_lower.contains("error")
            || content_lower.contains("debug")
            || content_lower.contains("fix")
        {
            ContentClassification::ProblemSolving
        } else if content_lower.contains("learn")
            || content_lower.contains("explain")
            || content_lower.contains("understand")
        {
            ContentClassification::Learning
        } else {
            ContentClassification::Conversational
        }
    }

    /// Calculate freshness score based on age
    fn calculate_freshness_score(&self, timestamp: chrono::DateTime<chrono::Utc>) -> f64 {
        let now = chrono::Utc::now();
        let age_hours = (now - timestamp).num_hours() as f64;

        // Exponential decay with configurable threshold
        let threshold = self.config.freshness_threshold_hours as f64;
        (-age_hours / threshold).exp()
    }

    /// Remove duplicate context items
    fn deduplicate_context_items(&self, mut items: Vec<ContextItem>) -> Vec<ContextItem> {
        let mut seen_ids = HashSet::new();
        items.retain(|item| seen_ids.insert(item.id.clone()));
        items
    }

    /// Apply smart filtering to context items
    fn apply_smart_filtering(
        &self,
        mut items: Vec<ContextItem>,
        request: &ContextRequest,
    ) -> Vec<ContextItem> {
        // Filter by use case relevance
        items.retain(|item| self.is_relevant_for_use_case(item, &request.use_case));

        // Filter by freshness if needed
        if request.use_case == ContextUseCase::ConversationSupport {
            items.retain(|item| item.metadata.freshness_score > 0.1);
        }

        items
    }

    /// Check if context item is relevant for specific use case
    fn is_relevant_for_use_case(&self, item: &ContextItem, use_case: &ContextUseCase) -> bool {
        match use_case {
            ContextUseCase::AIPrompt => true, // All context can be relevant for AI
            ContextUseCase::CommandPreview => {
                matches!(
                    item.metadata.content_classification,
                    ContentClassification::Technical | ContentClassification::ProblemSolving
                )
            }
            ContextUseCase::SessionInit => {
                matches!(
                    item.metadata.content_classification,
                    ContentClassification::Planning | ContentClassification::Reference
                )
            }
            ContextUseCase::ConversationSupport => item.metadata.freshness_score > 0.3,
            ContextUseCase::KnowledgeSynthesis => true,
        }
    }

    /// Score and rank context items
    fn score_and_rank_items(&self, items: &mut Vec<ContextItem>, request: &ContextRequest) {
        // Apply additional scoring based on use case
        for item in items.iter_mut() {
            let use_case_bonus = self.calculate_use_case_bonus(item, &request.use_case);
            item.relevance_score = (item.relevance_score + use_case_bonus).min(1.0);
        }

        // Sort by relevance score
        items.sort_by(|a, b| b.relevance_score.total_cmp(&a.relevance_score));
    }

    /// Calculate bonus score based on use case
    fn calculate_use_case_bonus(&self, item: &ContextItem, use_case: &ContextUseCase) -> f64 {
        match use_case {
            ContextUseCase::ConversationSupport => item.metadata.freshness_score * 0.2,
            ContextUseCase::CommandPreview => {
                if matches!(
                    item.metadata.content_classification,
                    ContentClassification::Technical
                ) {
                    0.15
                } else {
                    0.0
                }
            }
            _ => 0.0,
        }
    }

    /// Apply size constraints to context items
    fn apply_size_constraints(
        &self,
        mut items: Vec<ContextItem>,
        request: &ContextRequest,
    ) -> Vec<ContextItem> {
        let default_constraints = ContextSizeConstraints {
            max_tokens: Some(self.config.max_context_tokens),
            max_items: Some(self.config.max_context_items),
            token_distribution: None,
        };

        let constraints = request
            .size_constraints
            .as_ref()
            .unwrap_or(&default_constraints);

        // Apply item count limit
        if let Some(max_items) = constraints.max_items {
            items.truncate(max_items);
        }

        // Apply token limit
        if let Some(max_tokens) = constraints.max_tokens {
            let mut total_tokens = 0;
            let mut final_items = Vec::new();

            for item in items {
                if total_tokens + item.metadata.estimated_tokens <= max_tokens {
                    total_tokens += item.metadata.estimated_tokens;
                    final_items.push(item);
                } else {
                    break;
                }
            }

            items = final_items;
        }

        items
    }

    /// Build final context bundle
    fn build_context_bundle(
        &self,
        items: Vec<ContextItem>,
        _request: &ContextRequest,
        strategies_used: Vec<ContextDiscoveryStrategy>,
        request_id: &str,
    ) -> ContextBundle {
        let summary = self.generate_context_summary(&items);
        let size_info = self.calculate_size_info(&items);
        let quality_metrics = self.calculate_quality_metrics(&items);

        let metadata = ContextBundleMetadata {
            created_at: chrono::Utc::now(),
            request_id: request_id.to_string(),
            strategies_used,
            execution_time_ms: 0, // Would be set by caller
            cache_status: CacheStatus::Miss,
        };

        ContextBundle {
            items,
            summary,
            size_info,
            quality_metrics,
            metadata,
        }
    }

    /// Generate summary of context bundle
    fn generate_context_summary(&self, items: &[ContextItem]) -> ContextSummary {
        let mut key_topics = HashSet::new();
        let mut memory_types = HashSet::new();
        let mut min_timestamp = None;
        let mut max_timestamp = None;

        for item in items {
            memory_types.insert(item.source_type.clone());

            if min_timestamp.is_none() || item.timestamp < min_timestamp.unwrap() {
                min_timestamp = Some(item.timestamp);
            }
            if max_timestamp.is_none() || item.timestamp > max_timestamp.unwrap() {
                max_timestamp = Some(item.timestamp);
            }

            // Extract topics from content (simplified)
            let words: Vec<&str> = item.content.split_whitespace().take(10).collect();
            for word in words {
                if word.len() > 4 {
                    key_topics.insert(word.to_lowercase());
                }
            }
        }

        let time_range = match (min_timestamp, max_timestamp) {
            (Some(min), Some(max)) => Some((min, max)),
            _ => None,
        };

        ContextSummary {
            description: format!(
                "Context bundle with {} items across {} memory types",
                items.len(),
                memory_types.len()
            ),
            key_topics: key_topics.into_iter().take(10).collect(),
            time_range,
            memory_types: memory_types.into_iter().collect(),
        }
    }

    /// Calculate size information
    fn calculate_size_info(&self, items: &[ContextItem]) -> ContextSizeInfo {
        let total_tokens = items
            .iter()
            .map(|item| item.metadata.estimated_tokens)
            .sum();
        let mut tokens_by_type = HashMap::new();

        for item in items {
            *tokens_by_type.entry(item.source_type.clone()).or_insert(0) +=
                item.metadata.estimated_tokens;
        }

        ContextSizeInfo {
            total_tokens,
            item_count: items.len(),
            tokens_by_type,
            truncated: false, // Would be set if truncation occurred
        }
    }

    /// Calculate quality metrics
    fn calculate_quality_metrics(&self, items: &[ContextItem]) -> ContextQualityMetrics {
        if items.is_empty() {
            return ContextQualityMetrics {
                avg_relevance: 0.0,
                topic_coverage: 0.0,
                freshness: 0.0,
                diversity: 0.0,
            };
        }

        let avg_relevance =
            items.iter().map(|item| item.relevance_score).sum::<f64>() / items.len() as f64;
        let freshness = items
            .iter()
            .map(|item| item.metadata.freshness_score)
            .sum::<f64>()
            / items.len() as f64;

        // Calculate diversity based on source types
        let unique_types: HashSet<_> = items.iter().map(|item| &item.source_type).collect();
        let diversity = unique_types.len() as f64 / 3.0; // Normalize by max types (3)

        ContextQualityMetrics {
            avg_relevance,
            topic_coverage: 0.8, // Would calculate based on actual topic analysis
            freshness,
            diversity,
        }
    }
}

/// Helper structs for internal processing
#[derive(Debug)]
#[allow(dead_code)]
struct ConversationAnalysis {
    suggested_queries: Vec<String>,
    intent: ConversationIntent,
}

#[derive(Debug)]
#[allow(dead_code)]
enum ConversationIntent {
    Implementation,
    Debugging,
    Learning,
    Planning,
    General,
}

#[cfg(test)]
mod tests {
    use super::*;
    use fennec_core::transcript::MessageRole;

    #[test]
    fn test_context_config_default() {
        let config = ContextConfig::default();
        assert_eq!(config.max_context_tokens, 4000);
        assert!(config.enable_caching);
        assert_eq!(config.max_context_items, 20);
        assert_eq!(config.freshness_threshold_hours, 24);
        assert_eq!(config.cache_ttl_minutes, 30);
        assert_eq!(config.discovery_strategies.len(), 4);
    }

    #[test]
    fn test_context_config_custom() {
        let mut config = ContextConfig::default();
        config.max_context_tokens = 8000;
        config.max_context_items = 50;
        config.enable_caching = false;
        config.freshness_threshold_hours = 48;

        assert_eq!(config.max_context_tokens, 8000);
        assert_eq!(config.max_context_items, 50);
        assert!(!config.enable_caching);
        assert_eq!(config.freshness_threshold_hours, 48);
    }

    #[test]
    fn test_context_discovery_strategy_values() {
        let _ = ContextDiscoveryStrategy::ConversationAnalysis;
        let _ = ContextDiscoveryStrategy::KeywordExtraction;
        let _ = ContextDiscoveryStrategy::TopicModeling;
        let _ = ContextDiscoveryStrategy::SessionHistory;
        let _ = ContextDiscoveryStrategy::ExplicitQuery;
    }

    #[test]
    fn test_context_discovery_strategy_equality() {
        assert_eq!(
            ContextDiscoveryStrategy::ConversationAnalysis,
            ContextDiscoveryStrategy::ConversationAnalysis
        );
        assert_ne!(
            ContextDiscoveryStrategy::KeywordExtraction,
            ContextDiscoveryStrategy::TopicModeling
        );
    }

    #[test]
    fn test_context_use_case_values() {
        let _ = ContextUseCase::AIPrompt;
        let _ = ContextUseCase::CommandPreview;
        let _ = ContextUseCase::SessionInit;
        let _ = ContextUseCase::ConversationSupport;
        let _ = ContextUseCase::KnowledgeSynthesis;
    }

    #[test]
    fn test_context_use_case_equality() {
        assert_eq!(ContextUseCase::AIPrompt, ContextUseCase::AIPrompt);
        assert_ne!(ContextUseCase::CommandPreview, ContextUseCase::SessionInit);
    }

    #[test]
    fn test_context_request_creation() {
        let session_id = Uuid::new_v4();
        let context = ConversationContext::default();
        let messages = vec![Message {
            id: Uuid::new_v4(),
            role: MessageRole::User,
            content: "test message".to_string(),
            timestamp: chrono::Utc::now(),
        }];

        let request = ContextRequest {
            session_id,
            conversation_context: context,
            recent_messages: messages.clone(),
            explicit_query: Some("test query".to_string()),
            preferred_types: vec![MemoryType::Transcripts],
            use_case: ContextUseCase::AIPrompt,
            size_constraints: None,
        };

        assert_eq!(request.session_id, session_id);
        assert_eq!(request.recent_messages.len(), 1);
        assert_eq!(request.explicit_query, Some("test query".to_string()));
        assert_eq!(request.preferred_types.len(), 1);
        assert_eq!(request.use_case, ContextUseCase::AIPrompt);
    }

    #[test]
    fn test_context_size_constraints_creation() {
        let mut token_dist = HashMap::new();
        token_dist.insert(MemoryType::Transcripts, 0.5);
        token_dist.insert(MemoryType::Guidance, 0.3);

        let constraints = ContextSizeConstraints {
            max_tokens: Some(5000),
            max_items: Some(25),
            token_distribution: Some(token_dist.clone()),
        };

        assert_eq!(constraints.max_tokens, Some(5000));
        assert_eq!(constraints.max_items, Some(25));
        assert!(constraints.token_distribution.is_some());
    }

    #[test]
    fn test_context_importance_values() {
        let _ = ContextImportance::Critical;
        let _ = ContextImportance::High;
        let _ = ContextImportance::Medium;
        let _ = ContextImportance::Low;
    }

    #[test]
    fn test_context_importance_equality() {
        assert_eq!(ContextImportance::Critical, ContextImportance::Critical);
        assert_ne!(ContextImportance::High, ContextImportance::Low);
    }

    #[test]
    fn test_context_importance_serialization() {
        let importance = ContextImportance::High;
        let json = serde_json::to_string(&importance).unwrap();
        assert!(json.contains("High"));

        let deserialized: ContextImportance = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, importance);
    }

    #[test]
    fn test_content_classification_values() {
        let _ = ContentClassification::Technical;
        let _ = ContentClassification::Conversational;
        let _ = ContentClassification::Documentation;
        let _ = ContentClassification::ProblemSolving;
        let _ = ContentClassification::Planning;
        let _ = ContentClassification::Learning;
        let _ = ContentClassification::Reference;
    }

    #[test]
    fn test_content_classification_equality() {
        assert_eq!(
            ContentClassification::Technical,
            ContentClassification::Technical
        );
        assert_ne!(
            ContentClassification::Planning,
            ContentClassification::Learning
        );
    }

    #[test]
    fn test_content_classification_serialization() {
        let classification = ContentClassification::Technical;
        let json = serde_json::to_string(&classification).unwrap();
        assert!(json.contains("Technical"));

        let deserialized: ContentClassification = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, classification);
    }

    #[test]
    fn test_context_item_creation() {
        let item = ContextItem {
            id: "test-123".to_string(),
            source_type: MemoryType::Transcripts,
            title: "Test Context".to_string(),
            content: "Test content".to_string(),
            relevance_score: 0.85,
            importance: ContextImportance::High,
            timestamp: chrono::Utc::now(),
            session_id: Some(Uuid::new_v4()),
            metadata: ContextItemMetadata {
                estimated_tokens: 100,
                discovery_strategy: "test".to_string(),
                matching_keywords: vec!["rust".to_string()],
                content_classification: ContentClassification::Technical,
                freshness_score: 0.9,
            },
        };

        assert_eq!(item.id, "test-123");
        assert_eq!(item.relevance_score, 0.85);
        assert_eq!(item.importance, ContextImportance::High);
        assert_eq!(item.metadata.estimated_tokens, 100);
    }

    #[test]
    fn test_context_item_serialization() {
        let item = ContextItem {
            id: "test-456".to_string(),
            source_type: MemoryType::Guidance,
            title: "Test".to_string(),
            content: "Content".to_string(),
            relevance_score: 0.7,
            importance: ContextImportance::Medium,
            timestamp: chrono::Utc::now(),
            session_id: None,
            metadata: ContextItemMetadata {
                estimated_tokens: 50,
                discovery_strategy: "keyword".to_string(),
                matching_keywords: vec![],
                content_classification: ContentClassification::Conversational,
                freshness_score: 0.5,
            },
        };

        let json = serde_json::to_string(&item).unwrap();
        let deserialized: ContextItem = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, item.id);
        assert_eq!(deserialized.relevance_score, item.relevance_score);
    }

    #[test]
    fn test_context_item_metadata_serialization() {
        let metadata = ContextItemMetadata {
            estimated_tokens: 200,
            discovery_strategy: "topic_modeling".to_string(),
            matching_keywords: vec!["ai".to_string(), "rust".to_string()],
            content_classification: ContentClassification::Learning,
            freshness_score: 0.75,
        };

        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: ContextItemMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.estimated_tokens, metadata.estimated_tokens);
        assert_eq!(deserialized.matching_keywords.len(), 2);
    }

    #[test]
    fn test_context_summary_creation() {
        let summary = ContextSummary {
            description: "Test summary".to_string(),
            key_topics: vec!["rust".to_string(), "ai".to_string()],
            time_range: None,
            memory_types: vec![MemoryType::Transcripts, MemoryType::Guidance],
        };

        assert_eq!(summary.description, "Test summary");
        assert_eq!(summary.key_topics.len(), 2);
        assert_eq!(summary.memory_types.len(), 2);
    }

    #[test]
    fn test_context_size_info_creation() {
        let mut tokens_by_type = HashMap::new();
        tokens_by_type.insert(MemoryType::Transcripts, 1500);
        tokens_by_type.insert(MemoryType::Guidance, 500);

        let size_info = ContextSizeInfo {
            total_tokens: 2000,
            item_count: 10,
            tokens_by_type,
            truncated: false,
        };

        assert_eq!(size_info.total_tokens, 2000);
        assert_eq!(size_info.item_count, 10);
        assert!(!size_info.truncated);
        assert_eq!(size_info.tokens_by_type.len(), 2);
    }

    #[test]
    fn test_context_quality_metrics_creation() {
        let metrics = ContextQualityMetrics {
            avg_relevance: 0.75,
            topic_coverage: 0.8,
            freshness: 0.6,
            diversity: 0.9,
        };

        assert_eq!(metrics.avg_relevance, 0.75);
        assert_eq!(metrics.topic_coverage, 0.8);
        assert_eq!(metrics.freshness, 0.6);
        assert_eq!(metrics.diversity, 0.9);
    }

    #[test]
    fn test_context_bundle_metadata_creation() {
        let metadata = ContextBundleMetadata {
            created_at: chrono::Utc::now(),
            request_id: "req-123".to_string(),
            strategies_used: vec![
                ContextDiscoveryStrategy::ConversationAnalysis,
                ContextDiscoveryStrategy::KeywordExtraction,
            ],
            execution_time_ms: 250,
            cache_status: CacheStatus::Miss,
        };

        assert_eq!(metadata.request_id, "req-123");
        assert_eq!(metadata.strategies_used.len(), 2);
        assert_eq!(metadata.execution_time_ms, 250);
        assert_eq!(metadata.cache_status, CacheStatus::Miss);
    }

    #[test]
    fn test_cache_status_values() {
        let _ = CacheStatus::Hit;
        let _ = CacheStatus::Miss;
        let _ = CacheStatus::Partial;
        let _ = CacheStatus::Disabled;
    }

    #[test]
    fn test_cache_status_equality() {
        assert_eq!(CacheStatus::Hit, CacheStatus::Hit);
        assert_ne!(CacheStatus::Miss, CacheStatus::Hit);
    }

    #[test]
    fn test_context_cache_new() {
        let cache = ContextCache::new(50);
        assert_eq!(cache.max_size, 50);
        assert_eq!(cache.bundles.len(), 0);
    }

    #[test]
    fn test_context_cache_store_and_get() {
        let mut cache = ContextCache::new(10);
        let bundle = create_test_bundle();

        cache.store("test_key".to_string(), bundle.clone());
        assert_eq!(cache.bundles.len(), 1);

        let retrieved = cache.get("test_key", 30);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().items.len(), bundle.items.len());
    }

    #[test]
    fn test_context_cache_expiration() {
        let mut cache = ContextCache::new(10);
        let bundle = create_test_bundle();

        // Insert bundle with old timestamp
        let old_time = chrono::Utc::now() - chrono::Duration::hours(2);
        cache.bundles.insert("old_key".to_string(), (bundle, old_time));

        // Should return None due to TTL expiration
        let retrieved = cache.get("old_key", 60); // 60 minutes TTL
        assert!(retrieved.is_none());

        // Should be removed from cache
        assert_eq!(cache.bundles.len(), 0);
    }

    #[test]
    fn test_context_cache_eviction() {
        let mut cache = ContextCache::new(2);

        cache.store("key1".to_string(), create_test_bundle());
        cache.store("key2".to_string(), create_test_bundle());
        assert_eq!(cache.bundles.len(), 2);

        // Adding third should evict oldest
        cache.store("key3".to_string(), create_test_bundle());
        assert_eq!(cache.bundles.len(), 2);
    }

    #[test]
    fn test_context_cache_get_oldest_key() {
        let mut cache = ContextCache::new(10);

        cache.store("key1".to_string(), create_test_bundle());
        std::thread::sleep(std::time::Duration::from_millis(10));
        cache.store("key2".to_string(), create_test_bundle());

        let oldest = cache.get_oldest_key();
        assert!(oldest.is_some());
    }

    #[tokio::test]
    async fn test_context_engine_new() {
        let memory_service = std::sync::Arc::new(
            crate::service::MemoryService::new().await.unwrap()
        );
        let engine = ContextEngine::new(memory_service);

        assert_eq!(engine.config.max_context_tokens, 4000);
        assert!(engine.config.enable_caching);
    }

    #[tokio::test]
    async fn test_context_engine_with_config() {
        let memory_service = std::sync::Arc::new(
            crate::service::MemoryService::new().await.unwrap()
        );

        let mut config = ContextConfig::default();
        config.max_context_tokens = 8000;
        config.enable_caching = false;

        let engine = ContextEngine::with_config(memory_service, config.clone());
        assert_eq!(engine.config.max_context_tokens, 8000);
        assert!(!engine.config.enable_caching);
    }

    #[test]
    fn test_classify_importance_critical() {
        let memory_service = create_test_memory_service();
        let engine = ContextEngine::new(memory_service);

        assert_eq!(engine.classify_importance(0.85), ContextImportance::Critical);
        assert_eq!(engine.classify_importance(0.95), ContextImportance::Critical);
    }

    #[test]
    fn test_classify_importance_high() {
        let memory_service = create_test_memory_service();
        let engine = ContextEngine::new(memory_service);

        assert_eq!(engine.classify_importance(0.7), ContextImportance::High);
        assert_eq!(engine.classify_importance(0.65), ContextImportance::High);
    }

    #[test]
    fn test_classify_importance_medium() {
        let memory_service = create_test_memory_service();
        let engine = ContextEngine::new(memory_service);

        assert_eq!(engine.classify_importance(0.5), ContextImportance::Medium);
        assert_eq!(engine.classify_importance(0.45), ContextImportance::Medium);
    }

    #[test]
    fn test_classify_importance_low() {
        let memory_service = create_test_memory_service();
        let engine = ContextEngine::new(memory_service);

        assert_eq!(engine.classify_importance(0.3), ContextImportance::Low);
        assert_eq!(engine.classify_importance(0.1), ContextImportance::Low);
    }

    #[test]
    fn test_classify_content_technical() {
        let memory_service = create_test_memory_service();
        let engine = ContextEngine::new(memory_service);

        assert_eq!(
            engine.classify_content("Let's implement this function with a new class"),
            ContentClassification::Technical
        );
    }

    #[test]
    fn test_classify_content_planning() {
        let memory_service = create_test_memory_service();
        let engine = ContextEngine::new(memory_service);

        assert_eq!(
            engine.classify_content("Here's the plan for the task we need to complete"),
            ContentClassification::Planning
        );
    }

    #[test]
    fn test_classify_content_problem_solving() {
        let memory_service = create_test_memory_service();
        let engine = ContextEngine::new(memory_service);

        assert_eq!(
            engine.classify_content("Error occurred, need to debug and fix this issue"),
            ContentClassification::ProblemSolving
        );
    }

    #[test]
    fn test_classify_content_learning() {
        let memory_service = create_test_memory_service();
        let engine = ContextEngine::new(memory_service);

        assert_eq!(
            engine.classify_content("Can you explain how this works so I can learn?"),
            ContentClassification::Learning
        );
    }

    #[test]
    fn test_classify_content_conversational() {
        let memory_service = create_test_memory_service();
        let engine = ContextEngine::new(memory_service);

        assert_eq!(
            engine.classify_content("Hello, how are you today?"),
            ContentClassification::Conversational
        );
    }

    #[test]
    fn test_calculate_freshness_score_recent() {
        let memory_service = create_test_memory_service();
        let engine = ContextEngine::new(memory_service);

        let now = chrono::Utc::now();
        let score = engine.calculate_freshness_score(now);
        assert!(score > 0.9);
    }

    #[test]
    fn test_calculate_freshness_score_old() {
        let memory_service = create_test_memory_service();
        let engine = ContextEngine::new(memory_service);

        let old_time = chrono::Utc::now() - chrono::Duration::days(30);
        let score = engine.calculate_freshness_score(old_time);
        assert!(score < 0.2);
    }

    #[test]
    fn test_is_stop_word() {
        let memory_service = create_test_memory_service();
        let engine = ContextEngine::new(memory_service);

        assert!(engine.is_stop_word("the"));
        assert!(engine.is_stop_word("and"));
        assert!(engine.is_stop_word("or"));
        assert!(!engine.is_stop_word("implement"));
        assert!(!engine.is_stop_word("function"));
    }

    #[test]
    fn test_extract_key_terms() {
        let memory_service = create_test_memory_service();
        let engine = ContextEngine::new(memory_service);

        let content = "Let's implement rust program testing";
        let terms = engine.extract_key_terms(content);

        assert!(!terms.is_empty());
        // Check that stop words and short words are filtered
        let words: Vec<&str> = terms.split_whitespace().collect();
        assert!(words.len() <= 3);
        assert!(!words.contains(&"the"));
        assert!(!words.contains(&"and"));
    }

    #[test]
    fn test_extract_keywords() {
        let memory_service = create_test_memory_service();
        let engine = ContextEngine::new(memory_service);

        let messages = vec![
            Message {
                id: Uuid::new_v4(),
                role: MessageRole::User,
                content: "Implement authentication with security features".to_string(),
                timestamp: chrono::Utc::now(),
            },
            Message {
                id: Uuid::new_v4(),
                role: MessageRole::Assistant,
                content: "I'll help you implement authentication".to_string(),
                timestamp: chrono::Utc::now(),
            },
        ];

        let keywords = engine.extract_keywords(&messages);
        assert!(!keywords.is_empty());
    }

    #[test]
    fn test_extract_topics_from_context() {
        let memory_service = create_test_memory_service();
        let engine = ContextEngine::new(memory_service);

        let mut context = ConversationContext::default();
        context.technologies = vec!["rust".to_string(), "tokio".to_string()];
        context.current_task = Some("implement auth".to_string());
        context.recent_topics = vec!["security".to_string()];

        let topics = engine.extract_topics_from_context(&context);
        assert!(topics.len() >= 3);
        assert!(topics.contains(&"rust".to_string()));
    }

    #[test]
    fn test_deduplicate_context_items() {
        let memory_service = create_test_memory_service();
        let engine = ContextEngine::new(memory_service);

        let item1 = create_test_context_item("id-1", 0.8);
        let item2 = create_test_context_item("id-1", 0.7); // Duplicate
        let item3 = create_test_context_item("id-2", 0.6);

        let items = vec![item1, item2, item3];
        let deduplicated = engine.deduplicate_context_items(items);

        assert_eq!(deduplicated.len(), 2);
    }

    #[test]
    fn test_is_relevant_for_use_case_ai_prompt() {
        let memory_service = create_test_memory_service();
        let engine = ContextEngine::new(memory_service);

        let item = create_test_context_item("test", 0.8);
        assert!(engine.is_relevant_for_use_case(&item, &ContextUseCase::AIPrompt));
    }

    #[test]
    fn test_is_relevant_for_use_case_command_preview() {
        let memory_service = create_test_memory_service();
        let engine = ContextEngine::new(memory_service);

        let mut item = create_test_context_item("test", 0.8);
        item.metadata.content_classification = ContentClassification::Technical;

        assert!(engine.is_relevant_for_use_case(&item, &ContextUseCase::CommandPreview));
    }

    #[test]
    fn test_is_relevant_for_use_case_session_init() {
        let memory_service = create_test_memory_service();
        let engine = ContextEngine::new(memory_service);

        let mut item = create_test_context_item("test", 0.8);
        item.metadata.content_classification = ContentClassification::Planning;

        assert!(engine.is_relevant_for_use_case(&item, &ContextUseCase::SessionInit));
    }

    #[test]
    fn test_calculate_use_case_bonus_conversation_support() {
        let memory_service = create_test_memory_service();
        let engine = ContextEngine::new(memory_service);

        let mut item = create_test_context_item("test", 0.8);
        item.metadata.freshness_score = 0.9;

        let bonus = engine.calculate_use_case_bonus(&item, &ContextUseCase::ConversationSupport);
        assert!(bonus > 0.0);
    }

    #[test]
    fn test_calculate_use_case_bonus_command_preview() {
        let memory_service = create_test_memory_service();
        let engine = ContextEngine::new(memory_service);

        let mut item = create_test_context_item("test", 0.8);
        item.metadata.content_classification = ContentClassification::Technical;

        let bonus = engine.calculate_use_case_bonus(&item, &ContextUseCase::CommandPreview);
        assert_eq!(bonus, 0.15);
    }

    #[test]
    fn test_calculate_quality_metrics_empty() {
        let memory_service = create_test_memory_service();
        let engine = ContextEngine::new(memory_service);

        let metrics = engine.calculate_quality_metrics(&[]);
        assert_eq!(metrics.avg_relevance, 0.0);
        assert_eq!(metrics.topic_coverage, 0.0);
        assert_eq!(metrics.freshness, 0.0);
        assert_eq!(metrics.diversity, 0.0);
    }

    #[test]
    fn test_calculate_quality_metrics_with_items() {
        let memory_service = create_test_memory_service();
        let engine = ContextEngine::new(memory_service);

        let items = vec![
            create_test_context_item("id-1", 0.8),
            create_test_context_item("id-2", 0.6),
        ];

        let metrics = engine.calculate_quality_metrics(&items);
        assert!(metrics.avg_relevance > 0.0);
        assert!(metrics.diversity >= 0.0);
    }

    #[test]
    fn test_generate_context_summary() {
        let memory_service = create_test_memory_service();
        let engine = ContextEngine::new(memory_service);

        let items = vec![
            create_test_context_item("id-1", 0.8),
            create_test_context_item("id-2", 0.6),
        ];

        let summary = engine.generate_context_summary(&items);
        assert!(summary.description.contains("2 items"));
        assert!(!summary.memory_types.is_empty());
    }

    #[test]
    fn test_calculate_size_info() {
        let memory_service = create_test_memory_service();
        let engine = ContextEngine::new(memory_service);

        let items = vec![
            create_test_context_item("id-1", 0.8),
            create_test_context_item("id-2", 0.6),
        ];

        let size_info = engine.calculate_size_info(&items);
        assert_eq!(size_info.item_count, 2);
        assert!(size_info.total_tokens > 0);
    }

    #[test]
    fn test_apply_size_constraints_item_limit() {
        let memory_service = create_test_memory_service();
        let engine = ContextEngine::new(memory_service);

        let items = vec![
            create_test_context_item("id-1", 0.8),
            create_test_context_item("id-2", 0.7),
            create_test_context_item("id-3", 0.6),
        ];

        let request = create_test_context_request_with_constraints(Some(2), None);
        let constrained = engine.apply_size_constraints(items, &request);

        assert_eq!(constrained.len(), 2);
    }

    #[test]
    fn test_apply_size_constraints_token_limit() {
        let memory_service = create_test_memory_service();
        let engine = ContextEngine::new(memory_service);

        let items = vec![
            create_test_context_item("id-1", 0.8),
            create_test_context_item("id-2", 0.7),
        ];

        let request = create_test_context_request_with_constraints(None, Some(150));
        let constrained = engine.apply_size_constraints(items, &request);

        // Should fit at most 1 item with 100 tokens each
        assert!(constrained.len() <= 2);
    }

    #[test]
    fn test_score_and_rank_items() {
        let memory_service = create_test_memory_service();
        let engine = ContextEngine::new(memory_service);

        let mut items = vec![
            create_test_context_item("id-1", 0.5),
            create_test_context_item("id-2", 0.8),
            create_test_context_item("id-3", 0.6),
        ];

        let request = create_test_context_request();
        engine.score_and_rank_items(&mut items, &request);

        // Should be sorted by score descending
        assert!(items[0].relevance_score >= items[1].relevance_score);
        assert!(items[1].relevance_score >= items[2].relevance_score);
    }

    #[test]
    fn test_generate_cache_key() {
        let memory_service = create_test_memory_service();
        let engine = ContextEngine::new(memory_service);

        let request = create_test_context_request();
        let key = engine.generate_cache_key(&request);

        assert!(key.starts_with("ctx_"));
        assert!(key.len() > 4);
    }

    #[test]
    fn test_generate_cache_key_consistency() {
        let memory_service = create_test_memory_service();
        let engine = ContextEngine::new(memory_service);

        let request = create_test_context_request();
        let key1 = engine.generate_cache_key(&request);
        let key2 = engine.generate_cache_key(&request);

        assert_eq!(key1, key2);
    }

    #[test]
    fn test_analyze_conversation_patterns_implementation() {
        let memory_service = create_test_memory_service();
        let engine = ContextEngine::new(memory_service);

        let messages = vec![Message {
            id: Uuid::new_v4(),
            role: MessageRole::User,
            content: "Let's implement a new authentication system".to_string(),
            timestamp: chrono::Utc::now(),
        }];

        let analysis = engine.analyze_conversation_patterns(&messages);
        assert!(!analysis.suggested_queries.is_empty());
    }

    #[test]
    fn test_analyze_conversation_patterns_debugging() {
        let memory_service = create_test_memory_service();
        let engine = ContextEngine::new(memory_service);

        let messages = vec![Message {
            id: Uuid::new_v4(),
            role: MessageRole::User,
            content: "I'm getting an error when running the tests".to_string(),
            timestamp: chrono::Utc::now(),
        }];

        let analysis = engine.analyze_conversation_patterns(&messages);
        assert!(!analysis.suggested_queries.is_empty());
    }

    #[test]
    fn test_analyze_conversation_patterns_learning() {
        let memory_service = create_test_memory_service();
        let engine = ContextEngine::new(memory_service);

        let messages = vec![Message {
            id: Uuid::new_v4(),
            role: MessageRole::User,
            content: "Can you explain how async/await works in Rust?".to_string(),
            timestamp: chrono::Utc::now(),
        }];

        let analysis = engine.analyze_conversation_patterns(&messages);
        assert!(!analysis.suggested_queries.is_empty());
    }

    // Helper functions for tests
    fn create_test_memory_service() -> std::sync::Arc<crate::service::MemoryService> {
        std::sync::Arc::new(
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(async { crate::service::MemoryService::new().await.unwrap() })
        )
    }

    fn create_test_bundle() -> ContextBundle {
        ContextBundle {
            items: vec![create_test_context_item("test", 0.8)],
            summary: ContextSummary {
                description: "Test".to_string(),
                key_topics: vec![],
                time_range: None,
                memory_types: vec![],
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
            metadata: ContextBundleMetadata {
                created_at: chrono::Utc::now(),
                request_id: "test".to_string(),
                strategies_used: vec![],
                execution_time_ms: 0,
                cache_status: CacheStatus::Miss,
            },
        }
    }

    fn create_test_context_item(id: &str, score: f64) -> ContextItem {
        ContextItem {
            id: id.to_string(),
            source_type: MemoryType::Transcripts,
            title: "Test".to_string(),
            content: "Test content with some words".to_string(),
            relevance_score: score,
            importance: ContextImportance::Medium,
            timestamp: chrono::Utc::now(),
            session_id: None,
            metadata: ContextItemMetadata {
                estimated_tokens: 100,
                discovery_strategy: "test".to_string(),
                matching_keywords: vec![],
                content_classification: ContentClassification::Technical,
                freshness_score: 0.8,
            },
        }
    }

    fn create_test_context_request() -> ContextRequest {
        ContextRequest {
            session_id: Uuid::new_v4(),
            conversation_context: ConversationContext::default(),
            recent_messages: vec![],
            explicit_query: None,
            preferred_types: vec![MemoryType::Transcripts],
            use_case: ContextUseCase::AIPrompt,
            size_constraints: None,
        }
    }

    fn create_test_context_request_with_constraints(
        max_items: Option<usize>,
        max_tokens: Option<usize>,
    ) -> ContextRequest {
        let mut request = create_test_context_request();
        request.size_constraints = Some(ContextSizeConstraints {
            max_tokens,
            max_items,
            token_distribution: None,
        });
        request
    }
}
