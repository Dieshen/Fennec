//! Log retention and cleanup implementation

use crate::{
    config::RetentionConfig,
    rotation::{LogFileInfo, LogFileManager},
    Result,
};
use std::path::Path;
use std::time::{Duration, SystemTime};
use tokio::time::{interval, Interval};
use tracing::{error, info, warn};

/// Manages log file retention policies and cleanup
pub struct RetentionManager {
    config: RetentionConfig,
    cleanup_interval: Interval,
}

impl RetentionManager {
    /// Create a new retention manager
    pub async fn new(config: RetentionConfig) -> Result<Self> {
        let cleanup_interval = interval(Duration::from_secs(
            config.cleanup_interval_hours as u64 * 3600,
        ));

        let manager = Self {
            config,
            cleanup_interval,
        };

        info!(
            telemetry.event = "retention_manager_created",
            max_files = manager.config.max_files,
            max_age_days = manager.config.max_age_days,
            max_total_size_mb = manager.config.max_total_size_mb,
            compress_old_files = manager.config.compress_old_files,
            "Retention manager initialized"
        );

        Ok(manager)
    }

    /// Start the retention manager background task
    pub async fn start(mut self, log_dir: std::path::PathBuf, base_name: String) {
        tokio::spawn(async move {
            loop {
                self.cleanup_interval.tick().await;

                if let Err(e) = self.perform_cleanup(&log_dir, &base_name).await {
                    error!(
                        telemetry.event = "retention_cleanup_failed",
                        error = %e,
                        "Failed to perform log cleanup"
                    );
                }
            }
        });
    }

    /// Perform immediate cleanup of log files
    pub async fn perform_cleanup(&self, log_dir: &Path, base_name: &str) -> Result<CleanupReport> {
        info!(
            telemetry.event = "retention_cleanup_started",
            log_dir = %log_dir.display(),
            base_name = base_name,
            "Starting log cleanup"
        );

        let mut report = CleanupReport::default();

        // Get all log files
        let mut log_files = LogFileManager::find_log_files(log_dir, base_name)?;
        if log_files.is_empty() {
            return Ok(report);
        }

        // Calculate initial statistics
        let initial_count = log_files.len();
        let initial_size = LogFileManager::calculate_total_size(&log_files);
        report.initial_file_count = initial_count;
        report.initial_total_size_mb = initial_size / (1024 * 1024);

        // Remove current log file from cleanup consideration
        log_files.retain(|f| !f.is_current);

        // Step 1: Compress old files if enabled
        if self.config.compress_old_files {
            report.files_compressed = self.compress_old_files(&mut log_files).await?;
        }

        // Step 2: Remove files by age
        report.files_removed_by_age = self.remove_old_files(&mut log_files).await?;

        // Step 3: Remove excess files by count
        report.files_removed_by_count = self.remove_excess_files(&mut log_files).await?;

        // Step 4: Remove files to meet size limit
        report.files_removed_by_size = self.remove_files_by_size(&mut log_files).await?;

        // Calculate final statistics
        let remaining_files = LogFileManager::find_log_files(log_dir, base_name)?;
        let final_size = LogFileManager::calculate_total_size(&remaining_files);
        report.final_file_count = remaining_files.len();
        report.final_total_size_mb = final_size / (1024 * 1024);

        info!(
            telemetry.event = "retention_cleanup_completed",
            initial_files = initial_count,
            final_files = report.final_file_count,
            initial_size_mb = report.initial_total_size_mb,
            final_size_mb = report.final_total_size_mb,
            files_compressed = report.files_compressed,
            files_removed_by_age = report.files_removed_by_age,
            files_removed_by_count = report.files_removed_by_count,
            files_removed_by_size = report.files_removed_by_size,
            "Log cleanup completed"
        );

        Ok(report)
    }

    /// Compress old uncompressed log files
    async fn compress_old_files(&self, log_files: &mut Vec<LogFileInfo>) -> Result<u32> {
        let mut compressed_count = 0;

        // Find uncompressed files older than 1 day
        let one_day_ago = SystemTime::now() - Duration::from_secs(24 * 60 * 60);

        let files_to_compress: Vec<_> = log_files
            .iter()
            .enumerate()
            .filter(|(_, f)| !f.is_compressed() && f.modified < one_day_ago)
            .map(|(idx, f)| (idx, f.path.clone()))
            .collect();

        for (index, file_path) in files_to_compress {
            match LogFileManager::compress_log_file(&file_path) {
                Ok(compressed_path) => {
                    compressed_count += 1;

                    // Update the file info in our list
                    log_files[index].path = compressed_path;
                }
                Err(e) => {
                    warn!(
                        telemetry.event = "compression_failed",
                        file = %file_path.display(),
                        error = %e,
                        "Failed to compress log file"
                    );
                }
            }
        }

        if compressed_count > 0 {
            info!(
                telemetry.event = "files_compressed",
                count = compressed_count,
                "Compressed old log files"
            );
        }

        Ok(compressed_count)
    }

    /// Remove files older than the maximum age
    async fn remove_old_files(&self, log_files: &mut Vec<LogFileInfo>) -> Result<u32> {
        let mut removed_count = 0;
        let max_age_seconds = self.config.max_age_days as f64 * 24.0 * 60.0 * 60.0;

        let files_to_remove: Vec<_> = log_files
            .iter()
            .filter(|f| f.age_days() * 24.0 * 60.0 * 60.0 > max_age_seconds)
            .collect();

        for file_info in files_to_remove {
            match tokio::fs::remove_file(&file_info.path).await {
                Ok(_) => {
                    removed_count += 1;
                    info!(
                        telemetry.event = "old_file_removed",
                        file = %file_info.path.display(),
                        age_days = file_info.age_days(),
                        "Removed old log file"
                    );
                }
                Err(e) => {
                    warn!(
                        telemetry.event = "file_removal_failed",
                        file = %file_info.path.display(),
                        error = %e,
                        "Failed to remove old log file"
                    );
                }
            }
        }

        // Remove deleted files from our list
        log_files.retain(|f| f.age_days() * 24.0 * 60.0 * 60.0 <= max_age_seconds);

        Ok(removed_count)
    }

    /// Remove excess files beyond the maximum count
    async fn remove_excess_files(&self, log_files: &mut Vec<LogFileInfo>) -> Result<u32> {
        let mut removed_count = 0;

        if log_files.len() > self.config.max_files as usize {
            // Sort by modification time (oldest first for removal)
            log_files.sort_by(|a, b| a.modified.cmp(&b.modified));

            let excess_count = log_files.len() - self.config.max_files as usize;
            let files_to_remove = log_files.drain(..excess_count).collect::<Vec<_>>();

            for file_info in files_to_remove {
                match tokio::fs::remove_file(&file_info.path).await {
                    Ok(_) => {
                        removed_count += 1;
                        info!(
                            telemetry.event = "excess_file_removed",
                            file = %file_info.path.display(),
                            "Removed excess log file"
                        );
                    }
                    Err(e) => {
                        warn!(
                            telemetry.event = "file_removal_failed",
                            file = %file_info.path.display(),
                            error = %e,
                            "Failed to remove excess log file"
                        );
                    }
                }
            }
        }

        Ok(removed_count)
    }

    /// Remove files to meet total size limit
    async fn remove_files_by_size(&self, log_files: &mut Vec<LogFileInfo>) -> Result<u32> {
        let mut removed_count = 0;
        let max_total_bytes = self.config.max_total_size_mb * 1024 * 1024;
        let current_total = LogFileManager::calculate_total_size(log_files);

        if current_total > max_total_bytes {
            // Sort by modification time (oldest first for removal)
            log_files.sort_by(|a, b| a.modified.cmp(&b.modified));

            let mut total_size = current_total;
            let mut files_to_remove = Vec::new();

            for file_info in log_files.iter() {
                if total_size <= max_total_bytes {
                    break;
                }

                files_to_remove.push(file_info.clone());
                total_size -= file_info.size;
            }

            for file_info in &files_to_remove {
                match tokio::fs::remove_file(&file_info.path).await {
                    Ok(_) => {
                        removed_count += 1;
                        info!(
                            telemetry.event = "oversized_file_removed",
                            file = %file_info.path.display(),
                            size_mb = file_info.size / (1024 * 1024),
                            "Removed log file to meet size limit"
                        );
                    }
                    Err(e) => {
                        warn!(
                            telemetry.event = "file_removal_failed",
                            file = %file_info.path.display(),
                            error = %e,
                            "Failed to remove oversized log file"
                        );
                    }
                }
            }

            // Remove deleted files from our list
            for removed_file in &files_to_remove {
                log_files.retain(|f| f.path != removed_file.path);
            }
        }

        Ok(removed_count)
    }

    /// Perform emergency cleanup to free disk space
    pub async fn emergency_cleanup(
        &self,
        log_dir: &Path,
        base_name: &str,
        target_size_mb: u64,
    ) -> Result<CleanupReport> {
        warn!(
            telemetry.event = "emergency_cleanup_started",
            target_size_mb = target_size_mb,
            "Starting emergency log cleanup"
        );

        let mut log_files = LogFileManager::find_log_files(log_dir, base_name)?;
        log_files.retain(|f| !f.is_current); // Don't remove current log

        // Sort by modification time (oldest first)
        log_files.sort_by(|a, b| a.modified.cmp(&b.modified));

        let target_bytes = target_size_mb * 1024 * 1024;
        let mut current_size = LogFileManager::calculate_total_size(&log_files);
        let mut removed_count = 0;

        while current_size > target_bytes && !log_files.is_empty() {
            let file_to_remove = log_files.remove(0);

            match tokio::fs::remove_file(&file_to_remove.path).await {
                Ok(_) => {
                    current_size -= file_to_remove.size;
                    removed_count += 1;
                    warn!(
                        telemetry.event = "emergency_file_removed",
                        file = %file_to_remove.path.display(),
                        "Emergency removal of log file"
                    );
                }
                Err(e) => {
                    error!(
                        telemetry.event = "emergency_removal_failed",
                        file = %file_to_remove.path.display(),
                        error = %e,
                        "Failed to remove file during emergency cleanup"
                    );
                }
            }
        }

        let mut report = CleanupReport::default();
        report.files_removed_by_size = removed_count;
        report.final_total_size_mb = current_size / (1024 * 1024);

        warn!(
            telemetry.event = "emergency_cleanup_completed",
            files_removed = removed_count,
            final_size_mb = report.final_total_size_mb,
            "Emergency cleanup completed"
        );

        Ok(report)
    }
}

/// Report of cleanup operations performed
#[derive(Debug, Default)]
pub struct CleanupReport {
    pub initial_file_count: usize,
    pub final_file_count: usize,
    pub initial_total_size_mb: u64,
    pub final_total_size_mb: u64,
    pub files_compressed: u32,
    pub files_removed_by_age: u32,
    pub files_removed_by_count: u32,
    pub files_removed_by_size: u32,
}

impl CleanupReport {
    pub fn total_files_removed(&self) -> u32 {
        self.files_removed_by_age + self.files_removed_by_count + self.files_removed_by_size
    }

    pub fn space_freed_mb(&self) -> i64 {
        self.initial_total_size_mb as i64 - self.final_total_size_mb as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RetentionConfig;
    use std::time::{Duration, SystemTime};
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_retention_manager_creation() {
        let config = RetentionConfig {
            max_files: 5,
            max_age_days: 7,
            compress_old_files: true,
            cleanup_interval_hours: 1,
            max_total_size_mb: 100,
        };

        let manager = RetentionManager::new(config).await.unwrap();
        assert_eq!(manager.config.max_files, 5);
    }

    #[tokio::test]
    async fn test_cleanup_by_count() {
        let temp_dir = TempDir::new().unwrap();

        // Create 10 test files
        for i in 0..10 {
            let file_path = temp_dir.path().join(format!("test_{:02}.log", i));
            tokio::fs::write(&file_path, format!("log content {}", i))
                .await
                .unwrap();

            // Set different modification times
            let time = SystemTime::now() - Duration::from_secs(i * 3600);
            filetime::set_file_mtime(&file_path, filetime::FileTime::from_system_time(time))
                .unwrap();
        }

        let config = RetentionConfig {
            max_files: 5,
            max_age_days: 365, // Don't remove by age
            compress_old_files: false,
            cleanup_interval_hours: 1,
            max_total_size_mb: 1000, // Don't remove by size
        };

        let manager = RetentionManager::new(config).await.unwrap();
        let report = manager
            .perform_cleanup(temp_dir.path(), "test")
            .await
            .unwrap();

        assert_eq!(report.files_removed_by_count, 5);
        assert_eq!(report.final_file_count, 5);
    }

    #[tokio::test]
    async fn test_cleanup_by_age() {
        let temp_dir = TempDir::new().unwrap();

        // Create files with different ages
        for i in 0..5 {
            let file_path = temp_dir.path().join(format!("test_{}.log", i));
            tokio::fs::write(&file_path, "log content").await.unwrap();

            // Set modification time (some older than max age)
            let days_old = if i < 3 { 40 } else { 5 }; // First 3 files are old
            let time = SystemTime::now() - Duration::from_secs(days_old * 24 * 3600);
            filetime::set_file_mtime(&file_path, filetime::FileTime::from_system_time(time))
                .unwrap();
        }

        let config = RetentionConfig {
            max_files: 100, // Don't remove by count
            max_age_days: 30,
            compress_old_files: false,
            cleanup_interval_hours: 1,
            max_total_size_mb: 1000, // Don't remove by size
        };

        let manager = RetentionManager::new(config).await.unwrap();
        let report = manager
            .perform_cleanup(temp_dir.path(), "test")
            .await
            .unwrap();

        assert_eq!(report.files_removed_by_age, 3);
        assert_eq!(report.final_file_count, 2);
    }
}
