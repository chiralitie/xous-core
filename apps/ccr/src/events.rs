//! CCR Event Types and Queue
//!
//! Fixed-size circular buffer for event storage.
//! Drops oldest events on overflow (never blocks).

extern crate alloc;
use alloc::string::String;

/// Maximum events in queue
pub const MAX_EVENTS: usize = 64;

/// Maximum characters per event field
pub const MAX_TEXT_LEN: usize = 128;

/// Event types from Claude Code
#[derive(Clone, Debug)]
pub enum CcrEvent {
    /// User input/prompt
    UserInput {
        text: String,
    },

    /// Assistant response text
    AssistantText {
        text: String,
    },

    /// Tool call (Bash, Read, Write, Edit, etc.)
    ToolCall {
        id: String,
        tool: String,
        args: String,
    },

    /// Tool result/output
    ToolResult {
        id: String,
        output: String,
        truncated: bool,
    },

    /// Permission request (needs user approval)
    PermissionRequest {
        id: String,
        tool: String,
        command: String,
        timeout_secs: u32,
    },

    /// Session statistics
    Stats {
        tokens: u32,
        cost_cents: u32,
    },

    /// Connection status
    Status {
        connected: bool,
        message: String,
    },
}

impl CcrEvent {
    /// Parse event from JSON bytes
    pub fn from_json(data: &[u8]) -> Option<Self> {
        let text = core::str::from_utf8(data).ok()?;

        // Extract type field
        let event_type = Self::get_json_string(text, "type")?;

        match event_type.as_str() {
            "user_input" => Some(CcrEvent::UserInput {
                text: Self::get_json_string(text, "text").unwrap_or_default(),
            }),

            "assistant_text" => Some(CcrEvent::AssistantText {
                text: Self::get_json_string(text, "text").unwrap_or_default(),
            }),

            "tool_call" => Some(CcrEvent::ToolCall {
                id: Self::get_json_string(text, "id").unwrap_or_default(),
                tool: Self::get_json_string(text, "tool").unwrap_or_default(),
                args: Self::get_json_string(text, "args").unwrap_or_default(),
            }),

            "tool_result" => Some(CcrEvent::ToolResult {
                id: Self::get_json_string(text, "id").unwrap_or_default(),
                output: Self::get_json_string(text, "output").unwrap_or_default(),
                truncated: Self::get_json_bool(text, "truncated").unwrap_or(false),
            }),

            "permission" => Some(CcrEvent::PermissionRequest {
                id: Self::get_json_string(text, "id").unwrap_or_default(),
                tool: Self::get_json_string(text, "tool").unwrap_or_default(),
                command: Self::get_json_string(text, "command").unwrap_or_default(),
                timeout_secs: Self::get_json_u32(text, "timeout").unwrap_or(30),
            }),

            "stats" => Some(CcrEvent::Stats {
                tokens: Self::get_json_u32(text, "tokens").unwrap_or(0),
                cost_cents: Self::get_json_u32(text, "cost_cents").unwrap_or(0),
            }),

            _ => None,
        }
    }

    /// Simple JSON string field extractor
    fn get_json_string(text: &str, key: &str) -> Option<String> {
        let pattern = alloc::format!("\"{}\":\"", key);
        let start = text.find(&pattern)? + pattern.len();
        let end = text[start..].find('"')? + start;
        let value = &text[start..end];
        // Truncate to max length
        let truncated = if value.len() > MAX_TEXT_LEN {
            &value[..MAX_TEXT_LEN]
        } else {
            value
        };
        Some(String::from(truncated))
    }

    /// Simple JSON boolean field extractor
    fn get_json_bool(text: &str, key: &str) -> Option<bool> {
        let pattern = alloc::format!("\"{}\":", key);
        let start = text.find(&pattern)? + pattern.len();
        let rest = text[start..].trim_start();
        if rest.starts_with("true") {
            Some(true)
        } else if rest.starts_with("false") {
            Some(false)
        } else {
            None
        }
    }

    /// Simple JSON u32 field extractor
    fn get_json_u32(text: &str, key: &str) -> Option<u32> {
        let pattern = alloc::format!("\"{}\":", key);
        let start = text.find(&pattern)? + pattern.len();
        let rest = text[start..].trim_start();
        let end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
        rest[..end].parse().ok()
    }

    /// Check if this event is a permission request
    pub fn is_permission(&self) -> bool {
        matches!(self, CcrEvent::PermissionRequest { .. })
    }

    /// Get event ID if applicable
    pub fn id(&self) -> Option<&str> {
        match self {
            CcrEvent::ToolCall { id, .. } => Some(id),
            CcrEvent::ToolResult { id, .. } => Some(id),
            CcrEvent::PermissionRequest { id, .. } => Some(id),
            _ => None,
        }
    }
}

/// Fixed-size circular event queue
pub struct EventQueue {
    events: [Option<CcrEvent>; MAX_EVENTS],
    head: usize,
    tail: usize,
    count: usize,
}

impl EventQueue {
    /// Create new empty queue
    pub const fn new() -> Self {
        const NONE: Option<CcrEvent> = None;
        Self {
            events: [NONE; MAX_EVENTS],
            head: 0,
            tail: 0,
            count: 0,
        }
    }

    /// Push event to queue (drops oldest on overflow)
    pub fn push(&mut self, event: CcrEvent) {
        if self.count == MAX_EVENTS {
            // Drop oldest event
            self.events[self.head] = None;
            self.head = (self.head + 1) % MAX_EVENTS;
            self.count -= 1;
        }

        self.events[self.tail] = Some(event);
        self.tail = (self.tail + 1) % MAX_EVENTS;
        self.count += 1;
    }

    /// Get number of events in queue
    pub fn len(&self) -> usize {
        self.count
    }

    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Get event at index (0 = oldest)
    pub fn get(&self, index: usize) -> Option<&CcrEvent> {
        if index >= self.count {
            return None;
        }
        let actual_index = (self.head + index) % MAX_EVENTS;
        self.events[actual_index].as_ref()
    }

    /// Find pending permission request
    pub fn find_pending_permission(&self) -> Option<(usize, &CcrEvent)> {
        for i in 0..self.count {
            if let Some(event) = self.get(i) {
                if event.is_permission() {
                    return Some((i, event));
                }
            }
        }
        None
    }

    /// Clear all events
    pub fn clear(&mut self) {
        for i in 0..MAX_EVENTS {
            self.events[i] = None;
        }
        self.head = 0;
        self.tail = 0;
        self.count = 0;
    }
}

impl Default for EventQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_queue_push_pop() {
        let mut queue = EventQueue::new();
        assert!(queue.is_empty());

        queue.push(CcrEvent::Stats { tokens: 100, cost_cents: 1 });
        assert_eq!(queue.len(), 1);

        queue.push(CcrEvent::Stats { tokens: 200, cost_cents: 2 });
        assert_eq!(queue.len(), 2);
    }

    #[test]
    fn test_event_queue_overflow() {
        let mut queue = EventQueue::new();

        // Fill queue
        for i in 0..MAX_EVENTS {
            queue.push(CcrEvent::Stats { tokens: i as u32, cost_cents: 0 });
        }
        assert_eq!(queue.len(), MAX_EVENTS);

        // Push one more - should drop oldest
        queue.push(CcrEvent::Stats { tokens: 999, cost_cents: 0 });
        assert_eq!(queue.len(), MAX_EVENTS);

        // First event should now be tokens=1 (0 was dropped)
        if let Some(CcrEvent::Stats { tokens, .. }) = queue.get(0) {
            assert_eq!(*tokens, 1);
        }
    }

    #[test]
    fn test_json_parsing() {
        let json = r#"{"type":"tool_call","id":"t1","tool":"Bash","args":"cargo build"}"#;
        let event = CcrEvent::from_json(json.as_bytes()).unwrap();

        if let CcrEvent::ToolCall { id, tool, args } = event {
            assert_eq!(id, "t1");
            assert_eq!(tool, "Bash");
            assert_eq!(args, "cargo build");
        } else {
            panic!("Wrong event type");
        }
    }
}
