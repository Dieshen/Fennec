use anyhow::Result;
use fennec_core::error::FennecError;
use fennec_security::SandboxLevel;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use uuid::Uuid;

/// Different strategies for editing files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EditStrategy {
    /// Replace entire file content
    Replace { content: String },
    /// Append content to the end of the file
    Append { content: String },
    /// Prepend content to the beginning of the file
    Prepend { content: String },
    /// Insert content at a specific line number (1-based)
    InsertAtLine { line_number: usize, content: String },
    /// Search and replace text within the file
    SearchReplace { search: String, replace: String },
    /// Replace content in a specific line range (1-based, inclusive)
    LineRange {
        start: usize,
        end: Option<usize>,
        content: String,
    },
}

/// Request for editing a file
#[derive(Debug, Clone)]
pub struct FileEditRequest {
    pub path: PathBuf,
    pub strategy: EditStrategy,
    pub create_backup: bool,
    pub create_if_missing: bool,
}

/// Result of a file edit operation
#[derive(Debug, Clone)]
pub struct FileEditResult {
    pub success: bool,
    pub original_content: String,
    pub new_content: String,
    pub backup_path: Option<PathBuf>,
    pub diff: String,
    pub bytes_written: usize,
}

/// Configuration for file operations
#[derive(Debug, Clone)]
pub struct FileOperationsConfig {
    pub backup_directory: Option<PathBuf>,
    pub max_file_size: usize,
    pub detect_encoding: bool,
    pub atomic_writes: bool,
}

impl Default for FileOperationsConfig {
    fn default() -> Self {
        Self {
            backup_directory: None,
            max_file_size: 100 * 1024 * 1024, // 100MB
            detect_encoding: true,
            atomic_writes: true,
        }
    }
}

/// Main struct for handling file operations
pub struct FileOperations {
    config: FileOperationsConfig,
}

impl FileOperations {
    pub fn new(config: FileOperationsConfig) -> Self {
        Self { config }
    }

    pub fn with_default_config() -> Self {
        Self::new(FileOperationsConfig::default())
    }

    /// Validate a file path for security and accessibility
    pub async fn validate_file_path(
        &self,
        file_path: &Path,
        sandbox_level: &SandboxLevel,
        workspace_path: Option<&str>,
    ) -> Result<PathBuf> {
        // Convert to absolute path
        let abs_path = if file_path.is_absolute() {
            file_path.to_path_buf()
        } else {
            std::env::current_dir()?.join(file_path)
        };

        // Normalize the path to resolve any .. or . components
        let canonical_path = self.normalize_path(&abs_path)?;

        // Security checks based on sandbox level
        match sandbox_level {
            SandboxLevel::ReadOnly => {
                return Err(FennecError::Security(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Cannot modify files in read-only mode",
                )))
                .into());
            }
            SandboxLevel::WorkspaceWrite => {
                // Only allow editing within the current workspace
                let workspace = if let Some(ws) = workspace_path {
                    PathBuf::from(ws)
                } else {
                    std::env::current_dir()?
                };

                let canonical_workspace = self.normalize_path(&workspace)?;
                if !canonical_path.starts_with(&canonical_workspace) {
                    return Err(FennecError::Security(Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Cannot edit files outside the current workspace",
                    )))
                    .into());
                }
            }
            SandboxLevel::FullAccess => {
                // Full access - minimal restrictions
            }
        }

        // Prevent editing sensitive system files and directories
        self.check_system_paths(&canonical_path)?;

        // Check for path traversal attempts
        self.check_path_traversal(&canonical_path)?;

        // Validate parent directory exists or can be created
        if let Some(parent) = canonical_path.parent() {
            if !parent.exists() {
                return Err(FennecError::Security(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Parent directory does not exist: {}", parent.display()),
                )))
                .into());
            }
        }

        Ok(canonical_path)
    }

    /// Normalize a path by resolving . and .. components
    fn normalize_path(&self, path: &Path) -> Result<PathBuf> {
        // For now, use a simple canonicalize approach
        // In production, we might want more sophisticated normalization
        let mut normalized = PathBuf::new();

        for component in path.components() {
            match component {
                std::path::Component::ParentDir => {
                    if !normalized.pop() {
                        return Err(FennecError::Security(Box::new(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "Path traversal attempt detected",
                        )))
                        .into());
                    }
                }
                std::path::Component::CurDir => {
                    // Skip current directory components
                }
                _ => {
                    normalized.push(component);
                }
            }
        }

        Ok(normalized)
    }

    /// Check for dangerous system paths
    fn check_system_paths(&self, path: &Path) -> Result<()> {
        let dangerous_paths = [
            "/etc",
            "/usr",
            "/sys",
            "/proc",
            "/dev",
            "/boot",
            "/root",
            "C:\\Windows",
            "C:\\Program Files",
            "C:\\System32",
            "C:\\ProgramData",
        ];

        for dangerous in &dangerous_paths {
            if path.starts_with(dangerous) {
                return Err(FennecError::Security(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Cannot edit files in system directory: {}", dangerous),
                )))
                .into());
            }
        }

        Ok(())
    }

    /// Check for path traversal attempts
    fn check_path_traversal(&self, path: &Path) -> Result<()> {
        let path_str = path.to_string_lossy();

        // Check for various path traversal patterns
        let dangerous_patterns = ["../", "..\\", "%2e%2e", "%2e%2e%2f", "%2e%2e%5c"];

        for pattern in &dangerous_patterns {
            if path_str.contains(pattern) {
                return Err(FennecError::Security(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Path traversal attempt detected",
                )))
                .into());
            }
        }

        Ok(())
    }

    /// Safely read a file with encoding detection
    pub async fn safe_read_file(&self, path: &Path) -> Result<String> {
        // Check file size
        let metadata = fs::metadata(path).await.map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to read file metadata {}: {}", path.display(), e),
            )))
        })?;

        if metadata.len() > self.config.max_file_size as u64 {
            return Err(FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!(
                    "File too large: {} bytes (max: {})",
                    metadata.len(),
                    self.config.max_file_size
                ),
            )))
            .into());
        }

        // Read the file as bytes first
        let bytes = fs::read(path).await.map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to read file {}: {}", path.display(), e),
            )))
        })?;

        // Detect encoding if enabled
        if self.config.detect_encoding {
            self.decode_with_detection(&bytes, path)
        } else {
            // Default to UTF-8
            String::from_utf8(bytes).map_err(|e| {
                FennecError::Command(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("File {} is not valid UTF-8: {}", path.display(), e),
                )))
                .into()
            })
        }
    }

    /// Decode bytes with encoding detection
    fn decode_with_detection(&self, bytes: &[u8], path: &Path) -> Result<String> {
        // Check for UTF-8 BOM
        if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
            return String::from_utf8(bytes[3..].to_vec()).map_err(|e| {
                FennecError::Command(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!(
                        "File {} has UTF-8 BOM but invalid UTF-8 content: {}",
                        path.display(),
                        e
                    ),
                )))
                .into()
            });
        }

        // Check for UTF-16 BOMs
        if bytes.starts_with(&[0xFF, 0xFE]) {
            return <String as StringFromUtf16>::from_utf16le(&bytes[2..]).map_err(|e| {
                FennecError::Command(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!(
                        "File {} has UTF-16LE BOM but invalid content: {}",
                        path.display(),
                        e
                    ),
                )))
                .into()
            });
        }

        if bytes.starts_with(&[0xFE, 0xFF]) {
            return <String as StringFromUtf16>::from_utf16be(&bytes[2..]).map_err(|e| {
                FennecError::Command(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!(
                        "File {} has UTF-16BE BOM but invalid content: {}",
                        path.display(),
                        e
                    ),
                )))
                .into()
            });
        }

        // Try UTF-8 first (most common)
        if let Ok(utf8_string) = String::from_utf8(bytes.to_vec()) {
            return Ok(utf8_string);
        }

        // Check if it's binary data
        if bytes
            .iter()
            .any(|&b| b == 0 || (b < 32 && b != 9 && b != 10 && b != 13))
        {
            return Err(FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!(
                    "File {} appears to be binary, cannot edit as text",
                    path.display()
                ),
            )))
            .into());
        }

        // Fall back to lossy UTF-8 conversion
        Ok(String::from_utf8_lossy(bytes).to_string())
    }

    /// Create a backup of a file
    pub async fn create_backup(&self, path: &Path) -> Result<PathBuf> {
        if !path.exists() {
            return Err(FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Cannot backup non-existent file: {}", path.display()),
            )))
            .into());
        }

        let backup_dir = if let Some(ref backup_dir) = self.config.backup_directory {
            backup_dir.clone()
        } else {
            path.parent()
                .ok_or_else(|| {
                    FennecError::Command(Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Cannot determine backup directory",
                    )))
                })?
                .to_path_buf()
        };

        // Create backup directory if it doesn't exist
        if !backup_dir.exists() {
            fs::create_dir_all(&backup_dir).await.map_err(|e| {
                FennecError::Command(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to create backup directory: {}", e),
                )))
            })?;
        }

        // Generate backup filename with timestamp and UUID for uniqueness
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let uuid = Uuid::new_v4().simple();
        let file_name = path
            .file_name()
            .ok_or_else(|| {
                FennecError::Command(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Invalid file name for backup",
                )))
            })?
            .to_string_lossy();

        let backup_name = format!("{}.backup.{}.{}", file_name, timestamp, uuid);
        let backup_path = backup_dir.join(backup_name);

        // Copy the file
        fs::copy(path, &backup_path).await.map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to create backup: {}", e),
            )))
        })?;

        Ok(backup_path)
    }

    /// Atomically write content to a file
    pub async fn atomic_write_file(&self, path: &Path, content: &str) -> Result<usize> {
        if !self.config.atomic_writes {
            // Direct write (less safe but simpler)
            fs::write(path, content).await.map_err(|e| {
                FennecError::Command(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to write file {}: {}", path.display(), e),
                )))
            })?;
            return Ok(content.len());
        }

        // Atomic write using temporary file
        let temp_path = path.with_extension(format!("tmp.{}", Uuid::new_v4().simple()));

        // Write to temporary file first
        fs::write(&temp_path, content).await.map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to write temporary file: {}", e),
            )))
        })?;

        // Rename to final destination (atomic on most filesystems)
        match fs::rename(&temp_path, path).await {
            Ok(_) => {}
            Err(e) => {
                // Clean up temp file on failure
                let _ = fs::remove_file(&temp_path).await;
                return Err(FennecError::Command(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to rename temporary file: {}", e),
                )))
                .into());
            }
        }

        Ok(content.len())
    }

    /// Apply an edit strategy to file content
    pub fn apply_edit_strategy(
        &self,
        original_content: &str,
        strategy: &EditStrategy,
    ) -> Result<String> {
        match strategy {
            EditStrategy::Replace { content } => Ok(content.clone()),

            EditStrategy::Append { content } => {
                if original_content.is_empty() {
                    Ok(content.clone())
                } else if original_content.ends_with('\n') {
                    Ok(format!("{}{}", original_content, content))
                } else {
                    Ok(format!("{}\n{}", original_content, content))
                }
            }

            EditStrategy::Prepend { content } => {
                if original_content.is_empty() {
                    Ok(content.clone())
                } else if content.ends_with('\n') {
                    Ok(format!("{}{}", content, original_content))
                } else {
                    Ok(format!("{}\n{}", content, original_content))
                }
            }

            EditStrategy::InsertAtLine {
                line_number,
                content,
            } => {
                if *line_number == 0 {
                    return Err(FennecError::Command(Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Line numbers must be 1-based (start from 1)",
                    )))
                    .into());
                }

                let mut lines: Vec<&str> = original_content.lines().collect();
                let insert_index = line_number.saturating_sub(1);

                if insert_index > lines.len() {
                    return Err(FennecError::Command(Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!(
                            "Line {} is beyond file length ({})",
                            line_number,
                            lines.len()
                        ),
                    )))
                    .into());
                }

                lines.insert(insert_index, content);
                Ok(lines.join("\n"))
            }

            EditStrategy::SearchReplace { search, replace } => {
                if search.is_empty() {
                    return Err(FennecError::Command(Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Search string cannot be empty",
                    )))
                    .into());
                }
                Ok(original_content.replace(search, replace))
            }

            EditStrategy::LineRange {
                start,
                end,
                content,
            } => {
                if *start == 0 {
                    return Err(FennecError::Command(Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Line numbers must be 1-based (start from 1)",
                    )))
                    .into());
                }

                let lines: Vec<&str> = original_content.lines().collect();
                let start_idx = start.saturating_sub(1);
                let end_idx = end.unwrap_or(*start).saturating_sub(1);

                if start_idx >= lines.len() {
                    return Err(FennecError::Command(Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Line {} is beyond file length ({})", start, lines.len()),
                    )))
                    .into());
                }

                if end_idx < start_idx {
                    return Err(FennecError::Command(Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "End line must be greater than or equal to start line",
                    )))
                    .into());
                }

                let mut new_lines = lines[..start_idx].to_vec();
                new_lines.push(content);
                new_lines.extend_from_slice(&lines[(end_idx + 1)..]);

                Ok(new_lines.join("\n"))
            }
        }
    }

    /// Perform a complete file edit operation
    pub async fn edit_file(
        &self,
        request: FileEditRequest,
        sandbox_level: &SandboxLevel,
        workspace_path: Option<&str>,
    ) -> Result<FileEditResult> {
        // Validate the file path
        let validated_path = self
            .validate_file_path(&request.path, sandbox_level, workspace_path)
            .await?;

        // Read existing content or create empty if file doesn't exist
        let original_content = if validated_path.exists() {
            self.safe_read_file(&validated_path).await?
        } else if request.create_if_missing {
            // Create parent directories if needed
            if let Some(parent) = validated_path.parent() {
                fs::create_dir_all(parent).await.map_err(|e| {
                    FennecError::Command(Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Failed to create directory {}: {}", parent.display(), e),
                    )))
                })?;
            }
            String::new()
        } else {
            return Err(FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!(
                    "File {} does not exist and create_if_missing is false",
                    validated_path.display()
                ),
            )))
            .into());
        };

        // Apply the edit strategy
        let new_content = self.apply_edit_strategy(&original_content, &request.strategy)?;

        // Generate diff for preview
        let diff = self.generate_diff(&original_content, &new_content)?;

        // Create backup if requested and file exists
        let backup_path = if request.create_backup && validated_path.exists() {
            Some(self.create_backup(&validated_path).await?)
        } else {
            None
        };

        // Write the new content atomically
        let bytes_written = self
            .atomic_write_file(&validated_path, &new_content)
            .await?;

        Ok(FileEditResult {
            success: true,
            original_content,
            new_content,
            backup_path,
            diff,
            bytes_written,
        })
    }

    /// Generate a diff between old and new content
    pub fn generate_diff(&self, old_content: &str, new_content: &str) -> Result<String> {
        use similar::{ChangeTag, TextDiff};

        let diff = TextDiff::from_lines(old_content, new_content);
        let mut output = Vec::new();

        output.push("--- original".to_string());
        output.push("+++ modified".to_string());

        for group in diff.grouped_ops(3) {
            for op in &group {
                for change in diff.iter_changes(op) {
                    let sign = match change.tag() {
                        ChangeTag::Delete => "-",
                        ChangeTag::Insert => "+",
                        ChangeTag::Equal => " ",
                    };
                    output.push(format!("{}{}", sign, change.value().trim_end()));
                }
            }
        }

        Ok(output.join("\n"))
    }
}

// Helper trait for UTF-16 decoding
trait StringFromUtf16 {
    fn from_utf16le(bytes: &[u8]) -> Result<String>;
    fn from_utf16be(bytes: &[u8]) -> Result<String>;
}

impl StringFromUtf16 for String {
    fn from_utf16le(bytes: &[u8]) -> Result<String> {
        if bytes.len() % 2 != 0 {
            return Err(FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Invalid UTF-16LE data: odd number of bytes",
            )))
            .into());
        }

        let utf16_data: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();

        String::from_utf16(&utf16_data).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Invalid UTF-16LE content: {}", e),
            )))
            .into()
        })
    }

    fn from_utf16be(bytes: &[u8]) -> Result<String> {
        if bytes.len() % 2 != 0 {
            return Err(FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Invalid UTF-16BE data: odd number of bytes",
            )))
            .into());
        }

        let utf16_data: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
            .collect();

        String::from_utf16(&utf16_data).map_err(|e| {
            FennecError::Command(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Invalid UTF-16BE content: {}", e),
            )))
            .into()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio::fs::write;

    #[tokio::test]
    async fn test_edit_strategy_replace() {
        let file_ops = FileOperations::with_default_config();
        let original = "line 1\nline 2\nline 3";
        let strategy = EditStrategy::Replace {
            content: "new content".to_string(),
        };

        let result = file_ops.apply_edit_strategy(original, &strategy).unwrap();
        assert_eq!(result, "new content");
    }

    #[tokio::test]
    async fn test_edit_strategy_append() {
        let file_ops = FileOperations::with_default_config();
        let original = "line 1\nline 2";
        let strategy = EditStrategy::Append {
            content: "line 3".to_string(),
        };

        let result = file_ops.apply_edit_strategy(original, &strategy).unwrap();
        assert_eq!(result, "line 1\nline 2\nline 3");
    }

    #[tokio::test]
    async fn test_edit_strategy_prepend() {
        let file_ops = FileOperations::with_default_config();
        let original = "line 2\nline 3";
        let strategy = EditStrategy::Prepend {
            content: "line 1".to_string(),
        };

        let result = file_ops.apply_edit_strategy(original, &strategy).unwrap();
        assert_eq!(result, "line 1\nline 2\nline 3");
    }

    #[tokio::test]
    async fn test_edit_strategy_insert_at_line() {
        let file_ops = FileOperations::with_default_config();
        let original = "line 1\nline 3";
        let strategy = EditStrategy::InsertAtLine {
            line_number: 2,
            content: "line 2".to_string(),
        };

        let result = file_ops.apply_edit_strategy(original, &strategy).unwrap();
        assert_eq!(result, "line 1\nline 2\nline 3");
    }

    #[tokio::test]
    async fn test_edit_strategy_search_replace() {
        let file_ops = FileOperations::with_default_config();
        let original = "Hello world\nGoodbye world";
        let strategy = EditStrategy::SearchReplace {
            search: "world".to_string(),
            replace: "universe".to_string(),
        };

        let result = file_ops.apply_edit_strategy(original, &strategy).unwrap();
        assert_eq!(result, "Hello universe\nGoodbye universe");
    }

    #[tokio::test]
    async fn test_safe_file_operations() {
        let temp_dir = tempdir().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        let content = "Hello, world!";

        write(&test_file, content).await.unwrap();

        let file_ops = FileOperations::with_default_config();

        // Test reading
        let read_content = file_ops.safe_read_file(&test_file).await.unwrap();
        assert_eq!(read_content, content);

        // Test backup creation
        let backup_path = file_ops.create_backup(&test_file).await.unwrap();
        assert!(backup_path.exists());
        let backup_content = file_ops.safe_read_file(&backup_path).await.unwrap();
        assert_eq!(backup_content, content);

        // Test atomic write
        let new_content = "Hello, universe!";
        let bytes_written = file_ops
            .atomic_write_file(&test_file, new_content)
            .await
            .unwrap();
        assert_eq!(bytes_written, new_content.len());

        let updated_content = file_ops.safe_read_file(&test_file).await.unwrap();
        assert_eq!(updated_content, new_content);
    }

    #[tokio::test]
    async fn test_complete_file_edit() {
        let temp_dir = tempdir().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        let original_content = "line 1\nline 2\nline 3";

        write(&test_file, original_content).await.unwrap();

        let file_ops = FileOperations::with_default_config();
        let request = FileEditRequest {
            path: test_file.clone(),
            strategy: EditStrategy::SearchReplace {
                search: "line 2".to_string(),
                replace: "modified line 2".to_string(),
            },
            create_backup: true,
            create_if_missing: false,
        };

        let result = file_ops
            .edit_file(request, &SandboxLevel::FullAccess, None)
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.backup_path.is_some());
        assert!(result.diff.contains("-line 2"));
        assert!(result.diff.contains("+modified line 2"));

        let final_content = file_ops.safe_read_file(&test_file).await.unwrap();
        assert_eq!(final_content, "line 1\nmodified line 2\nline 3");
    }
}
