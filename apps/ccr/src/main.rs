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
mod ui_improved;

use alloc::string::String;
use alloc::format;
use core::fmt::Write;
use num_traits::*;

use events::{CcrEvent, EventQueue};
use ui_improved::{UiState, ViewMode};

/// Truncate string for display
fn truncate_str(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len {
        s
    } else {
        &s[..max_len]
    }
}

// Xous imports
use blitstr2::GlyphStyle;
use ux_api::minigfx::*;
use ux_api::service::api::Gid;

// Networking imports (hosted mode uses std)
#[cfg(feature = "hosted")]
use std::net::TcpStream;
#[cfg(feature = "hosted")]
use std::io::{Read, Write as IoWrite};
#[cfg(feature = "hosted")]
use std::time::Duration;
#[cfg(feature = "hosted")]
use std::sync::{Arc, Mutex};
#[cfg(feature = "hosted")]
use std::sync::atomic::{AtomicBool, Ordering};

/// Server name for xous-names registration
pub const SERVER_NAME_CCR: &str = "_Claude Code Remote_";

/// MQTT broker address (localhost for hosted mode)
pub const MQTT_BROKER: &str = "127.0.0.1:1883";

/// MQTT topics
pub const TOPIC_EVENTS: &str = "ccr/events";
pub const TOPIC_PERM_REQUEST: &str = "ccr/permissions/request";
pub const TOPIC_PERM_RESPONSE: &str = "ccr/permissions/response";

/// Message opcodes
#[derive(Debug, num_derive::FromPrimitive, num_derive::ToPrimitive)]
pub enum CcrOp {
    /// Redraw the UI
    Redraw = 0,
    /// A line of text has arrived from IME
    Line,
    /// Raw key event (for d-pad navigation)
    RawKey,
    /// MQTT message received (scalar: connection status, or memory: topic+payload)
    MqttMessage,
    /// Timer tick (for MQTT polling)
    Tick,
    /// Quit the application
    Quit,
}

/// MQTT connection state for thread communication
#[cfg(feature = "hosted")]
struct MqttThreadState {
    stream: Option<TcpStream>,
    connected: bool,
    packet_id: u16,
}

#[cfg(feature = "hosted")]
impl MqttThreadState {
    fn new() -> Self {
        Self {
            stream: None,
            connected: false,
            packet_id: 1,
        }
    }

    fn next_packet_id(&mut self) -> u16 {
        let id = self.packet_id;
        self.packet_id = self.packet_id.wrapping_add(1);
        if self.packet_id == 0 {
            self.packet_id = 1;
        }
        id
    }
}

/// Layout constants
const MARGIN_X: isize = 8;
const MARGIN_Y: isize = 4;
const BUBBLE_SPACE: isize = 2;
const BUBBLE_RADIUS: u16 = 4;

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
    /// Bubble width (80% of screen)
    bubble_width: u16,
    /// Bubble margin
    bubble_margin: Point,
    /// Connection to self for MQTT thread messages
    #[cfg(feature = "hosted")]
    self_cid: xous::CID,
    /// MQTT thread running flag
    #[cfg(feature = "hosted")]
    mqtt_running: Arc<AtomicBool>,
    /// MQTT thread state
    #[cfg(feature = "hosted")]
    mqtt_state: Arc<Mutex<MqttThreadState>>,
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
                predictor: Some(String::from(ime_plugin_shell::SERVER_NAME_IME_PLUGIN_SHELL)),
                listener: sid.to_array(),
                redraw_id: CcrOp::Redraw.to_u32().unwrap(),
                gotinput_id: Some(CcrOp::Line.to_u32().unwrap()),
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

        // Calculate bubble dimensions (80% width)
        let bubble_width = ((screensize.x * 4) / 5) as u16;
        let bubble_margin = Point::new(4, 2);

        // Initialize MQTT thread (hosted mode only)
        #[cfg(feature = "hosted")]
        let self_cid = xous::connect(sid).expect("Can't connect to self");

        #[cfg(feature = "hosted")]
        let mqtt_running = Arc::new(AtomicBool::new(true));

        #[cfg(feature = "hosted")]
        let mqtt_state = Arc::new(Mutex::new(MqttThreadState::new()));

        // Start MQTT thread
        #[cfg(feature = "hosted")]
        {
            let running = mqtt_running.clone();
            let state = mqtt_state.clone();
            let cid = self_cid;
            std::thread::spawn(move || {
                mqtt_thread_main(MQTT_BROKER, running, state, cid);
            });
        }

        Self {
            events: EventQueue::new(),
            ui: UiState::new(),
            sid,
            gam,
            _gam_token: gam_token,
            content,
            screensize,
            bubble_width,
            bubble_margin,
            #[cfg(feature = "hosted")]
            self_cid,
            #[cfg(feature = "hosted")]
            mqtt_running,
            #[cfg(feature = "hosted")]
            mqtt_state,
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
            // Permission shown inline in Chat view, no view switch needed
        }

        // Clear pending permission if resolved/timeout
        if let CcrEvent::PermissionResolved { request_id, .. } |
               CcrEvent::PermissionTimeout { request_id, .. } = &event {
            if self.ui.pending_permission.as_deref() == Some(request_id) {
                self.ui.clear_pending_permission();
            }
        }

        // Add to queue
        self.events.push(event);

        // Auto-scroll to show new event
        self.ui.auto_scroll(self.events.len());
    }

    /// Handle raw key event for d-pad navigation
    fn handle_rawkey(&mut self, key: char) {
        // D-pad navigation:
        // Up (↑ U+2191): move selection up (visually up = higher index = newer event)
        // Down (↓ U+2193): move selection down (visually down = lower index = older event)
        // Right (→ U+2192): expand selected bubble (detail view)
        // Left (← U+2190): collapse/clear selection
        match key {
            '↑' | '\u{2191}' => {
                // Move selection up (visually) = to older event = lower index
                if self.ui.selected > 0 {
                    self.ui.selected -= 1;
                }
            }
            '↓' | '\u{2193}' => {
                // Move selection down (visually) = to newer event = higher index
                if self.ui.selected < self.events.len().saturating_sub(1) {
                    self.ui.selected += 1;
                }
            }
            '→' | '\u{2192}' => {
                // Expand: switch to detail view
                if self.ui.has_selection() && !self.events.is_empty() {
                    self.ui.view = ViewMode::Detail;
                }
            }
            '←' | '\u{2190}' => {
                // Collapse: if in detail view, go back to chat
                // If in chat view, clear selection
                if self.ui.view == ViewMode::Detail {
                    self.ui.view = ViewMode::Chat;
                } else {
                    self.ui.clear_selection();
                }
            }
            _ => {}
        }
    }

    /// Handle a line of text input from IME
    fn handle_line(&mut self, line: &str) {
        log::info!("CCR: Processing line: {}", line);

        // Check for special commands
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return;
        }

        // Check for permission commands
        if self.ui.has_pending_permission() {
            match trimmed.to_lowercase().as_str() {
                "allow" | "yes" | "y" | "a" => {
                    self.ui.permission_choice = true;
                    self.send_permission_response();
                    return;
                }
                "deny" | "no" | "n" | "d" => {
                    self.ui.permission_choice = false;
                    self.send_permission_response();
                    return;
                }
                _ => {}
            }
        }

        // Otherwise treat as user input to send
        self.ui.input_text = String::from(trimmed);
        self.send_user_input();
    }

    /// Send user input via MQTT
    fn send_user_input(&mut self) {
        let text = self.ui.input_get().to_string();
        if text.is_empty() {
            return;
        }

        let payload = format!(
            r#"{{"session_id":"{}","text":"{}"}}"#,
            self.ui.session_id,
            text.replace('"', "\\\"")
        );

        log::info!("CCR: Sending user input: {}", text);

        // Publish to MQTT
        #[cfg(feature = "hosted")]
        {
            if let Ok(mut state) = self.mqtt_state.lock() {
                let packet_id = state.next_packet_id();
                if let Some(stream) = &mut state.stream {
                    let publish = mqtt::build_publish_packet(
                        "ccr/user_input",
                        payload.as_bytes(),
                        packet_id
                    );
                    use std::io::Write as IoWrite;
                    let _ = stream.write_all(&publish);
                    let _ = stream.flush();
                }
            }
        }

        // Add to event queue
        self.events.push(CcrEvent::UserInput {
            text,
            session_id: self.ui.session_id.clone(),
        });

        // Clear input
        self.ui.input_clear();
        self.ui.auto_scroll(self.events.len());
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

            // Actually publish to MQTT
            #[cfg(feature = "hosted")]
            {
                if let Ok(mut state) = self.mqtt_state.lock() {
                    let packet_id = state.next_packet_id();
                    if let Some(stream) = &mut state.stream {
                        let publish = mqtt::build_publish_packet(
                            TOPIC_PERM_RESPONSE,
                            payload.as_bytes(),
                            packet_id
                        );
                        use std::io::Write as IoWrite;
                        let _ = stream.write_all(&publish);
                        let _ = stream.flush();
                    }
                }
            }

            // Add resolved event to queue
            self.events.push(CcrEvent::PermissionResolved {
                request_id: request_id.clone(),
                decision: String::from(decision),
                session_id: self.ui.session_id.clone(),
            });

            // Clear pending and return to chat
            self.ui.clear_pending_permission();
            self.ui.view = ViewMode::Chat;
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

        match self.ui.view {
            ViewMode::Chat => self.redraw_chat(),
            ViewMode::Detail => self.redraw_detail(),
            ViewMode::Permission => {
                // Permissions shown inline in Chat view
                self.ui.view = ViewMode::Chat;
                self.redraw_chat();
            }
        }

        self.gam.redraw().expect("Could not redraw screen");
    }

    /// Redraw chat view with bubbles
    fn redraw_chat(&mut self) {
        // Use clear_area on canvas to avoid dirty rendering
        self.clear_area();

        // Start from bottom of content area, grow upward
        let mut bubble_baseline = self.screensize.y - MARGIN_Y;

        // Track if there are more events above (older) that aren't shown
        let mut has_more_above = false;

        // Draw events from newest to oldest (bottom to top)
        // Iterate by index in reverse since EventQueueIter doesn't support .rev()
        let event_count = self.events.len();
        let mut first_shown_idx: Option<usize> = None;

        for i in (0..event_count).rev() {
            let event = match self.events.get(i) {
                Some(e) => e,
                None => continue,
            };

            if bubble_baseline <= 0 {
                // There are more events we couldn't show
                has_more_above = true;
                break;
            }

            first_shown_idx = Some(i);

            // (text, is_user_input, border_width, font_style)
            // Use Regular for most content, Bold only for short titles
            let (text, is_user_input, border_width, font_style) = match event {
                CcrEvent::SessionStart { source, .. } => {
                    (format!("Session {}", source), false, 1, GlyphStyle::Regular)
                }
                CcrEvent::SessionEnd { reason, .. } => {
                    (format!("End: {}", reason), false, 1, GlyphStyle::Regular)
                }
                CcrEvent::Stop { .. } => {
                    (String::from("Stopped"), false, 1, GlyphStyle::Regular)
                }
                CcrEvent::UserInput { text, .. } => {
                    (truncate_str(text, 40).to_string(), true, 1, GlyphStyle::Regular)
                }
                CcrEvent::ToolCall { tool, args, .. } => {
                    // Tool name as title, args on next line in regular font
                    (format!("{}\n{}", tool, truncate_str(args, 30)), false, 1, GlyphStyle::Regular)
                }
                CcrEvent::ToolResult { output, .. } => {
                    (truncate_str(output, 35).to_string(), false, 1, GlyphStyle::Monospace)
                }
                CcrEvent::PermissionPending { tool, command, .. } => {
                    // Permission request - render like other events
                    (format!("PERMISSION: {}\n{}", tool, truncate_str(command, 30)), false, 1, GlyphStyle::Regular)
                }
                CcrEvent::PermissionResolved { decision, .. } => {
                    (format!("Permission {}", decision), false, 1, GlyphStyle::Regular)
                }
                CcrEvent::PermissionTimeout { .. } => {
                    (String::from("Permission timeout"), false, 1, GlyphStyle::Regular)
                }
                CcrEvent::Notification { message, .. } => {
                    (truncate_str(message, 35).to_string(), false, 1, GlyphStyle::Regular)
                }
                CcrEvent::Status { connected, message } => {
                    let status = if *connected { "Connected" } else { "Disconnected" };
                    (format!("{}: {}", status, truncate_str(message, 25)), false, 1, GlyphStyle::Regular)
                }
            };

            // Create bubble - right-align for user input, left-align for others
            let mut bubble_tv = if is_user_input {
                TextView::new(
                    self.content,
                    TextBounds::GrowableFromBr(
                        Point::new(self.screensize.x - MARGIN_X, bubble_baseline),
                        self.bubble_width,
                    ),
                )
            } else {
                TextView::new(
                    self.content,
                    TextBounds::GrowableFromBl(
                        Point::new(MARGIN_X, bubble_baseline),
                        self.bubble_width,
                    ),
                )
            };

            bubble_tv.border_width = border_width;
            bubble_tv.draw_border = true;
            bubble_tv.clear_area = true;
            bubble_tv.rounded_border = Some(BUBBLE_RADIUS);
            bubble_tv.style = font_style;
            bubble_tv.margin = self.bubble_margin;
            bubble_tv.ellipsis = false;
            // Use thicker border for selected bubble (invert requires trust level)
            if self.ui.is_selected(i) {
                bubble_tv.border_width = 2;
            }
            write!(bubble_tv.text, "{}", text).ok();
            self.gam.post_textview(&mut bubble_tv).expect("couldn't render bubble");

            if let Some(bounds) = bubble_tv.bounds_computed {
                bubble_baseline -= (bounds.br.y - bounds.tl.y) + BUBBLE_SPACE + self.bubble_margin.y;
            }
        }

        // Show "more" indicator at top if there are hidden events
        if has_more_above {
            let mut more_tv = TextView::new(
                self.content,
                TextBounds::GrowableFromTl(
                    Point::new(MARGIN_X, MARGIN_Y),
                    (self.screensize.x - MARGIN_X * 2) as u16,
                ),
            );
            more_tv.style = GlyphStyle::Small;
            more_tv.draw_border = false;
            more_tv.clear_area = false;
            write!(more_tv.text, "> more").ok();
            self.gam.post_textview(&mut more_tv).expect("couldn't render more indicator");
        }

        // If no events, show waiting message
        if self.events.is_empty() {
            let mut wait_tv = TextView::new(
                self.content,
                TextBounds::CenteredTop(Rectangle::new(
                    Point::new(0, self.screensize.y / 3),
                    Point::new(self.screensize.x, self.screensize.y / 3 + 40),
                )),
            );
            wait_tv.style = GlyphStyle::Regular;
            wait_tv.draw_border = false;
            let status = if self.ui.connected { "connected" } else { "waiting" };
            write!(wait_tv.text, "CCR: {}", status).ok();
            self.gam.post_textview(&mut wait_tv).expect("couldn't render wait text");
        }
    }

    /// Redraw detail view
    fn redraw_detail(&mut self) {
        // Clear the content area first
        // Use clear_area on canvas
        self.clear_area();

        let event = match self.events.get(self.ui.selected) {
            Some(e) => e,
            None => {
                // No valid selection, go back to chat view
                self.ui.view = ViewMode::Chat;
                return;
            }
        };

        let detail = ui_improved::render_event_detail(event);

        let mut text_view = TextView::new(
            self.content,
            TextBounds::GrowableFromTl(
                Point::new(MARGIN_X, MARGIN_Y),
                (self.screensize.x - MARGIN_X * 2) as u16,
            ),
        );

        text_view.style = GlyphStyle::Regular;
        text_view.border_width = 1;
        text_view.draw_border = true;
        text_view.clear_area = true;
        text_view.rounded_border = Some(BUBBLE_RADIUS);
        text_view.margin = self.bubble_margin;

        write!(text_view.text, "{}", detail).ok();
        self.gam.post_textview(&mut text_view).expect("Could not render detail view");
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

/// MQTT background thread (hosted mode only)
#[cfg(feature = "hosted")]
fn mqtt_thread_main(
    broker: &str,
    running: Arc<AtomicBool>,
    state: Arc<Mutex<MqttThreadState>>,
    main_cid: xous::CID,
) {
    log::info!("CCR MQTT: Thread started, connecting to {}", broker);

    while running.load(Ordering::SeqCst) {
        // Try to connect
        match TcpStream::connect(broker) {
            Ok(mut stream) => {
                log::info!("CCR MQTT: TCP connected to {}", broker);

                // Set timeouts
                stream.set_read_timeout(Some(Duration::from_millis(100))).ok();
                stream.set_write_timeout(Some(Duration::from_millis(5000))).ok();

                // Send CONNECT packet
                let connect_packet = mqtt::build_connect_packet("ccr-precursor");
                if let Err(e) = stream.write_all(&connect_packet) {
                    log::error!("CCR MQTT: Failed to send CONNECT: {:?}", e);
                    std::thread::sleep(Duration::from_secs(5));
                    continue;
                }
                stream.flush().ok();

                // Wait for CONNACK
                let mut connack_buf = [0u8; 4];
                match stream.read_exact(&mut connack_buf) {
                    Ok(_) => {
                        if mqtt::is_connack_success(&connack_buf) {
                            log::info!("CCR MQTT: CONNACK received, connected!");
                        } else {
                            log::error!("CCR MQTT: CONNACK rejected");
                            std::thread::sleep(Duration::from_secs(5));
                            continue;
                        }
                    }
                    Err(e) => {
                        log::error!("CCR MQTT: Failed to read CONNACK: {:?}", e);
                        std::thread::sleep(Duration::from_secs(5));
                        continue;
                    }
                }

                // Subscribe to topics
                {
                    let mut st = state.lock().unwrap();

                    // Subscribe to events topic
                    let packet_id = st.next_packet_id();
                    let sub_packet = mqtt::build_subscribe_packet(packet_id, TOPIC_EVENTS);
                    if let Err(e) = stream.write_all(&sub_packet) {
                        log::error!("CCR MQTT: Failed to send SUBSCRIBE: {:?}", e);
                        continue;
                    }
                    stream.flush().ok();
                    log::info!("CCR MQTT: Subscribed to {}", TOPIC_EVENTS);

                    // Subscribe to permission requests topic
                    let packet_id = st.next_packet_id();
                    let sub_packet = mqtt::build_subscribe_packet(packet_id, TOPIC_PERM_REQUEST);
                    if let Err(e) = stream.write_all(&sub_packet) {
                        log::error!("CCR MQTT: Failed to send SUBSCRIBE: {:?}", e);
                        continue;
                    }
                    stream.flush().ok();
                    log::info!("CCR MQTT: Subscribed to {}", TOPIC_PERM_REQUEST);
                }

                // Update state and notify main thread
                {
                    let mut st = state.lock().unwrap();
                    st.stream = Some(stream.try_clone().expect("Failed to clone stream"));
                    st.connected = true;
                }
                notify_main_connected(main_cid, true);

                // Poll loop
                let mut read_buf = [0u8; 2048];
                let mut last_ping = std::time::Instant::now();
                let ping_interval = Duration::from_secs(30);

                while running.load(Ordering::SeqCst) {
                    // Try to read data
                    match stream.read(&mut read_buf) {
                        Ok(0) => {
                            log::info!("CCR MQTT: Connection closed by broker");
                            break;
                        }
                        Ok(n) => {
                            // Parse and handle packets
                            let data = &read_buf[..n];
                            if let Some((topic, payload)) = mqtt::parse_publish_packet(data) {
                                let payload_str = String::from_utf8_lossy(&payload);
                                send_mqtt_message_to_main(main_cid, &topic, &payload_str);
                            } else if mqtt::is_pingresp(data) {
                                log::debug!("CCR MQTT: PINGRESP received");
                            } else if mqtt::is_suback(data) {
                                log::debug!("CCR MQTT: SUBACK received");
                            }
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            // No data available, continue
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                            // Timeout, continue
                        }
                        Err(e) => {
                            log::error!("CCR MQTT: Read error: {:?}", e);
                            break;
                        }
                    }

                    // Send ping if needed
                    if last_ping.elapsed() > ping_interval {
                        let ping_packet = mqtt::build_pingreq_packet();
                        if let Err(e) = stream.write_all(&ping_packet) {
                            log::error!("CCR MQTT: Failed to send PINGREQ: {:?}", e);
                            break;
                        }
                        stream.flush().ok();
                        last_ping = std::time::Instant::now();
                        log::debug!("CCR MQTT: PINGREQ sent");
                    }

                    std::thread::sleep(Duration::from_millis(50));
                }

                // Disconnected
                {
                    let mut st = state.lock().unwrap();
                    st.stream = None;
                    st.connected = false;
                }
                notify_main_connected(main_cid, false);
                log::info!("CCR MQTT: Disconnected, will retry in 5s");
            }
            Err(e) => {
                log::warn!("CCR MQTT: Connection failed: {:?}", e);
            }
        }

        // Wait before retry
        if running.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_secs(5));
        }
    }

    log::info!("CCR MQTT: Thread exiting");
}

/// Notify main thread of connection status change
#[cfg(feature = "hosted")]
fn notify_main_connected(main_cid: xous::CID, connected: bool) {
    let _ = xous::try_send_message(
        main_cid,
        xous::Message::new_scalar(
            CcrOp::MqttMessage.to_usize().unwrap(),
            if connected { 1 } else { 0 },
            0, 0, 0,
        ),
    );
}

/// Send MQTT message to main thread
#[cfg(feature = "hosted")]
fn send_mqtt_message_to_main(main_cid: xous::CID, topic: &str, payload: &str) {
    // Format: "topic\0payload"
    let mut data = String::from(topic);
    data.push('\0');
    data.push_str(payload);

    if let Ok(buf) = xous_ipc::Buffer::into_buf(data) {
        let _ = buf.send(main_cid, CcrOp::MqttMessage.to_u32().unwrap());
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
            Some(CcrOp::Line) => {
                // A line of text from IME
                let buffer = unsafe { xous_ipc::Buffer::from_memory_message(msg.body.memory_message().unwrap()) };
                let line = buffer.as_flat::<String, _>().unwrap();
                log::info!("CCR: Got input line: {}", line.as_str());
                app.handle_line(line.as_str());
                app.redraw();
            }
            Some(CcrOp::RawKey) => {
                // Raw key event for d-pad navigation
                xous::msg_scalar_unpack!(msg, k1, k2, k3, k4, {
                    let keys = [
                        core::char::from_u32(k1 as u32),
                        core::char::from_u32(k2 as u32),
                        core::char::from_u32(k3 as u32),
                        core::char::from_u32(k4 as u32),
                    ];
                    for key in keys.iter().flatten() {
                        log::debug!("CCR: RawKey '{}'", key);
                        app.handle_rawkey(*key);
                    }
                });
                app.redraw();
            }
            Some(CcrOp::MqttMessage) => {
                match &msg.body {
                    xous::Message::Scalar(scalar) => {
                        // Connection status change (arg1: 1=connected, 0=disconnected)
                        let connected = scalar.arg1 != 0;
                        log::info!("CCR: MQTT connection status: {}", if connected { "connected" } else { "disconnected" });
                        app.handle_event(CcrEvent::Status {
                            connected,
                            message: if connected {
                                String::from("Connected to MQTT broker")
                            } else {
                                String::from("Disconnected from MQTT broker")
                            },
                        });
                        app.redraw();
                    }
                    xous::Message::Move(mem) => {
                        // MQTT message with topic and payload
                        let buf = unsafe { xous_ipc::Buffer::from_memory_message(mem) };
                        if let Ok(data) = buf.to_original::<String, _>() {
                            if let Some((topic, payload)) = data.split_once('\0') {
                                log::info!("CCR: MQTT message on {}: {} bytes", topic, payload.len());
                                app.handle_mqtt_message(topic, payload);
                                app.redraw();
                            }
                        }
                    }
                    _ => {
                        log::debug!("CCR: Unknown MQTT message type");
                    }
                }
            }
            Some(CcrOp::Tick) => {
                // Tick is handled by MQTT thread in hosted mode
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
