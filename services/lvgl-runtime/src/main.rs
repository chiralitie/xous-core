#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]

mod api;

use api::*;
use num_traits::FromPrimitive;
use num_traits::ToPrimitive;
use log::info;

// Graphics server opcodes (from graphics-server api)
#[derive(Debug, num_derive::FromPrimitive, num_derive::ToPrimitive)]
#[allow(dead_code)]
enum GfxOpcode {
    Line = 1,
    Rectangle = 2,
    Circle = 3,
    RoundedRectangle = 4,
    Flush = 7,
    Clear = 14,
}

// Graphics server name
const SERVER_NAME_GFX: &str = "_Gfx_";

// Display constants for Precursor
const DISP_WIDTH: u32 = 336;
const DISP_HEIGHT: u32 = 536;

// Maximum number of LVGL objects that can be tracked
const MAX_OBJECTS: usize = 64;

// LVGL FFI bindings
mod lvgl_ffi {
    #![allow(non_camel_case_types)]
    #![allow(dead_code)]

    use core::ffi::{c_char, c_void};

    // Opaque types
    #[repr(C)]
    pub struct lv_disp_t { _private: [u8; 0] }

    #[repr(C)]
    pub struct lv_disp_draw_buf_t { _private: [u8; 0] }

    #[repr(C)]
    pub struct lv_disp_drv_t { _private: [u8; 0] }

    #[repr(C)]
    pub struct lv_obj_t { _private: [u8; 0] }

    #[repr(C)]
    pub struct lv_indev_t { _private: [u8; 0] }

    #[repr(C)]
    pub struct lv_indev_drv_t { _private: [u8; 0] }

    // Display flush callback type
    pub type lv_disp_flush_cb_t = Option<unsafe extern "C" fn(
        disp_drv: *mut lv_disp_drv_t,
        area: *const lv_area_t,
        color_p: *mut lv_color_t
    )>;

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct lv_area_t {
        pub x1: i16,
        pub y1: i16,
        pub x2: i16,
        pub y2: i16,
    }

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct lv_color_t {
        pub full: u8,
    }

    extern "C" {
        pub fn lv_init();
        pub fn lv_tick_inc(tick_period: u32);
        pub fn lv_timer_handler() -> u32;

        // Display
        pub fn lv_display_create(hor_res: i32, ver_res: i32) -> *mut lv_disp_t;
        pub fn lv_display_set_buffers(
            disp: *mut lv_disp_t,
            buf1: *mut c_void,
            buf2: *mut c_void,
            buf_size: u32,
            render_mode: i32,
        );
        pub fn lv_display_set_flush_cb(
            disp: *mut lv_disp_t,
            flush_cb: lv_disp_flush_cb_t,
        );
        pub fn lv_display_flush_ready(disp: *mut lv_disp_t);

        // Objects
        pub fn lv_screen_active() -> *mut lv_obj_t;
        pub fn lv_obj_create(parent: *mut lv_obj_t) -> *mut lv_obj_t;
        pub fn lv_obj_delete(obj: *mut lv_obj_t);
        pub fn lv_obj_set_pos(obj: *mut lv_obj_t, x: i32, y: i32);

        // Label widget
        pub fn lv_label_create(parent: *mut lv_obj_t) -> *mut lv_obj_t;
        pub fn lv_label_set_text(obj: *mut lv_obj_t, text: *const c_char);
        pub fn lv_obj_align(obj: *mut lv_obj_t, align: i32, x_ofs: i32, y_ofs: i32);
        pub fn lv_obj_set_style_text_color(obj: *mut lv_obj_t, color: lv_color_t, selector: u32);
        pub fn lv_obj_set_style_bg_color(obj: *mut lv_obj_t, color: lv_color_t, selector: u32);
        pub fn lv_obj_set_style_bg_opa(obj: *mut lv_obj_t, opa: u8, selector: u32);

        // Button widget
        pub fn lv_button_create(parent: *mut lv_obj_t) -> *mut lv_obj_t;
        pub fn lv_obj_set_size(obj: *mut lv_obj_t, w: i32, h: i32);
    }

    // Alignment constants
    pub const LV_ALIGN_CENTER: i32 = 0;
    pub const LV_ALIGN_TOP_LEFT: i32 = 1;
    pub const LV_ALIGN_TOP_MID: i32 = 2;
    pub const LV_ALIGN_TOP_RIGHT: i32 = 3;

    // Render modes
    pub const LV_DISPLAY_RENDER_MODE_PARTIAL: i32 = 0;
    pub const LV_DISPLAY_RENDER_MODE_DIRECT: i32 = 1;
    pub const LV_DISPLAY_RENDER_MODE_FULL: i32 = 2;
}

use lvgl_ffi::*;

// Object handle management
struct ObjectRegistry {
    objects: [Option<*mut lv_obj_t>; MAX_OBJECTS],
    next_handle: u32,
}

impl ObjectRegistry {
    pub fn new() -> Self {
        ObjectRegistry {
            objects: [None; MAX_OBJECTS],
            next_handle: 2, // Handle 1 is reserved for screen
        }
    }

    pub fn register(&mut self, obj: *mut lv_obj_t) -> Option<u32> {
        for i in 0..MAX_OBJECTS {
            if self.objects[i].is_none() {
                self.objects[i] = Some(obj);
                let handle = (i + 2) as u32; // Handles start at 2
                if handle >= self.next_handle {
                    self.next_handle = handle + 1;
                }
                return Some(handle);
            }
        }
        None // Registry full
    }

    pub fn get(&self, handle: u32) -> Option<*mut lv_obj_t> {
        if handle == 1 {
            // Special case: handle 1 is always the screen
            unsafe { Some(lv_screen_active()) }
        } else if handle >= 2 && (handle - 2) < MAX_OBJECTS as u32 {
            self.objects[(handle - 2) as usize]
        } else {
            None
        }
    }

    pub fn remove(&mut self, handle: u32) -> Option<*mut lv_obj_t> {
        if handle >= 2 && (handle - 2) < MAX_OBJECTS as u32 {
            let idx = (handle - 2) as usize;
            let obj = self.objects[idx];
            self.objects[idx] = None;
            obj
        } else {
            None
        }
    }
}

// Display buffer - for 1-bit color, we need 1 bit per pixel
// For a full line: 336 pixels = 42 bytes = 11 u32s
// Let's use a 10-line buffer
const DISP_BUF_SIZE: usize = (DISP_WIDTH as usize / 8) * 10;
// Note: static mut is used here for LVGL C library compatibility
// This is safe because lvgl-runtime is single-threaded
#[allow(static_mut_refs)]
static mut DISP_BUF: [u8; DISP_BUF_SIZE] = [0; DISP_BUF_SIZE];

// Global display reference for callback
static mut DISPLAY: *mut lv_disp_t = core::ptr::null_mut();

// Global graphics server connection for flush callback
static mut GFX_CID: Option<xous::CID> = None;

// Helper to encode Point into usize for IPC
fn point_to_usize(x: i16, y: i16) -> usize {
    ((x as u16 as usize) << 16) | (y as u16 as usize)
}

// Helper to encode DrawStyle into usize for IPC
// Format: stroke_width(8) | stroke_color(1) | fill_color(1) | flags(2)
fn style_to_usize(fill_dark: bool, stroke_dark: bool) -> usize {
    let fill_bit = if fill_dark { 1usize } else { 0usize };
    let stroke_bit = if stroke_dark { 1usize } else { 0usize };
    // flags: bit 1 = has_fill, bit 3 = has_stroke
    let flags = 0b1010usize; // both fill and stroke present
    let stroke_width = 1usize;
    (stroke_width << 24) | (stroke_bit << 16) | (fill_bit << 8) | flags
}

// Flush callback - sends rendered pixels to graphics-server
unsafe extern "C" fn disp_flush_cb(
    _disp: *mut lv_disp_drv_t,
    area: *const lv_area_t,
    color_p: *mut lv_color_t
) {
    let area = &*area;
    let x1 = area.x1 as isize;
    let y1 = area.y1 as isize;
    let x2 = area.x2 as isize;
    let y2 = area.y2 as isize;

    let width = (x2 - x1 + 1) as usize;
    let height = (y2 - y1 + 1) as usize;
    let stride = (width + 7) / 8;  // bytes per row for 1-bit color

    log::trace!("LVGL flush: ({},{}) to ({},{}) - {}x{}", x1, y1, x2, y2, width, height);

    if let Some(cid) = GFX_CID {
        let px_bytes = color_p as *const u8;

        // Process each row with run-length encoding for efficiency
        for dy in 0..height {
            let mut dx = 0usize;
            while dx < width {
                // Get current pixel
                let byte_idx = dy * stride + dx / 8;
                let bit_idx = 7 - (dx % 8);  // MSB = leftmost
                let pixel = (*px_bytes.add(byte_idx) >> bit_idx) & 1;
                let is_dark = pixel != 0;

                // Find run length (consecutive same-color pixels)
                let mut run_len = 1;
                while dx + run_len < width {
                    let next_byte_idx = dy * stride + (dx + run_len) / 8;
                    let next_bit_idx = 7 - ((dx + run_len) % 8);
                    let next_pixel = (*px_bytes.add(next_byte_idx) >> next_bit_idx) & 1;
                    if (next_pixel != 0) != is_dark {
                        break;
                    }
                    run_len += 1;
                }

                // Draw filled rectangle for this run
                let rx1 = (x1 + dx as isize) as i16;
                let ry1 = (y1 + dy as isize) as i16;
                let rx2 = (x1 + (dx + run_len) as isize - 1) as i16;
                let ry2 = ry1;

                let _ = xous::send_message(
                    cid,
                    xous::Message::new_scalar(
                        GfxOpcode::Rectangle.to_usize().unwrap(),
                        point_to_usize(rx1, ry1),
                        point_to_usize(rx2, ry2),
                        style_to_usize(is_dark, is_dark),
                        0,
                    )
                );

                dx += run_len;
            }
        }

        // Flush to display
        let _ = xous::send_message(
            cid,
            xous::Message::new_scalar(
                GfxOpcode::Flush.to_usize().unwrap(),
                0, 0, 0, 0
            )
        );
    }

    // Mark flush as complete
    if !DISPLAY.is_null() {
        lv_display_flush_ready(DISPLAY);
    }
}

struct LvglRuntime {
    initialized: bool,
    registry: ObjectRegistry,
}

impl LvglRuntime {
    pub fn new() -> Self {
        info!("Initializing LVGL runtime");

        unsafe {
            // Initialize LVGL
            lv_init();

            // Create display
            DISPLAY = lv_display_create(DISP_WIDTH as i32, DISP_HEIGHT as i32);

            if !DISPLAY.is_null() {
                // Set up display buffer
                #[allow(static_mut_refs)]
                lv_display_set_buffers(
                    DISPLAY,
                    DISP_BUF.as_mut_ptr() as *mut core::ffi::c_void,
                    core::ptr::null_mut(),
                    DISP_BUF_SIZE as u32,
                    LV_DISPLAY_RENDER_MODE_PARTIAL,
                );

                // Set flush callback
                lv_display_set_flush_cb(DISPLAY, Some(disp_flush_cb));

                info!("LVGL display initialized: {}x{}", DISP_WIDTH, DISP_HEIGHT);
            } else {
                log::error!("Failed to create LVGL display");
            }
        }

        LvglRuntime {
            initialized: true,
            registry: ObjectRegistry::new(),
        }
    }

    pub fn create_demo_ui(&self) {
        if !self.initialized {
            return;
        }

        unsafe {
            let screen = lv_screen_active();
            if screen.is_null() {
                log::error!("No active screen");
                return;
            }

            // Create a label
            let label = lv_label_create(screen);
            if !label.is_null() {
                let text = b"Hello Xous!\0";
                lv_label_set_text(label, text.as_ptr() as *const core::ffi::c_char);
                lv_obj_align(label, LV_ALIGN_CENTER, 0, 0);
                info!("Created demo label");
            }
        }
    }

    pub fn tick(&self, ms: u32) {
        unsafe {
            lv_tick_inc(ms);
        }
    }

    pub fn handler(&self) -> u32 {
        unsafe {
            lv_timer_handler()
        }
    }

    pub fn get_screen(&self) -> u32 {
        // Screen is always handle 1
        1
    }

    pub fn create_label(&mut self, parent_handle: u32) -> u32 {
        if !self.initialized {
            return 0;
        }

        let parent = match self.registry.get(parent_handle) {
            Some(p) => p,
            None => {
                log::warn!("Invalid parent handle: {}", parent_handle);
                return 0;
            }
        };

        unsafe {
            let label = lv_label_create(parent);
            if label.is_null() {
                log::error!("Failed to create label");
                return 0;
            }

            match self.registry.register(label) {
                Some(handle) => {
                    info!("Created label with handle {}", handle);
                    handle
                }
                None => {
                    log::error!("Object registry full");
                    0
                }
            }
        }
    }

    pub fn create_button(&mut self, parent_handle: u32) -> u32 {
        if !self.initialized {
            return 0;
        }

        let parent = match self.registry.get(parent_handle) {
            Some(p) => p,
            None => {
                log::warn!("Invalid parent handle: {}", parent_handle);
                return 0;
            }
        };

        unsafe {
            let button = lv_button_create(parent);
            if button.is_null() {
                log::error!("Failed to create button");
                return 0;
            }

            match self.registry.register(button) {
                Some(handle) => {
                    info!("Created button with handle {}", handle);
                    handle
                }
                None => {
                    log::error!("Object registry full");
                    0
                }
            }
        }
    }

    pub fn set_text(&mut self, handle: u32, text: &[u8]) -> i32 {
        if !self.initialized {
            return -1;
        }

        let obj = match self.registry.get(handle) {
            Some(o) => o,
            None => {
                log::warn!("Invalid handle for set_text: {}", handle);
                return -1;
            }
        };

        unsafe {
            lv_label_set_text(obj, text.as_ptr() as *const core::ffi::c_char);
        }

        info!("Set text on handle {}", handle);
        0
    }

    pub fn delete_object(&mut self, handle: u32) -> i32 {
        if !self.initialized {
            return -1;
        }

        // Don't allow deleting the screen
        if handle == 1 {
            log::warn!("Cannot delete screen (handle 1)");
            return -1;
        }

        match self.registry.remove(handle) {
            Some(obj) => {
                unsafe {
                    lv_obj_delete(obj);
                }
                info!("Deleted object with handle {}", handle);
                0
            }
            None => {
                log::warn!("Invalid handle for delete: {}", handle);
                -1
            }
        }
    }

    pub fn set_pos(&mut self, handle: u32, x: i32, y: i32) -> i32 {
        if !self.initialized {
            return -1;
        }

        let obj = match self.registry.get(handle) {
            Some(o) => o,
            None => {
                log::warn!("Invalid handle for set_pos: {}", handle);
                return -1;
            }
        };

        unsafe {
            lv_obj_set_pos(obj, x, y);
        }

        info!("Set position of handle {} to ({}, {})", handle, x, y);
        0
    }

    pub fn set_style_text_color(&mut self, handle: u32, r: u8, g: u8, b: u8) -> i32 {
        if !self.initialized {
            return -1;
        }

        let obj = match self.registry.get(handle) {
            Some(o) => o,
            None => {
                log::warn!("Invalid handle for set_style_text_color: {}", handle);
                return -1;
            }
        };

        // For 1-bit display, convert RGB to grayscale then to black/white
        // Simple luminance: (r + g + b) / 3 > 127 => white (0xFF), else black (0x00)
        let gray = ((r as u16 + g as u16 + b as u16) / 3) as u8;
        let mono = if gray > 127 { 0xFF } else { 0x00 };

        unsafe {
            let color = lv_color_t { full: mono };
            lv_obj_set_style_text_color(obj, color, 0);
        }

        info!("Set text color of handle {} to RGB({},{},{}) -> mono {}", handle, r, g, b, mono);
        0
    }

    pub fn set_style_bg_color(&mut self, handle: u32, r: u8, g: u8, b: u8) -> i32 {
        if !self.initialized {
            return -1;
        }

        let obj = match self.registry.get(handle) {
            Some(o) => o,
            None => {
                log::warn!("Invalid handle for set_style_bg_color: {}", handle);
                return -1;
            }
        };

        // For 1-bit display, convert RGB to grayscale then to black/white
        let gray = ((r as u16 + g as u16 + b as u16) / 3) as u8;
        let mono = if gray > 127 { 0xFF } else { 0x00 };

        unsafe {
            let color = lv_color_t { full: mono };
            lv_obj_set_style_bg_color(obj, color, 0);
            lv_obj_set_style_bg_opa(obj, 255, 0); // Full opacity
        }

        info!("Set bg color of handle {} to RGB({},{},{}) -> mono {}", handle, r, g, b, mono);
        0
    }

    pub fn align_object(&mut self, handle: u32, align: i32, x_ofs: i32, y_ofs: i32) -> i32 {
        if !self.initialized {
            return -1;
        }

        let obj = match self.registry.get(handle) {
            Some(o) => o,
            None => {
                log::warn!("Invalid handle for align: {}", handle);
                return -1;
            }
        };

        unsafe {
            lv_obj_align(obj, align, x_ofs, y_ofs);
        }

        info!("Aligned handle {} to ({}, {}) with align={}", handle, x_ofs, y_ofs, align);
        0
    }

    pub fn set_size(&mut self, handle: u32, width: i32, height: i32) -> i32 {
        if !self.initialized {
            return -1;
        }

        let obj = match self.registry.get(handle) {
            Some(o) => o,
            None => {
                log::warn!("Invalid handle for set_size: {}", handle);
                return -1;
            }
        };

        unsafe {
            lv_obj_set_size(obj, width, height);
        }

        info!("Set size of handle {} to {}x{}", handle, width, height);
        0
    }
}

fn main() -> ! {
    log_server::init_wait().unwrap();
    log::set_max_level(log::LevelFilter::Info);
    info!("LVGL Runtime starting, PID: {}", xous::process::id());

    let xns = xous_names::XousNames::new().unwrap();
    let lvgl_sid = xns
        .register_name(api::SERVER_NAME_LVGL, None)
        .expect("can't register LVGL server");
    info!("LVGL server registered with NS: {:?}", lvgl_sid);

    // Connect to graphics-server for display output
    unsafe {
        match xns.request_connection_blocking(SERVER_NAME_GFX) {
            Ok(cid) => {
                GFX_CID = Some(cid);
                info!("Connected to graphics-server");
            }
            Err(e) => {
                log::warn!("Failed to connect to graphics-server: {:?} - display output disabled", e);
            }
        }
    }

    let mut runtime = LvglRuntime::new();

    // Create demo UI
    runtime.create_demo_ui();

    // Get ticktimer for periodic updates
    let tt = ticktimer_server::Ticktimer::new().unwrap();

    info!("LVGL runtime initialized, ready to accept requests");

    let mut last_tick = tt.elapsed_ms();

    loop {
        // Process LVGL timers periodically
        let now = tt.elapsed_ms();
        let elapsed = (now - last_tick) as u32;
        if elapsed > 0 {
            runtime.tick(elapsed);
            last_tick = now;
        }

        // Run LVGL handler
        let _next_run = runtime.handler();

        // Check for messages
        let msg = xous::receive_message(lvgl_sid);
        match msg {
            Ok(envelope) => {
                let opcode = FromPrimitive::from_usize(envelope.body.id());
                match opcode {
                    Some(Opcode::Init) => {
                        info!("Received Init request");
                    }
                    Some(Opcode::GetScreen) => {
                        info!("Received GetScreen request");
                        let handle = runtime.get_screen();
                        if envelope.body.scalar_message().is_some() {
                            xous::return_scalar(envelope.sender, handle as usize)
                                .expect("couldn't return screen handle");
                        }
                    }
                    Some(Opcode::CreateLabel) => {
                        if let Some(scalar) = envelope.body.scalar_message() {
                            let parent_handle = scalar.arg1 as u32;
                            info!("Received CreateLabel request (parent={})", parent_handle);
                            let handle = runtime.create_label(parent_handle);
                            xous::return_scalar(envelope.sender, handle as usize)
                                .expect("couldn't return label handle");
                        }
                    }
                    Some(Opcode::CreateButton) => {
                        if let Some(scalar) = envelope.body.scalar_message() {
                            let parent_handle = scalar.arg1 as u32;
                            info!("Received CreateButton request (parent={})", parent_handle);
                            let handle = runtime.create_button(parent_handle);
                            xous::return_scalar(envelope.sender, handle as usize)
                                .expect("couldn't return button handle");
                        }
                    }
                    Some(Opcode::SetText) => {
                        // SetText uses memory message: offset=handle, valid=text_len
                        if let Some(mem) = envelope.body.memory_message() {
                            let handle = mem.offset.map(|o| o.get()).unwrap_or(0) as u32;
                            let text_len = mem.valid.map(|v| v.get()).unwrap_or(0);
                            info!("Received SetText request (handle={}, len={})", handle, text_len);

                            // Get text from memory buffer
                            let text_slice = unsafe {
                                core::slice::from_raw_parts(
                                    mem.buf.as_ptr() as *const u8,
                                    text_len.min(mem.buf.len())
                                )
                            };

                            let _result = runtime.set_text(handle, text_slice);
                        }
                    }
                    Some(Opcode::AlignObject) => {
                        if let Some(scalar) = envelope.body.scalar_message() {
                            let handle = scalar.arg1 as u32;
                            let align = scalar.arg2 as i32;
                            let x_ofs = scalar.arg3 as i32;
                            let y_ofs = scalar.arg4 as i32;
                            info!("Received AlignObject request (handle={}, align={}, x={}, y={})",
                                handle, align, x_ofs, y_ofs);
                            let result = runtime.align_object(handle, align, x_ofs, y_ofs);
                            xous::return_scalar(envelope.sender, result as usize)
                                .expect("couldn't return align result");
                        }
                    }
                    Some(Opcode::SetSize) => {
                        if let Some(scalar) = envelope.body.scalar_message() {
                            let handle = scalar.arg1 as u32;
                            let width = scalar.arg2 as i32;
                            let height = scalar.arg3 as i32;
                            info!("Received SetSize request (handle={}, w={}, h={})",
                                handle, width, height);
                            let result = runtime.set_size(handle, width, height);
                            xous::return_scalar(envelope.sender, result as usize)
                                .expect("couldn't return set_size result");
                        }
                    }
                    Some(Opcode::Refresh) => {
                        info!("Received Refresh request");
                    }
                    Some(Opcode::Quit) => {
                        info!("Quit received, shutting down LVGL runtime");
                        break;
                    }
                    Some(Opcode::DeleteObject) => {
                        if let Some(scalar) = envelope.body.scalar_message() {
                            let handle = scalar.arg1 as u32;
                            info!("Received DeleteObject request (handle={})", handle);
                            let result = runtime.delete_object(handle);
                            xous::return_scalar(envelope.sender, result as usize)
                                .expect("couldn't return delete result");
                        }
                    }
                    Some(Opcode::SetStyleTextColor) => {
                        if let Some(scalar) = envelope.body.scalar_message() {
                            let handle = scalar.arg1 as u32;
                            // RGB packed: arg2 = r | (g << 8), arg3 = b
                            let rg = scalar.arg2;
                            let r = (rg & 0xFF) as u8;
                            let g = ((rg >> 8) & 0xFF) as u8;
                            let b = (scalar.arg3 & 0xFF) as u8;
                            info!("Received SetStyleTextColor request (handle={}, r={}, g={}, b={})",
                                handle, r, g, b);
                            let result = runtime.set_style_text_color(handle, r, g, b);
                            xous::return_scalar(envelope.sender, result as usize)
                                .expect("couldn't return set_style_text_color result");
                        }
                    }
                    Some(Opcode::SetStyleBgColor) => {
                        if let Some(scalar) = envelope.body.scalar_message() {
                            let handle = scalar.arg1 as u32;
                            // RGB packed: arg2 = r | (g << 8), arg3 = b
                            let rg = scalar.arg2;
                            let r = (rg & 0xFF) as u8;
                            let g = ((rg >> 8) & 0xFF) as u8;
                            let b = (scalar.arg3 & 0xFF) as u8;
                            info!("Received SetStyleBgColor request (handle={}, r={}, g={}, b={})",
                                handle, r, g, b);
                            let result = runtime.set_style_bg_color(handle, r, g, b);
                            xous::return_scalar(envelope.sender, result as usize)
                                .expect("couldn't return set_style_bg_color result");
                        }
                    }
                    Some(Opcode::SetPos) => {
                        if let Some(scalar) = envelope.body.scalar_message() {
                            let handle = scalar.arg1 as u32;
                            let x = scalar.arg2 as i32;
                            let y = scalar.arg3 as i32;
                            info!("Received SetPos request (handle={}, x={}, y={})",
                                handle, x, y);
                            let result = runtime.set_pos(handle, x, y);
                            xous::return_scalar(envelope.sender, result as usize)
                                .expect("couldn't return set_pos result");
                        }
                    }
                    _ => {
                        log::warn!("Unknown opcode: {:?}", envelope.body.id());
                    }
                }
            }
            Err(_) => {
                // Timeout or error, continue processing
            }
        }
    }

    info!("Cleaning up LVGL runtime");
    xns.unregister_server(lvgl_sid).unwrap();
    xous::destroy_server(lvgl_sid).unwrap();
    info!("LVGL runtime shutdown complete");
    xous::terminate_process(0)
}
