use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use ratatui::widgets::BorderType;

/// Represents the different panes in the application
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Chat,
    Input,
    Preview,
    StatusBar,
}

/// Layout configuration for different screen sizes
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutConfig {
    /// Whether to show the preview panel
    pub show_preview: bool,
    /// Minimum terminal width required
    pub min_width: u16,
    /// Minimum terminal height required
    pub min_height: u16,
    /// Chat pane width percentage (when preview is shown)
    pub chat_width_percent: u16,
    /// Preview pane width percentage
    pub preview_width_percent: u16,
    /// Input area height in lines
    pub input_height: u16,
    /// Status bar height
    pub status_bar_height: u16,
    /// Border style
    pub border_style: BorderStyle,
}

/// Border styling configuration
#[derive(Debug, Clone, PartialEq)]
pub struct BorderStyle {
    pub border_type: BorderType,
    pub show_borders: bool,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            show_preview: true,
            min_width: 80,
            min_height: 24,
            chat_width_percent: 60,
            preview_width_percent: 40,
            input_height: 3,
            status_bar_height: 1,
            border_style: BorderStyle {
                border_type: BorderType::Rounded,
                show_borders: true,
            },
        }
    }
}

/// Represents the computed layout areas for the application
#[derive(Debug, Clone)]
pub struct AppLayout {
    /// Full terminal area
    pub full_area: Rect,
    /// Chat messages area
    pub chat_area: Rect,
    /// Text input area
    pub input_area: Rect,
    /// Preview panel area (if enabled)
    pub preview_area: Option<Rect>,
    /// Status bar area
    pub status_area: Rect,
    /// Currently focused pane
    pub focused_pane: Pane,
    /// Whether the layout is in compact mode (small screen)
    pub is_compact: bool,
}

/// Layout manager for handling responsive design and pane management
#[derive(Debug, Clone)]
pub struct LayoutManager {
    config: LayoutConfig,
    current_layout: Option<AppLayout>,
}

impl Default for LayoutManager {
    fn default() -> Self {
        Self::new(LayoutConfig::default())
    }
}

impl LayoutManager {
    /// Create a new layout manager with the given configuration
    pub fn new(config: LayoutConfig) -> Self {
        Self {
            config,
            current_layout: None,
        }
    }

    /// Get the current configuration
    pub fn config(&self) -> &LayoutConfig {
        &self.config
    }

    /// Update the configuration
    pub fn set_config(&mut self, config: LayoutConfig) {
        self.config = config;
        // Invalidate current layout to force recalculation
        self.current_layout = None;
    }

    /// Toggle preview panel visibility
    pub fn toggle_preview(&mut self) {
        self.config.show_preview = !self.config.show_preview;
        self.current_layout = None;
    }

    /// Get the current layout, recalculating if necessary
    pub fn layout(&mut self, area: Rect) -> &AppLayout {
        if self.current_layout.is_none() || self.current_layout.as_ref().unwrap().full_area != area
        {
            self.current_layout = Some(self.calculate_layout(area));
        }
        self.current_layout.as_ref().unwrap()
    }

    /// Calculate the layout for the given terminal area
    fn calculate_layout(&self, area: Rect) -> AppLayout {
        let is_compact = area.width < self.config.min_width || area.height < self.config.min_height;

        let focused_pane = Pane::Chat; // Default focus

        if is_compact {
            self.calculate_compact_layout(area, focused_pane, is_compact)
        } else {
            self.calculate_normal_layout(area, focused_pane, is_compact)
        }
    }

    /// Calculate layout for normal (large) screens
    fn calculate_normal_layout(
        &self,
        area: Rect,
        focused_pane: Pane,
        is_compact: bool,
    ) -> AppLayout {
        // Create vertical split: [main area][status bar]
        let main_vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),                                // Main area
                Constraint::Length(self.config.status_bar_height), // Status bar
            ])
            .split(area);

        let main_area = main_vertical[0];
        let status_area = main_vertical[1];

        // Split main area vertically: [content][input]
        let content_vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),                           // Content area
                Constraint::Length(self.config.input_height), // Input area
            ])
            .split(main_area);

        let content_area = content_vertical[0];
        let input_area = content_vertical[1];

        // Split content area horizontally if preview is enabled
        let (chat_area, preview_area) = if self.config.show_preview {
            let content_horizontal = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(self.config.chat_width_percent),
                    Constraint::Percentage(self.config.preview_width_percent),
                ])
                .split(content_area);

            (content_horizontal[0], Some(content_horizontal[1]))
        } else {
            (content_area, None)
        };

        AppLayout {
            full_area: area,
            chat_area,
            input_area,
            preview_area,
            status_area,
            focused_pane,
            is_compact,
        }
    }

    /// Calculate layout for compact (small) screens
    fn calculate_compact_layout(
        &self,
        area: Rect,
        focused_pane: Pane,
        is_compact: bool,
    ) -> AppLayout {
        // In compact mode, we stack everything vertically and hide preview by default
        let vertical_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),                                // Chat area (most space)
                Constraint::Length(self.config.input_height),      // Input area
                Constraint::Length(self.config.status_bar_height), // Status bar
            ])
            .split(area);

        AppLayout {
            full_area: area,
            chat_area: vertical_layout[0],
            input_area: vertical_layout[1],
            preview_area: None, // Hidden in compact mode
            status_area: vertical_layout[2],
            focused_pane,
            is_compact,
        }
    }

    /// Get the area for a specific pane
    pub fn get_pane_area(&mut self, area: Rect, pane: Pane) -> Option<Rect> {
        let layout = self.layout(area);
        match pane {
            Pane::Chat => Some(layout.chat_area),
            Pane::Input => Some(layout.input_area),
            Pane::Preview => layout.preview_area,
            Pane::StatusBar => Some(layout.status_area),
        }
    }

    /// Check if a pane is visible in the current layout
    pub fn is_pane_visible(&mut self, area: Rect, pane: Pane) -> bool {
        self.get_pane_area(area, pane).is_some()
    }

    /// Get the next focusable pane
    pub fn next_focusable_pane(&mut self, area: Rect, current: Pane) -> Pane {
        let layout = self.layout(area).clone();
        let focusable_panes = self.get_focusable_panes(&layout);

        if let Some(current_index) = focusable_panes.iter().position(|&p| p == current) {
            let next_index = (current_index + 1) % focusable_panes.len();
            focusable_panes[next_index]
        } else {
            focusable_panes[0]
        }
    }

    /// Get the previous focusable pane
    pub fn previous_focusable_pane(&mut self, area: Rect, current: Pane) -> Pane {
        let layout = self.layout(area).clone();
        let focusable_panes = self.get_focusable_panes(&layout);

        if let Some(current_index) = focusable_panes.iter().position(|&p| p == current) {
            let prev_index = if current_index == 0 {
                focusable_panes.len() - 1
            } else {
                current_index - 1
            };
            focusable_panes[prev_index]
        } else {
            focusable_panes[0]
        }
    }

    /// Get all focusable panes in the current layout
    fn get_focusable_panes(&self, layout: &AppLayout) -> Vec<Pane> {
        let mut panes = vec![Pane::Chat, Pane::Input];

        if layout.preview_area.is_some() {
            panes.push(Pane::Preview);
        }

        panes
    }

    /// Apply margin to a rectangle
    pub fn apply_margin(area: Rect, margin: Margin) -> Rect {
        Layout::default()
            .horizontal_margin(margin.horizontal)
            .vertical_margin(margin.vertical)
            .split(area)[0]
    }

    /// Create a centered rectangle within the given area
    pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
        let target_width = (area.width as u32 * percent_x as u32 / 100).max(1) as u16;
        let target_height = (area.height as u32 * percent_y as u32 / 100).max(1) as u16;

        let width = target_width.min(area.width);
        let height = target_height.min(area.height);

        let offset_x = (area.width.saturating_sub(width)) / 2;
        let offset_y = (area.height.saturating_sub(height)) / 2;

        Rect::new(area.x + offset_x, area.y + offset_y, width, height)
    }

    /// Check if the terminal size meets minimum requirements
    pub fn check_terminal_size(&self, area: Rect) -> Result<(), String> {
        if area.width < self.config.min_width {
            return Err(format!(
                "Terminal width ({}) is too small. Minimum required: {}",
                area.width, self.config.min_width
            ));
        }

        if area.height < self.config.min_height {
            return Err(format!(
                "Terminal height ({}) is too small. Minimum required: {}",
                area.height, self.config.min_height
            ));
        }

        Ok(())
    }
}

/// Helper functions for common layout operations
pub mod utils {
    use super::*;

    /// Create a popup area in the center of the given area
    pub fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
        LayoutManager::centered_rect(percent_x, percent_y, area)
    }

    /// Create a dialog area suitable for error messages
    pub fn dialog_area(area: Rect) -> Rect {
        popup_area(area, 60, 30)
    }

    /// Create a help area that takes up most of the screen
    pub fn help_area(area: Rect) -> Rect {
        popup_area(area, 80, 80)
    }

    /// Split an area into near-equal columns, distributing remainder pixels from left to right
    pub fn equal_columns(area: Rect, count: usize) -> Vec<Rect> {
        if count == 0 {
            return vec![area];
        }

        let count_u16 = count as u16;
        if count_u16 == 0 {
            return vec![area];
        }

        let base_width = area.width / count_u16;
        let mut remainder = area.width % count_u16;

        let mut columns = Vec::with_capacity(count);
        let mut current_x = area.x;

        for _ in 0..count_u16 {
            let mut width = base_width;
            if remainder > 0 {
                width = width.saturating_add(1);
                remainder -= 1;
            }

            let column = Rect::new(current_x, area.y, width, area.height);
            columns.push(column);
            current_x = current_x.saturating_add(width);
        }

        columns
    }

    /// Split an area into equal rows
    pub fn equal_rows(area: Rect, count: usize) -> Vec<Rect> {
        if count == 0 {
            return vec![area];
        }

        let percent_each = 100 / count as u16;
        let constraints: Vec<Constraint> = (0..count)
            .map(|_| Constraint::Percentage(percent_each))
            .collect();

        Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area)
            .to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_manager_creation() {
        let manager = LayoutManager::default();
        assert_eq!(manager.config().show_preview, true);
        assert_eq!(manager.config().min_width, 80);
    }

    #[test]
    fn test_toggle_preview() {
        let mut manager = LayoutManager::default();
        assert_eq!(manager.config().show_preview, true);

        manager.toggle_preview();
        assert_eq!(manager.config().show_preview, false);

        manager.toggle_preview();
        assert_eq!(manager.config().show_preview, true);
    }

    #[test]
    fn test_normal_layout_calculation() {
        let mut manager = LayoutManager::default();
        let area = Rect::new(0, 0, 100, 30);

        let layout = manager.layout(area);

        assert_eq!(layout.full_area, area);
        assert!(!layout.is_compact);
        assert!(layout.preview_area.is_some());
        assert_eq!(layout.focused_pane, Pane::Chat);
    }

    #[test]
    fn test_compact_layout_calculation() {
        let mut manager = LayoutManager::default();
        let area = Rect::new(0, 0, 50, 15); // Small area

        let layout = manager.layout(area);

        assert_eq!(layout.full_area, area);
        assert!(layout.is_compact);
        assert!(layout.preview_area.is_none()); // Hidden in compact mode
    }

    #[test]
    fn test_pane_visibility() {
        let mut manager = LayoutManager::default();
        let normal_area = Rect::new(0, 0, 100, 30);
        let compact_area = Rect::new(0, 0, 50, 15);

        // Normal layout
        assert!(manager.is_pane_visible(normal_area, Pane::Chat));
        assert!(manager.is_pane_visible(normal_area, Pane::Input));
        assert!(manager.is_pane_visible(normal_area, Pane::Preview));
        assert!(manager.is_pane_visible(normal_area, Pane::StatusBar));

        // Compact layout
        assert!(manager.is_pane_visible(compact_area, Pane::Chat));
        assert!(manager.is_pane_visible(compact_area, Pane::Input));
        assert!(!manager.is_pane_visible(compact_area, Pane::Preview));
        assert!(manager.is_pane_visible(compact_area, Pane::StatusBar));
    }

    #[test]
    fn test_pane_navigation() {
        let mut manager = LayoutManager::default();
        let area = Rect::new(0, 0, 100, 30);

        // Test next pane navigation
        let next = manager.next_focusable_pane(area, Pane::Chat);
        assert_eq!(next, Pane::Input);

        let next = manager.next_focusable_pane(area, Pane::Input);
        assert_eq!(next, Pane::Preview);

        let next = manager.next_focusable_pane(area, Pane::Preview);
        assert_eq!(next, Pane::Chat); // Wraps around

        // Test previous pane navigation
        let prev = manager.previous_focusable_pane(area, Pane::Chat);
        assert_eq!(prev, Pane::Preview);
    }

    #[test]
    fn test_terminal_size_check() {
        let manager = LayoutManager::default();

        // Valid size
        let valid_area = Rect::new(0, 0, 100, 30);
        assert!(manager.check_terminal_size(valid_area).is_ok());

        // Too small width
        let small_width = Rect::new(0, 0, 50, 30);
        assert!(manager.check_terminal_size(small_width).is_err());

        // Too small height
        let small_height = Rect::new(0, 0, 100, 15);
        assert!(manager.check_terminal_size(small_height).is_err());
    }

    #[test]
    fn test_centered_rect() {
        let area = Rect::new(0, 0, 100, 50);
        let centered = LayoutManager::centered_rect(50, 50, area);

        assert_eq!(centered.width, 50);
        assert_eq!(centered.height, 25);
        assert_eq!(centered.x, 25);
        assert_eq!(centered.y, 12);
    }

    #[test]
    fn test_equal_columns() {
        let area = Rect::new(0, 0, 100, 50);
        let columns = utils::equal_columns(area, 3);

        assert_eq!(columns.len(), 3);

        let total_width: u16 = columns.iter().map(|c| c.width).sum();
        assert_eq!(total_width, area.width);

        let min_width = columns.iter().map(|c| c.width).min().unwrap();
        let max_width = columns.iter().map(|c| c.width).max().unwrap();
        assert!(max_width - min_width <= 1);

        // Ensure columns are laid out sequentially without gaps
        let mut expected_x = area.x;
        for column in columns {
            assert_eq!(column.x, expected_x);
            expected_x = expected_x.saturating_add(column.width);
        }
    }
}
