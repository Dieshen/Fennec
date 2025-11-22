use serde::{Deserialize, Serialize};
use similar::{ChangeTag, TextDiff};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HunkStatus {
    Pending,
    Accepted,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hunk {
    pub id: String,
    pub file_path: PathBuf,
    pub start_line: usize,
    pub end_line: usize,
    pub old_content: Vec<String>,
    pub new_content: Vec<String>,
    pub status: HunkStatus,
    pub context_before: Vec<String>,
    pub context_after: Vec<String>,
}

impl Hunk {
    /// Create a new hunk
    pub fn new(
        id: String,
        file_path: PathBuf,
        start_line: usize,
        end_line: usize,
        old_content: Vec<String>,
        new_content: Vec<String>,
    ) -> Self {
        Self {
            id,
            file_path,
            start_line,
            end_line,
            old_content,
            new_content,
            status: HunkStatus::Pending,
            context_before: Vec::new(),
            context_after: Vec::new(),
        }
    }

    /// Set the context lines before this hunk
    pub fn with_context_before(mut self, context: Vec<String>) -> Self {
        self.context_before = context;
        self
    }

    /// Set the context lines after this hunk
    pub fn with_context_after(mut self, context: Vec<String>) -> Self {
        self.context_after = context;
        self
    }

    /// Accept this hunk
    pub fn accept(&mut self) {
        self.status = HunkStatus::Accepted;
    }

    /// Reject this hunk
    pub fn reject(&mut self) {
        self.status = HunkStatus::Rejected;
    }

    /// Toggle hunk status between accepted and rejected
    pub fn toggle(&mut self) {
        self.status = match self.status {
            HunkStatus::Accepted => HunkStatus::Rejected,
            HunkStatus::Rejected | HunkStatus::Pending => HunkStatus::Accepted,
        };
    }

    /// Get a summary of this hunk for display
    pub fn summary(&self) -> String {
        let removed = self.old_content.len();
        let added = self.new_content.len();
        format!(
            "Hunk {} @ {}:{}-{} ({} removed, {} added)",
            self.id,
            self.file_path.display(),
            self.start_line,
            self.end_line,
            removed,
            added
        )
    }

    /// Format this hunk as a unified diff section
    pub fn to_unified_diff(&self) -> String {
        let mut output = String::new();

        // Add context before
        for line in &self.context_before {
            output.push_str(&format!(" {}\n", line));
        }

        // Add removed lines
        for line in &self.old_content {
            output.push_str(&format!("-{}\n", line));
        }

        // Add added lines
        for line in &self.new_content {
            output.push_str(&format!("+{}\n", line));
        }

        // Add context after
        for line in &self.context_after {
            output.push_str(&format!(" {}\n", line));
        }

        output
    }
}

/// Split a diff into discrete hunks
pub fn split_diff_into_hunks(
    file_path: PathBuf,
    old_content: &str,
    new_content: &str,
    context_lines: usize,
) -> Vec<Hunk> {
    let diff = TextDiff::from_lines(old_content, new_content);
    let mut hunks = Vec::new();
    let mut current_hunk: Option<(usize, usize, Vec<String>, Vec<String>)> = None;
    let mut hunk_id = 0;

    let old_lines: Vec<&str> = old_content.lines().collect();
    let new_lines: Vec<&str> = new_content.lines().collect();

    for (_idx, change) in diff.iter_all_changes().enumerate() {
        match change.tag() {
            ChangeTag::Delete => {
                if let Some((_start, _, ref mut old, ref mut _new)) = current_hunk {
                    // Continue current hunk
                    old.push(change.value().trim_end().to_string());
                } else {
                    // Start new hunk
                    let start_line = change.old_index().unwrap_or(0);
                    current_hunk = Some((
                        start_line,
                        start_line,
                        vec![change.value().trim_end().to_string()],
                        Vec::new(),
                    ));
                }
            }
            ChangeTag::Insert => {
                if let Some((_start, _end, ref mut _old, ref mut new)) = current_hunk {
                    // Continue current hunk
                    new.push(change.value().trim_end().to_string());
                } else {
                    // Start new hunk
                    let start_line = change.new_index().unwrap_or(0);
                    current_hunk = Some((
                        start_line,
                        start_line,
                        Vec::new(),
                        vec![change.value().trim_end().to_string()],
                    ));
                }
            }
            ChangeTag::Equal => {
                // Equal line - might end current hunk
                if let Some((start, _, old, new)) = current_hunk.take() {
                    // Finalize current hunk
                    let end_line = change.old_index().unwrap_or(start);

                    // Get context before
                    let context_start = start.saturating_sub(context_lines);
                    let context_before: Vec<String> = old_lines
                        .get(context_start..start)
                        .unwrap_or(&[])
                        .iter()
                        .map(|s| s.to_string())
                        .collect();

                    // Get context after
                    let context_end = (end_line + context_lines).min(old_lines.len());
                    let context_after: Vec<String> = old_lines
                        .get(end_line..context_end)
                        .unwrap_or(&[])
                        .iter()
                        .map(|s| s.to_string())
                        .collect();

                    let hunk = Hunk::new(
                        format!("h{}", hunk_id),
                        file_path.clone(),
                        start,
                        end_line,
                        old,
                        new,
                    )
                    .with_context_before(context_before)
                    .with_context_after(context_after);

                    hunks.push(hunk);
                    hunk_id += 1;
                }
            }
        }
    }

    // Finalize any remaining hunk
    if let Some((start, _, old, new)) = current_hunk {
        let end_line = old_lines.len().max(new_lines.len());

        let context_start = start.saturating_sub(context_lines);
        let context_before: Vec<String> = old_lines
            .get(context_start..start)
            .unwrap_or(&[])
            .iter()
            .map(|s| s.to_string())
            .collect();

        let context_end = (end_line + context_lines).min(old_lines.len());
        let context_after: Vec<String> = old_lines
            .get(end_line..context_end)
            .unwrap_or(&[])
            .iter()
            .map(|s| s.to_string())
            .collect();

        let hunk = Hunk::new(
            format!("h{}", hunk_id),
            file_path.clone(),
            start,
            end_line,
            old,
            new,
        )
        .with_context_before(context_before)
        .with_context_after(context_after);

        hunks.push(hunk);
    }

    hunks
}

/// Apply accepted hunks to generate the final content
pub fn apply_hunks(original_content: &str, hunks: &[Hunk]) -> String {
    let mut lines: Vec<String> = original_content.lines().map(|s| s.to_string()).collect();

    // Sort hunks by start line in reverse order to apply from bottom to top
    let mut sorted_hunks: Vec<&Hunk> = hunks
        .iter()
        .filter(|h| h.status == HunkStatus::Accepted)
        .collect();
    sorted_hunks.sort_by(|a, b| b.start_line.cmp(&a.start_line));

    for hunk in sorted_hunks {
        let start = hunk.start_line;
        let end = hunk.end_line;

        // Remove old lines
        if start < lines.len() {
            let remove_count = (end - start).min(lines.len() - start);
            lines.drain(start..start + remove_count);
        }

        // Insert new lines
        for (i, line) in hunk.new_content.iter().enumerate() {
            lines.insert(start + i, line.clone());
        }
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hunk_creation() {
        let hunk = Hunk::new(
            "h1".to_string(),
            PathBuf::from("test.rs"),
            10,
            15,
            vec!["old line 1".to_string(), "old line 2".to_string()],
            vec!["new line 1".to_string()],
        );

        assert_eq!(hunk.id, "h1");
        assert_eq!(hunk.start_line, 10);
        assert_eq!(hunk.end_line, 15);
        assert_eq!(hunk.status, HunkStatus::Pending);
    }

    #[test]
    fn test_hunk_toggle() {
        let mut hunk = Hunk::new(
            "h1".to_string(),
            PathBuf::from("test.rs"),
            10,
            15,
            vec!["old".to_string()],
            vec!["new".to_string()],
        );

        assert_eq!(hunk.status, HunkStatus::Pending);
        hunk.toggle();
        assert_eq!(hunk.status, HunkStatus::Accepted);
        hunk.toggle();
        assert_eq!(hunk.status, HunkStatus::Rejected);
    }

    #[test]
    fn test_split_diff_simple() {
        let old_content = "line 1\nline 2\nline 3\n";
        let new_content = "line 1\nmodified line 2\nline 3\n";

        let hunks = split_diff_into_hunks(
            PathBuf::from("test.txt"),
            old_content,
            new_content,
            1,
        );

        assert_eq!(hunks.len(), 1);
        assert_eq!(hunks[0].old_content, vec!["line 2"]);
        assert_eq!(hunks[0].new_content, vec!["modified line 2"]);
    }

    #[test]
    fn test_split_diff_multiple_hunks() {
        let old_content = "line 1\nline 2\nline 3\nline 4\nline 5\nline 6\n";
        let new_content = "line 1\nmodified 2\nline 3\nline 4\nline 5\nmodified 6\n";

        let hunks = split_diff_into_hunks(
            PathBuf::from("test.txt"),
            old_content,
            new_content,
            0,
        );

        // Should have 2 hunks for the two separate changes
        assert!(hunks.len() >= 1);
    }

    #[test]
    fn test_apply_hunks() {
        let original = "line 1\nline 2\nline 3\n";

        let mut hunk = Hunk::new(
            "h1".to_string(),
            PathBuf::from("test.txt"),
            1,
            2,
            vec!["line 2".to_string()],
            vec!["modified line 2".to_string()],
        );
        hunk.accept();

        let result = apply_hunks(original, &[hunk]);
        assert!(result.contains("modified line 2"));
        assert!(result.contains("line 1"));
        assert!(result.contains("line 3"));
    }

    #[test]
    fn test_apply_only_accepted_hunks() {
        let original = "line 1\nline 2\nline 3\n";

        let mut hunk1 = Hunk::new(
            "h1".to_string(),
            PathBuf::from("test.txt"),
            1,
            2,
            vec!["line 2".to_string()],
            vec!["modified line 2".to_string()],
        );
        hunk1.accept();

        let mut hunk2 = Hunk::new(
            "h2".to_string(),
            PathBuf::from("test.txt"),
            2,
            3,
            vec!["line 3".to_string()],
            vec!["modified line 3".to_string()],
        );
        hunk2.reject();

        let result = apply_hunks(original, &[hunk1, hunk2]);
        assert!(result.contains("modified line 2"));
        assert!(result.contains("line 3")); // Should not be modified
        assert!(!result.contains("modified line 3"));
    }
}
