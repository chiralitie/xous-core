//! CCR - Claude Code Remote
//!
//! Main entry point for the CCR application.
//! This is a Xous app that displays Claude Code events.

#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]

extern crate alloc;

mod events;
mod mqtt;
mod ui;

use alloc::string::String;
use core::fmt::Write;
use num_traits::*;

use events::{CcrEvent, EventQueue};
use ui::UiState;

// Xous imports (conditional compilation for hosted vs hardware)
#[cfg(target_os = "xous")]
use blitstr2::GlyphStyle;
#[cfg(target_os = "xous")]
use ux_api::minigfx::*;
#[cfg(target_os = "xous")]
use ux_api::service::api::Gid;

/// Server name for xous-names registration
pub const SERVER_NAME_CCR: &str = "_Claude Code Remote_";

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

    // Xous-specific fields
    #[cfg(target_os = "xous")]
    gam: gam::Gam,
    #[cfg(target_os = "xous")]
    _gam_token: [u32; 4],
    #[cfg(target_os = "xous")]
    content: Gid,
    #[cfg(target_os = "xous")]
    screensize: Point,
}

impl CcrApp {
    /// Create new CCR application
    #[cfg(target_os = "xous")]
    fn new(xns: &xous_names::XousNames, sid: xous::SID) -> Self {
        let gam = gam::Gam::new(xns).expect("Can't connect to GAM");

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

        let content = gam.request_content_canvas(gam_token).expect("Could not get content canvas");
        let screensize = gam.get_canvas_bounds(content).expect("Could not get canvas dimensions");

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

    /// Create new CCR application (non-Xous, for testing)
    #[cfg(not(target_os = "xous"))]
    fn new(xns: &xous_names::XousNames, sid: xous::SID) -> Self {
        Self {
            events: EventQueue::new(),
            ui: UiState::new(),
            sid,
        }
    }

    /// Handle incoming event
    fn handle_event(&mut self, event: CcrEvent) {
        // Update stats if this is a stats event
        if let CcrEvent::Stats { tokens, cost_cents } = &event {
            self.ui.tokens = *tokens;
            self.ui.cost_cents = *cost_cents;
        }

        // Add to queue
        self.events.push(event);

        // Auto-scroll to show new event
        self.ui.auto_scroll(self.events.len());

        // Check for permission requests
        self.ui.select_next_permission(&self.events);
    }

    /// Handle key press
    fn handle_key(&mut self, key: char) {
        match key {
            // Up arrow (Unicode arrow or F1 key code)
            '↑' | '\u{0011}' | 'k' => {
                self.ui.scroll_up();
            }
            // Down arrow (Unicode arrow or F2 key code)
            '↓' | '\u{0012}' | 'j' => {
                self.ui.scroll_down(self.events.len());
            }
            // Left arrow - deny (Unicode arrow or F3 key code)
            '←' | '\u{0013}' | 'h' => {
                if let Some(idx) = self.ui.selected {
                    self.handle_permission_response(idx, false);
                }
            }
            // Right arrow - approve (Unicode arrow or F4 key code)
            '→' | '\u{0014}' | 'l' => {
                if let Some(idx) = self.ui.selected {
                    self.handle_permission_response(idx, true);
                }
            }
            // Enter - toggle expand (future)
            '\r' | '\n' => {
                // TODO: expand/collapse event details
            }
            _ => {}
        }
    }

    /// Handle permission approval/denial
    fn handle_permission_response(&mut self, _idx: usize, approved: bool) {
        if let Some(idx) = self.ui.selected {
            if let Some(event) = self.events.get(idx) {
                if let CcrEvent::PermissionRequest { id, .. } = event {
                    let response = alloc::format!(
                        r#"{{"type":"response","id":"{}","approved":{}}}"#,
                        id,
                        approved
                    );
                    log::info!("CCR: Permission response: {}", response);
                    // TODO: Send via MQTT
                }
            }
        }
        // Clear selection and find next permission
        self.ui.selected = None;
        self.ui.select_next_permission(&self.events);
    }

    /// Clear screen area
    #[cfg(target_os = "xous")]
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
    #[cfg(target_os = "xous")]
    fn redraw(&mut self) {
        self.clear_area();

        // Render content
        let status = ui::render_status_bar(&self.ui);
        let event_list = ui::render_event_list(&self.events, &self.ui);
        let footer = ui::render_footer(&self.ui);

        // Combine into single text
        let mut content = String::new();
        writeln!(content, "{}", status).ok();
        writeln!(content, "────────────────────────────────────").ok();
        write!(content, "{}", event_list).ok();
        writeln!(content, "────────────────────────────────────").ok();
        write!(content, "{}", footer).ok();

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

    /// Redraw (non-Xous, for testing)
    #[cfg(not(target_os = "xous"))]
    fn redraw(&mut self) {
        let status = ui::render_status_bar(&self.ui);
        let event_list = ui::render_event_list(&self.events, &self.ui);
        let footer = ui::render_footer(&self.ui);

        println!("\x1B[2J\x1B[H"); // Clear screen
        println!("{}", status);
        println!("────────────────────────────────────────");
        print!("{}", event_list);
        println!("────────────────────────────────────────");
        println!("{}", footer);
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

    // Add some demo events for testing
    app.handle_event(CcrEvent::Status {
        connected: true,
        message: String::from("Ready"),
    });
    app.handle_event(CcrEvent::UserInput {
        text: String::from("Fix the bug in main.rs"),
    });
    app.handle_event(CcrEvent::ToolCall {
        id: String::from("t1"),
        tool: String::from("Read"),
        args: String::from("src/main.rs"),
    });
    app.handle_event(CcrEvent::ToolResult {
        id: String::from("t1"),
        output: String::from("fn main() { ... }"),
        truncated: true,
    });
    app.handle_event(CcrEvent::PermissionRequest {
        id: String::from("p1"),
        tool: String::from("Bash"),
        command: String::from("cargo build --release"),
        timeout_secs: 30,
    });
    app.handle_event(CcrEvent::Stats {
        tokens: 1234,
        cost_cents: 2,
    });

    log::info!("CCR: Entering main loop");

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
                log::debug!("CCR: RawKey");
                // Extract raw keys from scalar message
                if let xous::Message::Scalar(scalar) = msg.body {
                    // Keys are sent as 4 chars in arg1-arg4
                    for &key_val in &[scalar.arg1, scalar.arg2, scalar.arg3, scalar.arg4] {
                        if key_val != 0 {
                            if let Some(c) = char::from_u32(key_val as u32) {
                                log::debug!("CCR: Key pressed: {:?} (0x{:04x})", c, key_val);
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
