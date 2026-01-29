//! CCR UI Rendering - Improved Claude VSCode Style
//!
//! Chat-style interface with text input and permission dialogs.
//! Designed for 336x536 monochrome display (Precursor/Clipin).

extern crate alloc;
use alloc::string::String;
use core::fmt::Write;

use crate::events::{CcrEvent, EventQueue};

/// Display dimensions (Precursor/Clipin)
pub const DISPLAY_WIDTH: usize = 336;
pub const DISPLAY_HEIGHT: usize = 536;
pub const STATUS_BAR_HEIGHT: usize = 32;  // Managed by GAM
pub const USABLE_HEIGHT: usize = 504;     // 536 - 32

/// Characters per line (8px monospace font)
pub const CHARS_PER_LINE: usize = 42;

/// Layout constants
pub const HEADER_HEIGHT: usize = 3;  // lines
pub const INPUT_HEIGHT: usize = 3;   // lines
pub const PERM_HEIGHT: usize = 8;    // lines when shown
pub const CHAT_LINES: usize = 22;    // lines for messages

/// Current view mode
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ViewMode {
    /// Chat view with input
    Chat,
    /// Event detail view
    Detail,
    /// Permission dialog view
    Permission,
}

/// UI State
pub struct UiState {
    /// Current view mode
    pub view: ViewMode,

    /// Current scroll position (index of first visible event)
    pub scroll_pos: usize,

    /// Currently selected event index
    pub selected: usize,

    /// Pending permission request_id (if any)
    pub pending_permission: Option<String>,

    /// Permission choice: true = allow, false = deny
    pub permission_choice: bool,

    /// Connection status
    pub connected: bool,

    /// Current session ID
    pub session_id: String,

    /// Event count for display
    pub event_count: usize,

    /// Text input buffer
    pub input_text: String,

    /// Input cursor position
    pub input_cursor: usize,
}

impl UiState {
    pub fn new() -> Self {
        Self {
            view: ViewMode::Chat,
            scroll_pos: 0,
            selected: 0,
            pending_permission: None,
            permission_choice: true, // Default to allow
            connected: false,
            session_id: String::new(),
            event_count: 0,
            input_text: String::new(),
            input_cursor: 0,
        }
    }

    /// Scroll up by one event
    pub fn scroll_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            if self.selected < self.scroll_pos {
                self.scroll_pos = self.selected;
            }
        }
    }

    /// Scroll down by one event
    pub fn scroll_down(&mut self, queue_len: usize) {
        if queue_len > 0 && self.selected < queue_len - 1 {
            self.selected += 1;
            let visible = if self.has_pending_permission() {
                CHAT_LINES - PERM_HEIGHT
            } else {
                CHAT_LINES
            };
            if self.selected >= self.scroll_pos + visible / 2 {
                self.scroll_pos = self.selected - visible / 2 + 1;
            }
        }
    }

    /// Auto-scroll to show latest events
    pub fn auto_scroll(&mut self, queue_len: usize) {
        self.event_count = queue_len;
        if queue_len > 0 {
            self.selected = queue_len - 1;
            let visible = if self.has_pending_permission() {
                CHAT_LINES - PERM_HEIGHT
            } else {
                CHAT_LINES
            };
            if queue_len > visible / 2 {
                self.scroll_pos = queue_len - visible / 2;
            } else {
                self.scroll_pos = 0;
            }
        }
    }

    /// Scroll to top (latest event)
    pub fn scroll_to_top(&mut self) {
        if self.event_count > 0 {
            self.selected = self.event_count - 1;
            self.scroll_pos = self.selected.saturating_sub(CHAT_LINES / 2);
        }
    }

    /// Scroll to bottom (earliest event)
    pub fn scroll_to_bottom(&mut self, queue_len: usize) {
        self.selected = 0;
        self.scroll_pos = 0;
        self.event_count = queue_len;
    }

    /// Set pending permission
    pub fn set_pending_permission(&mut self, request_id: &str) {
        self.pending_permission = Some(String::from(request_id));
        self.permission_choice = true; // Default to allow
    }

    /// Clear pending permission
    pub fn clear_pending_permission(&mut self) {
        self.pending_permission = None;
    }

    /// Toggle permission choice
    pub fn toggle_permission_choice(&mut self) {
        self.permission_choice = !self.permission_choice;
    }

    /// Check if there's a pending permission
    pub fn has_pending_permission(&self) -> bool {
        self.pending_permission.is_some()
    }

    /// Check if there's a valid selection
    pub fn has_selection(&self) -> bool {
        self.selected < self.event_count
    }

    /// Clear selection (set to invalid index)
    pub fn clear_selection(&mut self) {
        // Set selected to max value to indicate no selection
        self.selected = usize::MAX;
    }

    /// Check if given index is selected
    pub fn is_selected(&self, index: usize) -> bool {
        self.selected == index && self.selected != usize::MAX
    }

    /// Add character to input
    pub fn input_add_char(&mut self, c: char) {
        if self.input_text.len() < 200 {  // Max input length
            self.input_text.push(c);
            self.input_cursor = self.input_text.len();
        }
    }

    /// Delete character from input (backspace)
    pub fn input_backspace(&mut self) {
        if !self.input_text.is_empty() {
            self.input_text.pop();
            self.input_cursor = self.input_text.len();
        }
    }

    /// Clear input
    pub fn input_clear(&mut self) {
        self.input_text.clear();
        self.input_cursor = 0;
    }

    /// Get input text
    pub fn input_get(&self) -> &str {
        &self.input_text
    }
}

impl Default for UiState {
    fn default() -> Self {
        Self::new()
    }
}

/// Render header bar
pub fn render_header(state: &UiState) -> String {
    let status_icon = if state.connected { "●" } else { "○" };
    let session_short = if state.session_id.len() > 10 {
        &state.session_id[..10]
    } else if state.session_id.is_empty() {
        "waiting"
    } else {
        &state.session_id
    };

    // Reverse counting: last event shown = 1, first event = total
    let current = if state.event_count > 0 {
        state.event_count - state.selected
    } else {
        0
    };

    let perm_indicator = if state.has_pending_permission() {
        " [!]"
    } else {
        ""
    };

    alloc::format!(
        "[{}:{}]  {}  {}{}",
        current,
        state.event_count,
        session_short,
        status_icon,
        perm_indicator
    )
}

/// Render input area
pub fn render_input(state: &UiState) -> String {
    let mut output = String::new();

    writeln!(output, "──────────────────────────────────────────").ok();

    if state.input_text.is_empty() {
        writeln!(output, "> Type your message...").ok();
    } else {
        // Word wrap input text
        let lines = word_wrap(&state.input_text, CHARS_PER_LINE - 2);
        for (i, line) in lines.iter().enumerate() {
            if i == 0 {
                writeln!(output, "> {}", line).ok();
            } else {
                writeln!(output, "  {}", line).ok();
            }
        }
    }

    output
}

/// Render permission area (when active)
pub fn render_permission_area(event: &CcrEvent, choice: bool) -> String {
    let mut output = String::new();

    if let CcrEvent::PermissionPending { request_id, tool, command, .. } = event {
        writeln!(output, "╔═══════════════════════════════════════╗").ok();
        writeln!(output, "║   PERMISSION REQUIRED                 ║").ok();
        writeln!(output, "╚═══════════════════════════════════════╝").ok();

        let cmd_short = if command.len() > 35 {
            alloc::format!("{}...", &command[..32])
        } else {
            command.clone()
        };

        writeln!(output, "{}: {}", tool, cmd_short).ok();
        writeln!(output).ok();

        if choice {
            writeln!(output, "  [▶ ALLOW]    DENY").ok();
        } else {
            writeln!(output, "    ALLOW    [▶ DENY]").ok();
        }

        writeln!(output, "←→:Select  Enter:Confirm  ↑:Cancel").ok();
    }

    output
}

/// Render single event line for chat view
fn render_event_line(event: &CcrEvent, selected: bool) -> String {
    let marker = if selected { ">" } else { " " };
    let icon = event.icon();
    let summary = event.summary();

    // Truncate summary to fit line
    let max_summary = CHARS_PER_LINE - 6; // marker + icon + spaces
    let summary_display = if summary.len() > max_summary {
        let mut s = String::from(&summary[..max_summary - 3]);
        s.push_str("...");
        s
    } else {
        summary
    };

    alloc::format!("{} [{}] {}", marker, icon, summary_display)
}

/// Render chat view
pub fn render_chat(queue: &EventQueue, state: &UiState) -> String {
    let mut output = String::new();

    // Header
    writeln!(output, "{}", render_header(state)).ok();
    writeln!(output, "──────────────────────────────────────────").ok();

    // Calculate visible lines
    let visible_lines = if state.has_pending_permission() {
        CHAT_LINES - PERM_HEIGHT
    } else {
        CHAT_LINES
    };

    if queue.is_empty() {
        writeln!(output).ok();
        writeln!(output, "  Waiting for events...").ok();
        writeln!(output).ok();
        writeln!(output, "  MQTT: {}", if state.connected { "connected" } else { "disconnected" }).ok();
    } else {
        let start = state.scroll_pos;
        let end = (start + visible_lines / 2).min(queue.len());

        for i in start..end {
            if let Some(event) = queue.get(i) {
                let is_selected = i == state.selected;
                let line = render_event_line(event, is_selected);
                writeln!(output, "{}", line).ok();
            }
        }

        // Pad with empty lines
        let rendered = end - start;
        for _ in rendered..(visible_lines / 2) {
            writeln!(output).ok();
        }
    }

    // Permission area (if active)
    if state.has_pending_permission() {
        if let Some(req_id) = &state.pending_permission {
            if let Some(event) = queue.iter().find(|e| e.request_id() == Some(req_id)) {
                write!(output, "{}", render_permission_area(event, state.permission_choice)).ok();
            }
        }
    }

    // Input area
    write!(output, "{}", render_input(state)).ok();

    output
}

/// Render event detail view
pub fn render_event_detail(event: &CcrEvent) -> String {
    let mut output = String::new();

    match event {
        CcrEvent::SessionStart { session_id, source, model } => {
            writeln!(output, "SESSION START").ok();
            writeln!(output).ok();
            writeln!(output, "Session: {}", session_id).ok();
            writeln!(output, "Source:  {}", source).ok();
            if !model.is_empty() {
                writeln!(output, "Model:   {}", model).ok();
            }
        }

        CcrEvent::UserInput { text, session_id } => {
            writeln!(output, "USER INPUT").ok();
            writeln!(output).ok();
            writeln!(output, "Session: {}", truncate_id(session_id)).ok();
            writeln!(output).ok();
            for line in word_wrap(text, CHARS_PER_LINE - 2) {
                writeln!(output, "{}", line).ok();
            }
        }

        CcrEvent::ToolCall { id, tool, args, session_id } => {
            writeln!(output, "TOOL CALL: {}", tool).ok();
            writeln!(output).ok();
            writeln!(output, "ID:      {}", id).ok();
            writeln!(output, "Session: {}", truncate_id(session_id)).ok();
            writeln!(output).ok();
            writeln!(output, "Arguments:").ok();
            for line in word_wrap(args, CHARS_PER_LINE - 2) {
                writeln!(output, "  {}", line).ok();
            }
        }

        CcrEvent::ToolResult { id, output: result_output, session_id, .. } => {
            writeln!(output, "TOOL RESULT").ok();
            writeln!(output).ok();
            writeln!(output, "ID:      {}", id).ok();
            writeln!(output, "Session: {}", truncate_id(session_id)).ok();
            writeln!(output).ok();
            writeln!(output, "Output:").ok();
            for line in word_wrap(result_output, CHARS_PER_LINE - 2) {
                writeln!(output, "  {}", line).ok();
            }
        }

        CcrEvent::PermissionPending { request_id, tool, command, session_id } => {
            writeln!(output, "PERMISSION REQUEST").ok();
            writeln!(output).ok();
            writeln!(output, "Request: {}", request_id).ok();
            writeln!(output, "Tool:    {}", tool).ok();
            writeln!(output, "Session: {}", truncate_id(session_id)).ok();
            writeln!(output).ok();
            writeln!(output, "Command:").ok();
            for line in word_wrap(command, CHARS_PER_LINE - 2) {
                writeln!(output, "  {}", line).ok();
            }
        }

        CcrEvent::Status { connected, message } => {
            let status = if *connected { "CONNECTED" } else { "DISCONNECTED" };
            writeln!(output, "STATUS: {}", status).ok();
            writeln!(output).ok();
            writeln!(output, "{}", message).ok();
        }

        _ => {
            writeln!(output, "{}", event.summary()).ok();
        }
    }

    output
}

/// Truncate session ID for display
fn truncate_id(id: &str) -> &str {
    if id.len() > 12 {
        &id[..12]
    } else {
        id
    }
}

/// Word wrap text to specified width
fn word_wrap(text: &str, width: usize) -> alloc::vec::Vec<String> {
    let mut lines = alloc::vec::Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.is_empty() {
            if word.len() > width {
                // Word too long, split it
                let mut remaining = word;
                while remaining.len() > width {
                    lines.push(String::from(&remaining[..width]));
                    remaining = &remaining[width..];
                }
                current_line = String::from(remaining);
            } else {
                current_line = String::from(word);
            }
        } else if current_line.len() + 1 + word.len() <= width {
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            lines.push(current_line);
            current_line = String::from(word);
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}
