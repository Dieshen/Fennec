//! # Fennec Memory Service Demo
//!
//! This example demonstrates the core functionality of the Fennec memory service,
//! including AGENTS.md loading, transcript management, and search capabilities.

use anyhow::Result;
use fennec_core::{session::Session, transcript::MessageRole};
use fennec_memory::{MemoryService, MemoryConfig};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    println!("🦊 Fennec Memory Service Demo");
    println!("=====================================\n");

    // Create memory service with custom configuration
    let config = MemoryConfig {
        max_messages_in_memory: 500,
        auto_generate_summaries: true,
        guidance_context_window: 30,
        max_search_results: 5,
    };
    
    let memory = MemoryService::with_config(config).await?;
    
    // 1. Check AGENTS.md configuration
    println!("📋 1. Checking AGENTS.md Configuration");
    println!("---------------------------------------");
    
    if let Some(agents_config) = memory.get_agents_config() {
        println!("✓ AGENTS.md loaded from: {}", agents_config.source_path.display());
        println!("✓ Available guidance sections: {}", agents_config.sections.len());
        
        // Show available guidance sections
        for section_title in memory.get_available_guidance() {
            println!("  - {}", section_title);
        }
    } else {
        println!("⚠ No AGENTS.md found (checked ~/.fennec/AGENTS.md and ./AGENTS.md)");
    }
    
    println!();

    // 2. Create and manage sessions
    println!("🗂️  2. Session Management");
    println!("-------------------------");
    
    let session1 = Session::new();
    let session2 = Session::new();
    
    println!("✓ Created session 1: {}", session1.id);
    println!("✓ Created session 2: {}", session2.id);
    
    // Start tracking sessions
    memory.start_session(session1.clone()).await?;
    memory.start_session(session2.clone()).await?;
    
    println!("✓ Started tracking both sessions\n");

    // 3. Add messages to conversations
    println!("💬 3. Adding Messages to Conversations");
    println!("--------------------------------------");
    
    // Session 1: Rust programming conversation
    memory.add_message(
        session1.id,
        MessageRole::User,
        "How do I implement async functions in Rust?".to_string()
    ).await?;
    
    memory.add_message(
        session1.id,
        MessageRole::Assistant,
        "To implement async functions in Rust, you use the `async` keyword...".to_string()
    ).await?;
    
    memory.add_message(
        session1.id,
        MessageRole::User,
        "Can you show me an example with error handling?".to_string()
    ).await?;
    
    println!("✓ Added 3 messages to session 1 (Rust async conversation)");
    
    // Session 2: TUI development conversation
    memory.add_message(
        session2.id,
        MessageRole::User,
        "I'm building a TUI application with ratatui. How do I handle keyboard input?".to_string()
    ).await?;
    
    memory.add_message(
        session2.id,
        MessageRole::Assistant,
        "For keyboard input in ratatui, you'll want to use crossterm events...".to_string()
    ).await?;
    
    println!("✓ Added 2 messages to session 2 (TUI development conversation)\n");

    // 4. Memory injection for AI prompts
    println!("🧠 4. Memory Injection for AI Prompts");
    println!("-------------------------------------");
    
    // Get memory injection for Rust-related query
    let injection = memory.get_memory_injection(session1.id, Some("rust async")).await?;
    
    println!("🔍 Query: 'rust async'");
    println!("📚 Guidance matches found: {}", injection.guidance.len());
    for (i, guidance) in injection.guidance.iter().enumerate() {
        println!("  {}. {} (score: {})", i + 1, guidance.section_title, guidance.score);
    }
    
    println!("💾 Conversation history matches: {}", injection.conversation_history.len());
    for (i, conv) in injection.conversation_history.iter().enumerate() {
        println!("  {}. Session {} ({} messages)", 
                 i + 1, conv.session_id, conv.matching_messages.len());
    }
    
    println!("🎯 Session context topics: {:?}", injection.session_context.recent_topics);
    println!("⚡ Estimated tokens: {}\n", injection.estimated_tokens);

    // 5. Search across all memory
    println!("🔎 5. Searching Across All Memory");
    println!("---------------------------------");
    
    let search_results = memory.search("TUI application", Some(3)).await?;
    
    println!("🔍 Query: 'TUI application'");
    println!("📚 Guidance matches: {}", search_results.guidance_matches.len());
    println!("💾 Transcript matches: {}", search_results.transcript_matches.len());
    
    if !search_results.transcript_matches.is_empty() {
        println!("📝 Found conversations about TUI:");
        for result in &search_results.transcript_matches {
            println!("  - Session {}: {} messages (score: {})", 
                     result.session_id, result.matching_messages.len(), result.score);
        }
    }
    
    println!();

    // 6. List stored sessions
    println!("📊 6. Session Summary");
    println!("--------------------");
    
    let sessions = memory.list_sessions().await?;
    println!("📁 Total stored sessions: {}", sessions.len());
    
    for session_meta in sessions {
        println!("  🗂️  Session {}", session_meta.session_id);
        println!("      Messages: {}", session_meta.message_count);
        println!("      Tokens: ~{}", session_meta.estimated_tokens);
        println!("      Last updated: {}", session_meta.updated_at.format("%Y-%m-%d %H:%M:%S"));
        println!("      Active: {}", session_meta.is_active);
        println!();
    }

    // 7. Add tags and summary
    println!("🏷️  7. Adding Tags and Summary");
    println!("------------------------------");
    
    memory.add_session_tags(
        session1.id, 
        vec!["rust".to_string(), "async".to_string(), "programming".to_string()]
    ).await?;
    
    memory.set_session_summary(
        session1.id,
        "Conversation about implementing async functions in Rust with error handling examples".to_string()
    ).await?;
    
    println!("✓ Added tags and summary to session 1");
    
    memory.add_session_tags(
        session2.id,
        vec!["tui".to_string(), "ratatui".to_string(), "ui".to_string()]
    ).await?;
    
    println!("✓ Added tags to session 2\n");

    // 8. Clean up
    println!("🧹 8. Cleanup");
    println!("-------------");
    
    memory.stop_session(session1.id).await?;
    memory.stop_session(session2.id).await?;
    
    println!("✓ Stopped tracking sessions (data persisted to disk)");
    
    println!("\n🎉 Demo completed successfully!");
    println!("💾 All conversation data has been persisted and can be retrieved in future sessions.");
    println!("📁 Check ~/.local/share/fennec/transcripts/ for stored conversation files.");
    
    Ok(())
}