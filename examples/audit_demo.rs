use fennec_core::{
    command::{Capability, CommandPreview, CommandResult, PreviewAction},
    config::Config,
};
use fennec_security::{
    AuditSystem, AuditableCommandExecutor, AuditedCommandExecutionContext,
    AuditQueryEngine, AuditQueryFilter, ExportFormat, SandboxLevel,
    utils,
};
use std::sync::Arc;
use uuid::Uuid;

/// Comprehensive audit system demonstration
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("🔍 Fennec Audit System Demo");
    println!("================================");

    // Setup audit system
    let mut config = Config::default();
    config.security.audit_log_enabled = true;
    config.security.audit_log_path = Some(std::path::PathBuf::from("./demo_audit"));

    let audit_system = Arc::new(AuditSystem::new(&config).await?);
    println!("✅ Audit system initialized");

    // Start a session
    let session_id = Uuid::new_v4();
    let session_manager = audit_system
        .start_session(
            session_id,
            Some("demo_user".to_string()),
            Some("/demo/workspace".to_string()),
        )
        .await?;

    println!("📝 Session {} started", session_id);

    // Simulate command execution with full audit trail
    let command_id = Uuid::new_v4();
    let context = AuditedCommandExecutionContext::new(
        command_id,
        audit_system.clone(),
        session_id,
    );

    // Log command request
    context.auditor()
        .log_command_requested(
            command_id,
            "file_processor",
            &serde_json::json!({
                "input_file": "data.txt",
                "output_file": "processed_data.txt",
                "options": ["--verbose", "--check-integrity"]
            }),
            &[Capability::ReadFile, Capability::WriteFile],
            SandboxLevel::WorkspaceWrite,
        )
        .await?;

    // Generate and log command preview
    let preview = CommandPreview {
        command_id,
        description: "Process input file and create output with integrity checks".to_string(),
        actions: vec![
            PreviewAction::ReadFile { path: "data.txt".to_string() },
            PreviewAction::WriteFile {
                path: "processed_data.txt".to_string(),
                content: "Processed content with checksums".to_string(),
            },
        ],
        requires_approval: true,
    };

    context.auditor()
        .log_command_preview(command_id, &preview)
        .await?;

    // Log approval (simulated user approval)
    context.auditor()
        .log_command_approved(
            command_id,
            "interactive_prompt",
            "user_approved_after_review",
        )
        .await?;

    // Start execution
    context.start_execution().await?;

    // Simulate file operations
    let test_data = b"Sample data for processing\nLine 2\nLine 3";

    context.file_ops()
        .write_file("demo_input.txt", test_data, false)
        .await?;

    let read_data = context.file_ops()
        .read_file("demo_input.txt")
        .await?;

    let processed_data = format!(
        "Processed: {}\nChecksum: {}",
        String::from_utf8_lossy(&read_data),
        utils::sha256_checksum(&read_data)
    );

    context.file_ops()
        .write_file("demo_output.txt", processed_data.as_bytes(), true)
        .await?;

    // Complete execution
    let result = CommandResult {
        command_id,
        success: true,
        output: format!("Successfully processed {} bytes", read_data.len()),
        error: None,
    };

    context.complete_execution(&result).await?;

    println!("🚀 Command execution completed with full audit trail");

    // Simulate more commands for demonstration
    for i in 1..=3 {
        let cmd_id = Uuid::new_v4();
        let auditor = AuditableCommandExecutor::new(audit_system.clone(), session_id);

        auditor
            .log_command_requested(
                cmd_id,
                &format!("demo_command_{}", i),
                &serde_json::json!({"iteration": i}),
                &[Capability::ReadFile],
                SandboxLevel::ReadOnly,
            )
            .await?;

        auditor
            .log_permission_check(
                Capability::ReadFile,
                SandboxLevel::ReadOnly,
                true,
                Some("Read access granted for demo"),
            )
            .await?;
    }

    // End session
    audit_system.end_session(session_id, 4, 0).await?;
    println!("🔒 Session ended");

    // Demonstrate query capabilities
    println!("\n🔍 Audit Query Examples");
    println!("========================");

    let query_engine = AuditQueryEngine::new(audit_system.base_audit_path().clone());

    // Query all events for the session
    let session_filter = AuditQueryFilter {
        session_id: Some(session_id),
        ..Default::default()
    };

    let session_results = query_engine.query_events(session_filter).await?;
    println!(
        "📊 Found {} events for session {} (query took {}ms)",
        session_results.events.len(),
        session_id,
        session_results.query_duration_ms
    );

    // Get session summary
    let summary = query_engine.get_session_summary(session_id).await?;
    println!("📈 Session Summary:");
    println!("   Duration: {:?}ms", summary.duration_ms);
    println!("   Commands: {}", summary.command_count);
    println!("   File Operations: {}", summary.file_operations);
    println!("   Security Events: {}", summary.security_events);

    // Get command trail
    let trail = query_engine.get_command_trail(command_id).await?;
    println!("\n🛤️  Command {} Trail:", command_id);
    for entry in &trail.timeline {
        println!("   {} - {}", entry.timestamp.format("%H:%M:%S%.3f"), entry.event_type);
    }

    // Export audit logs
    let export_filter = AuditQueryFilter {
        session_id: Some(session_id),
        event_types: Some(vec!["CommandRequested".to_string(), "CommandCompleted".to_string()]),
        ..Default::default()
    };

    let json_export = query_engine
        .export_audit_logs(export_filter.clone(), ExportFormat::Json)
        .await?;

    let csv_export = query_engine
        .export_audit_logs(export_filter, ExportFormat::Csv)
        .await?;

    println!("\n📤 Export Examples:");
    println!("JSON export: {} characters", json_export.len());
    println!("CSV export: {} characters", csv_export.len());

    // Show sample CSV output
    let csv_lines: Vec<&str> = csv_export.lines().take(3).collect();
    println!("\nSample CSV output:");
    for line in csv_lines {
        println!("   {}", line);
    }

    // Cleanup demo files
    let _ = std::fs::remove_file("demo_input.txt");
    let _ = std::fs::remove_file("demo_output.txt");

    println!("\n✨ Demo completed successfully!");
    println!("📁 Audit files saved to: {}", audit_system.base_audit_path().display());

    Ok(())
}