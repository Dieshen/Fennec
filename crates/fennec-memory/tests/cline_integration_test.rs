//! Integration tests for Cline-style memory files

use anyhow::Result;
use fennec_core::{session::Session, transcript::MessageRole};
use fennec_memory::{Achievement, ClineFileType, MemoryService, ProjectStatus};
use uuid::Uuid;

#[tokio::test]
async fn test_cline_memory_integration() -> Result<()> {
    // Initialize tracing for the test
    let _ = tracing_subscriber::fmt::try_init();

    // Create memory service
    let memory = MemoryService::new().await?;

    // Test 1: Initialize a new project
    let project_id = Uuid::new_v4();
    memory.initialize_project_memory(project_id).await?;

    // Test 2: Update project goals and status
    let goals = vec![
        "Implement Cline-style memory files".to_string(),
        "Create automatic file updates".to_string(),
        "Add version tracking".to_string(),
    ];

    memory.update_project_goals(project_id, goals).await?;
    memory
        .update_project_status(project_id, ProjectStatus::Active)
        .await?;

    // Test 3: Verify project brief is rendered
    let project_brief = memory.get_project_brief(project_id).await?;
    assert!(project_brief.is_some());
    let brief_content = project_brief.unwrap();
    assert!(brief_content.contains("# Project Brief"));
    assert!(brief_content.contains("Active"));
    assert!(brief_content.contains("Implement Cline-style memory files"));

    // Test 4: Create sessions and add messages
    let session1 = Session::new();
    let session2 = Session::new();

    memory.start_session(session1.clone()).await?;
    memory.start_session(session2.clone()).await?;

    // Add messages that should trigger automatic context updates
    memory
        .add_message(
            session1.id,
            MessageRole::User,
            "I'm working on implementing Cline-style memory files in Rust using tokio".to_string(),
        )
        .await?;

    memory
        .add_message(
            session1.id,
            MessageRole::Assistant,
            "Great! You'll want to implement template-based markdown generation".to_string(),
        )
        .await?;

    // Test 5: Verify active context is updated
    let active_context = memory.get_active_context(project_id).await?;
    assert!(active_context.is_some());
    let context_content = active_context.unwrap();
    assert!(context_content.contains("# Active Context"));
    // Note: Without session-to-project mapping, context won't be auto-updated
    // but the file structure should exist

    // Test 6: Complete tasks
    memory
        .complete_task(
            project_id,
            Some(session1.id),
            "Implement ClineMemoryFileService".to_string(),
            "Successfully implemented with template engine".to_string(),
        )
        .await?;

    memory
        .complete_task(
            project_id,
            Some(session2.id),
            "Add automatic file updates".to_string(),
            "Implemented event-driven updates".to_string(),
        )
        .await?;

    // Test 7: Record achievement
    let achievement = Achievement {
        title: "Milestone 3 Completion".to_string(),
        description: "Successfully implemented Track 2: Cline-style Memory Files".to_string(),
        achieved_at: chrono::Utc::now(),
        session_id: Some(session1.id),
    };

    memory
        .record_achievement(project_id, Some(session1.id), achievement)
        .await?;

    // Test 8: End sessions
    memory.stop_session(session1.id).await?;
    memory.stop_session(session2.id).await?;

    // Test 9: Verify progress tracking
    let progress = memory.get_progress(project_id).await?;
    assert!(progress.is_some());
    let progress_content = progress.unwrap();
    assert!(progress_content.contains("# Progress Tracking"));
    assert!(progress_content.contains("**Completed Tasks**: 2"));
    assert!(progress_content.contains("Milestone 3 Completion"));

    // Test 10: Project lifecycle management
    let projects = memory.list_projects().await?;
    assert!(projects.contains(&project_id));

    // Test 11: Create backup
    let backup_path = memory.backup_project(project_id).await?;
    assert!(backup_path.exists());

    // Test 12: Verify all file types have content
    assert!(memory.get_project_brief(project_id).await?.is_some());
    assert!(memory.get_active_context(project_id).await?.is_some());
    assert!(memory.get_progress(project_id).await?.is_some());

    Ok(())
}

#[tokio::test]
async fn test_cline_file_templates() -> Result<()> {
    use fennec_memory::{
        ActiveContextContent, ClineFileContent, ClineFileMetadata, ClineMemoryFile,
        ProgressContent, ProjectBriefContent, TemplateEngine,
    };

    let template_engine = TemplateEngine::new();
    let now = chrono::Utc::now();
    let project_id = Uuid::new_v4();

    // Test project brief template
    let project_brief_content = ProjectBriefContent {
        project_id,
        status: ProjectStatus::Active,
        primary_technology: Some("Rust".to_string()),
        overview: "Test project for Cline memory files".to_string(),
        goals: vec!["Goal 1".to_string(), "Goal 2".to_string()],
        current_status: "In development".to_string(),
        technologies: vec!["Rust".to_string(), "tokio".to_string()],
    };

    let metadata = ClineFileMetadata {
        created_at: now,
        updated_at: now,
        last_session_id: None,
        version_history: Vec::new(),
    };

    let project_brief = ClineMemoryFile {
        file_type: ClineFileType::ProjectBrief,
        project_id,
        content: ClineFileContent::ProjectBrief(project_brief_content),
        metadata,
        version: 1,
        file_path: "projectbrief.md".into(),
    };

    let markdown = template_engine.render_to_markdown(&project_brief)?;
    assert!(markdown.contains("# Project Brief"));
    assert!(markdown.contains("Active"));
    assert!(markdown.contains("Test project for Cline memory files"));
    assert!(markdown.contains("- Goal 1"));
    assert!(markdown.contains("- Goal 2"));
    assert!(markdown.contains("- Rust"));
    assert!(markdown.contains("- tokio"));

    // Test active context template
    let active_context_content = ActiveContextContent {
        current_session_id: Some(Uuid::new_v4()),
        focus_area: Some("Testing".to_string()),
        current_task: Some("Write integration tests".to_string()),
        session_context: "Working on test implementation".to_string(),
        recent_topics: vec!["testing".to_string(), "integration".to_string()],
        immediate_focus: vec!["Fix failing test".to_string()],
        next_steps: vec!["Add more test cases".to_string()],
    };

    let active_context = ClineMemoryFile {
        file_type: ClineFileType::ActiveContext,
        project_id,
        content: ClineFileContent::ActiveContext(active_context_content),
        metadata: ClineFileMetadata {
            created_at: now,
            updated_at: now,
            last_session_id: None,
            version_history: Vec::new(),
        },
        version: 1,
        file_path: "activeContext.md".into(),
    };

    let markdown = template_engine.render_to_markdown(&active_context)?;
    assert!(markdown.contains("# Active Context"));
    assert!(markdown.contains("Testing"));
    assert!(markdown.contains("Write integration tests"));
    assert!(markdown.contains("- testing"));
    assert!(markdown.contains("- Fix failing test"));
    assert!(markdown.contains("- Add more test cases"));

    // Test progress template
    let progress_content = ProgressContent {
        total_sessions: 0,
        completed_tasks_count: 0,
        recent_sessions: Vec::new(),
        completed_tasks: Vec::new(),
        achievements: Vec::new(),
    };

    let progress = ClineMemoryFile {
        file_type: ClineFileType::Progress,
        project_id,
        content: ClineFileContent::Progress(progress_content),
        metadata: ClineFileMetadata {
            created_at: now,
            updated_at: now,
            last_session_id: None,
            version_history: Vec::new(),
        },
        version: 1,
        file_path: "progress.md".into(),
    };

    let markdown = template_engine.render_to_markdown(&progress)?;
    assert!(markdown.contains("# Progress Tracking"));
    assert!(markdown.contains("**Total Sessions**: 0"));
    assert!(markdown.contains("**Completed Tasks**: 0"));

    Ok(())
}

#[test]
fn test_cline_file_types() {
    use fennec_memory::ClineFileType;

    assert_eq!(ClineFileType::ProjectBrief.filename(), "projectbrief.md");
    assert_eq!(ClineFileType::ActiveContext.filename(), "activeContext.md");
    assert_eq!(ClineFileType::Progress.filename(), "progress.md");
    assert_eq!(
        ClineFileType::Custom("custom".to_string()).filename(),
        "custom.md"
    );

    assert_eq!(
        ClineFileType::ProjectBrief.metadata_filename(),
        "projectbrief.meta.json"
    );
    assert_eq!(
        ClineFileType::ActiveContext.metadata_filename(),
        "activeContext.meta.json"
    );
    assert_eq!(
        ClineFileType::Progress.metadata_filename(),
        "progress.meta.json"
    );
}
