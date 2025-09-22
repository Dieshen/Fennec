pub mod app;
pub mod components;
pub mod error;
pub mod events;
pub mod layout;
pub mod summary_panel;
pub mod theme;

// Re-export error types and components
pub use error::{ErrorDisplay, ErrorToast, Result as TuiResult, TuiError};

// Re-export summary panel components
pub use summary_panel::{SummaryGenerationStatus, SummaryPanel, SummaryPanelAction, SummaryTab};
