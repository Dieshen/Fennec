use crate::registry::{CommandContext, CommandExecutor};
use crate::summarize_enhanced::{
    EnhancedSummarizeCommand, OutputDestination, SummaryDepth, SummaryType,
};
use fennec_core::session::Session;
use fennec_core::transcript::{MessageRole, Transcript};
use fennec_memory::SessionMemory;
use fennec_security::SandboxLevel;
use serde_json::json;
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::fs;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

/// Helper function to create a test context
fn create_test_context(workspace_path: Option<PathBuf>) -> CommandContext {
    CommandContext {
        session_id: Uuid::new_v4(),
        user_id: None,
        workspace_path: workspace_path.map(|path| path.to_string_lossy().to_string()),
        sandbox_level: SandboxLevel::ReadOnly,
        dry_run: false,
        preview_only: false,
        cancellation_token: CancellationToken::new(),
    }
}

/// Helper function to create test session memory
#[allow(dead_code)]
async fn create_test_session_memory() -> SessionMemory {
    let session = Session::new();
    let mut transcript = Transcript::new(session.id);

    // Add some test messages
    transcript.add_message(
        MessageRole::User,
        "How do I implement a REST API in Rust?".to_string(),
    );
    transcript.add_message(MessageRole::Assistant, "To implement a REST API in Rust, you can use frameworks like Axum, Warp, or Actix-web. Here's an example with Axum...".to_string());
    transcript.add_message(
        MessageRole::User,
        "Can you show me how to add error handling?".to_string(),
    );
    transcript.add_message(
        MessageRole::Assistant,
        "Certainly! Here's how to add proper error handling to your Rust REST API...".to_string(),
    );

    let mut context = fennec_memory::ConversationContext::default();
    context.technologies.push("rust".to_string());
    context.technologies.push("axum".to_string());
    context.current_task = Some("implement REST API".to_string());
    context
        .recent_topics
        .push("REST API implementation".to_string());
    context
        .recent_topics
        .push("error handling in Rust".to_string());

    SessionMemory {
        session,
        transcript,
        context,
        is_dirty: false,
    }
}

#[tokio::test]
async fn test_enhanced_summarize_command_creation() {
    let command = EnhancedSummarizeCommand::new();
    let descriptor = command.descriptor();

    assert_eq!(descriptor.name, "summarize_enhanced");
    assert_eq!(descriptor.version, "2.0.0");
    assert!(descriptor.description.contains("enhanced summaries"));
    assert!(descriptor.supports_preview);
}

#[tokio::test]
async fn test_enhanced_summarize_with_memory_services() {
    let result = EnhancedSummarizeCommand::with_memory_services().await;
    assert!(result.is_ok());

    let command = result.unwrap();
    let descriptor = command.descriptor();
    assert_eq!(descriptor.name, "summarize_enhanced");
}

#[tokio::test]
async fn test_summarize_text_content() {
    let command = EnhancedSummarizeCommand::new();
    let context = create_test_context(None);

    let args = json!({
        "target": "Hello world\nThis is a test\nWith multiple lines\nAnd some Rust code:\nfn main() {\n    println!(\"Hello!\");\n}",
        "summary_type": "Text",
        "is_path": false,
        "max_lines": 10,
        "depth_level": "Standard"
    });

    let result = command.execute(&args, &context).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("Text Summary"));
    assert!(result.output.contains("**Lines:** 7"));
    assert!(result.output.contains("**Words:**"));
    assert!(result.output.contains("**Characters:**"));
}

#[tokio::test]
async fn test_summarize_file() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.rs");

    let content = r#"
use std::collections::HashMap;

fn main() {
    let mut map = HashMap::new();
    map.insert("key", "value");
    println!("Hello, world!");
}
"#;

    fs::write(&test_file, content).await.unwrap();

    let command = EnhancedSummarizeCommand::new();
    let context = create_test_context(Some(temp_dir.path().to_path_buf()));

    let args = json!({
        "target": test_file.to_string_lossy(),
        "summary_type": "File",
        "is_path": true,
        "max_lines": 20
    });

    let result = command.execute(&args, &context).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("File Summary"));
    assert!(result.output.contains("**Total lines:**"));
    assert!(result.output.contains("**File type:** rs"));
    assert!(result.output.contains("Content Preview"));
}

#[tokio::test]
async fn test_summarize_directory() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).await.unwrap();

    // Create some test files
    fs::write(src_dir.join("main.rs"), "fn main() {}")
        .await
        .unwrap();
    fs::write(src_dir.join("lib.rs"), "pub fn hello() {}")
        .await
        .unwrap();
    fs::write(
        temp_dir.path().join("Cargo.toml"),
        "[package]\nname = \"test\"",
    )
    .await
    .unwrap();

    let command = EnhancedSummarizeCommand::new();
    let context = create_test_context(Some(temp_dir.path().to_path_buf()));

    let args = json!({
        "target": temp_dir.path().to_string_lossy(),
        "summary_type": "File",
        "is_path": true,
        "include_structure": true
    });

    let result = command.execute(&args, &context).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("Directory Summary"));
    assert!(result.output.contains("**Files:**"));
    assert!(result.output.contains("**Directories:**"));
    assert!(result.output.contains("**.rs**: 2 files"));
    assert!(result.output.contains("**.toml**: 1 files"));
}

#[tokio::test]
async fn test_summarize_session_without_memory_service() {
    let command = EnhancedSummarizeCommand::new();
    let context = create_test_context(None);

    let args = json!({
        "target": "current",
        "summary_type": "Session",
        "depth_level": "Standard",
        "time_range_hours": 24
    });

    let result = command.execute(&args, &context).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("Session Summary"));
    assert!(result.output.contains("Memory service not available"));
}

#[tokio::test]
async fn test_summarize_project() {
    let temp_dir = TempDir::new().unwrap();
    let command = EnhancedSummarizeCommand::new();
    let context = create_test_context(Some(temp_dir.path().to_path_buf()));

    // Create a basic project structure
    fs::write(temp_dir.path().join("README.md"), "# Test Project")
        .await
        .unwrap();
    fs::write(temp_dir.path().join("main.py"), "print('hello')")
        .await
        .unwrap();

    let args = json!({
        "target": ".",
        "summary_type": "Project",
        "depth_level": "Standard"
    });

    let result = command.execute(&args, &context).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("Project Summary"));
    assert!(result.output.contains("Project Structure"));
    assert!(result.output.contains("**Files:**"));
}

#[tokio::test]
async fn test_summarize_commands() {
    let command = EnhancedSummarizeCommand::new();
    let context = create_test_context(None);

    let args = json!({
        "target": "recent",
        "summary_type": "Commands",
        "depth_level": "Standard",
        "time_range_hours": 6
    });

    let result = command.execute(&args, &context).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("Commands Summary"));
    assert!(result.output.contains("Recent Command Activity"));
    assert!(result.output.contains("**Time Range:** Last 6 hours"));
}

#[tokio::test]
async fn test_different_depth_levels() {
    let command = EnhancedSummarizeCommand::new();
    let context = create_test_context(None);

    let depths = vec!["Brief", "Standard", "Detailed", "Comprehensive"];

    for depth in depths {
        let args = json!({
            "target": "Hello world\nThis is a test",
            "summary_type": "Text",
            "is_path": false,
            "depth_level": depth
        });

        let result = command.execute(&args, &context).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("Text Summary"));
        assert!(result.output.contains("Lines"));
    }
}

#[tokio::test]
async fn test_output_destinations() {
    let temp_dir = TempDir::new().unwrap();
    let command = EnhancedSummarizeCommand::new();
    let context = create_test_context(Some(temp_dir.path().to_path_buf()));

    // Test console output (default)
    let args = json!({
        "target": "Test content",
        "summary_type": "Text",
        "is_path": false,
        "output_destination": "Console"
    });

    let result = command.execute(&args, &context).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("Text Summary"));

    // Test custom file output
    let output_file = temp_dir.path().join("summary.md");
    let args = json!({
        "target": "Test content for file",
        "summary_type": "Text",
        "is_path": false,
        "output_destination": {
            "CustomFile": output_file.to_string_lossy()
        }
    });

    let result = command.execute(&args, &context).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("Summary saved to"));
    assert!(output_file.exists());

    let saved_content = fs::read_to_string(&output_file).await.unwrap();
    assert!(saved_content.contains("Text Summary"));
}

#[tokio::test]
async fn test_progress_file_output() {
    let temp_dir = TempDir::new().unwrap();
    let command = EnhancedSummarizeCommand::new();
    let context = create_test_context(Some(temp_dir.path().to_path_buf()));

    let args = json!({
        "target": "Progress update test",
        "summary_type": "Text",
        "is_path": false,
        "output_destination": "ProgressFile"
    });

    let result = command.execute(&args, &context).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("Summary appended to progress.md"));

    let progress_file = temp_dir.path().join("progress.md");
    assert!(progress_file.exists());

    let content = fs::read_to_string(&progress_file).await.unwrap();
    assert!(content.contains("Text Summary"));
}

#[tokio::test]
async fn test_validation() {
    let command = EnhancedSummarizeCommand::new();

    // Test empty target
    let invalid_args = json!({
        "target": "",
        "summary_type": "Text"
    });

    let result = command.validate_args(&invalid_args);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Target cannot be empty"));

    // Test invalid max_lines
    let invalid_args = json!({
        "target": "test",
        "max_lines": 0
    });

    let result = command.validate_args(&invalid_args);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("max_lines must be greater than 0"));

    // Test invalid time_range_hours
    let invalid_args = json!({
        "target": "test",
        "time_range_hours": 0
    });

    let result = command.validate_args(&invalid_args);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("time_range_hours must be greater than 0"));

    // Test valid args
    let valid_args = json!({
        "target": "test",
        "summary_type": "Text",
        "max_lines": 50,
        "time_range_hours": 24
    });

    let result = command.validate_args(&valid_args);
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_preview_generation() {
    let command = EnhancedSummarizeCommand::new();
    let context = create_test_context(None);

    // Test session summary preview
    let args = json!({
        "target": "current",
        "summary_type": "Session"
    });

    let preview = command.preview(&args, &context).await.unwrap();
    assert!(preview.description.contains("session summary"));
    assert!(!preview.actions.is_empty());

    // Test file summary preview
    let args = json!({
        "target": "/path/to/file.rs",
        "summary_type": "File",
        "is_path": true
    });

    let preview = command.preview(&args, &context).await.unwrap();
    assert!(preview.description.contains("file summary"));

    // Test with memory file save
    let args = json!({
        "target": "test",
        "summary_type": "Text",
        "save_to_memory": true
    });

    let preview = command.preview(&args, &context).await.unwrap();
    assert!(preview.actions.iter().any(|action| {
        if let fennec_core::command::PreviewAction::ReadFile { path } = action {
            path.contains("Memory file creation")
        } else {
            false
        }
    }));
}

#[tokio::test]
async fn test_invalid_file_path() {
    let command = EnhancedSummarizeCommand::new();
    let context = create_test_context(None);

    let args = json!({
        "target": "/nonexistent/path/file.txt",
        "summary_type": "File",
        "is_path": true
    });

    let result = command.execute(&args, &context).await.unwrap();
    assert!(!result.success);
    assert!(result.error.is_some());
    assert!(result.error.unwrap().contains("Path does not exist"));
}

#[tokio::test]
async fn test_invalid_session_id() {
    let command = EnhancedSummarizeCommand::new();
    let context = create_test_context(None);

    let args = json!({
        "target": "invalid-uuid",
        "summary_type": "Session"
    });

    let result = command.execute(&args, &context).await.unwrap();
    assert!(!result.success);
    assert!(result.error.is_some());
    assert!(result.error.unwrap().contains("Invalid session ID"));
}

#[tokio::test]
async fn test_serialization_deserialization() {
    // Test that all enum types can be serialized and deserialized

    // Test SummaryType
    let summary_types = vec![
        SummaryType::File,
        SummaryType::Session,
        SummaryType::Project,
        SummaryType::Commands,
        SummaryType::Text,
    ];

    for summary_type in summary_types {
        let json = serde_json::to_string(&summary_type).unwrap();
        let deserialized: SummaryType = serde_json::from_str(&json).unwrap();
        assert_eq!(
            std::mem::discriminant(&summary_type),
            std::mem::discriminant(&deserialized)
        );
    }

    // Test SummaryDepth
    let depth_levels = vec![
        SummaryDepth::Brief,
        SummaryDepth::Standard,
        SummaryDepth::Detailed,
        SummaryDepth::Comprehensive,
    ];

    for depth in depth_levels {
        let json = serde_json::to_string(&depth).unwrap();
        let deserialized: SummaryDepth = serde_json::from_str(&json).unwrap();
        assert_eq!(
            std::mem::discriminant(&depth),
            std::mem::discriminant(&deserialized)
        );
    }

    // Test OutputDestination
    let destinations = vec![
        OutputDestination::Console,
        OutputDestination::MemoryFile("test.md".to_string()),
        OutputDestination::ProgressFile,
        OutputDestination::CustomFile("/path/to/file.md".to_string()),
        OutputDestination::Both("memory_test.md".to_string()),
    ];

    for destination in destinations {
        let json = serde_json::to_string(&destination).unwrap();
        let deserialized: OutputDestination = serde_json::from_str(&json).unwrap();
        assert_eq!(
            std::mem::discriminant(&destination),
            std::mem::discriminant(&deserialized)
        );
    }
}

#[tokio::test]
async fn test_args_with_all_options() {
    let command = EnhancedSummarizeCommand::new();
    let context = create_test_context(None);

    let args = json!({
        "target": "Comprehensive test content",
        "summary_type": "Text",
        "is_path": false,
        "max_lines": 200,
        "include_extensions": ["rs", "md"],
        "include_structure": true,
        "output_destination": "Console",
        "depth_level": "Comprehensive",
        "time_range_hours": 48,
        "save_to_memory": false,
        "memory_tags": ["test", "comprehensive"]
    });

    let result = command.execute(&args, &context).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("Text Summary"));
    assert!(result.output.contains("**Lines:**"));
}
