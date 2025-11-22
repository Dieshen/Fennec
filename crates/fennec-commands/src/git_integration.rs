use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;

/// Represents a git commit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitCommit {
    pub hash: String,
    pub author: String,
    pub email: String,
    pub date: String,
    pub message: String,
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
}

/// Represents file changes in a commit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    pub path: PathBuf,
    pub change_type: ChangeType,
    pub insertions: usize,
    pub deletions: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChangeType {
    Added,
    Modified,
    Deleted,
    Renamed,
}

/// Parse git log output to extract commits
pub async fn get_commits(
    repo_path: &str,
    branch: Option<&str>,
    limit: Option<usize>,
) -> Result<Vec<GitCommit>, std::io::Error> {
    let mut cmd = Command::new("git");
    cmd.current_dir(repo_path);
    cmd.arg("log");

    if let Some(branch) = branch {
        cmd.arg(branch);
    }

    if let Some(limit) = limit {
        cmd.arg(format!("-{}", limit));
    }

    // Format: hash|author|email|date|message
    cmd.arg("--pretty=format:%H|%an|%ae|%ai|%s");
    cmd.arg("--shortstat");

    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::null());

    let output = cmd.output().await?;

    if !output.status.success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to get git log",
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_git_log(&stdout)
}

fn parse_git_log(log: &str) -> Result<Vec<GitCommit>, std::io::Error> {
    let mut commits = Vec::new();
    let lines: Vec<&str> = log.lines().collect();

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim();

        if line.is_empty() {
            i += 1;
            continue;
        }

        // Parse commit line
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 5 {
            let hash = parts[0].to_string();
            let author = parts[1].to_string();
            let email = parts[2].to_string();
            let date = parts[3].to_string();
            let message = parts[4..].join("|");

            // Look for stats line
            let mut files_changed = 0;
            let mut insertions = 0;
            let mut deletions = 0;

            if i + 1 < lines.len() {
                let stats_line = lines[i + 1].trim();
                if stats_line.contains("file")
                    || stats_line.contains("insertion")
                    || stats_line.contains("deletion")
                {
                    // Parse stats: "1 file changed, 5 insertions(+), 2 deletions(-)"
                    if let Some(files) = stats_line.split("file").next() {
                        files_changed = files.trim().parse().unwrap_or(0);
                    }

                    if let Some(ins) = stats_line.split("insertion").next() {
                        if let Some(num) = ins.split(',').last() {
                            insertions = num.trim().parse().unwrap_or(0);
                        }
                    }

                    if let Some(del) = stats_line.split("deletion").next() {
                        if let Some(num) = del.split(',').last() {
                            deletions = num.trim().parse().unwrap_or(0);
                        }
                    }

                    i += 1; // Skip stats line
                }
            }

            commits.push(GitCommit {
                hash,
                author,
                email,
                date,
                message,
                files_changed,
                insertions,
                deletions,
            });
        }

        i += 1;
    }

    Ok(commits)
}

/// Get the current branch name
pub async fn get_current_branch(repo_path: &str) -> Result<String, std::io::Error> {
    let output = Command::new("git")
        .current_dir(repo_path)
        .arg("branch")
        .arg("--show-current")
        .output()
        .await?;

    if !output.status.success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to get current branch",
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Get the diff between two commits or branches
pub async fn get_diff(
    repo_path: &str,
    from: &str,
    to: Option<&str>,
) -> Result<String, std::io::Error> {
    let mut cmd = Command::new("git");
    cmd.current_dir(repo_path);
    cmd.arg("diff");
    cmd.arg(from);

    if let Some(to) = to {
        cmd.arg(to);
    }

    cmd.stdout(Stdio::piped());

    let output = cmd.output().await?;

    if !output.status.success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to get git diff",
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Generate a PR summary from commits
pub fn generate_pr_summary(commits: &[GitCommit]) -> String {
    if commits.is_empty() {
        return "No commits found.".to_string();
    }

    let mut summary = String::new();

    // Summary header
    summary.push_str("## Summary\n\n");
    summary.push_str(&format!(
        "This PR includes {} commit(s).\n\n",
        commits.len()
    ));

    // Overall stats
    let total_files: usize = commits.iter().map(|c| c.files_changed).sum();
    let total_insertions: usize = commits.iter().map(|c| c.insertions).sum();
    let total_deletions: usize = commits.iter().map(|c| c.deletions).sum();

    summary.push_str(&format!("- **Files changed**: {}\n", total_files));
    summary.push_str(&format!("- **Insertions**: +{}\n", total_insertions));
    summary.push_str(&format!("- **Deletions**: -{}\n\n", total_deletions));

    // Commit list
    summary.push_str("## Commits\n\n");
    for commit in commits {
        summary.push_str(&format!(
            "- {} - {} ({})\n",
            &commit.hash[..7],
            commit.message,
            commit.author
        ));
    }

    summary.push_str("\n## Changes by Author\n\n");

    // Group by author
    let mut authors: std::collections::HashMap<String, Vec<&GitCommit>> =
        std::collections::HashMap::new();
    for commit in commits {
        authors
            .entry(commit.author.clone())
            .or_default()
            .push(commit);
    }

    for (author, author_commits) in authors {
        summary.push_str(&format!(
            "### {} ({} commit{})\n\n",
            author,
            author_commits.len(),
            if author_commits.len() == 1 { "" } else { "s" }
        ));

        for commit in author_commits {
            summary.push_str(&format!("- {}\n", commit.message));
        }
        summary.push('\n');
    }

    summary
}

/// Generate a commit message template based on staged changes
pub async fn generate_commit_template(repo_path: &str) -> Result<String, std::io::Error> {
    // Get list of staged files
    let output = Command::new("git")
        .current_dir(repo_path)
        .arg("diff")
        .arg("--cached")
        .arg("--name-status")
        .output()
        .await?;

    if !output.status.success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to get staged changes",
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    if lines.is_empty() {
        return Ok(
            "No staged changes found.\n\nPlease stage your changes with 'git add' first."
                .to_string(),
        );
    }

    // Analyze changes
    let mut added_files = Vec::new();
    let mut modified_files = Vec::new();
    let mut deleted_files = Vec::new();

    for line in lines {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let status = parts[0];
            let file = parts[1];

            match status {
                "A" => added_files.push(file),
                "M" => modified_files.push(file),
                "D" => deleted_files.push(file),
                _ => {}
            }
        }
    }

    // Determine commit type
    let commit_type =
        if !added_files.is_empty() && modified_files.is_empty() && deleted_files.is_empty() {
            "feat"
        } else if !deleted_files.is_empty() && added_files.is_empty() && modified_files.is_empty() {
            "chore"
        } else if modified_files.iter().any(|f| f.contains("test")) {
            "test"
        } else if modified_files
            .iter()
            .any(|f| f.contains(".md") || f.contains("README"))
        {
            "docs"
        } else {
            "fix"
        };

    // Generate template
    let mut template = String::new();
    template.push_str(&format!("{}: ", commit_type));

    // Add brief description placeholder
    template.push_str("<brief description>\n\n");

    // Add detailed description
    template.push_str("## Changes\n\n");

    if !added_files.is_empty() {
        template.push_str("**Added:**\n");
        for file in &added_files {
            template.push_str(&format!("- {}\n", file));
        }
        template.push('\n');
    }

    if !modified_files.is_empty() {
        template.push_str("**Modified:**\n");
        for file in &modified_files {
            template.push_str(&format!("- {}\n", file));
        }
        template.push('\n');
    }

    if !deleted_files.is_empty() {
        template.push_str("**Deleted:**\n");
        for file in &deleted_files {
            template.push_str(&format!("- {}\n", file));
        }
        template.push('\n');
    }

    template.push_str("## Description\n\n");
    template.push_str("<detailed description of what changed and why>\n\n");

    template.push_str("## Testing\n\n");
    template.push_str("<how this was tested>\n");

    Ok(template)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_git_log_single_commit() {
        let log = "abc123|John Doe|john@example.com|2024-01-15 10:00:00|feat: Add new feature\n 1 file changed, 5 insertions(+), 2 deletions(-)";

        let commits = parse_git_log(log).unwrap();
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].hash, "abc123");
        assert_eq!(commits[0].author, "John Doe");
        assert_eq!(commits[0].message, "feat: Add new feature");
        assert_eq!(commits[0].files_changed, 1);
    }

    #[test]
    fn test_parse_git_log_multiple_commits() {
        let log = "abc123|John Doe|john@example.com|2024-01-15 10:00:00|First commit\n 1 file changed, 5 insertions(+)\n\ndef456|Jane Smith|jane@example.com|2024-01-16 11:00:00|Second commit\n 2 files changed, 10 insertions(+), 3 deletions(-)";

        let commits = parse_git_log(log).unwrap();
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].hash, "abc123");
        assert_eq!(commits[1].hash, "def456");
    }

    #[test]
    fn test_generate_pr_summary() {
        let commits = vec![
            GitCommit {
                hash: "abc123def".to_string(),
                author: "John Doe".to_string(),
                email: "john@example.com".to_string(),
                date: "2024-01-15".to_string(),
                message: "feat: Add feature A".to_string(),
                files_changed: 2,
                insertions: 10,
                deletions: 3,
            },
            GitCommit {
                hash: "def456ghi".to_string(),
                author: "John Doe".to_string(),
                email: "john@example.com".to_string(),
                date: "2024-01-16".to_string(),
                message: "fix: Fix bug B".to_string(),
                files_changed: 1,
                insertions: 5,
                deletions: 2,
            },
        ];

        let summary = generate_pr_summary(&commits);
        assert!(summary.contains("2 commit(s)"));
        assert!(summary.contains("Files changed"));
        assert!(summary.contains("feat: Add feature A"));
        assert!(summary.contains("fix: Fix bug B"));
        assert!(summary.contains("John Doe"));
    }

    #[test]
    fn test_generate_pr_summary_empty() {
        let summary = generate_pr_summary(&[]);
        assert_eq!(summary, "No commits found.");
    }
}
