use crate::events::InputMode;
use crate::theme::{ComponentType, ThemeManager};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Clear, Gauge, List, ListItem, ListState, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState, StatefulWidget, Widget, Wrap,
    },
};

/// Represents a chat message
#[derive(Debug, Clone)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    pub timestamp: String,
}

/// Role of the message sender
#[derive(Debug, Clone, PartialEq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

/// Chat view component for displaying conversation history
#[derive(Debug, Clone)]
pub struct ChatView {
    messages: Vec<Message>,
    scroll_state: ScrollbarState,
    selected_index: Option<usize>,
    auto_scroll: bool,
}

impl Default for ChatView {
    fn default() -> Self {
        Self::new()
    }
}

impl ChatView {
    /// Create a new chat view
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            scroll_state: ScrollbarState::default(),
            selected_index: None,
            auto_scroll: true,
        }
    }

    /// Add a message to the chat
    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
        if self.auto_scroll {
            self.scroll_to_bottom();
        }
    }

    /// Get all messages
    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    /// Clear all messages
    pub fn clear(&mut self) {
        self.messages.clear();
        self.scroll_state = ScrollbarState::default();
        self.selected_index = None;
    }

    /// Scroll up
    pub fn scroll_up(&mut self, lines: usize) {
        self.auto_scroll = false;
        // In ratatui 0.24, ScrollbarState methods modify in-place
        for _ in 0..lines {
            self.scroll_state.prev();
        }
    }

    /// Scroll down
    pub fn scroll_down(&mut self, lines: usize) {
        // Use next() method multiple times
        for _ in 0..lines {
            self.scroll_state.next();
        }

        // For simplicity, we'll assume we're at bottom if we've scrolled recently
        // In a real implementation, you'd want to track the position more carefully
    }

    /// Scroll to top
    pub fn scroll_to_top(&mut self) {
        self.auto_scroll = false;
        self.scroll_state.first();
    }

    /// Scroll to bottom
    pub fn scroll_to_bottom(&mut self) {
        self.auto_scroll = true;
        self.scroll_state.last();
    }

    /// Render the chat view
    pub fn render(&mut self, area: Rect, buf: &mut Buffer, theme: &ThemeManager, focused: bool) {
        let border_style = if focused {
            theme.get_style(ComponentType::Highlight)
        } else {
            theme.get_style(ComponentType::Border)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title("Chat")
            .title_style(theme.get_style(ComponentType::Title))
            .border_style(border_style);

        let inner = block.inner(area);
        block.render(area, buf);

        if self.messages.is_empty() {
            let empty_text = Paragraph::new("No messages yet. Type 'i' to start chatting.")
                .style(theme.get_style(ComponentType::Muted))
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true });
            empty_text.render(inner, buf);
            return;
        }

        // Convert messages to list items
        let items: Vec<ListItem> = self
            .messages
            .iter()
            .map(|msg| self.message_to_list_item(msg, theme))
            .collect();

        let list = List::new(items)
            .style(theme.get_style(ComponentType::Text))
            .highlight_style(theme.get_style(ComponentType::Selection));

        let mut list_state = ListState::default();
        if let Some(selected) = self.selected_index {
            list_state.select(Some(selected));
        }

        StatefulWidget::render(list, inner, buf, &mut list_state);

        // Render scrollbar
        self.scroll_state = self.scroll_state.content_length(self.messages.len());
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"))
            .track_style(theme.get_style(ComponentType::ScrollbarTrack))
            .thumb_style(theme.get_style(ComponentType::ScrollbarThumb));

        StatefulWidget::render(scrollbar, area, buf, &mut self.scroll_state);
    }

    /// Convert a message to a list item
    fn message_to_list_item<'a>(&self, message: &'a Message, theme: &ThemeManager) -> ListItem<'a> {
        let role_style = match message.role {
            MessageRole::User => theme.get_style(ComponentType::ChatUser),
            MessageRole::Assistant => theme.get_style(ComponentType::ChatAssistant),
            MessageRole::System => theme.get_style(ComponentType::ChatSystem),
        };

        let role_prefix = match message.role {
            MessageRole::User => "[You]",
            MessageRole::Assistant => "[Assistant]",
            MessageRole::System => "[System]",
        };

        let content = Text::from(vec![
            Line::from(vec![
                Span::styled(role_prefix, role_style),
                Span::styled(
                    format!(" {}", message.timestamp),
                    theme.get_style(ComponentType::Muted),
                ),
            ]),
            Line::from(Span::styled(
                &message.content,
                theme.get_style(ComponentType::Text),
            )),
            Line::from(""), // Empty line for spacing
        ]);

        ListItem::new(content)
    }
}

/// Input field component for text entry
#[derive(Debug, Clone)]
pub struct InputField {
    content: String,
    cursor_position: usize,
    scroll_offset: usize,
    placeholder: String,
}

impl Default for InputField {
    fn default() -> Self {
        Self::new()
    }
}

impl InputField {
    /// Create a new input field
    pub fn new() -> Self {
        Self {
            content: String::new(),
            cursor_position: 0,
            scroll_offset: 0,
            placeholder: "Type your message...".to_string(),
        }
    }

    /// Set placeholder text
    pub fn with_placeholder(mut self, placeholder: String) -> Self {
        self.placeholder = placeholder;
        self
    }

    /// Get the current content
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Set the content
    pub fn set_content(&mut self, content: String) {
        self.content = content;
        self.cursor_position = self.content.len();
        self.update_scroll();
    }

    /// Clear the content
    pub fn clear(&mut self) {
        self.content.clear();
        self.cursor_position = 0;
        self.scroll_offset = 0;
    }

    /// Insert a character at the cursor position
    pub fn insert_char(&mut self, c: char) {
        self.content.insert(self.cursor_position, c);
        self.cursor_position += 1;
        self.update_scroll();
    }

    /// Delete character before cursor (backspace)
    pub fn backspace(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
            self.content.remove(self.cursor_position);
            self.update_scroll();
        }
    }

    /// Delete character at cursor (delete)
    pub fn delete(&mut self) {
        if self.cursor_position < self.content.len() {
            self.content.remove(self.cursor_position);
        }
    }

    /// Move cursor left
    pub fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
            self.update_scroll();
        }
    }

    /// Move cursor right
    pub fn move_cursor_right(&mut self) {
        if self.cursor_position < self.content.len() {
            self.cursor_position += 1;
            self.update_scroll();
        }
    }

    /// Move cursor to beginning
    pub fn move_cursor_to_start(&mut self) {
        self.cursor_position = 0;
        self.scroll_offset = 0;
    }

    /// Move cursor to end
    pub fn move_cursor_to_end(&mut self) {
        self.cursor_position = self.content.len();
        self.update_scroll();
    }

    /// Update scroll offset based on cursor position
    fn update_scroll(&mut self) {
        // This is a simplified scroll calculation
        // In a real implementation, you'd want to consider the actual widget width
        let visible_width = 50; // Placeholder
        if self.cursor_position < self.scroll_offset {
            self.scroll_offset = self.cursor_position;
        } else if self.cursor_position >= self.scroll_offset + visible_width {
            self.scroll_offset = self.cursor_position.saturating_sub(visible_width) + 1;
        }
    }

    /// Render the input field
    pub fn render(&self, area: Rect, buf: &mut Buffer, theme: &ThemeManager, mode: InputMode) {
        let (title, border_style) = match mode {
            InputMode::Insert => ("Input (INSERT)", theme.get_style(ComponentType::Highlight)),
            InputMode::Command => ("Command", theme.get_style(ComponentType::Warning)),
            InputMode::Search => ("Search", theme.get_style(ComponentType::Info)),
            InputMode::Normal => ("Input", theme.get_style(ComponentType::Border)),
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .title_style(theme.get_style(ComponentType::Title))
            .border_style(border_style);

        let inner = block.inner(area);
        block.render(area, buf);

        let text = if self.content.is_empty() && mode == InputMode::Normal {
            Span::styled(&self.placeholder, theme.get_style(ComponentType::Muted))
        } else {
            Span::styled(&self.content, theme.get_style(ComponentType::Text))
        };

        let paragraph = Paragraph::new(text).wrap(Wrap { trim: false });

        paragraph.render(inner, buf);

        // Render cursor in insert mode
        if mode == InputMode::Insert || mode == InputMode::Command || mode == InputMode::Search {
            if let Some(cursor_area) = self.get_cursor_area(inner) {
                buf.set_style(
                    cursor_area,
                    Style::default().add_modifier(Modifier::REVERSED),
                );
            }
        }
    }

    /// Get the cursor area for rendering
    fn get_cursor_area(&self, area: Rect) -> Option<Rect> {
        if area.width == 0 || area.height == 0 {
            return None;
        }

        let visible_cursor_pos = self.cursor_position.saturating_sub(self.scroll_offset);
        if visible_cursor_pos < area.width as usize {
            Some(Rect {
                x: area.x + visible_cursor_pos as u16,
                y: area.y,
                width: 1,
                height: 1,
            })
        } else {
            None
        }
    }
}

/// Status bar component
#[derive(Debug, Clone)]
pub struct StatusBar {
    left_items: Vec<StatusItem>,
    right_items: Vec<StatusItem>,
}

/// Individual status item
#[derive(Debug, Clone)]
pub struct StatusItem {
    pub label: String,
    pub value: String,
    pub style: ComponentType,
}

impl StatusBar {
    /// Create a new status bar
    pub fn new() -> Self {
        Self {
            left_items: Vec::new(),
            right_items: Vec::new(),
        }
    }

    /// Add item to the left side
    pub fn add_left(&mut self, item: StatusItem) {
        self.left_items.push(item);
    }

    /// Add item to the right side
    pub fn add_right(&mut self, item: StatusItem) {
        self.right_items.push(item);
    }

    /// Clear all items
    pub fn clear(&mut self) {
        self.left_items.clear();
        self.right_items.clear();
    }

    /// Render the status bar
    pub fn render(&self, area: Rect, buf: &mut Buffer, theme: &ThemeManager) {
        // Clear the area with background color
        let bg_style = Style::default().bg(theme.get_color(ComponentType::Background));
        for x in area.x..area.x + area.width {
            for y in area.y..area.y + area.height {
                buf.get_mut(x, y).set_style(bg_style);
            }
        }

        // Render left items
        let mut current_x = area.x;
        for item in &self.left_items {
            let text = format!("{}: {} ", item.label, item.value);
            let text_len = text.len() as u16;
            let spans = vec![Span::styled(text, theme.get_style(item.style))];
            let line = Line::from(spans);

            if current_x < area.x + area.width {
                buf.set_line(current_x, area.y, &line, area.width - (current_x - area.x));
                current_x += text_len;
            }
        }

        // Render right items
        let right_text: String = self
            .right_items
            .iter()
            .map(|item| format!("{}: {}", item.label, item.value))
            .collect::<Vec<_>>()
            .join(" | ");

        if !right_text.is_empty() {
            let right_x = area.x + area.width.saturating_sub(right_text.len() as u16);
            if right_x >= current_x {
                let spans = vec![Span::styled(
                    right_text,
                    theme.get_style(ComponentType::Text),
                )];
                let line = Line::from(spans);
                buf.set_line(right_x, area.y, &line, area.width - (right_x - area.x));
            }
        }
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}

/// Preview panel component for showing additional information
#[derive(Debug, Clone)]
pub struct PreviewPanel {
    title: String,
    content: Vec<String>,
    scroll_state: ScrollbarState,
}

impl Default for PreviewPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl PreviewPanel {
    /// Create a new preview panel
    pub fn new() -> Self {
        Self {
            title: "Preview".to_string(),
            content: Vec::new(),
            scroll_state: ScrollbarState::default(),
        }
    }

    /// Set the title
    pub fn set_title(&mut self, title: String) {
        self.title = title;
    }

    /// Set the content
    pub fn set_content(&mut self, content: Vec<String>) {
        let content_len = content.len();
        self.content = content;
        self.scroll_state = ScrollbarState::default().content_length(content_len);
    }

    /// Add a line to the content
    pub fn add_line(&mut self, line: String) {
        self.content.push(line);
        self.scroll_state = self.scroll_state.content_length(self.content.len());
    }

    /// Clear the content
    pub fn clear(&mut self) {
        self.content.clear();
        self.scroll_state = ScrollbarState::default();
    }

    /// Scroll up
    pub fn scroll_up(&mut self, lines: usize) {
        for _ in 0..lines {
            self.scroll_state.prev();
        }
    }

    /// Scroll down
    pub fn scroll_down(&mut self, lines: usize) {
        for _ in 0..lines {
            self.scroll_state.next();
        }
    }

    /// Render the preview panel
    pub fn render(&mut self, area: Rect, buf: &mut Buffer, theme: &ThemeManager, focused: bool) {
        let border_style = if focused {
            theme.get_style(ComponentType::PreviewBorder)
        } else {
            theme.get_style(ComponentType::Border)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(self.title.as_str())
            .title_style(theme.get_style(ComponentType::Title))
            .border_style(border_style);

        let inner = block.inner(area);
        block.render(area, buf);

        if self.content.is_empty() {
            let empty_text = Paragraph::new("No preview available")
                .style(theme.get_style(ComponentType::Muted))
                .alignment(Alignment::Center);
            empty_text.render(inner, buf);
            return;
        }

        let items: Vec<ListItem> = self
            .content
            .iter()
            .map(|line| {
                ListItem::new(Line::from(Span::styled(
                    line,
                    theme.get_style(ComponentType::Text),
                )))
            })
            .collect();

        let list = List::new(items).style(theme.get_style(ComponentType::Text));

        let mut list_state = ListState::default();
        StatefulWidget::render(list, inner, buf, &mut list_state);

        // Render scrollbar
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"))
            .track_style(theme.get_style(ComponentType::ScrollbarTrack))
            .thumb_style(theme.get_style(ComponentType::ScrollbarThumb));

        StatefulWidget::render(scrollbar, area, buf, &mut self.scroll_state);
    }
}

/// Progress indicator component
#[derive(Debug, Clone)]
pub struct ProgressIndicator {
    progress: f64,
    label: String,
    show_percentage: bool,
}

impl ProgressIndicator {
    /// Create a new progress indicator
    pub fn new(label: String) -> Self {
        Self {
            progress: 0.0,
            label,
            show_percentage: true,
        }
    }

    /// Set the progress (0.0 to 1.0)
    pub fn set_progress(&mut self, progress: f64) {
        self.progress = progress.clamp(0.0, 1.0);
    }

    /// Set whether to show percentage
    pub fn show_percentage(mut self, show: bool) -> Self {
        self.show_percentage = show;
        self
    }

    /// Render the progress indicator
    pub fn render(&self, area: Rect, buf: &mut Buffer, theme: &ThemeManager) {
        let label = if self.show_percentage {
            format!("{} ({:.0}%)", self.label, self.progress * 100.0)
        } else {
            self.label.clone()
        };

        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title("Progress"))
            .gauge_style(theme.get_style(ComponentType::Success))
            .label(label)
            .ratio(self.progress);

        gauge.render(area, buf);
    }
}

/// Popup dialog component
#[derive(Debug, Clone)]
pub struct PopupDialog {
    title: String,
    message: String,
    dialog_type: DialogType,
}

/// Type of dialog
#[derive(Debug, Clone, PartialEq)]
pub enum DialogType {
    Info,
    Warning,
    Error,
    Confirm,
}

impl PopupDialog {
    /// Create a new popup dialog
    pub fn new(title: String, message: String, dialog_type: DialogType) -> Self {
        Self {
            title,
            message,
            dialog_type,
        }
    }

    /// Create an info dialog
    pub fn info(title: String, message: String) -> Self {
        Self::new(title, message, DialogType::Info)
    }

    /// Create a warning dialog
    pub fn warning(title: String, message: String) -> Self {
        Self::new(title, message, DialogType::Warning)
    }

    /// Create an error dialog
    pub fn error(title: String, message: String) -> Self {
        Self::new(title, message, DialogType::Error)
    }

    /// Render the popup dialog
    pub fn render(&self, area: Rect, buf: &mut Buffer, theme: &ThemeManager) {
        // Clear background
        Clear.render(area, buf);

        let style = match self.dialog_type {
            DialogType::Info => theme.get_style(ComponentType::Info),
            DialogType::Warning => theme.get_style(ComponentType::Warning),
            DialogType::Error => theme.get_style(ComponentType::Error),
            DialogType::Confirm => theme.get_style(ComponentType::Highlight),
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(self.title.as_str())
            .title_style(style)
            .border_style(style);

        let inner = block.inner(area);
        block.render(area, buf);

        let paragraph = Paragraph::new(self.message.as_str())
            .style(theme.get_style(ComponentType::Text))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        paragraph.render(inner, buf);
    }
}

/// File tree entry representing a file or directory
#[derive(Debug, Clone, PartialEq)]
pub struct FileTreeEntry {
    pub name: String,
    pub path: std::path::PathBuf,
    pub is_dir: bool,
    pub is_expanded: bool,
    pub depth: usize,
    pub children: Vec<FileTreeEntry>,
}

impl FileTreeEntry {
    /// Create a new file entry
    pub fn new(name: String, path: std::path::PathBuf, is_dir: bool, depth: usize) -> Self {
        Self {
            name,
            path,
            is_dir,
            is_expanded: false,
            depth,
            children: Vec::new(),
        }
    }

    /// Toggle expansion state
    pub fn toggle_expand(&mut self) {
        if self.is_dir {
            self.is_expanded = !self.is_expanded;
        }
    }

    /// Get flattened list of visible entries
    pub fn flatten_visible(&self) -> Vec<FileTreeEntry> {
        let mut result = vec![self.clone()];
        if self.is_expanded {
            for child in &self.children {
                result.extend(child.flatten_visible());
            }
        }
        result
    }
}

/// File tree browser component
#[derive(Debug, Clone)]
pub struct FileTreeBrowser {
    root: Option<FileTreeEntry>,
    visible_entries: Vec<FileTreeEntry>,
    selected_index: usize,
    scroll_state: ScrollbarState,
    list_state: ListState,
    filter: String,
    show_hidden: bool,
}

impl Default for FileTreeBrowser {
    fn default() -> Self {
        Self::new()
    }
}

impl FileTreeBrowser {
    /// Create a new file tree browser
    pub fn new() -> Self {
        Self {
            root: None,
            visible_entries: Vec::new(),
            selected_index: 0,
            scroll_state: ScrollbarState::default(),
            list_state: ListState::default(),
            filter: String::new(),
            show_hidden: false,
        }
    }

    /// Load directory tree from path
    pub fn load_directory(&mut self, path: &std::path::Path) -> Result<(), std::io::Error> {
        let root = self.load_entry(path, 0)?;
        self.root = Some(root);
        self.update_visible_entries();
        Ok(())
    }

    /// Recursively load directory entries
    fn load_entry(
        &self,
        path: &std::path::Path,
        depth: usize,
    ) -> Result<FileTreeEntry, std::io::Error> {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(".")
            .to_string();

        let is_dir = path.is_dir();
        let mut entry = FileTreeEntry::new(name, path.to_path_buf(), is_dir, depth);

        if is_dir && depth < 3 {
            // Limit initial depth to 3
            if let Ok(entries) = std::fs::read_dir(path) {
                let mut children: Vec<FileTreeEntry> = entries
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        self.show_hidden
                            || !e
                                .file_name()
                                .to_str()
                                .map(|s| s.starts_with('.'))
                                .unwrap_or(false)
                    })
                    .filter_map(|e| self.load_entry(&e.path(), depth + 1).ok())
                    .collect();

                // Sort: directories first, then alphabetically
                children.sort_by(|a, b| match (a.is_dir, b.is_dir) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                });

                entry.children = children;
            }
        }

        Ok(entry)
    }

    /// Update the list of visible entries
    fn update_visible_entries(&mut self) {
        if let Some(ref root) = self.root {
            self.visible_entries = root.flatten_visible();

            // Apply filter if set
            if !self.filter.is_empty() {
                let filter_lower = self.filter.to_lowercase();
                self.visible_entries.retain(|entry| {
                    entry.name.to_lowercase().contains(&filter_lower)
                        || entry
                            .path
                            .to_string_lossy()
                            .to_lowercase()
                            .contains(&filter_lower)
                });
            }

            self.scroll_state = self.scroll_state.content_length(self.visible_entries.len());

            // Ensure selected index is valid
            if self.selected_index >= self.visible_entries.len() && !self.visible_entries.is_empty()
            {
                self.selected_index = self.visible_entries.len() - 1;
            }
        }
    }

    /// Get the currently selected entry
    pub fn selected_entry(&self) -> Option<&FileTreeEntry> {
        self.visible_entries.get(self.selected_index)
    }

    /// Toggle expansion of selected directory
    pub fn toggle_selected(&mut self) {
        if let Some(entry) = self.visible_entries.get(self.selected_index) {
            if entry.is_dir {
                if let Some(ref mut root) = self.root {
                    Self::toggle_entry_at_path(root, &entry.path);
                    self.update_visible_entries();
                }
            }
        }
    }

    /// Toggle expansion of entry at path (recursive helper)
    fn toggle_entry_at_path(entry: &mut FileTreeEntry, target_path: &std::path::Path) -> bool {
        if entry.path == target_path {
            entry.toggle_expand();
            return true;
        }

        for child in &mut entry.children {
            if Self::toggle_entry_at_path(child, target_path) {
                return true;
            }
        }

        false
    }

    /// Move selection up
    pub fn select_previous(&mut self) {
        if !self.visible_entries.is_empty() && self.selected_index > 0 {
            self.selected_index -= 1;
            self.list_state.select(Some(self.selected_index));
            self.scroll_state.prev();
        }
    }

    /// Move selection down
    pub fn select_next(&mut self) {
        if self.selected_index < self.visible_entries.len().saturating_sub(1) {
            self.selected_index += 1;
            self.list_state.select(Some(self.selected_index));
            self.scroll_state.next();
        }
    }

    /// Set filter string
    pub fn set_filter(&mut self, filter: String) {
        self.filter = filter;
        self.update_visible_entries();
    }

    /// Clear filter
    pub fn clear_filter(&mut self) {
        self.filter.clear();
        self.update_visible_entries();
    }

    /// Toggle showing hidden files
    pub fn toggle_hidden(&mut self) {
        self.show_hidden = !self.show_hidden;
        // Would need to reload directory to apply this change
    }

    /// Refresh the directory tree
    pub fn refresh(&mut self) -> Result<(), std::io::Error> {
        if let Some(ref root) = self.root.clone() {
            self.load_directory(&root.path)?;
        }
        Ok(())
    }

    /// Render the file tree browser
    pub fn render(&mut self, area: Rect, buf: &mut Buffer, theme: &ThemeManager, focused: bool) {
        let border_style = if focused {
            theme.get_style(ComponentType::PreviewBorder)
        } else {
            theme.get_style(ComponentType::Border)
        };

        let title = if self.filter.is_empty() {
            "Files".to_string()
        } else {
            format!("Files [Filter: {}]", self.filter)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .title_style(theme.get_style(ComponentType::Title))
            .border_style(border_style);

        let inner = block.inner(area);
        block.render(area, buf);

        if self.visible_entries.is_empty() {
            let empty_text = Paragraph::new("No files to display")
                .style(theme.get_style(ComponentType::Muted))
                .alignment(Alignment::Center);
            empty_text.render(inner, buf);
            return;
        }

        // Build list items
        let items: Vec<ListItem> = self
            .visible_entries
            .iter()
            .enumerate()
            .map(|(idx, entry)| {
                let indent = "  ".repeat(entry.depth);
                let icon = if entry.is_dir {
                    if entry.is_expanded {
                        "▼ 📁 "
                    } else {
                        "▶ 📁 "
                    }
                } else {
                    "  📄 "
                };

                let style = if idx == self.selected_index {
                    theme
                        .get_style(ComponentType::StatusActive)
                        .add_modifier(Modifier::BOLD)
                } else if entry.is_dir {
                    theme
                        .get_style(ComponentType::Text)
                        .add_modifier(Modifier::BOLD)
                } else {
                    theme.get_style(ComponentType::Text)
                };

                let content = format!("{}{}{}", indent, icon, entry.name);
                ListItem::new(Line::from(Span::styled(content, style)))
            })
            .collect();

        let list = List::new(items)
            .style(theme.get_style(ComponentType::Text))
            .highlight_style(
                theme
                    .get_style(ComponentType::StatusActive)
                    .add_modifier(Modifier::BOLD | Modifier::REVERSED),
            );

        self.list_state.select(Some(self.selected_index));
        StatefulWidget::render(list, inner, buf, &mut self.list_state);

        // Render scrollbar
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"))
            .track_style(theme.get_style(ComponentType::ScrollbarTrack))
            .thumb_style(theme.get_style(ComponentType::ScrollbarThumb));

        StatefulWidget::render(scrollbar, area, buf, &mut self.scroll_state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_view_creation() {
        let chat_view = ChatView::new();
        assert_eq!(chat_view.messages().len(), 0);
        assert!(chat_view.auto_scroll);
    }

    #[test]
    fn test_chat_view_add_message() {
        let mut chat_view = ChatView::new();
        let message = Message {
            role: MessageRole::User,
            content: "Hello".to_string(),
            timestamp: "12:00".to_string(),
        };

        chat_view.add_message(message);
        assert_eq!(chat_view.messages().len(), 1);
    }

    #[test]
    fn test_input_field_operations() {
        let mut input = InputField::new();

        // Test insertion
        input.insert_char('H');
        input.insert_char('i');
        assert_eq!(input.content(), "Hi");
        assert_eq!(input.cursor_position, 2);

        // Test backspace
        input.backspace();
        assert_eq!(input.content(), "H");
        assert_eq!(input.cursor_position, 1);

        // Test clear
        input.clear();
        assert_eq!(input.content(), "");
        assert_eq!(input.cursor_position, 0);
    }

    #[test]
    fn test_status_bar_items() {
        let mut status_bar = StatusBar::new();

        status_bar.add_left(StatusItem {
            label: "Mode".to_string(),
            value: "Normal".to_string(),
            style: ComponentType::StatusActive,
        });

        status_bar.add_right(StatusItem {
            label: "Messages".to_string(),
            value: "5".to_string(),
            style: ComponentType::Text,
        });

        assert_eq!(status_bar.left_items.len(), 1);
        assert_eq!(status_bar.right_items.len(), 1);
    }

    #[test]
    fn test_preview_panel_content() {
        let mut preview = PreviewPanel::new();

        preview.add_line("Line 1".to_string());
        preview.add_line("Line 2".to_string());

        assert_eq!(preview.content.len(), 2);

        preview.clear();
        assert_eq!(preview.content.len(), 0);
    }

    #[test]
    fn test_progress_indicator() {
        let mut progress = ProgressIndicator::new("Loading".to_string());

        progress.set_progress(0.5);
        assert_eq!(progress.progress, 0.5);

        progress.set_progress(1.5); // Should be clamped
        assert_eq!(progress.progress, 1.0);

        progress.set_progress(-0.5); // Should be clamped
        assert_eq!(progress.progress, 0.0);
    }

    #[test]
    fn test_popup_dialog_creation() {
        let dialog = PopupDialog::error("Error".to_string(), "Something went wrong".to_string());

        assert_eq!(dialog.title, "Error");
        assert_eq!(dialog.message, "Something went wrong");
        assert_eq!(dialog.dialog_type, DialogType::Error);
    }
}
