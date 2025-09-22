use fennec_core::{ErrorCategory, ErrorInfo, ErrorSeverity, RecoveryAction};
use ratatui::{
    prelude::*,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
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
    CommandExecution(#[from] Box<dyn std::error::Error + Send + Sync>),

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
#[derive(Debug, Clone)]
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
