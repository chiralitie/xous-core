//! CCR - Claude Code Remote
//!
//! Real-time Claude Code session monitor for Precursor hardware.
//! Displays events from ccr_bridge.py via MQTT.
//!
//! MQTT Topics:
//! - ccr/events: All events for display
//! - ccr/permissions/request: Permission requests (subscribe)
//! - ccr/permissions/response: Permission responses (publish)

#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]

extern crate alloc;

mod events;
mod mqtt;
mod ui;

use alloc::string::String;
use alloc::format;
use core::fmt::Write;
use num_traits::*;

use events::{CcrEvent, EventQueue};
use ui::{UiState, ViewMode};

// Xous imports
use blitstr2::GlyphStyle;
use ux_api::minigfx::*;
use ux_api::service::api::Gid;

/// Server name for xous-names registration
pub const SERVER_NAME_CCR: &str = "_Claude Code Remote_";

/// MQTT broker address (Docker host from container)
pub const MQTT_BROKER: &str = "172.17.0.1:1883";

/// MQTT topics
pub const TOPIC_EVENTS: &str = "ccr/events";
pub const TOPIC_PERM_REQUEST: &str = "ccr/permissions/request";
pub const TOPIC_PERM_RESPONSE: &str = "ccr/permissions/response";

/// Message opcodes
#[derive(Debug, num_derive::FromPrimitive, num_derive::ToPrimitive)]
pub enum CcrOp {
    /// Redraw the UI
    Redraw = 0,
    /// Handle keyboard input
    KeyPress,
    /// Raw key event
    RawKey,
    /// MQTT message received
    MqttMessage,
    /// Timer tick (for MQTT polling)
    Tick,
    /// Quit the application
    Quit,
}

/// Application state
struct CcrApp {
    /// Event queue
    events: EventQueue,
    /// UI state
    ui: UiState,
    /// Server ID
    sid: xous::SID,
    /// GAM connection
    gam: gam::Gam,
    /// GAM token
    _gam_token: [u32; 4],
    /// Content canvas
    content: Gid,
    /// Screen size
    screensize: Point,
}

impl CcrApp {
    /// Create new CCR application
    fn new(xns: &xous_names::XousNames, sid: xous::SID) -> Self {
        log::info!("CCR: Connecting to GAM...");
        let gam = gam::Gam::new(xns).expect("Can't connect to GAM");

        log::info!("CCR: Registering UX context as '{}'...", gam::APP_NAME_CCR);
        let gam_token = gam
            .register_ux(gam::UxRegistration {
                app_name: String::from(gam::APP_NAME_CCR),
                ux_type: gam::UxType::Chat,
                predictor: None,
                listener: sid.to_array(),
                redraw_id: CcrOp::Redraw.to_u32().unwrap(),
                gotinput_id: Some(CcrOp::KeyPress.to_u32().unwrap()),
                audioframe_id: None,
                rawkeys_id: Some(CcrOp::RawKey.to_u32().unwrap()),
                focuschange_id: None,
            })
            .expect("Could not register GAM UX")
            .unwrap();
        log::info!("CCR: UX registered successfully, token: {:x?}", gam_token);

        let content = gam.request_content_canvas(gam_token).expect("Could not get content canvas");
        let screensize = gam.get_canvas_bounds(content).expect("Could not get canvas dimensions");
        log::info!("CCR: Canvas acquired, size: {}x{}", screensize.x, screensize.y);

        Self {
            events: EventQueue::new(),
            ui: UiState::new(),
            sid,
            gam,
            _gam_token: gam_token,
            content,
            screensize,
        }
    }

    /// Handle incoming MQTT message
    fn handle_mqtt_message(&mut self, topic: &str, payload: &str) {
        log::debug!("CCR: MQTT {} -> {}", topic, &payload[..payload.len().min(50)]);

        let event = if topic == TOPIC_EVENTS {
            CcrEvent::from_json(payload)
        } else if topic == TOPIC_PERM_REQUEST {
            CcrEvent::from_permission_request(payload)
        } else {
            None
        };

        if let Some(event) = event {
            self.handle_event(event);
        }
    }

    /// Handle incoming event
    fn handle_event(&mut self, event: CcrEvent) {
        // Extract session ID from event
        match &event {
            CcrEvent::SessionStart { session_id, .. } |
            CcrEvent::SessionEnd { session_id, .. } |
            CcrEvent::Stop { session_id } |
            CcrEvent::UserInput { session_id, .. } |
            CcrEvent::ToolCall { session_id, .. } |
            CcrEvent::ToolResult { session_id, .. } |
            CcrEvent::PermissionPending { session_id, .. } |
            CcrEvent::PermissionResolved { session_id, .. } |
            CcrEvent::PermissionTimeout { session_id, .. } |
            CcrEvent::Notification { session_id, .. } => {
                if !session_id.is_empty() {
                    self.ui.session_id = session_id.clone();
                }
            }
            CcrEvent::Status { connected, .. } => {
                self.ui.connected = *connected;
            }
        }

        // Handle permission events specially
        if let CcrEvent::PermissionPending { request_id, .. } = &event {
            self.ui.set_pending_permission(request_id);
            // Auto-switch to permission view
            self.ui.view = ViewMode::Permission;
        }

        // Clear pending permission if resolved/timeout
        if let CcrEvent::PermissionResolved { request_id, .. } |
               CcrEvent::PermissionTimeout { request_id, .. } = &event {
            if self.ui.pending_permission.as_deref() == Some(request_id) {
                self.ui.clear_pending_permission();
                if self.ui.view == ViewMode::Permission {
                    self.ui.view = ViewMode::List;
                }
            }
        }

        // Add to queue
        self.events.push(event);

        // Auto-scroll to show new event
        self.ui.auto_scroll(self.events.len());
    }

    /// Handle key press
    fn handle_key(&mut self, key: char) {
        log::debug!("CCR: Key {:?} (0x{:04x})", key, key as u32);

        match self.ui.view {
            ViewMode::Permission => self.handle_key_permission(key),
            ViewMode::Detail => self.handle_key_detail(key),
            ViewMode::List => self.handle_key_list(key),
        }
    }

    /// Handle key in list view
    fn handle_key_list(&mut self, key: char) {
        match key {
            // Up arrow
            '↑' | '\u{0011}' | 'k' => {
                self.ui.scroll_up();
            }
            // Down arrow
            '↓' | '\u{0012}' | 'j' => {
                self.ui.scroll_down(self.events.len());
            }
            // Right arrow - view detail
            '→' | '\u{0014}' | 'l' => {
                if self.events.len() > 0 {
                    self.ui.view = ViewMode::Detail;
                }
            }
            // Left arrow - go to permission if pending
            '←' | '\u{0013}' | 'h' => {
                if self.ui.has_pending_permission() {
                    self.ui.view = ViewMode::Permission;
                }
            }
            // Enter - view detail
            '\r' | '\n' => {
                if self.events.len() > 0 {
                    self.ui.view = ViewMode::Detail;
                }
            }
            // Home/center button - toggle permission view
            '∴' => {
                if self.ui.has_pending_permission() {
                    self.ui.view = ViewMode::Permission;
                }
            }
            _ => {}
        }
    }

    /// Handle key in detail view
    fn handle_key_detail(&mut self, key: char) {
        match key {
            // Up arrow - previous event
            '↑' | '\u{0011}' | 'k' => {
                self.ui.scroll_up();
            }
            // Down arrow - next event
            '↓' | '\u{0012}' | 'j' => {
                self.ui.scroll_down(self.events.len());
            }
            // Left arrow - back to list
            '←' | '\u{0013}' | 'h' | '\x1b' => {
                self.ui.view = ViewMode::List;
            }
            // Enter - back to list
            '\r' | '\n' => {
                self.ui.view = ViewMode::List;
            }
            _ => {}
        }
    }

    /// Handle key in permission view
    fn handle_key_permission(&mut self, key: char) {
        match key {
            // Left/Right - toggle choice
            '←' | '\u{0013}' | 'h' => {
                self.ui.permission_choice = true; // Allow
            }
            '→' | '\u{0014}' | 'l' => {
                self.ui.permission_choice = false; // Deny
            }
            // Enter - confirm choice
            '\r' | '\n' | '∴' => {
                self.send_permission_response();
            }
            // Escape - cancel (back to list)
            '\x1b' => {
                self.ui.view = ViewMode::List;
            }
            // Up - back to list
            '↑' | '\u{0011}' | 'k' => {
                self.ui.view = ViewMode::List;
            }
            _ => {}
        }
    }

    /// Send permission response via MQTT
    fn send_permission_response(&mut self) {
        if let Some(request_id) = &self.ui.pending_permission {
            let decision = if self.ui.permission_choice { "allow" } else { "deny" };

            let payload = format!(
                r#"{{"request_id":"{}","decision":"{}"}}"#,
                request_id,
                decision
            );

            log::info!("CCR: Sending permission response: {}", payload);

            // TODO: Actually send via MQTT
            // mqtt_publish(TOPIC_PERM_RESPONSE, payload.as_bytes());

            // Add resolved event to queue
            self.events.push(CcrEvent::PermissionResolved {
                request_id: request_id.clone(),
                decision: String::from(decision),
                session_id: self.ui.session_id.clone(),
            });

            // Clear pending and return to list
            self.ui.clear_pending_permission();
            self.ui.view = ViewMode::List;
            self.ui.auto_scroll(self.events.len());
        }
    }

    /// Clear screen area
    fn clear_area(&self) {
        self.gam
            .draw_rectangle(
                self.content,
                Rectangle::new_with_style(
                    Point::new(0, 0),
                    self.screensize,
                    DrawStyle {
                        fill_color: Some(PixelColor::Light),
                        stroke_color: None,
                        stroke_width: 0,
                    },
                ),
            )
            .expect("can't clear content area");
    }

    /// Redraw the UI
    fn redraw(&mut self) {
        self.clear_area();

        // Render content based on view
        let content = match self.ui.view {
            ViewMode::List => {
                let header = ui::render_header(&self.ui);
                let list = ui::render_event_list(&self.events, &self.ui);
                let footer = ui::render_footer(&self.ui);

                let mut output = String::new();
                writeln!(output, "{}", header).ok();
                writeln!(output, "──────────────────────────────────────────").ok();
                write!(output, "{}", list).ok();
                writeln!(output, "──────────────────────────────────────────").ok();
                write!(output, "{}", footer).ok();
                output
            }
            ViewMode::Detail => {
                let header = ui::render_header(&self.ui);
                let detail = if let Some(event) = self.events.get(self.ui.selected) {
                    ui::render_event_detail(event)
                } else {
                    String::from("  No event selected")
                };
                let footer = ui::render_footer(&self.ui);

                let mut output = String::new();
                writeln!(output, "{}", header).ok();
                writeln!(output, "──────────────────────────────────────────").ok();
                write!(output, "{}", detail).ok();
                writeln!(output).ok();
                writeln!(output, "──────────────────────────────────────────").ok();
                write!(output, "{}", footer).ok();
                output
            }
            ViewMode::Permission => {
                // Find the pending permission event
                let perm_event = self.ui.pending_permission.as_ref().and_then(|req_id| {
                    self.events.iter().find(|e| e.request_id() == Some(req_id))
                });

                if let Some(event) = perm_event {
                    ui::render_permission_dialog(event, self.ui.permission_choice)
                } else {
                    // No permission found, go back to list
                    self.ui.view = ViewMode::List;
                    String::from("  No pending permission")
                }
            }
        };

        // Create text view
        let mut text_view = TextView::new(
            self.content,
            TextBounds::GrowableFromBr(
                Point::new(self.screensize.x - 10, self.screensize.y - 10),
                (self.screensize.x - 20) as u16,
            ),
        );

        text_view.style = GlyphStyle::Small;
        text_view.border_width = 0;
        text_view.draw_border = false;
        text_view.clear_area = true;

        write!(text_view.text, "{}", content).expect("Could not write to text view");

        self.gam.post_textview(&mut text_view).expect("Could not render text view");
        self.gam.redraw().expect("Could not redraw screen");
    }

    /// Add demo events for testing
    fn add_demo_events(&mut self) {
        self.handle_event(CcrEvent::Status {
            connected: true,
            message: String::from("Demo mode - no MQTT"),
        });

        self.handle_event(CcrEvent::SessionStart {
            session_id: String::from("demo-session-123"),
            source: String::from("startup"),
            model: String::from("claude-opus-4"),
        });

        self.handle_event(CcrEvent::UserInput {
            text: String::from("Fix the bug in main.rs that causes the crash"),
            session_id: String::from("demo-session-123"),
        });

        self.handle_event(CcrEvent::ToolCall {
            id: String::from("t1"),
            tool: String::from("Read"),
            args: String::from("/home/user/project/src/main.rs"),
            session_id: String::from("demo-session-123"),
        });

        self.handle_event(CcrEvent::ToolResult {
            id: String::from("t1"),
            output: String::from("fn main() { let x = None; x.unwrap(); }"),
            session_id: String::from("demo-session-123"),
        });

        self.handle_event(CcrEvent::ToolCall {
            id: String::from("t2"),
            tool: String::from("Bash"),
            args: String::from("cargo build"),
            session_id: String::from("demo-session-123"),
        });

        self.handle_event(CcrEvent::PermissionPending {
            request_id: String::from("p1"),
            tool: String::from("Bash"),
            command: String::from("rm -rf target/ && cargo build --release"),
            session_id: String::from("demo-session-123"),
        });
    }
}

/// Main entry point
fn main() -> ! {
    log_server::init_wait().unwrap();
    log::set_max_level(log::LevelFilter::Info);
    log::info!("CCR starting, PID {}", xous::process::id());

    let xns = xous_names::XousNames::new().unwrap();
    let sid = xns.register_name(SERVER_NAME_CCR, None).expect("can't register server");

    let mut app = CcrApp::new(&xns, sid);

    log::info!("CCR: Entering main loop");
    log::info!("CCR: Subscribe to {} and {}", TOPIC_EVENTS, TOPIC_PERM_REQUEST);

    loop {
        let msg = xous::receive_message(sid).unwrap();

        match FromPrimitive::from_usize(msg.body.id()) {
            Some(CcrOp::Redraw) => {
                log::debug!("CCR: Redraw");
                app.redraw();
            }
            Some(CcrOp::KeyPress) => {
                log::debug!("CCR: KeyPress");
                // TODO: Extract key from message
            }
            Some(CcrOp::RawKey) => {
                // Extract raw keys from scalar message
                if let xous::Message::Scalar(scalar) = msg.body {
                    for &key_val in &[scalar.arg1, scalar.arg2, scalar.arg3, scalar.arg4] {
                        if key_val != 0 {
                            if let Some(c) = char::from_u32(key_val as u32) {
                                app.handle_key(c);
                                app.redraw();
                            }
                        }
                    }
                }
            }
            Some(CcrOp::MqttMessage) => {
                log::debug!("CCR: MQTT message");
                // TODO: Parse and handle MQTT message
            }
            Some(CcrOp::Tick) => {
                // TODO: Poll MQTT, send ping if needed
            }
            Some(CcrOp::Quit) => {
                log::info!("CCR: Quitting");
                break;
            }
            _ => {
                log::debug!("CCR: Unknown message");
            }
        }
    }

    xous::terminate_process(0)
}
