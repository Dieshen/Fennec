use crate::theme::{ComponentType, ThemeManager};
use fennec_commands::{EnhancedSummarizeArgs, OutputDestination, SummaryDepth, SummaryType};
use fennec_memory::{MemoryFileMetadata, MemoryFileType};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState, StatefulWidget, Tabs, Widget, Wrap,
    },
};
use uuid::Uuid;

/// Summary panel component for displaying and managing summaries
#[derive(Debug, Clone)]
pub struct SummaryPanel {
    /// Current summary content being displayed
    pub current_summary: Option<String>,
    /// List of available memory files
    pub memory_files: Vec<MemoryFileMetadata>,
    /// Currently selected memory file
    pub selected_memory_file: Option<usize>,
    /// Memory file list state
    pub memory_file_list_state: ListState,
    /// Summary generation arguments
    pub summary_args: EnhancedSummarizeArgs,
    /// Current tab (Summary, Memory Files, Settings)
    pub current_tab: SummaryTab,
    /// Scroll state for summary content
    pub summary_scroll_state: ScrollbarState,
    /// Current scroll position for summary content
    pub summary_scroll_position: usize,
    /// Total number of lines in the current summary content
    pub summary_content_length: usize,
    /// Current viewport height for summary content
    pub summary_viewport_length: usize,
    /// Whether the panel is currently loading
    pub is_loading: bool,
    /// Last update timestamp
    pub last_updated: Option<chrono::DateTime<chrono::Utc>>,
    /// Summary generation status
    pub generation_status: SummaryGenerationStatus,
}

/// Available tabs in the summary panel
#[derive(Debug, Clone, PartialEq)]
pub enum SummaryTab {
    Summary,
    MemoryFiles,
    Settings,
}

/// Status of summary generation
#[derive(Debug, Clone)]
pub enum SummaryGenerationStatus {
    Idle,
    Generating,
    Success(String), // Success message
    Error(String),   // Error message
}

impl Default for SummaryPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl SummaryPanel {
    /// Create a new summary panel
    pub fn new() -> Self {
        Self {
            current_summary: None,
            memory_files: Vec::new(),
            selected_memory_file: None,
            memory_file_list_state: ListState::default(),
            summary_args: EnhancedSummarizeArgs {
                target: "current".to_string(),
                summary_type: Some(SummaryType::Session),
                is_path: None,
                max_lines: Some(100),
                include_extensions: None,
                include_structure: None,
                output_destination: Some(OutputDestination::Console),
                depth_level: Some(SummaryDepth::Standard),
                time_range_hours: Some(24),
                save_to_memory: Some(false),
                memory_tags: None,
            },
            current_tab: SummaryTab::Summary,
            summary_scroll_state: ScrollbarState::default(),
            summary_scroll_position: 0,
            summary_content_length: 0,
            summary_viewport_length: 1,
            is_loading: false,
            last_updated: None,
            generation_status: SummaryGenerationStatus::Idle,
        }
    }

    /// Set the current summary content
    pub fn set_summary(&mut self, summary: String) {
        self.current_summary = Some(summary);
        self.last_updated = Some(chrono::Utc::now());
        self.is_loading = false;
        self.summary_scroll_position = 0;
        self.summary_content_length = self
            .current_summary
            .as_ref()
            .map(|s| s.lines().count())
            .unwrap_or(0);
        self.update_scrollbar_state();
    }

    /// Set loading state
    pub fn set_loading(&mut self, loading: bool) {
        self.is_loading = loading;
        if loading {
            self.generation_status = SummaryGenerationStatus::Generating;
            self.summary_content_length = 0;
            self.summary_scroll_position = 0;
            self.update_scrollbar_state();
        }
    }

    /// Set generation status
    pub fn set_generation_status(&mut self, status: SummaryGenerationStatus) {
        if matches!(
            &status,
            SummaryGenerationStatus::Success(_) | SummaryGenerationStatus::Error(_)
        ) {
            self.is_loading = false;
        }
        self.generation_status = status;
    }

    /// Update memory files list
    pub fn update_memory_files(&mut self, files: Vec<MemoryFileMetadata>) {
        self.memory_files = files;
        // Reset selection if current selection is invalid
        if let Some(selected) = self.selected_memory_file {
            if selected >= self.memory_files.len() {
                self.selected_memory_file = None;
                self.memory_file_list_state.select(None);
            }
        }
    }

    /// Select next tab
    pub fn next_tab(&mut self) {
        self.current_tab = match self.current_tab {
            SummaryTab::Summary => SummaryTab::MemoryFiles,
            SummaryTab::MemoryFiles => SummaryTab::Settings,
            SummaryTab::Settings => SummaryTab::Summary,
        };
    }

    /// Select previous tab
    pub fn previous_tab(&mut self) {
        self.current_tab = match self.current_tab {
            SummaryTab::Summary => SummaryTab::Settings,
            SummaryTab::MemoryFiles => SummaryTab::Summary,
            SummaryTab::Settings => SummaryTab::MemoryFiles,
        };
    }

    /// Move memory file selection up
    pub fn select_previous_memory_file(&mut self) {
        if self.memory_files.is_empty() {
            return;
        }

        let selected = match self.selected_memory_file {
            Some(i) => {
                if i == 0 {
                    self.memory_files.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };

        self.selected_memory_file = Some(selected);
        self.memory_file_list_state.select(Some(selected));
    }

    /// Move memory file selection down
    pub fn select_next_memory_file(&mut self) {
        if self.memory_files.is_empty() {
            return;
        }

        let selected = match self.selected_memory_file {
            Some(i) => {
                if i >= self.memory_files.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };

        self.selected_memory_file = Some(selected);
        self.memory_file_list_state.select(Some(selected));
    }

    /// Get currently selected memory file
    pub fn get_selected_memory_file(&self) -> Option<&MemoryFileMetadata> {
        self.selected_memory_file
            .and_then(|i| self.memory_files.get(i))
    }

    /// Update summary arguments
    pub fn update_summary_args(&mut self, args: EnhancedSummarizeArgs) {
        self.summary_args = args;
    }

    fn update_scrollbar_state(&mut self) {
        let content_length = self.summary_content_length.max(1);
        let viewport_length = self.summary_viewport_length.max(1);
        let max_position = content_length.saturating_sub(viewport_length);
        self.summary_scroll_position = self.summary_scroll_position.min(max_position);

        self.summary_scroll_state = ScrollbarState::new(content_length)
            .viewport_content_length(viewport_length)
            .position(self.summary_scroll_position);
    }

    /// Scroll summary content up
    pub fn scroll_summary_up(&mut self) {
        if self.summary_content_length <= self.summary_viewport_length {
            return;
        }

        self.summary_scroll_position = self.summary_scroll_position.saturating_sub(3);
        self.update_scrollbar_state();
    }

    /// Scroll summary content down
    pub fn scroll_summary_down(&mut self) {
        if self.summary_content_length <= self.summary_viewport_length {
            return;
        }

        let max_pos = self
            .summary_content_length
            .saturating_sub(self.summary_viewport_length); // avoid underflow
        self.summary_scroll_position = (self.summary_scroll_position + 3).min(max_pos);
        self.update_scrollbar_state();
    }

    /// Render the summary panel
    pub fn render(&mut self, area: Rect, buf: &mut Buffer, theme: &ThemeManager) {
        // Create main block
        let block = Block::default()
            .title("Summary Panel")
            .borders(Borders::ALL)
            .style(theme.get_style(ComponentType::Border));

        let inner = block.inner(area);
        block.render(area, buf);

        // Create tab layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(inner);

        // Render tabs
        self.render_tabs(chunks[0], buf, theme);

        // Render current tab content
        match self.current_tab {
            SummaryTab::Summary => self.render_summary_tab(chunks[1], buf, theme),
            SummaryTab::MemoryFiles => self.render_memory_files_tab(chunks[1], buf, theme),
            SummaryTab::Settings => self.render_settings_tab(chunks[1], buf, theme),
        }
    }

    /// Render tab bar
    fn render_tabs(&self, area: Rect, buf: &mut Buffer, theme: &ThemeManager) {
        let tab_titles = vec!["Summary", "Memory Files", "Settings"];
        let selected_tab = match self.current_tab {
            SummaryTab::Summary => 0,
            SummaryTab::MemoryFiles => 1,
            SummaryTab::Settings => 2,
        };

        let tabs = Tabs::new(tab_titles)
            .block(Block::default().borders(Borders::BOTTOM))
            .select(selected_tab)
            .style(theme.get_style(ComponentType::Tab))
            .highlight_style(theme.get_style(ComponentType::TabSelected));

        tabs.render(area, buf);
    }

    /// Render summary tab content
    fn render_summary_tab(&mut self, area: Rect, buf: &mut Buffer, theme: &ThemeManager) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        // Render status bar
        self.render_status_bar(chunks[0], buf, theme);

        // Render summary content
        if let Some(summary) = self.current_summary.clone() {
            self.summary_content_length = summary.lines().count();
            self.summary_viewport_length = chunks[1].height.max(1) as usize;
            self.update_scrollbar_state();

            let paragraph = Paragraph::new(summary)
                .block(
                    Block::default()
                        .title("Current Summary")
                        .borders(Borders::ALL)
                        .style(theme.get_style(ComponentType::Border)),
                )
                .scroll((self.summary_scroll_position as u16, 0))
                .wrap(Wrap { trim: false })
                .style(theme.get_style(ComponentType::Text));

            paragraph.render(chunks[1], buf);

            // Render scrollbar
            let scrollbar = Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));

            scrollbar.render(chunks[1], buf, &mut self.summary_scroll_state);
        } else if self.is_loading {
            let loading_text = "Generating summary...";
            let paragraph = Paragraph::new(loading_text)
                .block(
                    Block::default()
                        .title("Summary")
                        .borders(Borders::ALL)
                        .style(theme.get_style(ComponentType::Border)),
                )
                .alignment(Alignment::Center)
                .style(theme.get_style(ComponentType::Text));

            paragraph.render(chunks[1], buf);
            self.summary_content_length = 0;
            self.summary_viewport_length = chunks[1].height.max(1) as usize;
            self.update_scrollbar_state();
        } else {
            let help_text = "No summary generated yet.\n\nPress 'g' to generate a new summary or select a memory file to view.";
            let paragraph = Paragraph::new(help_text)
                .block(
                    Block::default()
                        .title("Summary")
                        .borders(Borders::ALL)
                        .style(theme.get_style(ComponentType::Border)),
                )
                .alignment(Alignment::Center)
                .style(theme.get_style(ComponentType::Text))
                .wrap(Wrap { trim: false });

            paragraph.render(chunks[1], buf);
            self.summary_content_length = 0;
            self.summary_viewport_length = chunks[1].height.max(1) as usize;
            self.update_scrollbar_state();
        }
    }

    /// Render memory files tab content
    fn render_memory_files_tab(&mut self, area: Rect, buf: &mut Buffer, theme: &ThemeManager) {
        if self.memory_files.is_empty() {
            let help_text = "No memory files found.\n\nGenerate summaries with 'save_to_memory: true' to create memory files.";
            let paragraph = Paragraph::new(help_text)
                .block(
                    Block::default()
                        .title("Memory Files")
                        .borders(Borders::ALL)
                        .style(theme.get_style(ComponentType::Border)),
                )
                .alignment(Alignment::Center)
                .style(theme.get_style(ComponentType::Text))
                .wrap(Wrap { trim: false });

            paragraph.render(area, buf);
            return;
        }

        // Create list items
        let items: Vec<ListItem> = self
            .memory_files
            .iter()
            .map(|file| {
                let file_type_icon = match file.file_type {
                    MemoryFileType::ProjectContext => "🏗️",
                    MemoryFileType::DebuggingPatterns => "🐛",
                    MemoryFileType::CodePatterns => "💻",
                    MemoryFileType::Architecture => "🏛️",
                    MemoryFileType::Learning => "📚",
                    MemoryFileType::Knowledge => "🧠",
                    MemoryFileType::Templates => "📋",
                };

                let content = vec![
                    Line::from(vec![Span::styled(
                        format!("{} {}", file_type_icon, file.name),
                        Style::default().add_modifier(Modifier::BOLD),
                    )]),
                    Line::from(vec![Span::styled(
                        format!("  Tags: {}", file.tags.join(", ")),
                        Style::default(),
                    )]),
                    Line::from(vec![Span::styled(
                        format!("  Updated: {}", file.updated_at.format("%Y-%m-%d %H:%M")),
                        Style::default(),
                    )]),
                ];

                ListItem::new(content)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title(format!("Memory Files ({})", self.memory_files.len()))
                    .borders(Borders::ALL)
                    .style(theme.get_style(ComponentType::Border)),
            )
            .style(theme.get_style(ComponentType::Text))
            .highlight_style(theme.get_style(ComponentType::ListSelected));

        StatefulWidget::render(list, area, buf, &mut self.memory_file_list_state);
    }

    /// Render settings tab content
    fn render_settings_tab(&self, area: Rect, buf: &mut Buffer, theme: &ThemeManager) {
        let settings_lines = vec![
            Line::from("Summary Generation Settings:"),
            Line::from(""),
            Line::from(vec![
                Span::styled("Target: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(&self.summary_args.target, Style::default()),
            ]),
            Line::from(vec![
                Span::styled("Type: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(
                    format!(
                        "{:?}",
                        self.summary_args
                            .summary_type
                            .as_ref()
                            .unwrap_or(&SummaryType::Session)
                    ),
                    Style::default(),
                ),
            ]),
            Line::from(vec![
                Span::styled("Depth: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(
                    format!(
                        "{:?}",
                        self.summary_args
                            .depth_level
                            .as_ref()
                            .unwrap_or(&SummaryDepth::Standard)
                    ),
                    Style::default(),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Time Range: ",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{} hours", self.summary_args.time_range_hours.unwrap_or(24)),
                    Style::default(),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Save to Memory: ",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    if self.summary_args.save_to_memory.unwrap_or(false) {
                        "Yes"
                    } else {
                        "No"
                    },
                    Style::default(),
                ),
            ]),
            Line::from(""),
            Line::from("Keyboard Shortcuts:"),
            Line::from("  g - Generate summary"),
            Line::from("  r - Refresh memory files"),
            Line::from("  Tab - Switch tabs"),
            Line::from("  ↑/↓ - Navigate memory files"),
            Line::from("  Enter - View selected memory file"),
        ];

        let paragraph = Paragraph::new(settings_lines)
            .block(
                Block::default()
                    .title("Settings & Help")
                    .borders(Borders::ALL)
                    .style(theme.get_style(ComponentType::Border)),
            )
            .style(theme.get_style(ComponentType::Text))
            .wrap(Wrap { trim: false });

        paragraph.render(area, buf);
    }

    /// Render status bar
    fn render_status_bar(&self, area: Rect, buf: &mut Buffer, theme: &ThemeManager) {
        let status_text = match &self.generation_status {
            SummaryGenerationStatus::Idle => {
                if let Some(updated) = self.last_updated {
                    format!("Last updated: {}", updated.format("%H:%M:%S"))
                } else {
                    "Ready".to_string()
                }
            }
            SummaryGenerationStatus::Generating => "Generating summary...".to_string(),
            SummaryGenerationStatus::Success(msg) => format!("✅ {}", msg),
            SummaryGenerationStatus::Error(msg) => format!("❌ {}", msg),
        };

        let paragraph = Paragraph::new(status_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .style(theme.get_style(ComponentType::Border)),
            )
            .style(theme.get_style(ComponentType::Text))
            .alignment(Alignment::Center);

        paragraph.render(area, buf);
    }
}

/// Summary panel events and actions
#[derive(Debug, Clone)]
pub enum SummaryPanelAction {
    /// Generate a new summary
    GenerateSummary(EnhancedSummarizeArgs),
    /// Load a memory file
    LoadMemoryFile(Uuid),
    /// Refresh memory files list
    RefreshMemoryFiles,
    /// Save current summary to memory
    SaveToMemory(String), // filename
    /// Update settings
    UpdateSettings(EnhancedSummarizeArgs),
    /// Export summary to file
    ExportSummary(String), // file path
}

/// Helper functions for summary panel integration
impl SummaryPanel {
    /// Create summary args for current session
    pub fn create_session_summary_args(&self) -> EnhancedSummarizeArgs {
        EnhancedSummarizeArgs {
            target: "current".to_string(),
            summary_type: Some(SummaryType::Session),
            is_path: Some(false),
            max_lines: Some(100),
            include_extensions: None,
            include_structure: None,
            output_destination: Some(OutputDestination::Console),
            depth_level: Some(SummaryDepth::Standard),
            time_range_hours: Some(24),
            save_to_memory: Some(false),
            memory_tags: Some(vec!["session".to_string(), "tui".to_string()]),
        }
    }

    /// Create summary args for project summary
    pub fn create_project_summary_args(&self) -> EnhancedSummarizeArgs {
        EnhancedSummarizeArgs {
            target: ".".to_string(),
            summary_type: Some(SummaryType::Project),
            is_path: Some(true),
            max_lines: Some(50),
            include_extensions: None,
            include_structure: Some(true),
            output_destination: Some(OutputDestination::Console),
            depth_level: Some(SummaryDepth::Standard),
            time_range_hours: Some(24),
            save_to_memory: Some(false),
            memory_tags: Some(vec!["project".to_string(), "tui".to_string()]),
        }
    }

    /// Toggle save to memory setting
    pub fn toggle_save_to_memory(&mut self) {
        self.summary_args.save_to_memory = Some(!self.summary_args.save_to_memory.unwrap_or(false));
    }

    /// Cycle through depth levels
    pub fn cycle_depth_level(&mut self) {
        let current = self
            .summary_args
            .depth_level
            .as_ref()
            .unwrap_or(&SummaryDepth::Standard);
        self.summary_args.depth_level = Some(match current {
            SummaryDepth::Brief => SummaryDepth::Standard,
            SummaryDepth::Standard => SummaryDepth::Detailed,
            SummaryDepth::Detailed => SummaryDepth::Comprehensive,
            SummaryDepth::Comprehensive => SummaryDepth::Brief,
        });
    }

    /// Cycle through summary types
    pub fn cycle_summary_type(&mut self) {
        let current = self
            .summary_args
            .summary_type
            .as_ref()
            .unwrap_or(&SummaryType::Session);
        self.summary_args.summary_type = Some(match current {
            SummaryType::Session => SummaryType::Project,
            SummaryType::Project => SummaryType::Commands,
            SummaryType::Commands => SummaryType::File,
            SummaryType::File => SummaryType::Text,
            SummaryType::Text => SummaryType::Session,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_summary_panel_creation() {
        let panel = SummaryPanel::new();
        assert_eq!(panel.current_tab, SummaryTab::Summary);
        assert!(panel.current_summary.is_none());
        assert!(!panel.is_loading);
    }

    #[test]
    fn test_tab_navigation() {
        let mut panel = SummaryPanel::new();

        assert_eq!(panel.current_tab, SummaryTab::Summary);

        panel.next_tab();
        assert_eq!(panel.current_tab, SummaryTab::MemoryFiles);

        panel.next_tab();
        assert_eq!(panel.current_tab, SummaryTab::Settings);

        panel.next_tab();
        assert_eq!(panel.current_tab, SummaryTab::Summary);

        panel.previous_tab();
        assert_eq!(panel.current_tab, SummaryTab::Settings);
    }

    #[test]
    fn test_summary_args_creation() {
        let panel = SummaryPanel::new();

        let session_args = panel.create_session_summary_args();
        assert_eq!(session_args.target, "current");
        assert_eq!(session_args.summary_type, Some(SummaryType::Session));

        let project_args = panel.create_project_summary_args();
        assert_eq!(project_args.target, ".");
        assert_eq!(project_args.summary_type, Some(SummaryType::Project));
    }

    #[test]
    fn test_settings_cycling() {
        let mut panel = SummaryPanel::new();

        // Test depth level cycling
        panel.summary_args.depth_level = Some(SummaryDepth::Brief);
        panel.cycle_depth_level();
        assert_eq!(panel.summary_args.depth_level, Some(SummaryDepth::Standard));

        // Test summary type cycling
        panel.summary_args.summary_type = Some(SummaryType::Session);
        panel.cycle_summary_type();
        assert_eq!(panel.summary_args.summary_type, Some(SummaryType::Project));

        // Test save to memory toggle
        panel.summary_args.save_to_memory = Some(false);
        panel.toggle_save_to_memory();
        assert_eq!(panel.summary_args.save_to_memory, Some(true));
    }
}
