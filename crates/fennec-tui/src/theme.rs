use ratatui::style::{Color, Modifier, Style};

/// Represents different UI component types for theming
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComponentType {
    Background,
    Border,
    Title,
    Text,
    Highlight,
    Selection,
    Error,
    Warning,
    Info,
    Success,
    Muted,
    ChatUser,
    ChatAssistant,
    ChatSystem,
    StatusActive,
    StatusInactive,
    PreviewBorder,
    ScrollbarThumb,
    ScrollbarTrack,
    Tab,
    TabSelected,
    ListSelected,
}

/// Color theme configuration
#[derive(Debug, Clone)]
pub struct ColorTheme {
    pub name: String,
    pub background: Color,
    pub border: Color,
    pub title: Color,
    pub text: Color,
    pub highlight: Color,
    pub selection: Color,
    pub error: Color,
    pub warning: Color,
    pub info: Color,
    pub success: Color,
    pub muted: Color,
    pub chat_user: Color,
    pub chat_assistant: Color,
    pub chat_system: Color,
    pub status_active: Color,
    pub status_inactive: Color,
    pub preview_border: Color,
    pub scrollbar_thumb: Color,
    pub scrollbar_track: Color,
    pub tab: Color,
    pub tab_selected: Color,
    pub list_selected: Color,
}

impl Default for ColorTheme {
    fn default() -> Self {
        Self::default_dark()
    }
}

impl ColorTheme {
    /// Default dark theme
    pub fn default_dark() -> Self {
        Self {
            name: "dark".to_string(),
            background: Color::Rgb(26, 27, 38),
            border: Color::Rgb(68, 71, 90),
            title: Color::Rgb(199, 146, 234),
            text: Color::Rgb(192, 202, 245),
            highlight: Color::Rgb(137, 180, 250),
            selection: Color::Rgb(49, 50, 68),
            error: Color::Rgb(243, 139, 168),
            warning: Color::Rgb(249, 226, 175),
            info: Color::Rgb(116, 199, 236),
            success: Color::Rgb(166, 227, 161),
            muted: Color::Rgb(108, 112, 134),
            chat_user: Color::Rgb(148, 226, 213),
            chat_assistant: Color::Rgb(203, 166, 247),
            chat_system: Color::Rgb(250, 179, 135),
            status_active: Color::Rgb(166, 227, 161),
            status_inactive: Color::Rgb(108, 112, 134),
            preview_border: Color::Rgb(137, 180, 250),
            scrollbar_thumb: Color::Rgb(108, 112, 134),
            scrollbar_track: Color::Rgb(49, 50, 68),
            tab: Color::Rgb(137, 180, 250),
            tab_selected: Color::Rgb(166, 227, 161),
            list_selected: Color::Rgb(49, 50, 68),
        }
    }

    /// Default light theme
    pub fn default_light() -> Self {
        Self {
            name: "light".to_string(),
            background: Color::Rgb(239, 241, 245),
            border: Color::Rgb(140, 143, 161),
            title: Color::Rgb(136, 57, 239),
            text: Color::Rgb(76, 79, 105),
            highlight: Color::Rgb(30, 102, 245),
            selection: Color::Rgb(220, 224, 232),
            error: Color::Rgb(210, 15, 57),
            warning: Color::Rgb(254, 100, 11),
            info: Color::Rgb(4, 165, 229),
            success: Color::Rgb(64, 160, 43),
            muted: Color::Rgb(156, 160, 176),
            chat_user: Color::Rgb(23, 146, 229),
            chat_assistant: Color::Rgb(136, 57, 239),
            chat_system: Color::Rgb(254, 100, 11),
            status_active: Color::Rgb(64, 160, 43),
            status_inactive: Color::Rgb(156, 160, 176),
            preview_border: Color::Rgb(30, 102, 245),
            scrollbar_thumb: Color::Rgb(156, 160, 176),
            scrollbar_track: Color::Rgb(220, 224, 232),
            tab: Color::Rgb(30, 102, 245),
            tab_selected: Color::Rgb(64, 160, 43),
            list_selected: Color::Rgb(220, 224, 232),
        }
    }

    /// Get color for a specific component type
    pub fn get_color(&self, component: ComponentType) -> Color {
        match component {
            ComponentType::Background => self.background,
            ComponentType::Border => self.border,
            ComponentType::Title => self.title,
            ComponentType::Text => self.text,
            ComponentType::Highlight => self.highlight,
            ComponentType::Selection => self.selection,
            ComponentType::Error => self.error,
            ComponentType::Warning => self.warning,
            ComponentType::Info => self.info,
            ComponentType::Success => self.success,
            ComponentType::Muted => self.muted,
            ComponentType::ChatUser => self.chat_user,
            ComponentType::ChatAssistant => self.chat_assistant,
            ComponentType::ChatSystem => self.chat_system,
            ComponentType::StatusActive => self.status_active,
            ComponentType::StatusInactive => self.status_inactive,
            ComponentType::PreviewBorder => self.preview_border,
            ComponentType::ScrollbarThumb => self.scrollbar_thumb,
            ComponentType::ScrollbarTrack => self.scrollbar_track,
            ComponentType::Tab => self.tab,
            ComponentType::TabSelected => self.tab_selected,
            ComponentType::ListSelected => self.list_selected,
        }
    }

    /// Get style for a specific component type
    pub fn get_style(&self, component: ComponentType) -> Style {
        let color = self.get_color(component);
        match component {
            ComponentType::Title => Style::default().fg(color).add_modifier(Modifier::BOLD),
            ComponentType::Highlight => Style::default().fg(color).add_modifier(Modifier::BOLD),
            ComponentType::Selection => Style::default().bg(color),
            ComponentType::Error => Style::default().fg(color).add_modifier(Modifier::BOLD),
            ComponentType::Warning => Style::default().fg(color),
            ComponentType::ChatUser => Style::default().fg(color).add_modifier(Modifier::BOLD),
            ComponentType::ChatAssistant => Style::default().fg(color),
            ComponentType::ChatSystem => Style::default().fg(color).add_modifier(Modifier::ITALIC),
            ComponentType::Tab => Style::default().fg(color),
            ComponentType::TabSelected => Style::default().fg(color).add_modifier(Modifier::BOLD),
            ComponentType::ListSelected => Style::default().bg(color),
            _ => Style::default().fg(color),
        }
    }
}

/// Theme manager for handling multiple themes and theme switching
#[derive(Debug, Clone)]
pub struct ThemeManager {
    current_theme: ColorTheme,
    available_themes: Vec<ColorTheme>,
}

impl Default for ThemeManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ThemeManager {
    /// Create a new theme manager with default themes
    pub fn new() -> Self {
        let available_themes = vec![ColorTheme::default_dark(), ColorTheme::default_light()];

        Self {
            current_theme: available_themes[0].clone(),
            available_themes,
        }
    }

    /// Get the current active theme
    pub fn current_theme(&self) -> &ColorTheme {
        &self.current_theme
    }

    /// Get all available themes
    pub fn available_themes(&self) -> &[ColorTheme] {
        &self.available_themes
    }

    /// Switch to theme by name
    pub fn set_theme(&mut self, theme_name: &str) -> Result<(), String> {
        if let Some(theme) = self
            .available_themes
            .iter()
            .find(|t| t.name == theme_name)
            .cloned()
        {
            self.current_theme = theme;
            Ok(())
        } else {
            Err(format!("Theme '{}' not found", theme_name))
        }
    }

    /// Switch to next theme in the list
    pub fn next_theme(&mut self) {
        if let Some(current_index) = self
            .available_themes
            .iter()
            .position(|t| t.name == self.current_theme.name)
        {
            let next_index = (current_index + 1) % self.available_themes.len();
            self.current_theme = self.available_themes[next_index].clone();
        }
    }

    /// Switch to previous theme in the list
    pub fn previous_theme(&mut self) {
        if let Some(current_index) = self
            .available_themes
            .iter()
            .position(|t| t.name == self.current_theme.name)
        {
            let prev_index = if current_index == 0 {
                self.available_themes.len() - 1
            } else {
                current_index - 1
            };
            self.current_theme = self.available_themes[prev_index].clone();
        }
    }

    /// Add a custom theme
    pub fn add_theme(&mut self, theme: ColorTheme) {
        // Remove existing theme with same name if it exists
        self.available_themes.retain(|t| t.name != theme.name);
        self.available_themes.push(theme);
    }

    /// Get color for component type from current theme
    pub fn get_color(&self, component: ComponentType) -> Color {
        self.current_theme.get_color(component)
    }

    /// Get style for component type from current theme
    pub fn get_style(&self, component: ComponentType) -> Style {
        self.current_theme.get_style(component)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_manager_creation() {
        let manager = ThemeManager::new();
        assert_eq!(manager.available_themes().len(), 2);
        assert_eq!(manager.current_theme().name, "dark");
    }

    #[test]
    fn test_theme_switching() {
        let mut manager = ThemeManager::new();

        // Switch to light theme
        assert!(manager.set_theme("light").is_ok());
        assert_eq!(manager.current_theme().name, "light");

        // Try to switch to non-existent theme
        assert!(manager.set_theme("nonexistent").is_err());
    }

    #[test]
    fn test_theme_cycling() {
        let mut manager = ThemeManager::new();
        let initial_theme = manager.current_theme().name.clone();

        // Cycle through themes
        manager.next_theme();
        let second_theme = manager.current_theme().name.clone();
        assert_ne!(initial_theme, second_theme);

        manager.previous_theme();
        assert_eq!(manager.current_theme().name, initial_theme);
    }

    #[test]
    fn test_custom_theme_addition() {
        let mut manager = ThemeManager::new();
        let initial_count = manager.available_themes().len();

        let custom_theme = ColorTheme {
            name: "custom".to_string(),
            ..ColorTheme::default_dark()
        };

        manager.add_theme(custom_theme);
        assert_eq!(manager.available_themes().len(), initial_count + 1);

        assert!(manager.set_theme("custom").is_ok());
        assert_eq!(manager.current_theme().name, "custom");
    }

    #[test]
    fn test_component_styling() {
        let theme = ColorTheme::default_dark();

        // Test that different components have different colors
        assert_ne!(
            theme.get_color(ComponentType::Error),
            theme.get_color(ComponentType::Success)
        );

        // Test that title has bold modifier
        let title_style = theme.get_style(ComponentType::Title);
        assert!(title_style.add_modifier.contains(Modifier::BOLD));
    }
}
