//! CCR Event Types and Queue
//!
//! Event types matching the ccr_bridge.py MQTT protocol.
//! Fixed-size circular buffer for event storage.

extern crate alloc;
use alloc::string::String;

/// Maximum events in queue
pub const MAX_EVENTS: usize = 64;

/// Maximum characters per event field
pub const MAX_TEXT_LEN: usize = 200;

/// Event types from Claude Code (via ccr_bridge.py)
///
/// MQTT Topics:
/// - ccr/events: All display events
/// - ccr/permissions/request: Permission requests
/// - ccr/permissions/response: Permission responses (outbound)
#[derive(Clone, Debug)]
pub enum CcrEvent {
    /// Session started
    SessionStart {
        session_id: String,
        source: String,  // "startup", "resume", "clear", "compact"
        model: String,
    },

    /// Session ended
    SessionEnd {
        session_id: String,
        reason: String,
    },

    /// Claude stopped responding
    Stop {
        session_id: String,
    },

    /// User input/prompt
    UserInput {
        text: String,
        session_id: String,
    },

    /// Tool call (Bash, Read, Write, Edit, Grep, Glob, Task)
    ToolCall {
        id: String,
        tool: String,
        args: String,
        session_id: String,
    },

    /// Tool result/output
    ToolResult {
        id: String,
        output: String,
        session_id: String,
    },

    /// Permission request pending (needs user approval)
    PermissionPending {
        request_id: String,
        tool: String,
        command: String,
        session_id: String,
    },

    /// Permission resolved (allow/deny)
    PermissionResolved {
        request_id: String,
        decision: String,  // "allow" or "deny"
        session_id: String,
    },

    /// Permission timed out
    PermissionTimeout {
        request_id: String,
        session_id: String,
    },

    /// Notification from Claude Code
    Notification {
        notification_type: String,
        message: String,
        session_id: String,
    },

    /// Connection status (internal)
    Status {
        connected: bool,
        message: String,
    },
}

impl CcrEvent {
    /// Parse event from JSON string (ccr/events topic)
    pub fn from_json(text: &str) -> Option<Self> {
        let event_type = Self::get_json_string(text, "type")?;

        match event_type.as_str() {
            "session_start" => Some(CcrEvent::SessionStart {
                session_id: Self::get_json_string(text, "session_id").unwrap_or_default(),
                source: Self::get_json_string(text, "source").unwrap_or_default(),
                model: Self::get_json_string(text, "model").unwrap_or_default(),
            }),

            "session_end" => Some(CcrEvent::SessionEnd {
                session_id: Self::get_json_string(text, "session_id").unwrap_or_default(),
                reason: Self::get_json_string(text, "reason").unwrap_or_default(),
            }),

            "stop" => Some(CcrEvent::Stop {
                session_id: Self::get_json_string(text, "session_id").unwrap_or_default(),
            }),

            "user_input" => Some(CcrEvent::UserInput {
                text: Self::get_json_string(text, "text").unwrap_or_default(),
                session_id: Self::get_json_string(text, "session_id").unwrap_or_default(),
            }),

            "tool_call" => Some(CcrEvent::ToolCall {
                id: Self::get_json_string(text, "id").unwrap_or_default(),
                tool: Self::get_json_string(text, "tool").unwrap_or_default(),
                args: Self::get_json_string(text, "args").unwrap_or_default(),
                session_id: Self::get_json_string(text, "session_id").unwrap_or_default(),
            }),

            "tool_result" => Some(CcrEvent::ToolResult {
                id: Self::get_json_string(text, "id").unwrap_or_default(),
                output: Self::get_json_string(text, "output").unwrap_or_default(),
                session_id: Self::get_json_string(text, "session_id").unwrap_or_default(),
            }),

            "permission_pending" => Some(CcrEvent::PermissionPending {
                request_id: Self::get_json_string(text, "request_id").unwrap_or_default(),
                tool: Self::get_json_string(text, "tool").unwrap_or_default(),
                command: Self::get_json_string(text, "command").unwrap_or_default(),
                session_id: Self::get_json_string(text, "session_id").unwrap_or_default(),
            }),

            "permission_resolved" => Some(CcrEvent::PermissionResolved {
                request_id: Self::get_json_string(text, "request_id").unwrap_or_default(),
                decision: Self::get_json_string(text, "decision").unwrap_or_default(),
                session_id: Self::get_json_string(text, "session_id").unwrap_or_default(),
            }),

            "permission_timeout" => Some(CcrEvent::PermissionTimeout {
                request_id: Self::get_json_string(text, "request_id").unwrap_or_default(),
                session_id: Self::get_json_string(text, "session_id").unwrap_or_default(),
            }),

            "notification" => Some(CcrEvent::Notification {
                notification_type: Self::get_json_string(text, "notification_type").unwrap_or_default(),
                message: Self::get_json_string(text, "message").unwrap_or_default(),
                session_id: Self::get_json_string(text, "session_id").unwrap_or_default(),
            }),

            _ => None,
        }
    }

    /// Parse permission request from ccr/permissions/request topic
    pub fn from_permission_request(text: &str) -> Option<Self> {
        let event_type = Self::get_json_string(text, "type")?;
        if event_type != "permission_request" {
            return None;
        }

        Some(CcrEvent::PermissionPending {
            request_id: Self::get_json_string(text, "request_id").unwrap_or_default(),
            tool: Self::get_json_string(text, "tool").unwrap_or_default(),
            command: Self::get_json_string(text, "command").unwrap_or_default(),
            session_id: Self::get_json_string(text, "session_id").unwrap_or_default(),
        })
    }

    /// Simple JSON string field extractor
    fn get_json_string(text: &str, key: &str) -> Option<String> {
        let pattern = alloc::format!("\"{}\":", key);
        let start_idx = text.find(&pattern)?;
        let after_key = &text[start_idx + pattern.len()..];
        let trimmed = after_key.trim_start();

        if trimmed.starts_with('"') {
            // String value
            let value_start = 1;
            let value_end = trimmed[1..].find('"')? + 1;
            let value = &trimmed[value_start..value_end];
            // Truncate to max length
            let truncated = if value.len() > MAX_TEXT_LEN {
                &value[..MAX_TEXT_LEN]
            } else {
                value
            };
            // Unescape basic sequences
            Some(Self::unescape_json(truncated))
        } else {
            None
        }
    }

    /// Basic JSON string unescaping
    fn unescape_json(s: &str) -> String {
        let mut result = String::with_capacity(s.len());
        let mut chars = s.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '\\' {
                match chars.next() {
                    Some('n') => result.push('\n'),
                    Some('r') => result.push('\r'),
                    Some('t') => result.push('\t'),
                    Some('"') => result.push('"'),
                    Some('\\') => result.push('\\'),
                    Some('/') => result.push('/'),
                    Some(other) => {
                        result.push('\\');
                        result.push(other);
                    }
                    None => result.push('\\'),
                }
            } else {
                result.push(c);
            }
        }
        result
    }

    /// Check if this event is a pending permission request
    pub fn is_permission_pending(&self) -> bool {
        matches!(self, CcrEvent::PermissionPending { .. })
    }

    /// Get request_id if this is a permission event
    pub fn request_id(&self) -> Option<&str> {
        match self {
            CcrEvent::PermissionPending { request_id, .. } => Some(request_id),
            CcrEvent::PermissionResolved { request_id, .. } => Some(request_id),
            CcrEvent::PermissionTimeout { request_id, .. } => Some(request_id),
            _ => None,
        }
    }

    /// Get tool name for display
    pub fn tool_name(&self) -> Option<&str> {
        match self {
            CcrEvent::ToolCall { tool, .. } => Some(tool),
            CcrEvent::PermissionPending { tool, .. } => Some(tool),
            _ => None,
        }
    }

    /// Get icon character for event type
    pub fn icon(&self) -> char {
        match self {
            CcrEvent::SessionStart { .. } => '▶',
            CcrEvent::SessionEnd { .. } => '■',
            CcrEvent::Stop { .. } => '⏸',
            CcrEvent::UserInput { .. } => '?',
            CcrEvent::ToolCall { tool, .. } => match tool.as_str() {
                "Bash" => '$',
                "Read" => 'R',
                "Write" | "Edit" => 'W',
                "Grep" | "Glob" => 'S',
                "Task" => 'T',
                _ => '*',
            },
            CcrEvent::ToolResult { .. } => '→',
            CcrEvent::PermissionPending { .. } => '!',
            CcrEvent::PermissionResolved { decision, .. } => {
                if decision == "allow" { '✓' } else { '✗' }
            }
            CcrEvent::PermissionTimeout { .. } => '⏱',
            CcrEvent::Notification { .. } => '🔔',
            CcrEvent::Status { connected, .. } => {
                if *connected { '●' } else { '○' }
            }
        }
    }

    /// Get short description for list view
    pub fn summary(&self) -> String {
        match self {
            CcrEvent::SessionStart { source, .. } => {
                alloc::format!("Session {}", source)
            }
            CcrEvent::SessionEnd { reason, .. } => {
                alloc::format!("End: {}", reason)
            }
            CcrEvent::Stop { .. } => String::from("Stopped"),
            CcrEvent::UserInput { text, .. } => {
                truncate(text, 35)
            }
            CcrEvent::ToolCall { tool, args, .. } => {
                let args_short = truncate(args, 25);
                alloc::format!("{}: {}", tool, args_short)
            }
            CcrEvent::ToolResult { output, .. } => {
                truncate(output, 35)
            }
            CcrEvent::PermissionPending { tool, command, .. } => {
                let cmd_short = truncate(command, 20);
                alloc::format!("{}: {}", tool, cmd_short)
            }
            CcrEvent::PermissionResolved { decision, .. } => {
                alloc::format!("Permission {}", decision)
            }
            CcrEvent::PermissionTimeout { .. } => String::from("Permission timeout"),
            CcrEvent::Notification { message, .. } => {
                truncate(message, 35)
            }
            CcrEvent::Status { message, .. } => {
                truncate(message, 35)
            }
        }
    }
}

/// Truncate string with ellipsis
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        String::from(s)
    } else if max_len > 3 {
        let mut result = String::from(&s[..max_len - 3]);
        result.push_str("...");
        result
    } else {
        String::from(&s[..max_len])
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

    /// Get mutable event at index
    pub fn get_mut(&mut self, index: usize) -> Option<&mut CcrEvent> {
        if index >= self.count {
            return None;
        }
        let actual_index = (self.head + index) % MAX_EVENTS;
        self.events[actual_index].as_mut()
    }

    /// Find first pending permission request
    pub fn find_pending_permission(&self) -> Option<(usize, &CcrEvent)> {
        for i in 0..self.count {
            if let Some(event) = self.get(i) {
                if event.is_permission_pending() {
                    return Some((i, event));
                }
            }
        }
        None
    }

    /// Find permission by request_id
    pub fn find_by_request_id(&self, request_id: &str) -> Option<(usize, &CcrEvent)> {
        for i in 0..self.count {
            if let Some(event) = self.get(i) {
                if event.request_id() == Some(request_id) {
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

    /// Iterate over events (oldest first)
    pub fn iter(&self) -> EventQueueIter<'_> {
        EventQueueIter {
            queue: self,
            index: 0,
        }
    }
}

impl Default for EventQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// Iterator over event queue
pub struct EventQueueIter<'a> {
    queue: &'a EventQueue,
    index: usize,
}

impl<'a> Iterator for EventQueueIter<'a> {
    type Item = &'a CcrEvent;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.queue.count {
            return None;
        }
        let event = self.queue.get(self.index);
        self.index += 1;
        event
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tool_call() {
        let json = r#"{"type":"tool_call","id":"t1","tool":"Bash","args":"cargo build","session_id":"s1"}"#;
        let event = CcrEvent::from_json(json).unwrap();

        if let CcrEvent::ToolCall { id, tool, args, session_id } = event {
            assert_eq!(id, "t1");
            assert_eq!(tool, "Bash");
            assert_eq!(args, "cargo build");
            assert_eq!(session_id, "s1");
        } else {
            panic!("Wrong event type");
        }
    }

    #[test]
    fn test_parse_permission_pending() {
        let json = r#"{"type":"permission_pending","request_id":"abc123","tool":"Bash","command":"rm -rf /","session_id":"s1"}"#;
        let event = CcrEvent::from_json(json).unwrap();

        if let CcrEvent::PermissionPending { request_id, tool, command, .. } = event {
            assert_eq!(request_id, "abc123");
            assert_eq!(tool, "Bash");
            assert_eq!(command, "rm -rf /");
        } else {
            panic!("Wrong event type");
        }
    }

    #[test]
    fn test_event_queue() {
        let mut queue = EventQueue::new();
        assert!(queue.is_empty());

        queue.push(CcrEvent::Status {
            connected: true,
            message: String::from("test"),
        });
        assert_eq!(queue.len(), 1);

        queue.push(CcrEvent::UserInput {
            text: String::from("hello"),
            session_id: String::from("s1"),
        });
        assert_eq!(queue.len(), 2);
    }

    #[test]
    fn test_event_queue_overflow() {
        let mut queue = EventQueue::new();

        for i in 0..MAX_EVENTS + 5 {
            queue.push(CcrEvent::Status {
                connected: true,
                message: alloc::format!("msg{}", i),
            });
        }

        assert_eq!(queue.len(), MAX_EVENTS);
    }
}
