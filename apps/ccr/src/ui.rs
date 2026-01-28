//! CCR UI Rendering
//!
//! Event list display with scrolling and permission highlighting.

extern crate alloc;
use alloc::string::String;
use core::fmt::Write;

use crate::events::{CcrEvent, EventQueue};

/// Display dimensions
pub const DISPLAY_WIDTH: i16 = 336;
pub const DISPLAY_HEIGHT: i16 = 536;

/// UI layout constants
pub const STATUS_BAR_HEIGHT: i16 = 20;
pub const FOOTER_HEIGHT: i16 = 20;
pub const CONTENT_TOP: i16 = STATUS_BAR_HEIGHT + 5;
pub const CONTENT_BOTTOM: i16 = DISPLAY_HEIGHT - FOOTER_HEIGHT - 5;
pub const CONTENT_HEIGHT: i16 = CONTENT_BOTTOM - CONTENT_TOP;

/// Lines visible in event list
pub const VISIBLE_LINES: usize = 25;

/// Characters per line
pub const CHARS_PER_LINE: usize = 40;

/// UI State
pub struct UiState {
    /// Current scroll position (index of first visible event)
    pub scroll_pos: usize,

    /// Currently selected event index (for permission approval)
    pub selected: Option<usize>,

    /// Connection status
    pub connected: bool,

    /// Session statistics
    pub tokens: u32,
    pub cost_cents: u32,
}

impl UiState {
    pub fn new() -> Self {
        Self {
            scroll_pos: 0,
            selected: None,
            connected: false,
            tokens: 0,
            cost_cents: 0,
        }
    }

    /// Scroll up by one event
    pub fn scroll_up(&mut self) {
        if self.scroll_pos > 0 {
            self.scroll_pos -= 1;
        }
    }

    /// Scroll down by one event
    pub fn scroll_down(&mut self, queue_len: usize) {
        if self.scroll_pos + VISIBLE_LINES < queue_len {
            self.scroll_pos += 1;
        }
    }

    /// Auto-scroll to show latest events
    pub fn auto_scroll(&mut self, queue_len: usize) {
        if queue_len > VISIBLE_LINES {
            self.scroll_pos = queue_len - VISIBLE_LINES;
        } else {
            self.scroll_pos = 0;
        }
    }

    /// Select next permission request
    pub fn select_next_permission(&mut self, queue: &EventQueue) {
        if let Some((idx, _)) = queue.find_pending_permission() {
            self.selected = Some(idx);
            // Scroll to show selected
            if idx < self.scroll_pos {
                self.scroll_pos = idx;
            } else if idx >= self.scroll_pos + VISIBLE_LINES {
                self.scroll_pos = idx.saturating_sub(VISIBLE_LINES / 2);
            }
        }
    }
}

impl Default for UiState {
    fn default() -> Self {
        Self::new()
    }
}

/// Render status bar text
pub fn render_status_bar(state: &UiState) -> String {
    let status = if state.connected { "●" } else { "○" };
    let conn_text = if state.connected { "Connected" } else { "Disconnected" };
    alloc::format!("CCR {} {}    ↑↓ scroll", status, conn_text)
}

/// Render footer text
pub fn render_footer(state: &UiState) -> String {
    let cost_dollars = state.cost_cents as f32 / 100.0;
    alloc::format!("Tokens: {}  Cost: ${:.2}", state.tokens, cost_dollars)
}

/// Render single event to text lines
pub fn render_event(event: &CcrEvent, selected: bool) -> String {
    let prefix = if selected { "▶ " } else { "◆ " };

    match event {
        CcrEvent::UserInput { text } => {
            let truncated = truncate_text(text, CHARS_PER_LINE - 4);
            alloc::format!("> {}", truncated)
        }

        CcrEvent::AssistantText { text } => {
            let truncated = truncate_text(text, CHARS_PER_LINE - 2);
            alloc::format!("  {}", truncated)
        }

        CcrEvent::ToolCall { tool, args, .. } => {
            let args_truncated = truncate_text(args, CHARS_PER_LINE - tool.len() - 5);
            alloc::format!("{}{}({})", prefix, tool, args_truncated)
        }

        CcrEvent::ToolResult { output, truncated, .. } => {
            let out_truncated = truncate_text(output, CHARS_PER_LINE - 4);
            let suffix = if *truncated { "..." } else { "" };
            alloc::format!("  → {}{}", out_truncated, suffix)
        }

        CcrEvent::PermissionRequest { tool, command, .. } => {
            let cmd_truncated = truncate_text(command, CHARS_PER_LINE - tool.len() - 5);
            let mut result = alloc::format!("{}{}({})", prefix, tool, cmd_truncated);
            if selected {
                result.push_str("\n  [← DENY]  [APPROVE →]");
            }
            result
        }

        CcrEvent::Stats { tokens, cost_cents } => {
            alloc::format!("  Stats: {} tokens, ${:.2}", tokens, *cost_cents as f32 / 100.0)
        }

        CcrEvent::Status { connected, message } => {
            let status = if *connected { "Connected" } else { "Disconnected" };
            alloc::format!("  {}: {}", status, message)
        }
    }
}

/// Render full event list to string
pub fn render_event_list(queue: &EventQueue, state: &UiState) -> String {
    let mut output = String::new();

    let start = state.scroll_pos;
    let end = (start + VISIBLE_LINES).min(queue.len());

    for i in start..end {
        if let Some(event) = queue.get(i) {
            let is_selected = state.selected == Some(i);
            let rendered = render_event(event, is_selected);
            writeln!(output, "{}", rendered).ok();
        }
    }

    // Pad with empty lines if needed
    let rendered_lines = end - start;
    for _ in rendered_lines..VISIBLE_LINES {
        writeln!(output).ok();
    }

    output
}

/// Truncate text to max length with ellipsis
fn truncate_text(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        String::from(text)
    } else if max_len > 3 {
        let mut result = String::from(&text[..max_len - 3]);
        result.push_str("...");
        result
    } else {
        String::from(&text[..max_len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_text() {
        assert_eq!(truncate_text("hello", 10), "hello");
        assert_eq!(truncate_text("hello world", 8), "hello...");
        assert_eq!(truncate_text("hi", 2), "hi");
    }

    #[test]
    fn test_render_tool_call() {
        let event = CcrEvent::ToolCall {
            id: String::from("t1"),
            tool: String::from("Bash"),
            args: String::from("cargo build"),
        };
        let rendered = render_event(&event, false);
        assert!(rendered.contains("Bash"));
        assert!(rendered.contains("cargo build"));
    }

    #[test]
    fn test_render_permission() {
        let event = CcrEvent::PermissionRequest {
            id: String::from("p1"),
            tool: String::from("Bash"),
            command: String::from("rm -rf target"),
            timeout_secs: 30,
        };

        // Not selected - no buttons
        let rendered = render_event(&event, false);
        assert!(!rendered.contains("APPROVE"));

        // Selected - show buttons
        let rendered = render_event(&event, true);
        assert!(rendered.contains("APPROVE"));
        assert!(rendered.contains("DENY"));
    }
}
