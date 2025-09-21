/*!
 * Integration Test for Fennec Provider System
 * 
 * This test demonstrates the complete provider abstraction layer working
 * with the SessionManager and core types.
 * 
 * To run this test with a real OpenAI API key:
 * ```bash
 * export OPENAI_API_KEY="your-api-key-here"
 * cargo test --test integration_test -- --nocapture
 * ```
 */

use fennec_core::{
    config::{Config, ProviderConfig, SecurityConfig, MemoryConfig, TuiConfig, KeyBindings},
    Result,
};
use fennec_orchestration::{SessionManager, ConversationStats};
use fennec_provider::{ProviderClientFactory, OpenAIClient, OpenAIConfig};
use fennec_security::audit::AuditLogger;
use futures::StreamExt;
use std::{env, path::PathBuf};
use tempfile::TempDir;
use tokio;
use tracing_subscriber;

#[tokio::test]
async fn test_full_provider_integration() -> Result<()> {
    // Initialize logging
    let _ = tracing_subscriber::fmt::try_init();

    println!("🚀 Starting Fennec Provider Integration Test");

    // Create temporary directory for test files
    let temp_dir = TempDir::new().unwrap();
    let audit_log_path = temp_dir.path().join("audit.jsonl");

    // Create configuration
    let config = create_test_config(&audit_log_path);

    // Skip test if no API key provided
    if config.provider.openai_api_key.is_none() {
        println!("⚠️  Skipping integration test - no OPENAI_API_KEY provided");
        return Ok(());
    }

    println!("✅ Configuration created successfully");

    // Test 1: Validate provider configuration
    println!("🔧 Testing provider configuration validation...");
    ProviderClientFactory::validate_config(&config.provider)?;
    println!("✅ Provider configuration is valid");

    // Test 2: Create provider client
    println!("🔧 Testing provider client creation...");
    let provider_client = ProviderClientFactory::create_client(&config.provider)?;
    println!("✅ Provider client created successfully");

    // Test 3: Create audit logger
    println!("🔧 Testing audit logger creation...");
    let audit_logger = AuditLogger::with_path(&audit_log_path).await?;
    println!("✅ Audit logger created successfully");

    // Test 4: Create session manager
    println!("🔧 Testing session manager creation...");
    let session_manager = SessionManager::new(config.clone(), audit_logger).await?;
    println!("✅ Session manager created successfully");

    // Test 5: Start a session
    println!("🔧 Testing session lifecycle...");
    let session_id = session_manager.start_session().await?;
    println!("✅ Session started with ID: {}", session_id);

    // Test 6: Send a message and get response
    println!("🔧 Testing message completion...");
    let user_message = "Hello! Please respond with a simple greeting.";
    let response = session_manager.send_message(user_message.to_string()).await?;
    println!("✅ Message sent and response received:");
    println!("   User: {}", user_message);
    println!("   Assistant: {}", response);

    // Verify response is not empty
    assert!(!response.is_empty(), "Response should not be empty");

    // Test 7: Send a streaming message
    println!("🔧 Testing streaming message...");
    let stream_message = "Please count from 1 to 3, with each number on a new line.";
    let mut stream = session_manager.send_message_stream(stream_message.to_string()).await?;
    
    println!("✅ Streaming initiated:");
    println!("   User: {}", stream_message);
    print!("   Assistant: ");
    
    let mut collected_response = String::new();
    let mut chunk_count = 0;
    
    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                print!("{}", chunk);
                collected_response.push_str(&chunk);
                chunk_count += 1;
                
                // Prevent infinite loops
                if chunk_count > 50 {
                    break;
                }
            }
            Err(e) => {
                println!("\n   Stream error: {}", e);
                break;
            }
        }
    }
    println!(); // New line after streaming

    assert!(chunk_count > 0, "Should have received at least one chunk");
    assert!(!collected_response.is_empty(), "Should have collected content");

    // Test 8: Get conversation statistics
    println!("🔧 Testing conversation statistics...");
    let stats = session_manager.conversation_stats().await;
    assert!(stats.is_some(), "Should have conversation stats");
    
    let stats = stats.unwrap();
    println!("✅ Conversation stats:");
    println!("   Total messages: {}", stats.total_messages);
    println!("   User messages: {}", stats.user_messages);
    println!("   Assistant messages: {}", stats.assistant_messages);
    println!("   Total characters: {}", stats.total_characters);

    assert!(stats.total_messages >= 4, "Should have at least 4 messages"); // 2 user + 2 assistant
    assert!(stats.user_messages >= 2, "Should have at least 2 user messages");
    assert!(stats.assistant_messages >= 2, "Should have at least 2 assistant messages");

    // Test 9: Clear conversation
    println!("🔧 Testing conversation clearing...");
    session_manager.clear_conversation().await?;
    let cleared_stats = session_manager.conversation_stats().await.unwrap();
    assert_eq!(cleared_stats.total_messages, 0, "Messages should be cleared");
    println!("✅ Conversation cleared successfully");

    // Test 10: End session
    println!("🔧 Testing session end...");
    session_manager.end_session().await?;
    assert_eq!(session_manager.current_session_id().await, None, "Session should be ended");
    println!("✅ Session ended successfully");

    // Test 11: Verify audit log
    println!("🔧 Testing audit log verification...");
    verify_audit_log(&audit_log_path).await?;
    println!("✅ Audit log verification completed");

    println!("🎉 All integration tests passed!");
    Ok(())
}

#[tokio::test]
async fn test_openai_client_direct() -> Result<()> {
    // Skip if no API key
    let api_key = match env::var("OPENAI_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            println!("⚠️  Skipping OpenAI client test - no OPENAI_API_KEY provided");
            return Ok(());
        }
    };

    println!("🚀 Testing OpenAI client directly");

    let config = OpenAIConfig {
        api_key,
        base_url: "https://api.openai.com/v1".to_string(),
        timeout: std::time::Duration::from_secs(30),
        max_retries: 3,
        initial_retry_delay: std::time::Duration::from_millis(500),
        max_retry_delay: std::time::Duration::from_secs(60),
        max_concurrent_requests: 10,
    };

    let client = OpenAIClient::new(config)?;

    // Test models endpoint
    println!("🔧 Testing models list...");
    let models = client.list_models().await?;
    println!("✅ Retrieved {} models", models.data.len());
    
    // Show first few models
    for model in models.data.iter().take(3) {
        println!("   - {}", model.id);
    }

    assert!(!models.data.is_empty(), "Should have at least one model");

    println!("🎉 OpenAI client test passed!");
    Ok(())
}

fn create_test_config(audit_log_path: &std::path::Path) -> Config {
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
            audit_log_path: Some(audit_log_path.to_path_buf()),
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

async fn verify_audit_log(audit_log_path: &std::path::Path) -> Result<()> {
    if !audit_log_path.exists() {
        return Err(fennec_core::FennecError::Unknown {
            message: "Audit log file does not exist".to_string(),
        });
    }

    let content = tokio::fs::read_to_string(audit_log_path).await?;
    let lines: Vec<&str> = content.lines().collect();

    println!("   Audit log contains {} entries", lines.len());

    // Verify we have some expected events
    let mut session_started = false;
    let mut user_messages = 0;
    let mut assistant_messages = 0;
    let mut session_ended = false;

    for line in lines {
        if let Ok(entry) = serde_json::from_str::<serde_json::Value>(line) {
            match entry["event_type"].as_str() {
                Some("session_event") => {
                    if entry["details"]["action"].as_str() == Some("session_started") {
                        session_started = true;
                    }
                    if entry["details"]["action"].as_str() == Some("session_ended") {
                        session_ended = true;
                    }
                }
                Some("user_message") => user_messages += 1,
                Some("assistant_message") => assistant_messages += 1,
                _ => {}
            }
        }
    }

    assert!(session_started, "Should have session_started event");
    assert!(user_messages >= 2, "Should have at least 2 user messages");
    assert!(assistant_messages >= 2, "Should have at least 2 assistant messages");
    assert!(session_ended, "Should have session_ended event");

    println!("   ✅ Session started: {}", session_started);
    println!("   ✅ User messages: {}", user_messages);
    println!("   ✅ Assistant messages: {}", assistant_messages);
    println!("   ✅ Session ended: {}", session_ended);

    Ok(())
}