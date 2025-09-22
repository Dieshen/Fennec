//! # Cline Memory Files Demo
//!
//! This example demonstrates the Cline-style memory files functionality,
//! including project initialization, automatic updates, and markdown rendering.

use anyhow::Result;
use fennec_core::{session::Session, transcript::MessageRole};
use fennec_memory::{
    Achievement, ClineFileType, MemoryService, ProjectStatus,
};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("🦊 Fennec Cline Memory Files Demo");
    println!("==================================\n");

    // Create memory service
    let memory = MemoryService::new().await?;

    // 1. Initialize a new project
    println!("📁 1. Initializing New Project");
    println!("------------------------------");

    let project_id = Uuid::new_v4();
    println!("✓ Created project ID: {}", project_id);

    memory.initialize_project_memory(project_id).await?;
    println!("✓ Initialized Cline memory files for project\n");

    // 2. Update project goals and status
    println!("🎯 2. Setting Project Goals and Status");
    println!("--------------------------------------");

    let goals = vec![
        "Implement Cline-style memory files".to_string(),
        "Create automatic file updates based on session events".to_string(),
        "Add version tracking and change history".to_string(),
        "Integrate with existing memory service".to_string(),
    ];

    memory.update_project_goals(project_id, goals).await?;
    memory.update_project_status(project_id, ProjectStatus::Active).await?;

    println!("✓ Set project goals and status to Active\n");

    // 3. Display project brief
    println!("📋 3. Project Brief");
    println!("-------------------");

    if let Some(project_brief) = memory.get_project_brief(project_id).await? {
        println!("{}", project_brief);
    } else {
        println!("⚠ No project brief found");
    }

    // 4. Create and track sessions
    println!("💬 4. Session Management");
    println!("------------------------");

    let session1 = Session::new();
    let session2 = Session::new();

    memory.start_session(session1.clone()).await?;
    memory.start_session(session2.clone()).await?;

    println!("✓ Started tracking sessions {} and {}", session1.id, session2.id);

    // Add messages to demonstrate automatic context updates
    memory.add_message(
        session1.id,
        MessageRole::User,
        "I'm working on implementing Cline-style memory files in Rust using tokio and serde".to_string()
    ).await?;

    memory.add_message(
        session1.id,
        MessageRole::Assistant,
        "Great! Cline-style memory files provide structured project context. You'll want to implement template-based markdown generation with automatic updates based on session events.".to_string()
    ).await?;

    memory.add_message(
        session2.id,
        MessageRole::User,
        "How do I test the memory file integration with the TUI?".to_string()
    ).await?;

    println!("✓ Added messages to sessions (this triggers automatic context updates)\n");

    // 5. Display active context
    println!("⚡ 5. Active Context");
    println!("-------------------");

    if let Some(active_context) = memory.get_active_context(project_id).await? {
        println!("{}", active_context);
    } else {
        println!("⚠ No active context found");
    }

    // 6. Complete some tasks
    println!("✅ 6. Task Completion");
    println!("---------------------");

    memory.complete_task(
        project_id,
        Some(session1.id),
        "Implement ClineMemoryFileService".to_string(),
        "Successfully implemented with template engine and event handling".to_string(),
    ).await?;

    memory.complete_task(
        project_id,
        Some(session1.id),
        "Add automatic file updates".to_string(),
        "Implemented event-driven updates for all file types".to_string(),
    ).await?;

    memory.complete_task(
        project_id,
        Some(session2.id),
        "Integrate with MemoryService".to_string(),
        "Added Cline service to MemoryService with full API integration".to_string(),
    ).await?;

    println!("✓ Completed 3 tasks with outcomes\n");

    // 7. Record achievements
    println!("🏆 7. Recording Achievements");
    println!("----------------------------");

    let achievement = Achievement {
        title: "Milestone 3 Completion".to_string(),
        description: "Successfully implemented Track 2: Cline-style Memory Files with all required features including template-based generation, automatic updates, version tracking, and TUI integration APIs.".to_string(),
        achieved_at: chrono::Utc::now(),
        session_id: Some(session1.id),
    };

    memory.record_achievement(project_id, Some(session1.id), achievement).await?;
    println!("✓ Recorded major achievement\n");

    // 8. End sessions
    memory.stop_session(session1.id).await?;
    memory.stop_session(session2.id).await?;
    println!("✓ Ended sessions (this triggers progress updates)\n");

    // 9. Display progress tracking
    println!("📊 9. Progress Tracking");
    println!("----------------------");

    if let Some(progress) = memory.get_progress(project_id).await? {
        println!("{}", progress);
    } else {
        println!("⚠ No progress tracking found");
    }

    // 10. Demonstrate project lifecycle management
    println!("🗂️  10. Project Lifecycle Management");
    println!("------------------------------------");

    // List all projects
    let projects = memory.list_projects().await?;
    println!("📁 Total projects with memory files: {}", projects.len());
    for project in &projects {
        println!("  - Project: {}", project);
    }

    // Create backup
    let backup_path = memory.backup_project(project_id).await?;
    println!("💾 Created backup at: {}", backup_path.display());

    // NOTE: Not archiving in demo to keep the files available for inspection
    println!("🗃️  Archive available via memory.archive_project() - skipped in demo\n");

    // 11. File structure demonstration
    println!("📂 11. File Structure");
    println!("---------------------");

    println!("Cline memory files are stored in:");
    println!("~/.local/share/fennec/projects/{}/", project_id);
    println!("├── projectbrief.md       # Project overview and goals");
    println!("├── activeContext.md      # Current session context");
    println!("├── progress.md           # Progress tracking and sessions");
    println!("├── projectbrief.md.json  # Structured data for projectbrief.md");
    println!("├── activeContext.md.json # Structured data for activeContext.md");
    println!("├── progress.md.json      # Structured data for progress.md");
    println!("└── .meta/                # Version history and metadata");
    println!("    ├── projectbrief.meta.json");
    println!("    ├── activeContext.meta.json");
    println!("    └── progress.meta.json\n");

    // 12. API Summary for TUI Integration
    println!("🖥️  12. TUI Integration APIs");
    println!("----------------------------");

    println!("The following APIs are available for TUI and other components:");
    println!("");
    println!("📋 Project Management:");
    println!("  - memory.initialize_project_memory(project_id)");
    println!("  - memory.list_projects()");
    println!("  - memory.archive_project(project_id)");
    println!("  - memory.backup_project(project_id)");
    println!("");
    println!("📄 File Rendering (Markdown):");
    println!("  - memory.get_project_brief(project_id)");
    println!("  - memory.get_active_context(project_id)");
    println!("  - memory.get_progress(project_id)");
    println!("");
    println!("⚙️  Content Updates:");
    println!("  - memory.update_project_goals(project_id, goals)");
    println!("  - memory.update_project_status(project_id, status)");
    println!("  - memory.complete_task(project_id, session_id, task, outcome)");
    println!("  - memory.record_achievement(project_id, session_id, achievement)");
    println!("");
    println!("🔄 Automatic Updates:");
    println!("  - Session start/end automatically updates activeContext and progress");
    println!("  - Message addition automatically extracts topics and technologies");
    println!("  - All changes include version tracking and timestamps");
    println!("");

    println!("🎉 Demo completed successfully!");
    println!("💾 All files have been persisted and are ready for TUI consumption.");
    println!("📁 Check ~/.local/share/fennec/projects/{}/", project_id);

    Ok(())
}