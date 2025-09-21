use crate::{OpenAIClient, OpenAIConfig, ProviderClientFactory};
use fennec_core::{
    config::ProviderConfig,
    provider::{ProviderMessage, ProviderRequest},
};
use futures::StreamExt;
use std::env;
use tokio;
use tracing_subscriber;
use uuid::Uuid;

/// Integration test that can be run manually with a real API key
///
/// Run with: cargo test --package fennec-provider integration_test -- --ignored --nocapture
/// Make sure to set OPENAI_API_KEY environment variable
#[tokio::test]
#[ignore] // Requires real API key and network access
async fn test_openai_integration() {
    // Initialize logging for debugging
    let _ = tracing_subscriber::fmt::try_init();

    // Get API key from environment
    let api_key = env::var("OPENAI_API_KEY")
        .expect("OPENAI_API_KEY environment variable is required for integration test");

    // Create provider config
    let provider_config = ProviderConfig {
        openai_api_key: Some(api_key),
        default_model: "gpt-3.5-turbo".to_string(),
        base_url: None,
        timeout_seconds: 30,
    };

    // Test 1: Create client using factory
    println!("Testing client creation...");
    let client = ProviderClientFactory::create_client(&provider_config)
        .expect("Failed to create provider client");

    // Test 2: Send a simple completion request
    println!("Testing completion request...");
    let request = ProviderRequest {
        id: Uuid::new_v4(),
        messages: vec![ProviderMessage {
            role: "user".to_string(),
            content: "Say 'Hello, Fennec!' in a friendly way.".to_string(),
        }],
        model: "gpt-3.5-turbo".to_string(),
        stream: false,
    };

    let response = client
        .complete(request)
        .await
        .expect("Failed to get completion");

    println!("Completion response: {}", response.content);
    assert!(!response.content.is_empty());
    assert!(response.content.to_lowercase().contains("hello"));

    // Test 3: Send a streaming request
    println!("Testing streaming request...");
    let stream_request = ProviderRequest {
        id: Uuid::new_v4(),
        messages: vec![ProviderMessage {
            role: "user".to_string(),
            content: "Count from 1 to 5, putting each number on a separate line.".to_string(),
        }],
        model: "gpt-3.5-turbo".to_string(),
        stream: true,
    };

    let mut stream = client
        .stream(stream_request)
        .await
        .expect("Failed to get stream");

    let mut collected_content = String::new();
    let mut chunk_count = 0;

    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                print!("{}", chunk); // Real-time output
                collected_content.push_str(&chunk);
                chunk_count += 1;

                // Prevent infinite loops in case of issues
                if chunk_count > 100 {
                    println!("\nBreaking after 100 chunks to prevent infinite loop");
                    break;
                }
            }
            Err(e) => {
                println!("\nStream error: {}", e);
                break;
            }
        }
    }

    println!("\n\nStreaming completed!");
    println!("Total chunks received: {}", chunk_count);
    println!("Collected content: {}", collected_content);

    assert!(chunk_count > 0, "Should have received at least one chunk");
    assert!(
        !collected_content.is_empty(),
        "Should have collected some content"
    );

    println!("All integration tests passed!");
}

/// Test with invalid API key to verify error handling
#[tokio::test]
#[ignore]
async fn test_invalid_api_key() {
    let provider_config = ProviderConfig {
        openai_api_key: Some("invalid-key".to_string()),
        default_model: "gpt-3.5-turbo".to_string(),
        base_url: None,
        timeout_seconds: 30,
    };

    let client = ProviderClientFactory::create_client(&provider_config)
        .expect("Client creation should succeed even with invalid key");

    let request = ProviderRequest {
        id: Uuid::new_v4(),
        messages: vec![ProviderMessage {
            role: "user".to_string(),
            content: "Hello".to_string(),
        }],
        model: "gpt-3.5-turbo".to_string(),
        stream: false,
    };

    let result = client.complete(request).await;
    assert!(result.is_err(), "Should fail with invalid API key");

    if let Err(e) = result {
        println!("Expected error with invalid key: {}", e);
        // Should be an authentication error
        assert!(
            e.to_string().to_lowercase().contains("authentication")
                || e.to_string().to_lowercase().contains("unauthorized")
                || e.to_string().to_lowercase().contains("401")
        );
    }
}

/// Test direct OpenAI client usage
#[tokio::test]
#[ignore]
async fn test_openai_client_direct() {
    let api_key = env::var("OPENAI_API_KEY")
        .expect("OPENAI_API_KEY environment variable is required for integration test");

    let config = OpenAIConfig {
        api_key,
        base_url: "https://api.openai.com/v1".to_string(),
        timeout: std::time::Duration::from_secs(30),
        max_retries: 3,
        initial_retry_delay: std::time::Duration::from_millis(500),
        max_retry_delay: std::time::Duration::from_secs(60),
        max_concurrent_requests: 10,
    };

    let client = OpenAIClient::new(config).expect("Failed to create OpenAI client");

    // Test models endpoint
    println!("Testing models list...");
    let models = client.list_models().await.expect("Failed to list models");
    println!("Available models: {}", models.data.len());

    for model in models.data.iter().take(5) {
        println!("  - {}", model.id);
    }

    assert!(!models.data.is_empty(), "Should have at least one model");

    println!("Direct OpenAI client test passed!");
}
