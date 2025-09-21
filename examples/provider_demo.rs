/*!
 * Fennec Provider Demo
 * 
 * This example demonstrates how to use the Fennec provider system
 * for AI conversations with proper error handling and logging.
 * 
 * To run this example:
 * ```bash
 * export OPENAI_API_KEY="your-api-key-here"
 * cargo run --example provider_demo
 * ```
 */

use fennec_core::{
    config::{Config, ProviderConfig, SecurityConfig, MemoryConfig, TuiConfig, KeyBindings},
    Result,
};
use fennec_orchestration::SessionManager;
use fennec_provider::ProviderClientFactory;
use fennec_security::audit::AuditLogger;
use futures::StreamExt;
use std::{env, path::PathBuf};
use tokio;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info,fennec_provider=debug,fennec_orchestration=debug")
        .init();

    println!("🚀 Fennec Provider Demo");
    println!("=======================");

    // Check for API key
    if env::var("OPENAI_API_KEY").is_err() {
        eprintln!("❌ Error: OPENAI_API_KEY environment variable is required");
        eprintln!("Please set your OpenAI API key and try again:");
        eprintln!("export OPENAI_API_KEY=\"your-api-key-here\"");
        std::process::exit(1);
    }

    // Create configuration
    let config = create_demo_config();
    println!("✅ Configuration loaded");

    // Validate provider configuration
    ProviderClientFactory::validate_config(&config.provider)?;
    println!("✅ Provider configuration validated");

    // Create audit logger
    let audit_logger = AuditLogger::new(&config).await?;
    println!("✅ Audit logger initialized");

    // Create session manager
    let session_manager = SessionManager::new(config, audit_logger).await?;
    println!("✅ Session manager created");

    // Start a conversation session
    let session_id = session_manager.start_session().await?;
    println!("✅ Session started: {}", session_id);

    println!("\n🗣️  Starting AI Conversation Demo");
    println!("=====================================");

    // Demo 1: Simple completion
    println!("\n1️⃣  Simple Completion Demo");
    println!("-------------------------");
    let question = "What are the three main benefits of using Rust for systems programming?";
    println!("Question: {}", question);
    
    print!("Answer: ");
    let answer = session_manager.send_message(question.to_string()).await?;
    println!("{}", answer);

    // Demo 2: Streaming completion
    println!("\n2️⃣  Streaming Completion Demo");
    println!("-----------------------------");
    let stream_question = "Please count from 1 to 5, explaining what each number represents in programming (like 1 for arrays starting at index 1 in some languages).";
    println!("Question: {}", stream_question);
    
    print!("Streaming Answer: ");
    let mut stream = session_manager.send_message_stream(stream_question.to_string()).await?;
    
    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => print!("{}", chunk),
            Err(e) => {
                eprintln!("\nStream error: {}", e);
                break;
            }
        }
    }
    println!(); // New line after streaming

    // Demo 3: Conversation context
    println!("\n3️⃣  Conversation Context Demo");
    println!("-----------------------------");
    let followup = "Based on your previous answer about Rust, which of those benefits is most important for web development?";
    println!("Follow-up: {}", followup);
    
    print!("Answer: ");
    let followup_answer = session_manager.send_message(followup.to_string()).await?;
    println!("{}", followup_answer);

    // Demo 4: Conversation statistics
    println!("\n4️⃣  Conversation Statistics");
    println!("---------------------------");
    if let Some(stats) = session_manager.conversation_stats().await {
        println!("📊 Conversation Stats:");
        println!("   Total messages: {}", stats.total_messages);
        println!("   User messages: {}", stats.user_messages);
        println!("   Assistant messages: {}", stats.assistant_messages);
        println!("   Total characters: {}", stats.total_characters);
    }

    // Demo 5: Error handling
    println!("\n5️⃣  Error Handling Demo");
    println!("------------------------");
    // This will demonstrate how errors are handled gracefully
    let very_long_message = "A".repeat(100_000); // Very long message to potentially trigger limits
    match session_manager.send_message(very_long_message).await {
        Ok(response) => println!("Unexpected success with very long message: {}", response.chars().take(100).collect::<String>()),
        Err(e) => println!("Expected error with very long message: {}", e),
    }

    // Clean up
    println!("\n🧹 Cleanup");
    println!("----------");
    session_manager.end_session().await?;
    println!("✅ Session ended");

    println!("\n🎉 Demo completed successfully!");
    println!("Check the audit log at: {}", 
        if let Some(path) = &session_manager.current_transcript().await {
            format!(".fennec/audit.jsonl")
        } else {
            "default location".to_string()
        });

    Ok(())
}

fn create_demo_config() -> Config {
    Config {
        provider: ProviderConfig {
            openai_api_key: env::var("OPENAI_API_KEY").ok(),
            default_model: "gpt-3.5-turbo".to_string(),
            base_url: None,
            timeout_seconds: 30,
        },
        security: SecurityConfig {
            default_sandbox_level: "workspace-write".to_string(),
            audit_log_enabled: true,
            audit_log_path: Some(PathBuf::from(".fennec/audit.jsonl")),
        },
        memory: MemoryConfig {
            storage_path: PathBuf::from(".fennec"),
            max_transcript_size: 10_000,
            enable_agents_md: true,
        },
        tui: TuiConfig {
            theme: "default".to_string(),
            key_bindings: KeyBindings {
                quit: "Ctrl+C".to_string(),
                help: "F1".to_string(),
                clear: "Ctrl+L".to_string(),
            },
        },
    }
}