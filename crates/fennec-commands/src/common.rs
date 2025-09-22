//! Common utilities and helper functions for commands
//! This module provides shared functionality used across different command implementations.

use anyhow::Result;
use fennec_core::error::FennecError;
use std::path::Path;
use std::sync::Arc;

use crate::registry::CommandRegistry;
use crate::{
    diff::DiffCommand, edit::EditCommand, plan::PlanCommand, run::RunCommand,
    summarize::SummarizeCommand, summarize_enhanced::EnhancedSummarizeCommand,
};

/// Initialize the command registry with all built-in commands
pub async fn initialize_builtin_commands() -> Result<CommandRegistry> {
    let registry = CommandRegistry::new();

    // Register all built-in commands
    let plan_command = PlanCommand::new().await?;
    registry.register_builtin(Arc::new(plan_command)).await?;
    registry
        .register_builtin(Arc::new(EditCommand::new()))
        .await?;
    registry
        .register_builtin(Arc::new(RunCommand::new()))
        .await?;
    registry
        .register_builtin(Arc::new(DiffCommand::new()))
        .await?;
    registry
        .register_builtin(Arc::new(SummarizeCommand::new()))
        .await?;

    // Register enhanced summarize command with memory services if available
    match EnhancedSummarizeCommand::with_memory_services().await {
        Ok(enhanced_summarize) => {
            registry
                .register_builtin(Arc::new(enhanced_summarize))
                .await?;
        }
        Err(_) => {
            // Fallback to basic enhanced summarize without memory services
            registry
                .register_builtin(Arc::new(EnhancedSummarizeCommand::new()))
                .await?;
        }
    }

    Ok(registry)
}

/// Validate that a file path is safe to access
pub fn validate_file_access(path: &str) -> Result<()> {
    let path = Path::new(path);

    // Convert to absolute path for validation
    let abs_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };

    // Check for path traversal attempts
    let canonical = abs_path.canonicalize().map_err(|_| {
        FennecError::Security(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Invalid file path",
        )))
    })?;

    // Ensure the canonical path is still within expected bounds
    if canonical.to_string_lossy().contains("..") {
        return Err(FennecError::Security(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Path traversal detected",
        )))
        .into());
    }

    Ok(())
}

/// Format file size in human-readable format
pub fn format_file_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];

    if bytes == 0 {
        return "0 B".to_string();
    }

    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

/// Truncate text to a maximum length with ellipsis
pub fn truncate_text(text: &str, max_length: usize) -> String {
    if text.len() <= max_length {
        text.to_string()
    } else {
        format!("{}...", &text[..max_length.saturating_sub(3)])
    }
}

/// Check if a file appears to be a text file based on its extension
pub fn is_text_file(path: &Path) -> bool {
    let text_extensions = [
        "txt", "md", "rs", "py", "js", "ts", "json", "yaml", "yml", "toml", "xml", "html", "css",
        "sql", "sh", "bash", "zsh", "fish", "ps1", "bat", "cmd", "c", "cpp", "h", "hpp", "java",
        "kt", "go", "rb", "php", "swift", "scala", "clj", "hs", "elm", "ex", "exs", "erl", "ml",
        "fs", "fsx",
    ];

    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| text_extensions.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Extract the first few lines of text for preview
pub fn extract_preview_lines(content: &str, max_lines: usize) -> Vec<String> {
    content
        .lines()
        .take(max_lines)
        .map(|line| truncate_text(line, 100))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_file_size() {
        assert_eq!(format_file_size(0), "0 B");
        assert_eq!(format_file_size(512), "512 B");
        assert_eq!(format_file_size(1024), "1.0 KB");
        assert_eq!(format_file_size(1536), "1.5 KB");
        assert_eq!(format_file_size(1024 * 1024), "1.0 MB");
    }

    #[test]
    fn test_truncate_text() {
        assert_eq!(truncate_text("short", 10), "short");
        assert_eq!(truncate_text("this is a very long text", 10), "this is...");
    }

    #[test]
    fn test_is_text_file() {
        assert!(is_text_file(Path::new("test.rs")));
        assert!(is_text_file(Path::new("README.md")));
        assert!(!is_text_file(Path::new("image.png")));
        assert!(!is_text_file(Path::new("binary.exe")));
    }

    #[test]
    fn test_extract_preview_lines() {
        let content = "line 1\nline 2\nline 3\nline 4";
        let preview = extract_preview_lines(content, 2);
        assert_eq!(preview.len(), 2);
        assert_eq!(preview[0], "line 1");
        assert_eq!(preview[1], "line 2");
    }

    #[tokio::test]
    async fn test_initialize_builtin_commands() {
        let registry = initialize_builtin_commands().await.unwrap();
        let commands = registry.list_commands().await;

        // Should have all 6 built-in commands (including enhanced summarize)
        assert_eq!(commands.len(), 6);

        let command_names: Vec<String> = commands.iter().map(|c| c.name.clone()).collect();
        assert!(command_names.contains(&"plan".to_string()));
        assert!(command_names.contains(&"edit".to_string()));
        assert!(command_names.contains(&"run".to_string()));
        assert!(command_names.contains(&"diff".to_string()));
        assert!(command_names.contains(&"summarize".to_string()));
        assert!(command_names.contains(&"summarize_enhanced".to_string()));
    }
}
