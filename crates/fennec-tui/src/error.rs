use fennec_core::error::{ErrorCategory, ErrorInfo, ErrorSeverity, RecoveryAction};
use ratatui::{
    prelude::*,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Widget, Wrap},
};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, TuiError>;

#[derive(Error, Debug)]
pub enum TuiError {
    // Terminal initialization and management errors
    #[error("Failed to initialize terminal: {reason}")]
    TerminalInitFailed { reason: String },

    #[error(
        "Terminal size too small: {width}x{height}. Minimum required: {min_width}x{min_height}"
    )]
    TerminalTooSmall {
        width: u16,
        height: u16,
        min_width: u16,
        min_height: u16,
    },

    #[error("Terminal capabilities insufficient: missing {capability}")]
    TerminalCapabilityMissing { capability: String },

    #[error("Failed to restore terminal: {reason}")]
    TerminalRestoreFailed { reason: String },

    // Input handling errors
    #[error("Invalid input: '{input}'. Expected: {expected}")]
    InvalidInput { input: String, expected: String },

    #[error("Input buffer full. Please wait or cancel current operation")]
    InputBufferFull,

    #[error("Input timeout: no input received within {timeout_ms}ms")]
    InputTimeout { timeout_ms: u64 },

    #[error("Unsupported input: '{input}' not available in current mode")]
    UnsupportedInput { input: String, mode: String },

    // Rendering and display errors
    #[error("Rendering failed for component '{component}': {reason}")]
    RenderingFailed { component: String, reason: String },

    #[error("Layout calculation failed: {reason}")]
    LayoutFailed { reason: String },

    #[error("Component state corrupted: '{component}' - {details}")]
    ComponentStateCorrupted { component: String, details: String },

    #[error("Display buffer overflow: {component} exceeds {limit} bytes")]
    DisplayBufferOverflow { component: String, limit: usize },

    // Theme and styling errors
    #[error("Theme not found: '{theme}'")]
    ThemeNotFound { theme: String },

    #[error("Theme loading failed: '{theme}' - {reason}")]
    ThemeLoadFailed { theme: String, reason: String },

    #[error("Invalid color specification: '{color}'. Use format: #RRGGBB or color name")]
    InvalidColor { color: String },

    #[error("Style parsing failed: '{style}' - {reason}")]
    StyleParsingFailed { style: String, reason: String },

    // Component and widget errors
    #[error("Component not found: '{component}'")]
    ComponentNotFound { component: String },

    #[error("Component initialization failed: '{component}' - {reason}")]
    ComponentInitFailed { component: String, reason: String },

    #[error("Widget configuration invalid: '{widget}' - {issue}")]
    WidgetConfigInvalid { widget: String, issue: String },

    #[error("Component update failed: '{component}' - {reason}")]
    ComponentUpdateFailed { component: String, reason: String },

    // State management errors
    #[error("Application state corrupted: {component}")]
    AppStateCorrupted { component: String },

    #[error("State transition invalid: from '{from}' to '{to}'")]
    InvalidStateTransition { from: String, to: String },

    #[error("State synchronization failed: {reason}")]
    StateSyncFailed { reason: String },

    #[error("State persistence failed: {operation} - {reason}")]
    StatePersistenceFailed { operation: String, reason: String },

    // Event handling errors
    #[error("Event processing failed: {event_type} - {reason}")]
    EventProcessingFailed { event_type: String, reason: String },

    #[error("Event queue overflow: {pending_count} events pending")]
    EventQueueOverflow { pending_count: usize },

    #[error("Event handling timeout: {event_type} exceeded {timeout_ms}ms")]
    EventTimeout { event_type: String, timeout_ms: u64 },

    // Data presentation errors
    #[error("Data formatting failed: {data_type} - {reason}")]
    DataFormattingFailed { data_type: String, reason: String },

    #[error("Content too large for display: {size} > {max_size}")]
    ContentTooLarge { size: usize, max_size: usize },

    #[error("Pagination failed: page {page} of {total_pages}")]
    PaginationFailed { page: usize, total_pages: usize },

    #[error("Scrolling failed: {direction} beyond {boundary}")]
    ScrollingFailed { direction: String, boundary: String },

    // Integration errors
    #[error("Backend service error: {0}")]
    BackendService(#[from] Box<dyn std::error::Error + Send + Sync>),

    #[error("Command execution error: {0}")]
    CommandExecution(Box<dyn std::error::Error + Send + Sync>),

    // IO errors (wrapped for TUI context)
    #[error("IO operation failed: {operation} - {source}")]
    Io {
        operation: String,
        #[source]
        source: std::io::Error,
    },

    // Generic errors
    #[error("TUI error: {message}")]
    Generic {
        message: String,
        context: Option<String>,
    },
}

impl ErrorInfo for TuiError {
    fn category(&self) -> ErrorCategory {
        match self {
            // User errors
            TuiError::InvalidInput { .. }
            | TuiError::UnsupportedInput { .. }
            | TuiError::ThemeNotFound { .. }
            | TuiError::InvalidColor { .. }
            | TuiError::ComponentNotFound { .. }
            | TuiError::WidgetConfigInvalid { .. } => ErrorCategory::User,

            // System errors
            TuiError::TerminalInitFailed { .. }
            | TuiError::TerminalTooSmall { .. }
            | TuiError::TerminalCapabilityMissing { .. }
            | TuiError::TerminalRestoreFailed { .. }
            | TuiError::InputBufferFull
            | TuiError::DisplayBufferOverflow { .. }
            | TuiError::EventQueueOverflow { .. }
            | TuiError::ContentTooLarge { .. }
            | TuiError::Io { .. } => ErrorCategory::System,

            // Internal errors
            TuiError::RenderingFailed { .. }
            | TuiError::LayoutFailed { .. }
            | TuiError::ComponentStateCorrupted { .. }
            | TuiError::ThemeLoadFailed { .. }
            | TuiError::StyleParsingFailed { .. }
            | TuiError::ComponentInitFailed { .. }
            | TuiError::ComponentUpdateFailed { .. }
            | TuiError::AppStateCorrupted { .. }
            | TuiError::InvalidStateTransition { .. }
            | TuiError::StateSyncFailed { .. }
            | TuiError::StatePersistenceFailed { .. }
            | TuiError::EventProcessingFailed { .. }
            | TuiError::DataFormattingFailed { .. }
            | TuiError::PaginationFailed { .. }
            | TuiError::ScrollingFailed { .. }
            | TuiError::BackendService(_)
            | TuiError::CommandExecution(_)
            | TuiError::Generic { .. } => ErrorCategory::Internal,

            TuiError::InputTimeout { .. } | TuiError::EventTimeout { .. } => ErrorCategory::System,
        }
    }

    fn severity(&self) -> ErrorSeverity {
        match self {
            // Critical errors that prevent TUI operation
            TuiError::TerminalInitFailed { .. }
            | TuiError::TerminalRestoreFailed { .. }
            | TuiError::AppStateCorrupted { .. } => ErrorSeverity::Critical,

            // Errors that significantly impact functionality
            TuiError::TerminalTooSmall { .. }
            | TuiError::TerminalCapabilityMissing { .. }
            | TuiError::ComponentStateCorrupted { .. }
            | TuiError::EventQueueOverflow { .. }
            | TuiError::StateSyncFailed { .. } => ErrorSeverity::Error,

            // Warnings for recoverable issues
            TuiError::InputTimeout { .. }
            | TuiError::EventTimeout { .. }
            | TuiError::ContentTooLarge { .. }
            | TuiError::DisplayBufferOverflow { .. } => ErrorSeverity::Warning,

            // Standard errors
            _ => ErrorSeverity::Error,
        }
    }

    fn recovery_actions(&self) -> Vec<RecoveryAction> {
        match self {
            TuiError::TerminalTooSmall {
                min_width,
                min_height,
                ..
            } => {
                vec![RecoveryAction::ManualAction(format!(
                    "Resize terminal to at least {}x{} characters",
                    min_width, min_height
                ))]
            }

            TuiError::TerminalCapabilityMissing { capability } => {
                vec![
                    RecoveryAction::CheckConfiguration(format!(
                        "Use a terminal that supports {}",
                        capability
                    )),
                    RecoveryAction::ManualAction("Try a different terminal emulator".to_string()),
                ]
            }

            TuiError::InvalidInput { expected, .. } => {
                vec![RecoveryAction::RetryWithChanges(format!(
                    "Use: {}",
                    expected
                ))]
            }

            TuiError::InputBufferFull => {
                vec![RecoveryAction::ManualAction(
                    "Wait for current operation to complete or press Ctrl+C to cancel".to_string(),
                )]
            }

            TuiError::ThemeNotFound { .. } => {
                vec![
                    RecoveryAction::CheckConfiguration(
                        "Use a valid theme name or reset to default".to_string(),
                    ),
                    RecoveryAction::RetryWithChanges("Use 'default' theme".to_string()),
                ]
            }

            TuiError::ComponentNotFound { component } => {
                vec![
                    RecoveryAction::ManualAction(format!("Initialize {} component", component)),
                    RecoveryAction::Retry,
                ]
            }

            TuiError::EventQueueOverflow { .. } => {
                vec![
                    RecoveryAction::ManualAction(
                        "Slow down input or wait for processing to catch up".to_string(),
                    ),
                    RecoveryAction::RetryWithChanges("Restart application".to_string()),
                ]
            }

            TuiError::ContentTooLarge { .. } => {
                vec![RecoveryAction::RetryWithChanges(
                    "Use pagination or filtering to reduce content".to_string(),
                )]
            }

            TuiError::AppStateCorrupted { .. } => {
                vec![
                    RecoveryAction::ManualAction("Restart the application".to_string()),
                    RecoveryAction::ContactSupport(
                        "State corruption may indicate a bug".to_string(),
                    ),
                ]
            }

            // Most errors can be retried
            _ => vec![
                RecoveryAction::Retry,
                RecoveryAction::ContactSupport(
                    "If the problem persists, check terminal compatibility".to_string(),
                ),
            ],
        }
    }

    fn user_message(&self) -> String {
        match self {
            TuiError::TerminalTooSmall { .. } => {
                "Terminal window is too small. Please resize your terminal.".to_string()
            }
            TuiError::TerminalCapabilityMissing { .. } => {
                "Your terminal doesn't support required features. Please use a different terminal."
                    .to_string()
            }
            TuiError::InvalidInput { .. } => {
                "Invalid input. Please check your input and try again.".to_string()
            }
            TuiError::InputBufferFull => {
                "System is busy processing. Please wait a moment.".to_string()
            }
            TuiError::ThemeNotFound { .. } => {
                "Theme not found. Please select a valid theme.".to_string()
            }
            TuiError::RenderingFailed { .. } => {
                "Display error occurred. Please try again.".to_string()
            }
            TuiError::ComponentNotFound { .. } => {
                "Interface component not available. Please try restarting.".to_string()
            }
            TuiError::EventQueueOverflow { .. } => {
                "Too many actions too quickly. Please slow down.".to_string()
            }
            TuiError::ContentTooLarge { .. } => {
                "Content is too large to display. Please use filtering or pagination.".to_string()
            }
            TuiError::AppStateCorrupted { .. } => {
                "Application state error. Please restart the application.".to_string()
            }
            _ => "A display error occurred. Please try again.".to_string(),
        }
    }

    fn debug_context(&self) -> Option<String> {
        match self {
            TuiError::TerminalTooSmall {
                width,
                height,
                min_width,
                min_height,
            } => Some(format!(
                "Current: {}x{}, Required: {}x{}",
                width, height, min_width, min_height
            )),
            TuiError::EventQueueOverflow { pending_count } => {
                Some(format!("Pending events: {}", pending_count))
            }
            TuiError::ContentTooLarge { size, max_size } => {
                Some(format!("Content size: {}, Max: {}", size, max_size))
            }
            TuiError::Generic {
                context: Some(context),
                ..
            } => Some(context.clone()),
            _ => None,
        }
    }
}

impl From<std::io::Error> for TuiError {
    fn from(err: std::io::Error) -> Self {
        TuiError::Io {
            operation: "terminal operation".to_string(),
            source: err,
        }
    }
}

impl From<TuiError> for fennec_core::FennecError {
    fn from(err: TuiError) -> Self {
        fennec_core::FennecError::Tui(Box::new(err))
    }
}

/// Error display widget for the TUI
#[derive(Debug)]
pub struct ErrorDisplay {
    /// The error being displayed
    pub error: Option<Box<dyn std::error::Error + Send + Sync>>,
    /// User-friendly error message
    pub user_message: String,
    /// Recovery actions available
    pub recovery_actions: Vec<RecoveryAction>,
    /// Error category for styling
    pub category: ErrorCategory,
    /// Error severity for styling
    pub severity: ErrorSeverity,
    /// Whether to show technical details
    pub show_details: bool,
    /// Debug context information
    pub debug_context: Option<String>,
}

impl ErrorDisplay {
    /// Create a new error display from any error
    pub fn from_error(error: Box<dyn std::error::Error + Send + Sync>) -> Self {
        let user_message =
            if let Some(fennec_error) = error.downcast_ref::<fennec_core::FennecError>() {
                fennec_error.user_message()
            } else if let Some(tui_error) = error.downcast_ref::<TuiError>() {
                tui_error.user_message()
            } else {
                "An unexpected error occurred. Please try again.".to_string()
            };

        let recovery_actions =
            if let Some(fennec_error) = error.downcast_ref::<fennec_core::FennecError>() {
                fennec_error.recovery_actions()
            } else if let Some(tui_error) = error.downcast_ref::<TuiError>() {
                tui_error.recovery_actions()
            } else {
                vec![RecoveryAction::Retry]
            };

        let category = if let Some(fennec_error) = error.downcast_ref::<fennec_core::FennecError>()
        {
            fennec_error.category()
        } else if let Some(tui_error) = error.downcast_ref::<TuiError>() {
            tui_error.category()
        } else {
            ErrorCategory::Internal
        };

        let severity = if let Some(fennec_error) = error.downcast_ref::<fennec_core::FennecError>()
        {
            fennec_error.severity()
        } else if let Some(tui_error) = error.downcast_ref::<TuiError>() {
            tui_error.severity()
        } else {
            ErrorSeverity::Error
        };

        let debug_context =
            if let Some(fennec_error) = error.downcast_ref::<fennec_core::FennecError>() {
                fennec_error.debug_context()
            } else if let Some(tui_error) = error.downcast_ref::<TuiError>() {
                tui_error.debug_context()
            } else {
                None
            };

        Self {
            error: Some(error),
            user_message,
            recovery_actions,
            category,
            severity,
            show_details: false,
            debug_context,
        }
    }

    /// Create an error display from a simple message
    pub fn from_message(message: String, category: ErrorCategory, severity: ErrorSeverity) -> Self {
        Self {
            error: None,
            user_message: message,
            recovery_actions: vec![RecoveryAction::Retry],
            category,
            severity,
            show_details: false,
            debug_context: None,
        }
    }

    /// Toggle technical details display
    pub fn toggle_details(&mut self) {
        self.show_details = !self.show_details;
    }

    /// Get the appropriate color for the error severity
    pub fn severity_color(&self) -> Color {
        match self.severity {
            ErrorSeverity::Info => Color::Blue,
            ErrorSeverity::Warning => Color::Yellow,
            ErrorSeverity::Error => Color::Red,
            ErrorSeverity::Critical => Color::Magenta,
        }
    }

    /// Get the appropriate icon for the error severity
    pub fn severity_icon(&self) -> &'static str {
        match self.severity {
            ErrorSeverity::Info => "ℹ",
            ErrorSeverity::Warning => "⚠",
            ErrorSeverity::Error => "✗",
            ErrorSeverity::Critical => "🔥",
        }
    }

    /// Render the error display widget
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        // Clear the area first
        Clear.render(area, buf);

        // Create the main container
        let block = Block::default()
            .title(format!(
                " {} {} Error ",
                self.severity_icon(),
                self.category
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.severity_color()));

        let inner = block.inner(area);
        block.render(area, buf);

        // Split the area for message and actions
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),                                         // Error message
                Constraint::Length(self.recovery_actions.len() as u16 + 2), // Recovery actions
            ])
            .split(inner);

        // Render error message
        let message_text = if self.show_details {
            if let Some(ref error) = self.error {
                format!("{}\n\nTechnical details: {}", self.user_message, error)
            } else {
                self.user_message.clone()
            }
        } else {
            self.user_message.clone()
        };

        let message_paragraph = Paragraph::new(message_text)
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: true });
        message_paragraph.render(chunks[0], buf);

        // Render recovery actions
        if !self.recovery_actions.is_empty() {
            let actions_title = "Suggested Actions:";
            let actions_items: Vec<ListItem> = std::iter::once(
                ListItem::new(Text::from(actions_title)).style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            )
            .chain(self.recovery_actions.iter().enumerate().map(|(i, action)| {
                ListItem::new(Text::from(format!("{}. {}", i + 1, action)))
                    .style(Style::default().fg(Color::Gray))
            }))
            .collect();

            let actions_list = List::new(actions_items);
            actions_list.render(chunks[1], buf);
        }

        // Show debug context if available and details are shown
        if self.show_details {
            if let Some(ref context) = self.debug_context {
                let debug_text = format!("\nDebug: {}", context);
                let debug_para =
                    Paragraph::new(debug_text).style(Style::default().fg(Color::DarkGray));
                // Render at bottom of message area
                let debug_area = Rect {
                    y: chunks[0].y + chunks[0].height.saturating_sub(2),
                    height: 2,
                    ..chunks[0]
                };
                debug_para.render(debug_area, buf);
            }
        }
    }
}

/// Error toast notification for brief error messages
#[derive(Debug, Clone)]
pub struct ErrorToast {
    pub message: String,
    pub severity: ErrorSeverity,
    pub duration_ms: u64,
    pub start_time: std::time::Instant,
}

impl ErrorToast {
    pub fn new(message: String, severity: ErrorSeverity, duration_ms: u64) -> Self {
        Self {
            message,
            severity,
            duration_ms,
            start_time: std::time::Instant::now(),
        }
    }

    pub fn is_expired(&self) -> bool {
        self.start_time.elapsed().as_millis() > self.duration_ms as u128
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        let color = match self.severity {
            ErrorSeverity::Info => Color::Blue,
            ErrorSeverity::Warning => Color::Yellow,
            ErrorSeverity::Error => Color::Red,
            ErrorSeverity::Critical => Color::Magenta,
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(color));

        let paragraph = Paragraph::new(self.message.clone())
            .style(Style::default().fg(Color::White))
            .block(block)
            .wrap(Wrap { trim: true });

        paragraph.render(area, buf);
    }
}

/// Helper functions for creating common error types
pub fn terminal_too_small(width: u16, height: u16, min_width: u16, min_height: u16) -> TuiError {
    TuiError::TerminalTooSmall {
        width,
        height,
        min_width,
        min_height,
    }
}

pub fn invalid_input(input: &str, expected: &str) -> TuiError {
    TuiError::InvalidInput {
        input: input.to_string(),
        expected: expected.to_string(),
    }
}

pub fn rendering_failed(component: &str, reason: &str) -> TuiError {
    TuiError::RenderingFailed {
        component: component.to_string(),
        reason: reason.to_string(),
    }
}

pub fn component_not_found(component: &str) -> TuiError {
    TuiError::ComponentNotFound {
        component: component.to_string(),
    }
}

pub fn theme_not_found(theme: &str) -> TuiError {
    TuiError::ThemeNotFound {
        theme: theme.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test error variants and display
    #[test]
    fn test_terminal_init_failed_error() {
        let err = TuiError::TerminalInitFailed {
            reason: "crossterm init failed".to_string(),
        };
        assert!(err.to_string().contains("Failed to initialize terminal"));
        assert_eq!(err.category(), ErrorCategory::System);
        assert_eq!(err.severity(), ErrorSeverity::Critical);
    }

    #[test]
    fn test_terminal_too_small_error() {
        let err = TuiError::TerminalTooSmall {
            width: 60,
            height: 20,
            min_width: 80,
            min_height: 24,
        };
        assert!(err.to_string().contains("Terminal size too small"));
        assert!(err.to_string().contains("60x20"));
        assert_eq!(err.category(), ErrorCategory::System);
        assert_eq!(err.severity(), ErrorSeverity::Error);
        let actions = err.recovery_actions();
        assert!(actions
            .iter()
            .any(|a| matches!(a, RecoveryAction::ManualAction(_))));
    }

    #[test]
    fn test_terminal_capability_missing_error() {
        let err = TuiError::TerminalCapabilityMissing {
            capability: "color support".to_string(),
        };
        assert!(err.to_string().contains("capabilities insufficient"));
        assert_eq!(err.category(), ErrorCategory::System);
        let actions = err.recovery_actions();
        assert!(actions.len() > 1);
    }

    #[test]
    fn test_terminal_restore_failed_error() {
        let err = TuiError::TerminalRestoreFailed {
            reason: "cleanup failed".to_string(),
        };
        assert!(err.to_string().contains("Failed to restore terminal"));
        assert_eq!(err.category(), ErrorCategory::System);
        assert_eq!(err.severity(), ErrorSeverity::Critical);
    }

    #[test]
    fn test_invalid_input_error() {
        let err = TuiError::InvalidInput {
            input: "xyz".to_string(),
            expected: "number 1-10".to_string(),
        };
        assert!(err.to_string().contains("Invalid input"));
        assert_eq!(err.category(), ErrorCategory::User);
        let msg = err.user_message();
        assert!(msg.contains("Invalid input"));
    }

    #[test]
    fn test_input_buffer_full_error() {
        let err = TuiError::InputBufferFull;
        assert!(err.to_string().contains("Input buffer full"));
        assert_eq!(err.category(), ErrorCategory::System);
        let msg = err.user_message();
        assert!(msg.contains("busy processing"));
    }

    #[test]
    fn test_input_timeout_error() {
        let err = TuiError::InputTimeout { timeout_ms: 5000 };
        assert!(err.to_string().contains("Input timeout"));
        assert_eq!(err.category(), ErrorCategory::System);
        assert_eq!(err.severity(), ErrorSeverity::Warning);
    }

    #[test]
    fn test_unsupported_input_error() {
        let err = TuiError::UnsupportedInput {
            input: "F12".to_string(),
            mode: "editor".to_string(),
        };
        assert!(err.to_string().contains("Unsupported input"));
        assert_eq!(err.category(), ErrorCategory::User);
    }

    #[test]
    fn test_rendering_failed_error() {
        let err = TuiError::RenderingFailed {
            component: "FileTree".to_string(),
            reason: "buffer overflow".to_string(),
        };
        assert!(err.to_string().contains("Rendering failed"));
        assert_eq!(err.category(), ErrorCategory::Internal);
        let msg = err.user_message();
        assert!(msg.contains("Display error"));
    }

    #[test]
    fn test_layout_failed_error() {
        let err = TuiError::LayoutFailed {
            reason: "constraints unsatisfiable".to_string(),
        };
        assert!(err.to_string().contains("Layout calculation failed"));
        assert_eq!(err.category(), ErrorCategory::Internal);
    }

    #[test]
    fn test_component_state_corrupted_error() {
        let err = TuiError::ComponentStateCorrupted {
            component: "Editor".to_string(),
            details: "cursor out of bounds".to_string(),
        };
        assert!(err.to_string().contains("Component state corrupted"));
        assert_eq!(err.category(), ErrorCategory::Internal);
        assert_eq!(err.severity(), ErrorSeverity::Error);
    }

    #[test]
    fn test_display_buffer_overflow_error() {
        let err = TuiError::DisplayBufferOverflow {
            component: "Terminal".to_string(),
            limit: 1024,
        };
        assert!(err.to_string().contains("Display buffer overflow"));
        assert_eq!(err.category(), ErrorCategory::System);
        assert_eq!(err.severity(), ErrorSeverity::Warning);
    }

    #[test]
    fn test_theme_not_found_error() {
        let err = TuiError::ThemeNotFound {
            theme: "monokai".to_string(),
        };
        assert!(err.to_string().contains("Theme not found"));
        assert_eq!(err.category(), ErrorCategory::User);
        let actions = err.recovery_actions();
        assert!(actions.len() > 1);
    }

    #[test]
    fn test_theme_load_failed_error() {
        let err = TuiError::ThemeLoadFailed {
            theme: "custom".to_string(),
            reason: "parse error".to_string(),
        };
        assert!(err.to_string().contains("Theme loading failed"));
        assert_eq!(err.category(), ErrorCategory::Internal);
    }

    #[test]
    fn test_invalid_color_error() {
        let err = TuiError::InvalidColor {
            color: "blue123".to_string(),
        };
        assert!(err.to_string().contains("Invalid color specification"));
        assert_eq!(err.category(), ErrorCategory::User);
    }

    #[test]
    fn test_style_parsing_failed_error() {
        let err = TuiError::StyleParsingFailed {
            style: "bold italic".to_string(),
            reason: "unknown modifier".to_string(),
        };
        assert!(err.to_string().contains("Style parsing failed"));
        assert_eq!(err.category(), ErrorCategory::Internal);
    }

    #[test]
    fn test_component_not_found_error() {
        let err = TuiError::ComponentNotFound {
            component: "StatusBar".to_string(),
        };
        assert!(err.to_string().contains("Component not found"));
        assert_eq!(err.category(), ErrorCategory::User);
        let msg = err.user_message();
        assert!(msg.contains("not available"));
    }

    #[test]
    fn test_component_init_failed_error() {
        let err = TuiError::ComponentInitFailed {
            component: "Editor".to_string(),
            reason: "no memory".to_string(),
        };
        assert!(err.to_string().contains("initialization failed"));
        assert_eq!(err.category(), ErrorCategory::Internal);
    }

    #[test]
    fn test_widget_config_invalid_error() {
        let err = TuiError::WidgetConfigInvalid {
            widget: "List".to_string(),
            issue: "negative height".to_string(),
        };
        assert!(err.to_string().contains("configuration invalid"));
        assert_eq!(err.category(), ErrorCategory::User);
    }

    #[test]
    fn test_component_update_failed_error() {
        let err = TuiError::ComponentUpdateFailed {
            component: "FileTree".to_string(),
            reason: "state mismatch".to_string(),
        };
        assert!(err.to_string().contains("update failed"));
        assert_eq!(err.category(), ErrorCategory::Internal);
    }

    #[test]
    fn test_app_state_corrupted_error() {
        let err = TuiError::AppStateCorrupted {
            component: "main".to_string(),
        };
        assert!(err.to_string().contains("state corrupted"));
        assert_eq!(err.category(), ErrorCategory::Internal);
        assert_eq!(err.severity(), ErrorSeverity::Critical);
        let msg = err.user_message();
        assert!(msg.contains("restart"));
    }

    #[test]
    fn test_invalid_state_transition_error() {
        let err = TuiError::InvalidStateTransition {
            from: "loading".to_string(),
            to: "editing".to_string(),
        };
        assert!(err.to_string().contains("State transition invalid"));
        assert_eq!(err.category(), ErrorCategory::Internal);
    }

    #[test]
    fn test_state_sync_failed_error() {
        let err = TuiError::StateSyncFailed {
            reason: "channel closed".to_string(),
        };
        assert!(err.to_string().contains("synchronization failed"));
        assert_eq!(err.category(), ErrorCategory::Internal);
        assert_eq!(err.severity(), ErrorSeverity::Error);
    }

    #[test]
    fn test_state_persistence_failed_error() {
        let err = TuiError::StatePersistenceFailed {
            operation: "save".to_string(),
            reason: "disk full".to_string(),
        };
        assert!(err.to_string().contains("persistence failed"));
        assert_eq!(err.category(), ErrorCategory::Internal);
    }

    #[test]
    fn test_event_processing_failed_error() {
        let err = TuiError::EventProcessingFailed {
            event_type: "KeyPress".to_string(),
            reason: "handler panicked".to_string(),
        };
        assert!(err.to_string().contains("Event processing failed"));
        assert_eq!(err.category(), ErrorCategory::Internal);
    }

    #[test]
    fn test_event_queue_overflow_error() {
        let err = TuiError::EventQueueOverflow {
            pending_count: 1000,
        };
        assert!(err.to_string().contains("Event queue overflow"));
        assert_eq!(err.category(), ErrorCategory::System);
        assert_eq!(err.severity(), ErrorSeverity::Error);
        let ctx = err.debug_context();
        assert!(ctx.is_some());
        assert!(ctx.unwrap().contains("1000"));
    }

    #[test]
    fn test_event_timeout_error() {
        let err = TuiError::EventTimeout {
            event_type: "Resize".to_string(),
            timeout_ms: 3000,
        };
        assert!(err.to_string().contains("Event handling timeout"));
        assert_eq!(err.category(), ErrorCategory::System);
        assert_eq!(err.severity(), ErrorSeverity::Warning);
    }

    #[test]
    fn test_data_formatting_failed_error() {
        let err = TuiError::DataFormattingFailed {
            data_type: "Json".to_string(),
            reason: "invalid utf8".to_string(),
        };
        assert!(err.to_string().contains("Data formatting failed"));
        assert_eq!(err.category(), ErrorCategory::Internal);
    }

    #[test]
    fn test_content_too_large_error() {
        let err = TuiError::ContentTooLarge {
            size: 100000,
            max_size: 50000,
        };
        assert!(err.to_string().contains("Content too large"));
        assert_eq!(err.category(), ErrorCategory::System);
        assert_eq!(err.severity(), ErrorSeverity::Warning);
        let ctx = err.debug_context();
        assert!(ctx.is_some());
    }

    #[test]
    fn test_pagination_failed_error() {
        let err = TuiError::PaginationFailed {
            page: 5,
            total_pages: 3,
        };
        assert!(err.to_string().contains("Pagination failed"));
        assert_eq!(err.category(), ErrorCategory::Internal);
    }

    #[test]
    fn test_scrolling_failed_error() {
        let err = TuiError::ScrollingFailed {
            direction: "up".to_string(),
            boundary: "top".to_string(),
        };
        assert!(err.to_string().contains("Scrolling failed"));
        assert_eq!(err.category(), ErrorCategory::Internal);
    }

    #[test]
    fn test_io_error() {
        let err = TuiError::Io {
            operation: "write".to_string(),
            source: std::io::Error::new(std::io::ErrorKind::BrokenPipe, "pipe broken"),
        };
        assert!(err.to_string().contains("IO operation failed"));
        assert_eq!(err.category(), ErrorCategory::System);
    }

    #[test]
    fn test_generic_error() {
        let err = TuiError::Generic {
            message: "something went wrong".to_string(),
            context: Some("during startup".to_string()),
        };
        assert!(err.to_string().contains("TUI error"));
        assert_eq!(err.category(), ErrorCategory::Internal);
        let ctx = err.debug_context();
        assert!(ctx.is_some());
    }

    // Test error conversions
    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "test");
        let tui_err: TuiError = io_err.into();
        assert!(matches!(tui_err, TuiError::Io { .. }));
    }

    #[test]
    fn test_fennec_error_conversion() {
        let tui_err = TuiError::InputBufferFull;
        let fennec_err: fennec_core::FennecError = tui_err.into();
        assert!(matches!(fennec_err, fennec_core::FennecError::Tui(_)));
    }

    // Test helper functions
    #[test]
    fn test_helper_terminal_too_small() {
        let err = terminal_too_small(60, 20, 80, 24);
        assert!(matches!(err, TuiError::TerminalTooSmall { .. }));
    }

    #[test]
    fn test_helper_invalid_input() {
        let err = invalid_input("abc", "number");
        assert!(matches!(err, TuiError::InvalidInput { .. }));
    }

    #[test]
    fn test_helper_rendering_failed() {
        let err = rendering_failed("Editor", "crash");
        assert!(matches!(err, TuiError::RenderingFailed { .. }));
    }

    #[test]
    fn test_helper_component_not_found() {
        let err = component_not_found("Menu");
        assert!(matches!(err, TuiError::ComponentNotFound { .. }));
    }

    #[test]
    fn test_helper_theme_not_found() {
        let err = theme_not_found("dark");
        assert!(matches!(err, TuiError::ThemeNotFound { .. }));
    }

    // Test recovery actions
    #[test]
    fn test_recovery_actions_terminal_too_small() {
        let err = TuiError::TerminalTooSmall {
            width: 60,
            height: 20,
            min_width: 80,
            min_height: 24,
        };
        let actions = err.recovery_actions();
        assert!(!actions.is_empty());
        assert!(actions
            .iter()
            .any(|a| matches!(a, RecoveryAction::ManualAction(_))));
    }

    #[test]
    fn test_recovery_actions_input_buffer_full() {
        let err = TuiError::InputBufferFull;
        let actions = err.recovery_actions();
        assert!(!actions.is_empty());
    }

    #[test]
    fn test_recovery_actions_app_state_corrupted() {
        let err = TuiError::AppStateCorrupted {
            component: "main".to_string(),
        };
        let actions = err.recovery_actions();
        assert!(actions.len() > 1);
        assert!(actions
            .iter()
            .any(|a| matches!(a, RecoveryAction::ContactSupport(_))));
    }

    // Test user messages
    #[test]
    fn test_user_messages() {
        let cases = vec![
            (
                TuiError::TerminalTooSmall {
                    width: 60,
                    height: 20,
                    min_width: 80,
                    min_height: 24,
                },
                "resize",
            ),
            (TuiError::InputBufferFull, "busy"),
            (
                TuiError::ThemeNotFound {
                    theme: "test".to_string(),
                },
                "Theme not found",
            ),
            (
                TuiError::AppStateCorrupted {
                    component: "main".to_string(),
                },
                "restart",
            ),
        ];

        for (err, expected) in cases {
            let msg = err.user_message();
            assert!(msg.to_lowercase().contains(&expected.to_lowercase()));
        }
    }

    // Test debug context
    #[test]
    fn test_debug_context_terminal_too_small() {
        let err = TuiError::TerminalTooSmall {
            width: 60,
            height: 20,
            min_width: 80,
            min_height: 24,
        };
        let ctx = err.debug_context();
        assert!(ctx.is_some());
        assert!(ctx.unwrap().contains("60x20"));
    }

    #[test]
    fn test_debug_context_event_queue_overflow() {
        let err = TuiError::EventQueueOverflow { pending_count: 500 };
        let ctx = err.debug_context();
        assert!(ctx.is_some());
        assert!(ctx.unwrap().contains("500"));
    }

    // Test ErrorDisplay
    #[test]
    fn test_error_display_from_message() {
        let display = ErrorDisplay::from_message(
            "Test error".to_string(),
            ErrorCategory::User,
            ErrorSeverity::Warning,
        );
        assert_eq!(display.user_message, "Test error");
        assert_eq!(display.category, ErrorCategory::User);
        assert_eq!(display.severity, ErrorSeverity::Warning);
        assert!(!display.show_details);
    }

    #[test]
    fn test_error_display_toggle_details() {
        let mut display = ErrorDisplay::from_message(
            "Test".to_string(),
            ErrorCategory::Internal,
            ErrorSeverity::Error,
        );
        assert!(!display.show_details);
        display.toggle_details();
        assert!(display.show_details);
        display.toggle_details();
        assert!(!display.show_details);
    }

    #[test]
    fn test_error_display_severity_color() {
        let cases = vec![
            (ErrorSeverity::Info, Color::Blue),
            (ErrorSeverity::Warning, Color::Yellow),
            (ErrorSeverity::Error, Color::Red),
            (ErrorSeverity::Critical, Color::Magenta),
        ];

        for (severity, expected_color) in cases {
            let display =
                ErrorDisplay::from_message("Test".to_string(), ErrorCategory::Internal, severity);
            assert_eq!(display.severity_color(), expected_color);
        }
    }

    #[test]
    fn test_error_display_severity_icon() {
        let cases = vec![
            (ErrorSeverity::Info, "ℹ"),
            (ErrorSeverity::Warning, "⚠"),
            (ErrorSeverity::Error, "✗"),
            (ErrorSeverity::Critical, "🔥"),
        ];

        for (severity, expected_icon) in cases {
            let display =
                ErrorDisplay::from_message("Test".to_string(), ErrorCategory::Internal, severity);
            assert_eq!(display.severity_icon(), expected_icon);
        }
    }

    // Test ErrorToast
    #[test]
    fn test_error_toast_new() {
        let toast = ErrorToast::new("Test message".to_string(), ErrorSeverity::Warning, 3000);
        assert_eq!(toast.message, "Test message");
        assert_eq!(toast.severity, ErrorSeverity::Warning);
        assert_eq!(toast.duration_ms, 3000);
        assert!(!toast.is_expired());
    }

    #[test]
    fn test_error_toast_expired() {
        let toast = ErrorToast::new("Test".to_string(), ErrorSeverity::Info, 0);
        std::thread::sleep(std::time::Duration::from_millis(1));
        assert!(toast.is_expired());
    }

    #[test]
    fn test_result_type_alias() {
        let ok_result: Result<i32> = Ok(42);
        assert!(ok_result.is_ok());
        assert_eq!(ok_result.unwrap(), 42);

        let err_result: Result<i32> = Err(TuiError::InputBufferFull);
        assert!(err_result.is_err());
    }
}
