use crate::components::{
    ChatView, InputField, Message, MessageRole, PopupDialog, PreviewPanel, StatusBar, StatusItem,
};
use crate::events::{spawn_event_listener, AppEvent, EventHandler, InputMode, KeyAction};
use crate::layout::{LayoutManager, Pane};
use crate::theme::{ComponentType, ThemeManager};

use fennec_core::Result;
use fennec_orchestration::SessionManager;
use fennec_security::{ApprovalManager, SandboxLevel, SandboxPolicy};

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, KeyEvent, MouseEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, buffer::Buffer, layout::Rect, Terminal};
use std::{
    io::{self, Stdout},
    time::{Duration, Instant},
};

use tracing::{debug, error, info, warn};

/// Application state
#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    Running,
    Quitting,
    Error(String),
}

/// Main application structure
pub struct App {
    // Core dependencies
    session_manager: SessionManager,
    sandbox_policy: Option<SandboxPolicy>,
    #[allow(dead_code)]
    approval_manager: Option<ApprovalManager>,

    // TUI components
    terminal: Terminal<CrosstermBackend<Stdout>>,
    event_handler: EventHandler,
    theme_manager: ThemeManager,
    layout_manager: LayoutManager,

    // UI components
    chat_view: ChatView,
    input_field: InputField,
    status_bar: StatusBar,
    preview_panel: PreviewPanel,

    // Application state
    state: AppState,
    focused_pane: Pane,
    show_help: bool,
    current_popup: Option<PopupDialog>,

    // Performance tracking
    last_render: Instant,
    frame_count: u64,
}

impl App {
    /// Create a new application instance (legacy method for backward compatibility)
    pub async fn new(session_manager: SessionManager, sandbox_level: SandboxLevel) -> Result<Self> {
        info!("Initializing Fennec TUI application (legacy mode)");
        warn!("Using legacy constructor - security features may be limited");

        // Initialize terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        // Initialize event handling
        let event_handler = EventHandler::new(Duration::from_millis(250));

        // Spawn background event listener
        spawn_event_listener(event_handler.sender());

        // Initialize managers and components
        let theme_manager = ThemeManager::new();
        let layout_manager = LayoutManager::default();
        let chat_view = ChatView::new();
        let input_field = InputField::new();
        let mut status_bar = StatusBar::new();
        let preview_panel = PreviewPanel::new();

        // Setup initial status bar
        Self::update_status_bar(&mut status_bar, InputMode::Normal, &sandbox_level, 0);

        Ok(Self {
            session_manager,
            sandbox_policy: None,
            approval_manager: None,
            terminal,
            event_handler,
            theme_manager,
            layout_manager,
            chat_view,
            input_field,
            status_bar,
            preview_panel,
            state: AppState::Running,
            focused_pane: Pane::Chat,
            show_help: false,
            current_popup: None,
            last_render: Instant::now(),
            frame_count: 0,
        })
    }

    /// Create a new application instance with full security integration
    pub async fn new_with_security(
        session_manager: SessionManager,
        sandbox_policy: SandboxPolicy,
        approval_manager: ApprovalManager,
    ) -> Result<Self> {
        info!("Initializing Fennec TUI application with security integration");
        info!("Sandbox level: {}", sandbox_policy.level());
        info!("Workspace: {}", sandbox_policy.workspace_path().display());
        info!("Approval required: {}", sandbox_policy.requires_approval());

        // Initialize terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        // Initialize event handling
        let event_handler = EventHandler::new(Duration::from_millis(250));

        // Spawn background event listener
        spawn_event_listener(event_handler.sender());

        // Initialize managers and components
        let theme_manager = ThemeManager::new();
        let layout_manager = LayoutManager::default();
        let chat_view = ChatView::new();
        let input_field = InputField::new();
        let mut status_bar = StatusBar::new();
        let preview_panel = PreviewPanel::new();

        // Setup initial status bar with security info
        Self::update_status_bar_with_security(
            &mut status_bar,
            InputMode::Normal,
            &sandbox_policy,
            0,
        );

        Ok(Self {
            session_manager,
            sandbox_policy: Some(sandbox_policy),
            approval_manager: Some(approval_manager),
            terminal,
            event_handler,
            theme_manager,
            layout_manager,
            chat_view,
            input_field,
            status_bar,
            preview_panel,
            state: AppState::Running,
            focused_pane: Pane::Chat,
            show_help: false,
            current_popup: None,
            last_render: Instant::now(),
            frame_count: 0,
        })
    }

    /// Main application run loop
    pub async fn run(&mut self) -> Result<()> {
        info!("Starting Fennec TUI main loop");

        // Add welcome message
        self.chat_view.add_message(Message {
            role: MessageRole::System,
            content: "Welcome to Fennec! Type 'i' to start chatting or '?' for help.".to_string(),
            timestamp: Self::current_timestamp(),
        });

        // Main event loop
        while self.state == AppState::Running {
            // Handle events
            if let Some(event) = self.event_handler.next_event().await {
                if let Err(e) = self.handle_event(event).await {
                    error!("Error handling event: {}", e);
                    self.show_error_popup(format!("Event handling error: {}", e));
                }
            }

            // Render the interface
            if let Err(e) = self.render() {
                error!("Render error: {}", e);
                self.state = AppState::Error(format!("Render error: {}", e));
            }

            // Update performance metrics
            self.frame_count += 1;
        }

        self.cleanup()?;

        // Handle final state
        match &self.state {
            AppState::Error(msg) => {
                eprintln!("Application exited with error: {}", msg);
                std::process::exit(1);
            }
            AppState::Quitting => {
                info!("Application exited normally");
            }
            _ => {}
        }

        Ok(())
    }

    /// Handle incoming events
    async fn handle_event(&mut self, event: AppEvent) -> Result<()> {
        match event {
            AppEvent::Input(input_event) => self.handle_input_event(input_event).await?,
            AppEvent::Tick => {
                self.handle_tick();
            }
            AppEvent::Resize(width, height) => {
                self.handle_resize(width, height)?;
            }
            AppEvent::Quit => {
                self.state = AppState::Quitting;
            }
            AppEvent::ThemeChanged(theme_name) => {
                if let Err(e) = self.theme_manager.set_theme(&theme_name) {
                    warn!("Failed to set theme '{}': {}", theme_name, e);
                }
            }
            AppEvent::NewMessage(content) => {
                self.chat_view.add_message(Message {
                    role: MessageRole::Assistant,
                    content,
                    timestamp: Self::current_timestamp(),
                });
            }
            AppEvent::Error(msg) => {
                self.show_error_popup(msg);
            }
            AppEvent::SessionStateChanged => {
                // Update UI based on session state changes
                self.update_status_bar_info();
            }
        }

        Ok(())
    }

    /// Handle terminal input events
    async fn handle_input_event(&mut self, event: Event) -> Result<()> {
        match event {
            Event::Key(key_event) => self.handle_key_event(key_event).await?,
            Event::Mouse(mouse_event) => {
                self.handle_mouse_event(mouse_event);
            }
            Event::Resize(width, height) => {
                self.handle_resize(width, height)?;
            }
            _ => {}
        }

        Ok(())
    }

    /// Handle keyboard input
    async fn handle_key_event(&mut self, key_event: KeyEvent) -> Result<()> {
        // Close popup if one is open
        if self.current_popup.is_some() {
            self.current_popup = None;
            return Ok(());
        }

        // Close help if open
        if self.show_help {
            self.show_help = false;
            return Ok(());
        }

        let action = self.event_handler.handle_key_event(key_event);
        self.handle_key_action(action).await
    }

    /// Handle key actions
    async fn handle_key_action(&mut self, action: KeyAction) -> Result<()> {
        match action {
            KeyAction::Quit => {
                self.state = AppState::Quitting;
            }
            KeyAction::EnterInsert => {
                self.event_handler.set_input_mode(InputMode::Insert);
                self.focused_pane = Pane::Input;
            }
            KeyAction::EnterNormal => {
                self.event_handler.set_input_mode(InputMode::Normal);
                self.focused_pane = Pane::Chat;
            }
            KeyAction::EnterCommand => {
                self.event_handler.set_input_mode(InputMode::Command);
                self.focused_pane = Pane::Input;
            }
            KeyAction::EnterSearch => {
                self.event_handler.set_input_mode(InputMode::Search);
                self.focused_pane = Pane::Input;
            }
            KeyAction::MoveUp => {
                self.handle_move_up();
            }
            KeyAction::MoveDown => {
                self.handle_move_down();
            }
            KeyAction::MoveLeft => {
                self.handle_move_left();
            }
            KeyAction::MoveRight => {
                self.handle_move_right();
            }
            KeyAction::PageUp => {
                self.handle_page_up();
            }
            KeyAction::PageDown => {
                self.handle_page_down();
            }
            KeyAction::GoToTop => {
                self.handle_go_to_top();
            }
            KeyAction::GoToBottom => {
                self.handle_go_to_bottom();
            }
            KeyAction::Send => {
                self.handle_send().await?;
            }
            KeyAction::Clear => {
                self.input_field.clear();
            }
            KeyAction::Delete => {
                self.input_field.delete();
            }
            KeyAction::Backspace => {
                self.input_field.backspace();
            }
            KeyAction::InsertChar(c) => {
                self.input_field.insert_char(c);
            }
            KeyAction::ToggleTheme => {
                self.theme_manager.next_theme();
            }
            KeyAction::FocusNext => {
                let area = self.terminal.size()?;
                self.focused_pane = self
                    .layout_manager
                    .next_focusable_pane(area, self.focused_pane);
            }
            KeyAction::FocusPrevious => {
                let area = self.terminal.size()?;
                self.focused_pane = self
                    .layout_manager
                    .previous_focusable_pane(area, self.focused_pane);
            }
            KeyAction::TogglePreview => {
                self.layout_manager.toggle_preview();
            }
            KeyAction::ShowHelp => {
                self.show_help = true;
            }
            KeyAction::Refresh => {
                // Force re-render
            }
            _ => {}
        }

        // Update status bar after any action
        self.update_status_bar_info();

        Ok(())
    }

    /// Handle mouse events
    fn handle_mouse_event(&mut self, _mouse_event: MouseEvent) {
        // TODO: Implement mouse event handling for click-to-focus, etc.
    }

    /// Handle terminal resize
    fn handle_resize(&mut self, width: u16, height: u16) -> Result<()> {
        debug!("Terminal resized to {}x{}", width, height);
        self.terminal.resize(Rect::new(0, 0, width, height))?;
        Ok(())
    }

    /// Handle tick events
    fn handle_tick(&mut self) {
        // Update any time-based animations or periodic updates
        self.update_status_bar_info();
    }

    /// Handle movement actions based on focused pane
    fn handle_move_up(&mut self) {
        match self.focused_pane {
            Pane::Chat => self.chat_view.scroll_up(1),
            Pane::Preview => self.preview_panel.scroll_up(1),
            _ => {}
        }
    }

    fn handle_move_down(&mut self) {
        match self.focused_pane {
            Pane::Chat => self.chat_view.scroll_down(1),
            Pane::Preview => self.preview_panel.scroll_down(1),
            _ => {}
        }
    }

    fn handle_move_left(&mut self) {
        if self.focused_pane == Pane::Input {
            self.input_field.move_cursor_left();
        }
    }

    fn handle_move_right(&mut self) {
        if self.focused_pane == Pane::Input {
            self.input_field.move_cursor_right();
        }
    }

    fn handle_page_up(&mut self) {
        match self.focused_pane {
            Pane::Chat => self.chat_view.scroll_up(10),
            Pane::Preview => self.preview_panel.scroll_up(10),
            _ => {}
        }
    }

    fn handle_page_down(&mut self) {
        match self.focused_pane {
            Pane::Chat => self.chat_view.scroll_down(10),
            Pane::Preview => self.preview_panel.scroll_down(10),
            _ => {}
        }
    }

    fn handle_go_to_top(&mut self) {
        match self.focused_pane {
            Pane::Chat => self.chat_view.scroll_to_top(),
            Pane::Input => self.input_field.move_cursor_to_start(),
            _ => {}
        }
    }

    fn handle_go_to_bottom(&mut self) {
        match self.focused_pane {
            Pane::Chat => self.chat_view.scroll_to_bottom(),
            Pane::Input => self.input_field.move_cursor_to_end(),
            _ => {}
        }
    }

    /// Handle send action (Enter key)
    async fn handle_send(&mut self) -> Result<()> {
        let content = self.input_field.content().trim().to_string();

        if content.is_empty() {
            return Ok(());
        }

        let mode = self.event_handler.input_mode();

        match mode {
            InputMode::Insert => {
                // Send message to chat
                self.chat_view.add_message(Message {
                    role: MessageRole::User,
                    content: content.clone(),
                    timestamp: Self::current_timestamp(),
                });

                // Forward to session manager / provider
                match self.session_manager.send_message(content.clone()).await {
                    Ok(response) => {
                        self.chat_view.add_message(Message {
                            role: MessageRole::Assistant,
                            content: response,
                            timestamp: Self::current_timestamp(),
                        });
                    }
                    Err(err) => {
                        let error_message = format!("Failed to send message: {}", err);
                        warn!("{}", error_message);
                        self.show_error_popup(error_message.clone());
                        self.chat_view.add_message(Message {
                            role: MessageRole::System,
                            content: error_message,
                            timestamp: Self::current_timestamp(),
                        });
                    }
                }

                self.input_field.clear();
            }
            InputMode::Command => {
                self.handle_command(&content).await?;
                self.input_field.clear();
                self.event_handler.set_input_mode(InputMode::Normal);
            }
            InputMode::Search => {
                self.handle_search(&content);
                self.input_field.clear();
                self.event_handler.set_input_mode(InputMode::Normal);
            }
            _ => {}
        }

        Ok(())
    }

    /// Handle command execution
    async fn handle_command(&mut self, command: &str) -> Result<()> {
        debug!("Executing command: {}", command);

        match command {
            "quit" | "q" => {
                self.state = AppState::Quitting;
            }
            "clear" => {
                self.chat_view.clear();
            }
            "theme" => {
                self.theme_manager.next_theme();
            }
            cmd if cmd.starts_with("theme ") => {
                let theme_name = cmd.strip_prefix("theme ").unwrap_or("");
                if let Err(_e) = self.theme_manager.set_theme(theme_name) {
                    self.show_error_popup(format!("Unknown theme: {}", theme_name));
                } else {
                    self.chat_view.add_message(Message {
                        role: MessageRole::System,
                        content: format!("Theme changed to: {}", theme_name),
                        timestamp: Self::current_timestamp(),
                    });
                }
            }
            "help" => {
                self.show_help = true;
            }
            _ => {
                self.show_error_popup(format!("Unknown command: {}", command));
            }
        }

        Ok(())
    }

    /// Handle search
    fn handle_search(&mut self, _query: &str) {
        // TODO: Implement search functionality
        self.show_error_popup("Search functionality not yet implemented".to_string());
    }

    /// Show an error popup
    fn show_error_popup(&mut self, message: String) {
        self.current_popup = Some(PopupDialog::error("Error".to_string(), message));
    }

    /// Update status bar information
    fn update_status_bar_info(&mut self) {
        self.status_bar.clear();

        if let Some(ref sandbox_policy) = self.sandbox_policy {
            // Use security-aware status bar
            Self::update_status_bar_with_security(
                &mut self.status_bar,
                self.event_handler.input_mode(),
                sandbox_policy,
                self.chat_view.messages().len(),
            );
        } else {
            // Fallback to legacy status bar
            Self::update_status_bar(
                &mut self.status_bar,
                self.event_handler.input_mode(),
                &SandboxLevel::WorkspaceWrite, // Default fallback
                self.chat_view.messages().len(),
            );
        }
    }

    /// Update status bar with current information (legacy method)
    fn update_status_bar(
        status_bar: &mut StatusBar,
        mode: InputMode,
        sandbox_level: &SandboxLevel,
        message_count: usize,
    ) {
        // Left side items
        let mode_text = match mode {
            InputMode::Normal => "NORMAL",
            InputMode::Insert => "INSERT",
            InputMode::Command => "COMMAND",
            InputMode::Search => "SEARCH",
        };

        let mode_style = match mode {
            InputMode::Normal => ComponentType::StatusInactive,
            _ => ComponentType::StatusActive,
        };

        status_bar.add_left(StatusItem {
            label: "Mode".to_string(),
            value: mode_text.to_string(),
            style: mode_style,
        });

        status_bar.add_left(StatusItem {
            label: "Sandbox".to_string(),
            value: format!("{:?}", sandbox_level),
            style: ComponentType::Text,
        });

        // Right side items
        status_bar.add_right(StatusItem {
            label: "Messages".to_string(),
            value: message_count.to_string(),
            style: ComponentType::Text,
        });

        status_bar.add_right(StatusItem {
            label: "Help".to_string(),
            value: "?".to_string(),
            style: ComponentType::Muted,
        });
    }

    /// Update status bar with security information
    fn update_status_bar_with_security(
        status_bar: &mut StatusBar,
        mode: InputMode,
        sandbox_policy: &SandboxPolicy,
        message_count: usize,
    ) {
        // Left side items
        let mode_text = match mode {
            InputMode::Normal => "NORMAL",
            InputMode::Insert => "INSERT",
            InputMode::Command => "COMMAND",
            InputMode::Search => "SEARCH",
        };

        let mode_style = match mode {
            InputMode::Normal => ComponentType::StatusInactive,
            _ => ComponentType::StatusActive,
        };

        status_bar.add_left(StatusItem {
            label: "Mode".to_string(),
            value: mode_text.to_string(),
            style: mode_style,
        });

        // Security status with color coding
        let (sandbox_display, sandbox_style) = match sandbox_policy.level() {
            SandboxLevel::ReadOnly => ("🔒 READ-ONLY".to_string(), ComponentType::StatusActive),
            SandboxLevel::WorkspaceWrite => ("📝 WORKSPACE".to_string(), ComponentType::Text),
            SandboxLevel::FullAccess => ("⚠️ DANGER".to_string(), ComponentType::Error),
        };

        status_bar.add_left(StatusItem {
            label: "Security".to_string(),
            value: sandbox_display,
            style: sandbox_style,
        });

        // Approval indicator
        if sandbox_policy.requires_approval() {
            status_bar.add_left(StatusItem {
                label: "Approval".to_string(),
                value: "🛡️ ON".to_string(),
                style: ComponentType::StatusActive,
            });
        }

        // Right side items
        status_bar.add_right(StatusItem {
            label: "Messages".to_string(),
            value: message_count.to_string(),
            style: ComponentType::Text,
        });

        // Workspace indicator (abbreviated path)
        let workspace_display = sandbox_policy
            .workspace_path()
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("workspace")
            .to_string();

        status_bar.add_right(StatusItem {
            label: "Workspace".to_string(),
            value: workspace_display,
            style: ComponentType::Muted,
        });

        status_bar.add_right(StatusItem {
            label: "Help".to_string(),
            value: "?".to_string(),
            style: ComponentType::Muted,
        });
    }

    /// Render the application
    fn render(&mut self) -> Result<()> {
        let start_time = Instant::now();

        let result = {
            let terminal = &mut self.terminal;
            let layout_manager = &mut self.layout_manager;
            let theme_manager = &self.theme_manager;
            let chat_view = &mut self.chat_view;
            let input_field = &self.input_field;
            let preview_panel = &mut self.preview_panel;
            let status_bar = &self.status_bar;
            let focused_pane = self.focused_pane;
            let input_mode = self.event_handler.input_mode();
            let show_help = self.show_help;
            let current_popup = &self.current_popup;

            terminal.draw(|frame| {
                let area = frame.size();

                // Check minimum terminal size
                if let Err(msg) = layout_manager.check_terminal_size(area) {
                    let popup_area = crate::layout::utils::dialog_area(area);
                    let dialog = PopupDialog::warning("Terminal Too Small".to_string(), msg);
                    dialog.render(popup_area, frame.buffer_mut(), theme_manager);
                    return;
                }

                let layout = layout_manager.layout(area).clone();

                // Render main components
                chat_view.render(
                    layout.chat_area,
                    frame.buffer_mut(),
                    theme_manager,
                    focused_pane == Pane::Chat,
                );

                input_field.render(
                    layout.input_area,
                    frame.buffer_mut(),
                    theme_manager,
                    input_mode,
                );

                if let Some(preview_area) = layout.preview_area {
                    preview_panel.render(
                        preview_area,
                        frame.buffer_mut(),
                        theme_manager,
                        focused_pane == Pane::Preview,
                    );
                }

                status_bar.render(layout.status_area, frame.buffer_mut(), theme_manager);

                // Render help overlay if needed
                if show_help {
                    let help_area = crate::layout::utils::help_area(area);
                    Self::render_help_static(help_area, frame.buffer_mut(), theme_manager);
                }

                // Render popup if needed
                if let Some(popup) = current_popup {
                    let popup_area = crate::layout::utils::dialog_area(area);
                    popup.render(popup_area, frame.buffer_mut(), theme_manager);
                }
            })
        };

        result?;

        let render_time = start_time.elapsed();
        if render_time > Duration::from_millis(16) {
            warn!(
                "Slow render: {:?} (frame {})",
                render_time, self.frame_count
            );
        }

        self.last_render = Instant::now();
        Ok(())
    }

    /// Render help overlay (static version)
    fn render_help_static(area: Rect, buf: &mut Buffer, theme_manager: &ThemeManager) {
        let help_text = vec![
            "Fennec TUI Help".to_string(),
            "".to_string(),
            "Navigation:".to_string(),
            "  hjkl / Arrow keys - Move around".to_string(),
            "  Tab / Shift+Tab  - Switch panes".to_string(),
            "  Ctrl+u/d        - Page up/down".to_string(),
            "  g/G             - Go to top/bottom".to_string(),
            "".to_string(),
            "Modes:".to_string(),
            "  i               - Insert mode (chat)".to_string(),
            "  :               - Command mode".to_string(),
            "  /               - Search mode".to_string(),
            "  Esc             - Normal mode".to_string(),
            "".to_string(),
            "Commands:".to_string(),
            "  :quit           - Exit application".to_string(),
            "  :clear          - Clear chat history".to_string(),
            "  :theme [name]   - Change theme".to_string(),
            "  :help           - Show this help".to_string(),
            "".to_string(),
            "Other:".to_string(),
            "  t               - Toggle theme".to_string(),
            "  p               - Toggle preview panel".to_string(),
            "  q               - Quit".to_string(),
            "  ?               - Show/hide help".to_string(),
            "".to_string(),
            "Press any key to close this help.".to_string(),
        ];

        let mut help_panel = PreviewPanel::new();
        help_panel.set_title("Help".to_string());
        help_panel.set_content(help_text);
        help_panel.render(area, buf, theme_manager, true);
    }

    /// Render help overlay
    #[allow(dead_code)]
    fn render_help(&self, area: Rect, buf: &mut Buffer) {
        Self::render_help_static(area, buf, &self.theme_manager);
    }

    /// Get current timestamp string
    fn current_timestamp() -> String {
        use chrono::Local;
        Local::now().format("%H:%M:%S").to_string()
    }

    /// Cleanup terminal state
    fn cleanup(&mut self) -> Result<()> {
        info!("Cleaning up terminal state");
        disable_raw_mode()?;
        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        self.terminal.show_cursor()?;
        Ok(())
    }
}

/// Ensure cleanup happens even if the app panics
impl Drop for App {
    fn drop(&mut self) {
        if let Err(e) = self.cleanup() {
            eprintln!("Error during cleanup: {}", e);
        }
    }
}
