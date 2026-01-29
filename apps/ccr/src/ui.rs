//! CCR UI Rendering
//!
//! Event list display with scrolling and permission highlighting.
//! Designed for 336x536 monochrome display (Precursor/Clipin).

extern crate alloc;
use alloc::string::String;
use core::fmt::Write;

use crate::events::{CcrEvent, EventQueue};

/// Display dimensions (Precursor/Clipin)
pub const DISPLAY_WIDTH: usize = 336;
pub const DISPLAY_HEIGHT: usize = 536;

/// Characters per line (8px monospace font)
pub const CHARS_PER_LINE: usize = 42;

/// Lines visible in event list
pub const VISIBLE_LINES: usize = 28;

/// Current view mode
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ViewMode {
    /// Event list view
    List,
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
}

impl UiState {
    pub fn new() -> Self {
        Self {
            view: ViewMode::List,
            scroll_pos: 0,
            selected: 0,
            pending_permission: None,
            permission_choice: true, // Default to allow
            connected: false,
            session_id: String::new(),
            event_count: 0,
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
            if self.selected >= self.scroll_pos + VISIBLE_LINES {
                self.scroll_pos = self.selected - VISIBLE_LINES + 1;
            }
        }
    }

    /// Auto-scroll to show latest events
    pub fn auto_scroll(&mut self, queue_len: usize) {
        self.event_count = queue_len;
        if queue_len > 0 {
            self.selected = queue_len - 1;
            if queue_len > VISIBLE_LINES {
                self.scroll_pos = queue_len - VISIBLE_LINES;
            } else {
                self.scroll_pos = 0;
            }
        }
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
}

impl Default for UiState {
    fn default() -> Self {
        Self::new()
    }
}

/// Render header bar
pub fn render_header(state: &UiState) -> String {
    let status_icon = if state.connected { "●" } else { "○" };
    let session_short = if state.session_id.len() > 8 {
        &state.session_id[..8]
    } else {
        &state.session_id
    };

    let perm_indicator = if state.has_pending_permission() {
        " [!PERM]"
    } else {
        ""
    };

    alloc::format!(
        "{} CCR  {}  [{}/{}]{}",
        status_icon,
        session_short,
        state.selected + 1,
        state.event_count,
        perm_indicator
    )
}

/// Render footer bar based on current view
pub fn render_footer(state: &UiState) -> String {
    match state.view {
        ViewMode::List => {
            if state.has_pending_permission() {
                String::from("↑↓:Nav  →:View  ←:Perm  Enter:Detail")
            } else {
                String::from("↑↓:Navigate  →:View  Enter:Detail")
            }
        }
        ViewMode::Detail => {
            String::from("↑↓:Prev/Next  ←:Back  →:Expand")
        }
        ViewMode::Permission => {
            let choice = if state.permission_choice {
                "[ALLOW] / deny"
            } else {
                "allow / [DENY]"
            };
            alloc::format!("←→:{}  Enter:Confirm  Esc:Cancel", choice)
        }
    }
}

/// Render single event line for list view
fn render_event_line(event: &CcrEvent, selected: bool, index: usize) -> String {
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

/// Render event list view
pub fn render_event_list(queue: &EventQueue, state: &UiState) -> String {
    let mut output = String::new();

    if queue.is_empty() {
        writeln!(output, "").ok();
        writeln!(output, "  Waiting for events...").ok();
        writeln!(output, "").ok();
        writeln!(output, "  Subscribe to MQTT topics:").ok();
        writeln!(output, "    ccr/events").ok();
        writeln!(output, "    ccr/permissions/request").ok();
        return output;
    }

    let start = state.scroll_pos;
    let end = (start + VISIBLE_LINES).min(queue.len());

    for i in start..end {
        if let Some(event) = queue.get(i) {
            let is_selected = i == state.selected;
            let line = render_event_line(event, is_selected, i);
            writeln!(output, "{}", line).ok();
        }
    }

    // Pad with empty lines if needed
    let rendered = end - start;
    for _ in rendered..VISIBLE_LINES {
        writeln!(output).ok();
    }

    output
}

/// Render event detail view
pub fn render_event_detail(event: &CcrEvent) -> String {
    let mut output = String::new();

    match event {
        CcrEvent::SessionStart { session_id, source, model } => {
            writeln!(output, "  SESSION START").ok();
            writeln!(output, "  ─────────────────────────────────").ok();
            writeln!(output, "  Session: {}", session_id).ok();
            writeln!(output, "  Source:  {}", source).ok();
            if !model.is_empty() {
                writeln!(output, "  Model:   {}", model).ok();
            }
        }

        CcrEvent::SessionEnd { session_id, reason } => {
            writeln!(output, "  SESSION END").ok();
            writeln!(output, "  ─────────────────────────────────").ok();
            writeln!(output, "  Session: {}", session_id).ok();
            writeln!(output, "  Reason:  {}", reason).ok();
        }

        CcrEvent::Stop { session_id } => {
            writeln!(output, "  STOPPED").ok();
            writeln!(output, "  ─────────────────────────────────").ok();
            writeln!(output, "  Session: {}", session_id).ok();
        }

        CcrEvent::UserInput { text, session_id } => {
            writeln!(output, "  USER INPUT").ok();
            writeln!(output, "  ─────────────────────────────────").ok();
            writeln!(output, "  Session: {}", truncate_id(session_id)).ok();
            writeln!(output).ok();
            // Word wrap text
            for line in word_wrap(text, CHARS_PER_LINE - 2) {
                writeln!(output, "  {}", line).ok();
            }
        }

        CcrEvent::ToolCall { id, tool, args, session_id } => {
            writeln!(output, "  TOOL CALL: {}", tool).ok();
            writeln!(output, "  ─────────────────────────────────").ok();
            writeln!(output, "  ID:      {}", id).ok();
            writeln!(output, "  Session: {}", truncate_id(session_id)).ok();
            writeln!(output).ok();
            writeln!(output, "  Arguments:").ok();
            for line in word_wrap(args, CHARS_PER_LINE - 4) {
                writeln!(output, "    {}", line).ok();
            }
        }

        CcrEvent::ToolResult { id, output: result, session_id } => {
            writeln!(output, "  TOOL RESULT").ok();
            writeln!(output, "  ─────────────────────────────────").ok();
            writeln!(output, "  ID:      {}", id).ok();
            writeln!(output, "  Session: {}", truncate_id(session_id)).ok();
            writeln!(output).ok();
            writeln!(output, "  Output:").ok();
            for line in word_wrap(result, CHARS_PER_LINE - 4) {
                writeln!(output, "    {}", line).ok();
            }
        }

        CcrEvent::PermissionPending { request_id, tool, command, session_id } => {
            writeln!(output, "  ╔═══════════════════════════════════╗").ok();
            writeln!(output, "  ║     PERMISSION REQUEST            ║").ok();
            writeln!(output, "  ╚═══════════════════════════════════╝").ok();
            writeln!(output).ok();
            writeln!(output, "  Request: {}", request_id).ok();
            writeln!(output, "  Tool:    {}", tool).ok();
            writeln!(output, "  Session: {}", truncate_id(session_id)).ok();
            writeln!(output).ok();
            writeln!(output, "  Command:").ok();
            for line in word_wrap(command, CHARS_PER_LINE - 4) {
                writeln!(output, "    {}", line).ok();
            }
        }

        CcrEvent::PermissionResolved { request_id, decision, session_id } => {
            let icon = if decision == "allow" { "✓" } else { "✗" };
            writeln!(output, "  PERMISSION {} {}", icon, decision.to_uppercase()).ok();
            writeln!(output, "  ─────────────────────────────────").ok();
            writeln!(output, "  Request: {}", request_id).ok();
            writeln!(output, "  Session: {}", truncate_id(session_id)).ok();
        }

        CcrEvent::PermissionTimeout { request_id, session_id } => {
            writeln!(output, "  PERMISSION TIMEOUT").ok();
            writeln!(output, "  ─────────────────────────────────").ok();
            writeln!(output, "  Request: {}", request_id).ok();
            writeln!(output, "  Session: {}", truncate_id(session_id)).ok();
        }

        CcrEvent::Notification { notification_type, message, session_id } => {
            writeln!(output, "  NOTIFICATION").ok();
            writeln!(output, "  ─────────────────────────────────").ok();
            writeln!(output, "  Type:    {}", notification_type).ok();
            writeln!(output, "  Session: {}", truncate_id(session_id)).ok();
            writeln!(output).ok();
            for line in word_wrap(message, CHARS_PER_LINE - 2) {
                writeln!(output, "  {}", line).ok();
            }
        }

        CcrEvent::Status { connected, message } => {
            let status = if *connected { "CONNECTED" } else { "DISCONNECTED" };
            writeln!(output, "  STATUS: {}", status).ok();
            writeln!(output, "  ─────────────────────────────────").ok();
            writeln!(output, "  {}", message).ok();
        }
    }

    output
}

/// Render permission dialog
pub fn render_permission_dialog(event: &CcrEvent, choice: bool) -> String {
    let mut output = String::new();

    if let CcrEvent::PermissionPending { request_id, tool, command, .. } = event {
        writeln!(output).ok();
        writeln!(output, "  ╔═══════════════════════════════════╗").ok();
        writeln!(output, "  ║     PERMISSION REQUIRED           ║").ok();
        writeln!(output, "  ╚═══════════════════════════════════╝").ok();
        writeln!(output).ok();
        writeln!(output, "  Tool: {}", tool).ok();
        writeln!(output, "  ID:   {}", request_id).ok();
        writeln!(output).ok();
        writeln!(output, "  Command:").ok();
        for line in word_wrap(command, CHARS_PER_LINE - 4) {
            writeln!(output, "    {}", line).ok();
        }
        writeln!(output).ok();
        writeln!(output).ok();

        // Choice display
        if choice {
            writeln!(output, "       ┌─────────┐   ┌─────────┐").ok();
            writeln!(output, "       │ ▶ALLOW◀ │   │  DENY   │").ok();
            writeln!(output, "       └─────────┘   └─────────┘").ok();
        } else {
            writeln!(output, "       ┌─────────┐   ┌─────────┐").ok();
            writeln!(output, "       │  ALLOW  │   │ ▶DENY◀  │").ok();
            writeln!(output, "       └─────────┘   └─────────┘").ok();
        }

        writeln!(output).ok();
        writeln!(output, "  ← → to select, Enter to confirm").ok();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_word_wrap() {
        let text = "This is a test of word wrapping functionality";
        let lines = word_wrap(text, 20);
        assert!(lines.len() > 1);
        for line in &lines {
            assert!(line.len() <= 20);
        }
    }

    #[test]
    fn test_render_header() {
        let mut state = UiState::new();
        state.connected = true;
        state.session_id = String::from("test-session-123");
        state.event_count = 5;
        state.selected = 2;

        let header = render_header(&state);
        assert!(header.contains("●"));
        assert!(header.contains("test-ses"));
        assert!(header.contains("[3/5]"));
    }

    #[test]
    fn test_ui_scroll() {
        let mut state = UiState::new();
        state.event_count = 10;
        state.selected = 5;

        state.scroll_up();
        assert_eq!(state.selected, 4);

        state.scroll_down(10);
        assert_eq!(state.selected, 5);
    }
}
