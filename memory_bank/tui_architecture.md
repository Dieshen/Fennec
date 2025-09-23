# Fennec TUI Architecture

## Terminal User Interface Overview

Fennec's TUI is built with ratatui and crossterm, providing a modern, responsive terminal interface optimized for developer workflows. The architecture emphasizes modularity, testability, and performance.

## Core TUI Components

### Application Structure
```rust
pub struct App {
    pub state: AppState,
    pub layout: Layout,
    pub event_handler: EventHandler,
    pub panels: PanelManager,
    pub theme: Theme,
}

pub enum AppState {
    Initializing,
    Ready,
    Processing,
    Error(String),
    Exiting,
}
```

### Layout Management
**Multi-Panel Layout**:
```
┌─────────────────────────────────────────┐
│ Header: Session Info & Status           │
├─────────────────┬───────────────────────┤
│                 │                       │
│ Chat Panel      │ Preview Panel         │
│ - Messages      │ - File Preview        │
│ - Input         │ - Diff View           │
│ - Commands      │ - Documentation       │
│                 │                       │
├─────────────────┴───────────────────────┤
│ Status Panel: Commands & Progress       │
└─────────────────────────────────────────┘
```

**Responsive Layout**:
- **Adaptive**: Adjusts to terminal size changes
- **Collapsible**: Panels can be hidden/shown as needed
- **Focus Management**: Keyboard navigation between panels
- **Split Views**: Horizontal and vertical panel arrangements

## Panel Architecture

### Panel Manager
```rust
pub struct PanelManager {
    panels: HashMap<PanelId, Box<dyn Panel>>,
    active_panel: PanelId,
    layout_config: LayoutConfig,
}

#[async_trait]
pub trait Panel: Send + Sync {
    fn render(&mut self, frame: &mut Frame, area: Rect);
    fn handle_input(&mut self, event: InputEvent) -> Result<EventResult>;
    fn update(&mut self, message: PanelMessage) -> Result<()>;
    fn focus(&mut self) -> Result<()>;
    fn blur(&mut self) -> Result<()>;
}
```

### Chat Panel
**Purpose**: Main conversation interface
**Components**:
- **Message List**: Scrollable conversation history
- **Input Field**: Command and message input
- **Status Indicators**: Typing indicators, processing status
- **Command Palette**: Quick command access

**Key Features**:
```rust
pub struct ChatPanel {
    messages: Vec<Message>,
    input: InputBuffer,
    scroll_state: ScrollState,
    command_palette: CommandPalette,
}

pub struct Message {
    pub id: MessageId,
    pub role: Role,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub metadata: MessageMetadata,
}
```

### Preview Panel
**Purpose**: File and content preview
**Modes**:
- **File Preview**: Syntax-highlighted code display
- **Diff View**: Git-style diff visualization
- **Documentation**: Rendered markdown display
- **Output View**: Command execution results

**Implementation**:
```rust
pub struct PreviewPanel {
    content: PreviewContent,
    syntax_highlighter: SyntaxHighlighter,
    scroll_state: ScrollState,
    search_state: SearchState,
}

pub enum PreviewContent {
    File { path: PathBuf, content: String },
    Diff { changes: Vec<DiffHunk> },
    Markdown { content: String },
    CommandOutput { output: CommandResult },
}
```

### Status Panel
**Purpose**: System status and progress indication
**Components**:
- **Command Status**: Current command execution
- **Progress Indicators**: Long-running operation progress
- **Memory Status**: Session memory and context info
- **Provider Status**: LLM provider connection status

### Summary Panel (Recently Added)
**Purpose**: Session summary management
**Features**:
- **Summary Display**: Current session summary
- **Summary History**: Previous session summaries
- **Summary Controls**: Manual summary generation
- **Export Options**: Summary export and sharing

## Event System

### Event Types
```rust
pub enum InputEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),
    Tick,
}

pub enum PanelMessage {
    MessageReceived(Message),
    CommandExecuted(CommandResult),
    FileChanged(PathBuf),
    StatusUpdate(Status),
    ThemeChanged(Theme),
}

pub enum EventResult {
    Handled,
    NotHandled,
    SwitchPanel(PanelId),
    ExecuteCommand(Command),
    Exit,
}
```

### Event Handler
```rust
pub struct EventHandler {
    key_bindings: KeyBindings,
    mouse_enabled: bool,
    event_queue: VecDeque<InputEvent>,
}

impl EventHandler {
    pub async fn handle_event(
        &mut self,
        event: InputEvent,
        app: &mut App,
    ) -> Result<EventResult> {
        match event {
            InputEvent::Key(key_event) => {
                self.handle_key_event(key_event, app).await
            }
            InputEvent::Mouse(mouse_event) => {
                self.handle_mouse_event(mouse_event, app).await
            }
            InputEvent::Resize(width, height) => {
                app.layout.resize(width, height);
                Ok(EventResult::Handled)
            }
            InputEvent::Tick => {
                app.update_animations();
                Ok(EventResult::Handled)
            }
        }
    }
}
```

## Keyboard Navigation

### Key Bindings
**Global Bindings**:
- `Ctrl+Q`: Quit application
- `Ctrl+C`: Cancel current operation
- `Tab`: Switch between panels
- `Ctrl+P`: Open command palette
- `F1`: Help and shortcuts

**Panel-Specific Bindings**:
```rust
pub struct KeyBindings {
    global: HashMap<KeyEvent, Action>,
    panel_specific: HashMap<PanelId, HashMap<KeyEvent, Action>>,
}

pub enum Action {
    Quit,
    SwitchPanel(PanelId),
    ExecuteCommand(String),
    ScrollUp,
    ScrollDown,
    TogglePreview,
    ShowHelp,
}
```

**Chat Panel Bindings**:
- `Enter`: Send message/command
- `Ctrl+Enter`: Multi-line input
- `Up/Down`: Navigate command history
- `Ctrl+L`: Clear chat
- `Esc`: Cancel input

**Preview Panel Bindings**:
- `J/K`: Scroll up/down
- `G/Shift+G`: Go to top/bottom
- `/`: Search within content
- `N`: Next search result
- `Q`: Close preview

## Theming and Styling

### Theme System
```rust
pub struct Theme {
    pub name: String,
    pub colors: ColorScheme,
    pub symbols: SymbolSet,
    pub styles: StyleConfig,
}

pub struct ColorScheme {
    pub primary: Color,
    pub secondary: Color,
    pub background: Color,
    pub foreground: Color,
    pub accent: Color,
    pub error: Color,
    pub warning: Color,
    pub success: Color,
}
```

### Built-in Themes
- **Default**: Balanced dark theme for general use
- **Light**: Light theme for bright environments
- **High Contrast**: Accessibility-focused high contrast
- **Dracula**: Popular dark theme
- **Gruvbox**: Retro, warm color scheme

### Custom Themes
**Theme Configuration**:
```toml
[theme]
name = "custom"

[theme.colors]
primary = "#61AFEF"
secondary = "#C678DD"
background = "#282C34"
foreground = "#ABB2BF"
accent = "#E06C75"
error = "#E06C75"
warning = "#E5C07B"
success = "#98C379"

[theme.symbols]
checkbox_checked = "☑"
checkbox_unchecked = "☐"
arrow_right = "→"
arrow_down = "↓"
```

## Syntax Highlighting

### Language Support
**Supported Languages**:
- **Rust**: Full syntax highlighting and semantic analysis
- **JavaScript/TypeScript**: ES6+ syntax support
- **Python**: Python 3+ syntax highlighting
- **Go**: Modern Go syntax support
- **C/C++**: Standard and modern C++ features
- **JSON/YAML**: Configuration file highlighting
- **Markdown**: Rich markdown rendering

### Highlighter Implementation
```rust
pub struct SyntaxHighlighter {
    syntect_set: SyntaxSet,
    theme_set: ThemeSet,
    current_theme: String,
}

impl SyntaxHighlighter {
    pub fn highlight(&self, content: &str, language: &str) -> Vec<StyledText> {
        let syntax = self.syntect_set
            .find_syntax_by_extension(language)
            .unwrap_or_else(|| self.syntect_set.find_syntax_plain_text());

        let theme = &self.theme_set.themes[&self.current_theme];
        let highlighter = HighlightLines::new(syntax, theme);

        // Implementation details
    }
}
```

## Performance Optimizations

### Rendering Optimizations
**Efficient Updates**:
- **Dirty Flagging**: Only re-render changed components
- **Viewport Culling**: Only render visible content
- **Lazy Loading**: Load content as needed
- **Caching**: Cache rendered content for reuse

**Memory Management**:
```rust
pub struct RenderCache {
    cached_frames: HashMap<CacheKey, CachedFrame>,
    cache_size_limit: usize,
    eviction_policy: EvictionPolicy,
}

pub struct CachedFrame {
    content: Vec<Cell>,
    timestamp: Instant,
    access_count: u64,
}
```

### Input Responsiveness
**Non-blocking Input**:
- **Async Events**: Non-blocking event processing
- **Input Buffering**: Smooth input handling under load
- **Priority Queues**: Prioritize user input events
- **Debouncing**: Reduce redundant events

### Large Content Handling
**Streaming Display**:
- **Lazy Rendering**: Render content as it becomes visible
- **Pagination**: Break large content into pages
- **Virtual Scrolling**: Efficient scrolling for large lists
- **Background Loading**: Load content in background

## Testing Strategy

### Component Testing
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn test_chat_panel_rendering() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut chat_panel = ChatPanel::new();
        chat_panel.add_message(Message::user("test message"));

        terminal.draw(|f| {
            let area = f.size();
            chat_panel.render(f, area);
        }).unwrap();

        // Assert rendered content
    }
}
```

### Integration Testing
**TUI Integration Tests**:
- **Event Simulation**: Simulate user input sequences
- **Layout Testing**: Verify layout correctness
- **State Testing**: Test state transitions
- **Performance Testing**: Measure rendering performance

### Accessibility Testing
**Accessibility Features**:
- **Screen Reader Support**: Compatible with terminal screen readers
- **High Contrast**: High contrast theme support
- **Keyboard Navigation**: Full keyboard accessibility
- **Font Scaling**: Respect terminal font size settings

## Error Handling and Recovery

### Error Display
```rust
pub struct ErrorPanel {
    error: Option<AppError>,
    stack_trace: Option<String>,
    recovery_actions: Vec<RecoveryAction>,
}

pub enum RecoveryAction {
    Retry,
    Reset,
    SaveAndExit,
    ReportBug,
}
```

### Graceful Degradation
**Fallback Strategies**:
- **Terminal Compatibility**: Fallback for limited terminals
- **Reduced Features**: Disable features on old terminals
- **Error Recovery**: Automatic recovery from UI errors
- **State Preservation**: Preserve state during errors

---

*This TUI architecture provides a robust, performant, and user-friendly terminal interface that enhances the developer experience while maintaining terminal compatibility and accessibility.*