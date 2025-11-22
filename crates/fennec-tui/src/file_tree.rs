use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, StatefulWidget},
};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Represents a node in the file tree
#[derive(Debug, Clone)]
pub struct FileNode {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub depth: usize,
    pub children: Vec<FileNode>,
    pub is_hidden: bool,
}

impl FileNode {
    /// Create a new file node
    pub fn new(path: PathBuf, depth: usize) -> std::io::Result<Self> {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let is_hidden = name.starts_with('.');
        let metadata = fs::metadata(&path)?;
        let is_dir = metadata.is_dir();

        Ok(Self {
            path,
            name,
            is_dir,
            depth,
            children: Vec::new(),
            is_hidden,
        })
    }

    /// Load children for this directory node
    pub fn load_children(&mut self, show_hidden: bool, max_depth: usize) -> std::io::Result<()> {
        if !self.is_dir || self.depth >= max_depth {
            return Ok(());
        }

        let mut entries: Vec<_> = fs::read_dir(&self.path)?
            .filter_map(|entry| entry.ok())
            .collect();

        // Sort: directories first, then by name
        entries.sort_by(|a, b| {
            let a_is_dir = a.path().is_dir();
            let b_is_dir = b.path().is_dir();

            match (a_is_dir, b_is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.file_name().cmp(&b.file_name()),
            }
        });

        for entry in entries {
            let path = entry.path();

            // Skip ignored directories
            if let Some(name) = path.file_name() {
                let name_str = name.to_string_lossy();
                if should_ignore(&name_str) {
                    continue;
                }

                if !show_hidden && name_str.starts_with('.') {
                    continue;
                }
            }

            if let Ok(node) = FileNode::new(path, self.depth + 1) {
                self.children.push(node);
            }
        }

        Ok(())
    }

    /// Get total number of visible nodes (including descendants)
    pub fn count_visible(&self, expanded: &HashSet<PathBuf>) -> usize {
        let mut count = 1; // Count self

        if self.is_dir && expanded.contains(&self.path) {
            for child in &self.children {
                count += child.count_visible(expanded);
            }
        }

        count
    }
}

/// File tree browser component
#[derive(Debug, Clone)]
pub struct FileTreeBrowser {
    root: FileNode,
    expanded: HashSet<PathBuf>,
    selected_index: usize,
    show_hidden: bool,
    max_depth: usize,
    list_state: ListState,
}

impl FileTreeBrowser {
    /// Create a new file tree browser
    pub fn new(root_path: PathBuf) -> std::io::Result<Self> {
        let mut root = FileNode::new(root_path.clone(), 0)?;
        root.load_children(false, 10)?;

        let mut expanded = HashSet::new();
        expanded.insert(root_path); // Root is always expanded

        Ok(Self {
            root,
            expanded,
            selected_index: 0,
            show_hidden: false,
            max_depth: 10,
            list_state: ListState::default(),
        })
    }

    /// Toggle expansion of selected directory
    pub fn toggle_expand(&mut self) {
        if let Some(node) = self.get_node_at_index(self.selected_index) {
            if node.is_dir {
                let path = node.path.clone();
                let show_hidden = self.show_hidden;
                let max_depth = self.max_depth;

                if self.expanded.contains(&path) {
                    self.expanded.remove(&path);
                } else {
                    self.expanded.insert(path.clone());

                    // Lazy load children if not already loaded
                    if let Some(node_mut) = self.get_node_mut_at_path(&path) {
                        if node_mut.children.is_empty() {
                            let _ = node_mut.load_children(show_hidden, max_depth);
                        }
                    }
                }
            }
        }
    }

    /// Toggle showing hidden files
    pub fn toggle_hidden(&mut self) {
        self.show_hidden = !self.show_hidden;
        // Reload all expanded nodes
        let expanded_paths: Vec<_> = self.expanded.iter().cloned().collect();
        let show_hidden = self.show_hidden;
        let max_depth = self.max_depth;

        for path in expanded_paths {
            if let Some(node) = self.get_node_mut_at_path(&path) {
                let _ = node.load_children(show_hidden, max_depth);
            }
        }
    }

    /// Move selection up
    pub fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Move selection down
    pub fn move_down(&mut self) {
        let total = self.root.count_visible(&self.expanded);
        if self.selected_index < total.saturating_sub(1) {
            self.selected_index += 1;
        }
    }

    /// Move to first item
    pub fn move_to_top(&mut self) {
        self.selected_index = 0;
    }

    /// Move to last item
    pub fn move_to_bottom(&mut self) {
        let total = self.root.count_visible(&self.expanded);
        self.selected_index = total.saturating_sub(1);
    }

    /// Get the currently selected path
    pub fn get_selected_path(&self) -> Option<PathBuf> {
        self.get_node_at_index(self.selected_index)
            .map(|node| node.path.clone())
    }

    /// Get node at specific index (considering expanded state)
    fn get_node_at_index(&self, index: usize) -> Option<&FileNode> {
        let mut current_index = 0;
        self.find_node_at_index(&self.root, index, &mut current_index)
    }

    /// Recursive helper to find node at index
    fn find_node_at_index<'a>(
        &self,
        node: &'a FileNode,
        target_index: usize,
        current_index: &mut usize,
    ) -> Option<&'a FileNode> {
        if *current_index == target_index {
            return Some(node);
        }

        *current_index += 1;

        if node.is_dir && self.expanded.contains(&node.path) {
            for child in &node.children {
                if let Some(found) = self.find_node_at_index(child, target_index, current_index) {
                    return Some(found);
                }
            }
        }

        None
    }

    /// Get mutable reference to node at path
    fn get_node_mut_at_path(&mut self, path: &Path) -> Option<&mut FileNode> {
        Self::find_node_mut_at_path_helper(&mut self.root, path)
    }

    /// Recursive helper to find mutable node
    fn find_node_mut_at_path_helper<'a>(
        node: &'a mut FileNode,
        path: &Path,
    ) -> Option<&'a mut FileNode> {
        if node.path == path {
            return Some(node);
        }

        for child in &mut node.children {
            if let Some(found) = Self::find_node_mut_at_path_helper(child, path) {
                return Some(found);
            }
        }

        None
    }

    /// Render the file tree
    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        // Build title
        let title = format!(
            " Files {} ",
            if self.show_hidden {
                "(showing hidden)"
            } else {
                ""
            }
        );

        // Build items without borrowing self mutably
        let mut items = Vec::new();
        let selected_index = self.selected_index;
        let expanded = &self.expanded;
        Self::collect_items_helper(&self.root, &mut items, 0, selected_index, expanded);

        // Update list state to show selection
        self.list_state.select(Some(self.selected_index));

        let list = List::new(items).block(Block::default().borders(Borders::ALL).title(title));

        StatefulWidget::render(list, area, buf, &mut self.list_state);
    }

    /// Helper to collect items without mutable borrow
    fn collect_items_helper(
        node: &FileNode,
        items: &mut Vec<ListItem>,
        index: usize,
        selected_index: usize,
        expanded: &HashSet<PathBuf>,
    ) {
        // Build the display line
        let indent = "  ".repeat(node.depth);

        let icon = if node.is_dir {
            if expanded.contains(&node.path) {
                "📂"
            } else {
                "📁"
            }
        } else {
            get_file_icon(&node.name)
        };

        let display_name = format!("{}{} {}", indent, icon, node.name);

        // Style based on selection and type
        let style = if index == selected_index {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else if node.is_dir {
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        items.push(ListItem::new(Line::from(Span::styled(display_name, style))));

        // Add children if expanded
        if node.is_dir && expanded.contains(&node.path) {
            let mut child_index = index + 1;
            for child in &node.children {
                Self::collect_items_helper(child, items, child_index, selected_index, expanded);
                child_index += child.count_visible(expanded);
            }
        }
    }
}

/// Get file icon based on file extension
fn get_file_icon(filename: &str) -> &'static str {
    if let Some(ext) = filename.rsplit('.').next() {
        match ext {
            "rs" => "🦀",
            "toml" => "⚙️",
            "md" => "📝",
            "txt" => "📄",
            "json" => "📋",
            "yaml" | "yml" => "📋",
            "lock" => "🔒",
            "sh" => "🐚",
            "py" => "🐍",
            "js" | "ts" => "📜",
            "html" | "css" => "🌐",
            _ => "📄",
        }
    } else {
        "📄"
    }
}

/// Check if a path should be ignored
fn should_ignore(name: &str) -> bool {
    matches!(
        name,
        "target" | "node_modules" | ".git" | "dist" | "build" | "__pycache__"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_file_node_creation() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_path_buf();

        let node = FileNode::new(path.clone(), 0).unwrap();
        assert_eq!(node.path, path);
        assert!(node.is_dir);
        assert_eq!(node.depth, 0);
    }

    #[test]
    fn test_load_children() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        // Create some test files
        fs::write(path.join("file1.txt"), "test").unwrap();
        fs::write(path.join("file2.rs"), "test").unwrap();
        fs::create_dir(path.join("subdir")).unwrap();

        let mut node = FileNode::new(path.to_path_buf(), 0).unwrap();
        node.load_children(false, 10).unwrap();

        assert_eq!(node.children.len(), 3);
    }

    #[test]
    fn test_ignore_patterns() {
        assert!(should_ignore("target"));
        assert!(should_ignore("node_modules"));
        assert!(should_ignore(".git"));
        assert!(!should_ignore("src"));
    }

    #[test]
    fn test_file_tree_browser_creation() {
        let temp_dir = TempDir::new().unwrap();
        let browser = FileTreeBrowser::new(temp_dir.path().to_path_buf());
        assert!(browser.is_ok());
    }

    #[test]
    fn test_toggle_expand() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir(temp_dir.path().join("subdir")).unwrap();

        let mut browser = FileTreeBrowser::new(temp_dir.path().to_path_buf()).unwrap();
        let initial_expanded = browser.expanded.len();

        // Toggle root (should collapse it)
        browser.toggle_expand();
        assert_eq!(browser.expanded.len(), initial_expanded - 1);

        // Toggle again (should expand it)
        browser.toggle_expand();
        assert_eq!(browser.expanded.len(), initial_expanded);
    }

    #[test]
    fn test_navigation() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("file1.txt"), "test").unwrap();
        fs::write(temp_dir.path().join("file2.txt"), "test").unwrap();

        let mut browser = FileTreeBrowser::new(temp_dir.path().to_path_buf()).unwrap();

        assert_eq!(browser.selected_index, 0);
        browser.move_down();
        assert_eq!(browser.selected_index, 1);
        browser.move_up();
        assert_eq!(browser.selected_index, 0);
    }

    #[test]
    fn test_get_selected_path() {
        let temp_dir = TempDir::new().unwrap();
        let browser = FileTreeBrowser::new(temp_dir.path().to_path_buf()).unwrap();

        let selected = browser.get_selected_path();
        assert!(selected.is_some());
        assert_eq!(selected.unwrap(), temp_dir.path());
    }
}
