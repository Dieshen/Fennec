use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent, MouseEventKind,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time::timeout;

/// Custom application events
#[derive(Debug, Clone, PartialEq)]
pub enum AppEvent {
    /// Terminal input event
    Input(Event),
    /// Tick event for periodic updates
    Tick,
    /// Resize event
    Resize(u16, u16),
    /// Request to quit the application
    Quit,
    /// Theme change event
    ThemeChanged(String),
    /// New message event
    NewMessage(String),
    /// Error event
    Error(String),
    /// Session state change
    SessionStateChanged,
}

/// Represents different input modes for the application
#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    /// Normal navigation mode
    Normal,
    /// Text input/editing mode
    Insert,
    /// Command mode
    Command,
    /// Search mode
    Search,
}

/// Key binding actions
#[derive(Debug, Clone, PartialEq)]
pub enum KeyAction {
    /// No action
    None,
    /// Quit the application
    Quit,
    /// Switch to insert mode
    EnterInsert,
    /// Switch to normal mode
    EnterNormal,
    /// Switch to command mode
    EnterCommand,
    /// Switch to search mode
    EnterSearch,
    /// Move cursor up
    MoveUp,
    /// Move cursor down
    MoveDown,
    /// Move cursor left
    MoveLeft,
    /// Move cursor right
    MoveRight,
    /// Page up
    PageUp,
    /// Page down
    PageDown,
    /// Go to top
    GoToTop,
    /// Go to bottom
    GoToBottom,
    /// Send message/input
    Send,
    /// Clear input
    Clear,
    /// Delete character
    Delete,
    /// Backspace
    Backspace,
    /// Insert character
    InsertChar(char),
    /// Toggle theme
    ToggleTheme,
    /// Focus next pane
    FocusNext,
    /// Focus previous pane
    FocusPrevious,
    /// Toggle preview panel
    TogglePreview,
    /// Copy selection
    Copy,
    /// Paste
    Paste,
    /// Show help
    ShowHelp,
    /// Refresh
    Refresh,
}

/// Event handler for managing input and application events
#[derive(Debug)]
pub struct EventHandler {
    /// Event receiver
    receiver: mpsc::UnboundedReceiver<AppEvent>,
    /// Event sender
    sender: mpsc::UnboundedSender<AppEvent>,
    /// Current input mode
    input_mode: InputMode,
    /// Last tick time
    last_tick: Instant,
    /// Tick rate for periodic updates
    tick_rate: Duration,
    /// Last key event for duplicate detection
    last_key_event: Option<(KeyEvent, Instant)>,
    /// Duplicate threshold for key events
    duplicate_threshold: Duration,
}

impl EventHandler {
    /// Create a new event handler
    pub fn new(tick_rate: Duration) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();

        Self {
            receiver,
            sender,
            input_mode: InputMode::Normal,
            last_tick: Instant::now(),
            tick_rate,
            last_key_event: None,
            duplicate_threshold: Duration::from_millis(50), // 50ms threshold for duplicate detection
        }
    }

    /// Get the event sender for external use
    pub fn sender(&self) -> mpsc::UnboundedSender<AppEvent> {
        self.sender.clone()
    }

    /// Get the current input mode
    pub fn input_mode(&self) -> InputMode {
        self.input_mode.clone()
    }

    /// Set the input mode
    pub fn set_input_mode(&mut self, mode: InputMode) {
        self.input_mode = mode;
    }

    /// Wait for the next event with timeout
    pub async fn next_event(&mut self) -> Option<AppEvent> {
        let timeout_duration = self
            .tick_rate
            .checked_sub(self.last_tick.elapsed())
            .unwrap_or(Duration::from_millis(0));

        if let Ok(event) = timeout(timeout_duration, self.receiver.recv()).await {
            event
        } else {
            // Tick timeout occurred
            self.last_tick = Instant::now();
            Some(AppEvent::Tick)
        }
    }

    /// Send an event
    pub fn send_event(&self, event: AppEvent) -> Result<(), String> {
        self.sender
            .send(event)
            .map_err(|e| format!("Failed to send event: {}", e))
    }

    /// Handle keyboard input and return the corresponding action
    pub fn handle_key_event(&mut self, key: KeyEvent) -> KeyAction {
        // CRITICAL FIX: Only process Press events, ignore Release events
        if key.kind == KeyEventKind::Release {
            return KeyAction::None;
        }

        let now = Instant::now();

        // Check for duplicate events within threshold
        if let Some((last_key, last_time)) = self.last_key_event {
            let time_diff = now.duration_since(last_time);
            if last_key == key && time_diff < self.duplicate_threshold {
                return KeyAction::None;
            }
        }

        self.last_key_event = Some((key, now));

        // Check for global keys that work in ANY mode FIRST
        if let Some(global_action) = self.handle_global_key(key) {
            return global_action;
        }

        // Then handle mode-specific keys
        match self.input_mode {
            InputMode::Normal => self.handle_normal_mode_key(key),
            InputMode::Insert => self.handle_insert_mode_key(key),
            InputMode::Command => self.handle_command_mode_key(key),
            InputMode::Search => self.handle_search_mode_key(key),
        }
    }

    /// Handle global keys that work in any mode
    fn handle_global_key(&self, key: KeyEvent) -> Option<KeyAction> {
        match (key.modifiers, key.code) {
            // Global quit - works in any mode
            (KeyModifiers::CONTROL, KeyCode::Char('c')) => Some(KeyAction::Quit),

            // Global pane navigation - works in any mode
            (KeyModifiers::NONE, KeyCode::Tab) => Some(KeyAction::FocusNext),
            (KeyModifiers::SHIFT, KeyCode::BackTab) => Some(KeyAction::FocusPrevious),

            // Global theme toggle - works in any mode (use Ctrl+T to avoid conflicts)
            (KeyModifiers::CONTROL, KeyCode::Char('t')) => Some(KeyAction::ToggleTheme),

            // Global preview toggle - works in any mode (use Ctrl+P to avoid conflicts)
            (KeyModifiers::CONTROL, KeyCode::Char('p')) => Some(KeyAction::TogglePreview),

            // Global help - works in any mode
            (KeyModifiers::NONE, KeyCode::F(1)) => Some(KeyAction::ShowHelp),
            (KeyModifiers::CONTROL, KeyCode::Char('?')) => Some(KeyAction::ShowHelp),

            // Global quit in normal mode only (to avoid conflicts with insert mode)
            (KeyModifiers::NONE, KeyCode::Char('q')) if self.input_mode == InputMode::Normal => {
                Some(KeyAction::Quit)
            }

            _ => None,
        }
    }

    /// Handle key events in normal mode
    fn handle_normal_mode_key(&self, key: KeyEvent) -> KeyAction {
        match (key.modifiers, key.code) {
            // Mode switches (only work in normal mode)
            (KeyModifiers::NONE, KeyCode::Char('i')) => KeyAction::EnterInsert,
            (KeyModifiers::NONE, KeyCode::Char(':')) => KeyAction::EnterCommand,
            (KeyModifiers::NONE, KeyCode::Char('/')) => KeyAction::EnterSearch,

            // Navigation
            (KeyModifiers::NONE, KeyCode::Up) | (KeyModifiers::NONE, KeyCode::Char('k')) => {
                KeyAction::MoveUp
            }
            (KeyModifiers::NONE, KeyCode::Down) | (KeyModifiers::NONE, KeyCode::Char('j')) => {
                KeyAction::MoveDown
            }
            (KeyModifiers::NONE, KeyCode::Left) | (KeyModifiers::NONE, KeyCode::Char('h')) => {
                KeyAction::MoveLeft
            }
            (KeyModifiers::NONE, KeyCode::Right) | (KeyModifiers::NONE, KeyCode::Char('l')) => {
                KeyAction::MoveRight
            }

            // Page navigation
            (KeyModifiers::CONTROL, KeyCode::Char('u')) => KeyAction::PageUp,
            (KeyModifiers::CONTROL, KeyCode::Char('d')) => KeyAction::PageDown,
            (KeyModifiers::NONE, KeyCode::PageUp) => KeyAction::PageUp,
            (KeyModifiers::NONE, KeyCode::PageDown) => KeyAction::PageDown,

            // Jump to top/bottom
            (KeyModifiers::NONE, KeyCode::Char('g')) => KeyAction::GoToTop,
            (KeyModifiers::NONE, KeyCode::Char('G')) => KeyAction::GoToBottom,
            (KeyModifiers::NONE, KeyCode::Home) => KeyAction::GoToTop,
            (KeyModifiers::NONE, KeyCode::End) => KeyAction::GoToBottom,

            // UI toggles (mode-specific, use single keys)
            (KeyModifiers::NONE, KeyCode::Char('t')) => KeyAction::ToggleTheme,
            (KeyModifiers::NONE, KeyCode::Char('p')) => KeyAction::TogglePreview,

            // Copy/paste
            (KeyModifiers::CONTROL, KeyCode::Char('y')) => KeyAction::Copy,
            // Note: Ctrl+P is now global for preview toggle, so remove this line
            // (KeyModifiers::CONTROL, KeyCode::Char('p')) => KeyAction::Paste,

            // Utility
            (KeyModifiers::NONE, KeyCode::Char('?')) => KeyAction::ShowHelp,
            (KeyModifiers::NONE, KeyCode::F(5)) => KeyAction::Refresh,
            (KeyModifiers::CONTROL, KeyCode::Char('r')) => KeyAction::Refresh,

            _ => KeyAction::None,
        }
    }

    /// Handle key events in insert mode
    fn handle_insert_mode_key(&self, key: KeyEvent) -> KeyAction {
        match (key.modifiers, key.code) {
            // Exit insert mode (Esc only, Ctrl+C is handled globally)
            (KeyModifiers::NONE, KeyCode::Esc) => KeyAction::EnterNormal,

            // Send message
            (KeyModifiers::NONE, KeyCode::Enter) => KeyAction::Send,
            (KeyModifiers::CONTROL, KeyCode::Char('m')) => KeyAction::Send,

            // Text editing
            (KeyModifiers::NONE, KeyCode::Backspace) => KeyAction::Backspace,
            (KeyModifiers::NONE, KeyCode::Delete) => KeyAction::Delete,
            (KeyModifiers::CONTROL, KeyCode::Char('u')) => KeyAction::Clear,

            // Navigation in insert mode
            (KeyModifiers::CONTROL, KeyCode::Char('a')) => KeyAction::GoToTop,
            (KeyModifiers::CONTROL, KeyCode::Char('e')) => KeyAction::GoToBottom,
            (KeyModifiers::CONTROL, KeyCode::Left) => KeyAction::MoveLeft,
            (KeyModifiers::CONTROL, KeyCode::Right) => KeyAction::MoveRight,

            // Insert character
            (KeyModifiers::NONE, KeyCode::Char(c)) => KeyAction::InsertChar(c),
            (KeyModifiers::SHIFT, KeyCode::Char(c)) => KeyAction::InsertChar(c),

            _ => KeyAction::None,
        }
    }

    /// Handle key events in command mode
    fn handle_command_mode_key(&self, key: KeyEvent) -> KeyAction {
        match (key.modifiers, key.code) {
            // Exit command mode (Esc only, Ctrl+C is handled globally)
            (KeyModifiers::NONE, KeyCode::Esc) => KeyAction::EnterNormal,

            // Execute command
            (KeyModifiers::NONE, KeyCode::Enter) => KeyAction::Send,

            // Text editing
            (KeyModifiers::NONE, KeyCode::Backspace) => KeyAction::Backspace,
            (KeyModifiers::NONE, KeyCode::Delete) => KeyAction::Delete,
            (KeyModifiers::CONTROL, KeyCode::Char('u')) => KeyAction::Clear,

            // Insert character
            (KeyModifiers::NONE, KeyCode::Char(c)) => KeyAction::InsertChar(c),
            (KeyModifiers::SHIFT, KeyCode::Char(c)) => KeyAction::InsertChar(c),

            _ => KeyAction::None,
        }
    }

    /// Handle key events in search mode
    fn handle_search_mode_key(&self, key: KeyEvent) -> KeyAction {
        match (key.modifiers, key.code) {
            // Exit search mode (Esc only, Ctrl+C is handled globally)
            (KeyModifiers::NONE, KeyCode::Esc) => KeyAction::EnterNormal,

            // Execute search
            (KeyModifiers::NONE, KeyCode::Enter) => KeyAction::Send,

            // Text editing
            (KeyModifiers::NONE, KeyCode::Backspace) => KeyAction::Backspace,
            (KeyModifiers::NONE, KeyCode::Delete) => KeyAction::Delete,
            (KeyModifiers::CONTROL, KeyCode::Char('u')) => KeyAction::Clear,

            // Insert character
            (KeyModifiers::NONE, KeyCode::Char(c)) => KeyAction::InsertChar(c),
            (KeyModifiers::SHIFT, KeyCode::Char(c)) => KeyAction::InsertChar(c),

            _ => KeyAction::None,
        }
    }

    /// Handle mouse events
    pub fn handle_mouse_event(&self, mouse: MouseEvent) -> KeyAction {
        match mouse.kind {
            MouseEventKind::ScrollUp => KeyAction::MoveUp,
            MouseEventKind::ScrollDown => KeyAction::MoveDown,
            MouseEventKind::Down(_) => {
                // Handle click events for focus changes
                KeyAction::None // For now, will be expanded when we have click targets
            }
            _ => KeyAction::None,
        }
    }
}

/// Global flag to prevent multiple event listeners from being spawned
static EVENT_LISTENER_SPAWNED: AtomicBool = AtomicBool::new(false);
static EVENT_LISTENER_LOCK: Mutex<()> = Mutex::new(());

/// Spawns a background task to capture terminal events
/// This function ensures that only one event listener is running at a time
pub fn spawn_event_listener(sender: mpsc::UnboundedSender<AppEvent>) {
    // Use mutex to prevent race conditions in event listener spawning
    let _lock = EVENT_LISTENER_LOCK.lock().unwrap();

    // Check if an event listener is already running
    if EVENT_LISTENER_SPAWNED.load(Ordering::SeqCst) {
        tracing::warn!("Event listener already spawned, skipping duplicate spawn");
        return;
    }

    EVENT_LISTENER_SPAWNED.store(true, Ordering::SeqCst);

    tokio::task::spawn_blocking(move || {
        tracing::debug!("Starting terminal event listener");
        loop {
            match crossterm::event::read() {
                Ok(Event::Resize(w, h)) => {
                    if sender.send(AppEvent::Resize(w, h)).is_err() {
                        break;
                    }
                }
                Ok(other) => {
                    if sender.send(AppEvent::Input(other)).is_err() {
                        break;
                    }
                }
                Err(err) => {
                    let msg = format!("Input error: {}", err);
                    // If we cannot notify the main loop, exit.
                    if sender.send(AppEvent::Error(msg)).is_err() {
                        break;
                    }
                }
            }
        }

        // Reset the flag when the event listener exits
        EVENT_LISTENER_SPAWNED.store(false, Ordering::SeqCst);
        tracing::debug!("Terminal event listener stopped");
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_handler_creation() {
        let handler = EventHandler::new(Duration::from_millis(250));
        assert_eq!(handler.input_mode(), InputMode::Normal);
    }

    #[test]
    fn test_input_mode_switching() {
        let mut handler = EventHandler::new(Duration::from_millis(250));

        // Test mode switching
        handler.set_input_mode(InputMode::Insert);
        assert_eq!(handler.input_mode(), InputMode::Insert);

        handler.set_input_mode(InputMode::Command);
        assert_eq!(handler.input_mode(), InputMode::Command);
    }

    #[test]
    fn test_normal_mode_key_handling() {
        let mut handler = EventHandler::new(Duration::from_millis(250));

        // Test quit keys
        let quit_key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        assert_eq!(handler.handle_key_event(quit_key), KeyAction::Quit);

        // Test mode switch
        let insert_key = KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE);
        assert_eq!(handler.handle_key_event(insert_key), KeyAction::EnterInsert);

        // Test navigation
        let up_key = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
        assert_eq!(handler.handle_key_event(up_key), KeyAction::MoveUp);
    }

    #[test]
    fn test_insert_mode_key_handling() {
        let mut handler = EventHandler::new(Duration::from_millis(250));
        handler.set_input_mode(InputMode::Insert);

        // Test exit insert mode
        let esc_key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(handler.handle_key_event(esc_key), KeyAction::EnterNormal);

        // Test character insertion
        let char_key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        assert_eq!(
            handler.handle_key_event(char_key),
            KeyAction::InsertChar('a')
        );

        // Test send
        let enter_key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(handler.handle_key_event(enter_key), KeyAction::Send);
    }

    #[test]
    fn test_event_sending() {
        let handler = EventHandler::new(Duration::from_millis(250));
        let sender = handler.sender();

        // Test that we can send events
        assert!(sender.send(AppEvent::Quit).is_ok());
        assert!(sender.send(AppEvent::Tick).is_ok());
    }

    #[tokio::test]
    async fn test_event_receiving() {
        let mut handler = EventHandler::new(Duration::from_millis(100));
        let sender = handler.sender();

        // Send a test event
        sender.send(AppEvent::Quit).unwrap();

        // Receive the event
        let event = handler.next_event().await;
        assert_eq!(event, Some(AppEvent::Quit));
    }
}
