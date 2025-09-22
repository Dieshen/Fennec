//! Log file rotation implementation

use crate::Result;
use chrono::{DateTime, Utc};
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use tracing_subscriber::fmt::MakeWriter;

/// A writer that automatically rotates log files based on size
pub struct RotatingFileWriter {
    base_path: PathBuf,
    base_name: String,
    max_size_bytes: u64,
    current_file: Arc<Mutex<Option<std::fs::File>>>,
    current_size: Arc<Mutex<u64>>,
    current_file_path: Arc<Mutex<Option<PathBuf>>>,
}

impl RotatingFileWriter {
    /// Create a new rotating file writer
    pub fn new(log_dir: PathBuf, base_name: String, max_size_mb: u64) -> Result<Self> {
        let max_size_bytes = max_size_mb * 1024 * 1024;

        Ok(Self {
            base_path: log_dir,
            base_name,
            max_size_bytes,
            current_file: Arc::new(Mutex::new(None)),
            current_size: Arc::new(Mutex::new(0)),
            current_file_path: Arc::new(Mutex::new(None)),
        })
    }

    /// Get the current log file path
    fn current_log_path(&self) -> PathBuf {
        self.base_path.join(format!("{}.log", self.base_name))
    }

    /// Generate a rotated file path with timestamp
    fn rotated_log_path(&self) -> PathBuf {
        let now: DateTime<Utc> = SystemTime::now().into();
        let timestamp = now.format("%Y%m%d_%H%M%S");
        self.base_path
            .join(format!("{}_{}.log", self.base_name, timestamp))
    }

    /// Rotate the current log file if it exceeds the size limit
    fn rotate_if_needed(&self) -> Result<()> {
        let current_size = *self.current_size.lock().unwrap();

        if current_size >= self.max_size_bytes {
            self.rotate()?;
        }

        Ok(())
    }

    /// Force rotation of the current log file
    fn rotate(&self) -> Result<()> {
        // Close current file
        {
            let mut current_file = self.current_file.lock().unwrap();
            if let Some(file) = current_file.take() {
                drop(file);
            }
        }

        // Move current log to rotated name
        let current_path = self.current_log_path();
        if current_path.exists() {
            let rotated_path = self.rotated_log_path();
            std::fs::rename(&current_path, &rotated_path)?;

            tracing::info!(
                telemetry.event = "log_rotated",
                old_file = %current_path.display(),
                new_file = %rotated_path.display(),
                "Log file rotated"
            );
        }

        // Reset size counter
        *self.current_size.lock().unwrap() = 0;

        // Clear current file path
        *self.current_file_path.lock().unwrap() = None;

        Ok(())
    }

    /// Ensure we have an open file handle
    fn ensure_file_open(&self) -> Result<()> {
        let mut current_file = self.current_file.lock().unwrap();
        let mut current_file_path = self.current_file_path.lock().unwrap();

        if current_file.is_none() {
            let log_path = self.current_log_path();

            // Ensure parent directory exists
            if let Some(parent) = log_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_path)?;

            // Get current file size
            let metadata = file.metadata()?;
            *self.current_size.lock().unwrap() = metadata.len();

            *current_file = Some(file);
            *current_file_path = Some(log_path.clone());

            tracing::debug!(
                telemetry.event = "log_file_opened",
                file_path = %log_path.display(),
                current_size = metadata.len(),
                "Log file opened"
            );
        }

        Ok(())
    }
}

impl Write for RotatingFileWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Check if rotation is needed
        if let Err(e) = self.rotate_if_needed() {
            eprintln!("Log rotation failed: {}", e);
        }

        // Ensure file is open
        if let Err(e) = self.ensure_file_open() {
            return Err(io::Error::new(io::ErrorKind::Other, e));
        }

        // Write to current file
        let bytes_written = {
            let mut current_file = self.current_file.lock().unwrap();
            if let Some(ref mut file) = *current_file {
                file.write(buf)?
            } else {
                return Err(io::Error::new(io::ErrorKind::Other, "No file open"));
            }
        };

        // Update size counter
        {
            let mut current_size = self.current_size.lock().unwrap();
            *current_size += bytes_written as u64;
        }

        Ok(bytes_written)
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut current_file = self.current_file.lock().unwrap();
        if let Some(ref mut file) = *current_file {
            file.flush()?;
        }
        Ok(())
    }
}

// Implement Clone for use with tracing-subscriber
impl Clone for RotatingFileWriter {
    fn clone(&self) -> Self {
        Self {
            base_path: self.base_path.clone(),
            base_name: self.base_name.clone(),
            max_size_bytes: self.max_size_bytes,
            current_file: Arc::clone(&self.current_file),
            current_size: Arc::clone(&self.current_size),
            current_file_path: Arc::clone(&self.current_file_path),
        }
    }
}

// Implement MakeWriter for use with tracing-subscriber
impl<'a> MakeWriter<'a> for RotatingFileWriter {
    type Writer = RotatingFileWriter;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

/// Utility functions for log file management
pub struct LogFileManager;

impl LogFileManager {
    /// Get all log files in a directory matching the base name pattern
    pub fn find_log_files(log_dir: &Path, base_name: &str) -> Result<Vec<LogFileInfo>> {
        let mut log_files = Vec::new();

        if !log_dir.exists() {
            return Ok(log_files);
        }

        for entry in std::fs::read_dir(log_dir)? {
            let entry = entry?;
            let path = entry.path();

            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if file_name.starts_with(base_name) && file_name.ends_with(".log") {
                    let metadata = entry.metadata()?;
                    let modified = metadata.modified()?;
                    let size = metadata.len();
                    let is_current = file_name == format!("{}.log", base_name);

                    log_files.push(LogFileInfo {
                        path,
                        size,
                        modified,
                        is_current,
                    });
                }
            }
        }

        // Sort by modification time (newest first)
        log_files.sort_by(|a, b| b.modified.cmp(&a.modified));

        Ok(log_files)
    }

    /// Compress a log file using gzip
    pub fn compress_log_file(file_path: &Path) -> Result<PathBuf> {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::fs::File;
        use std::io::copy;

        let compressed_path = file_path.with_extension("log.gz");

        let input_file = File::open(file_path)?;
        let output_file = File::create(&compressed_path)?;
        let mut encoder = GzEncoder::new(output_file, Compression::default());

        let mut reader = std::io::BufReader::new(input_file);
        copy(&mut reader, &mut encoder)?;
        encoder.finish()?;

        // Remove original file after successful compression
        std::fs::remove_file(file_path)?;

        tracing::info!(
            telemetry.event = "log_compressed",
            original_file = %file_path.display(),
            compressed_file = %compressed_path.display(),
            "Log file compressed"
        );

        Ok(compressed_path)
    }

    /// Calculate total disk usage of log files
    pub fn calculate_total_size(log_files: &[LogFileInfo]) -> u64 {
        log_files.iter().map(|f| f.size).sum()
    }
}

/// Information about a log file
#[derive(Debug, Clone)]
pub struct LogFileInfo {
    pub path: PathBuf,
    pub size: u64,
    pub modified: SystemTime,
    pub is_current: bool,
}

impl LogFileInfo {
    /// Get the age of the log file in days
    pub fn age_days(&self) -> f64 {
        let now = SystemTime::now();
        if let Ok(duration) = now.duration_since(self.modified) {
            duration.as_secs_f64() / (24.0 * 60.0 * 60.0)
        } else {
            0.0
        }
    }

    /// Check if this is a compressed log file
    pub fn is_compressed(&self) -> bool {
        self.path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext == "gz")
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_rotating_file_writer() {
        let temp_dir = TempDir::new().unwrap();
        let mut writer = RotatingFileWriter::new(
            temp_dir.path().to_path_buf(),
            "test".to_string(),
            1, // 1MB
        )
        .unwrap();

        // Write some data
        writer.write_all(b"Test log line 1\n").unwrap();
        writer.write_all(b"Test log line 2\n").unwrap();
        writer.flush().unwrap();

        // Check that file was created
        let log_file = temp_dir.path().join("test.log");
        assert!(log_file.exists());
    }

    #[test]
    fn test_log_file_discovery() {
        let temp_dir = TempDir::new().unwrap();

        // Create some test log files
        std::fs::write(temp_dir.path().join("test.log"), "current").unwrap();
        std::fs::write(temp_dir.path().join("test_20231201_120000.log"), "old1").unwrap();
        std::fs::write(temp_dir.path().join("test_20231202_120000.log"), "old2").unwrap();
        std::fs::write(temp_dir.path().join("other.log"), "other").unwrap();

        let log_files = LogFileManager::find_log_files(temp_dir.path(), "test").unwrap();

        assert_eq!(log_files.len(), 3);
        assert!(log_files.iter().any(|f| f.is_current));
    }

    #[test]
    fn test_log_compression() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.log");

        // Create a test file
        std::fs::write(&test_file, "Test log content for compression").unwrap();

        // Compress it
        let compressed = LogFileManager::compress_log_file(&test_file).unwrap();

        // Original should be gone, compressed should exist
        assert!(!test_file.exists());
        assert!(compressed.exists());
        assert!(compressed.extension().unwrap() == "gz");
    }
}
