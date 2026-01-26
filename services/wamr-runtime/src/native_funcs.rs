//! Native functions exposed to WebAssembly
//! These functions communicate with lvgl-runtime via IPC
//! Implements the Clipin SDK API for binary compatibility

use core::ffi::c_char;
use core::sync::atomic::{AtomicU32, AtomicU8, Ordering};
use log::info;

// Global connection to LVGL runtime (set during initialization)
static LVGL_CID: AtomicU32 = AtomicU32::new(0);

// Ticktimer for uptime
static mut TICKTIMER: Option<ticktimer_server::Ticktimer> = None;

// Keyboard connection
static mut KEYBOARD: Option<keyboard::Keyboard> = None;

// Button states (atomic for thread safety)
// Bit 0 = Up, Bit 1 = Down, Bit 2 = Left, Bit 3 = Right, Bit 4 = Center
static BUTTON_STATES: AtomicU8 = AtomicU8::new(0);

// Button ID constants
pub const BUTTON_UP: i32 = 0;
pub const BUTTON_DOWN: i32 = 1;
pub const BUTTON_LEFT: i32 = 2;
pub const BUTTON_RIGHT: i32 = 3;
pub const BUTTON_CENTER: i32 = 4;

// Text buffer for IPC (max 256 bytes)
const TEXT_BUF_SIZE: usize = 256;
static mut TEXT_BUFFER: [u8; TEXT_BUF_SIZE] = [0u8; TEXT_BUF_SIZE];

/// Initialize connection to LVGL runtime and keyboard
pub fn init_lvgl_connection() {
    match xous_names::XousNames::new() {
        Ok(xns) => {
            // Connect to LVGL runtime
            if LVGL_CID.load(Ordering::Relaxed) == 0 {
                match xns.request_connection_blocking("_LVGL Runtime_") {
                    Ok(cid) => {
                        LVGL_CID.store(cid.into(), Ordering::Release);
                        info!("Connected to LVGL runtime");
                    }
                    Err(e) => {
                        log::error!("Failed to connect to LVGL runtime: {:?}", e);
                    }
                }
            }

            // Connect to keyboard
            unsafe {
                if KEYBOARD.is_none() {
                    match keyboard::Keyboard::new(&xns) {
                        Ok(kbd) => {
                            info!("Connected to keyboard service");
                            KEYBOARD = Some(kbd);
                        }
                        Err(e) => {
                            log::error!("Failed to connect to keyboard: {:?}", e);
                        }
                    }
                }
            }
        }
        Err(e) => {
            log::error!("Failed to create XNS client: {:?}", e);
        }
    }

    // Initialize ticktimer
    unsafe {
        if TICKTIMER.is_none() {
            TICKTIMER = ticktimer_server::Ticktimer::new().ok();
        }
    }
}

/// Update button state from a character (called from key event handler)
pub fn update_button_from_char(c: char, pressed: bool) {
    let bit = match c {
        '\u{F700}' | '↑' => Some(0), // Up arrow
        '\u{F701}' | '↓' => Some(1), // Down arrow
        '\u{F702}' | '←' => Some(2), // Left arrow
        '\u{F703}' | '→' => Some(3), // Right arrow
        '\u{F729}' | '\n' => Some(4), // Home key / Enter = Center
        _ => None,
    };

    if let Some(bit) = bit {
        if pressed {
            BUTTON_STATES.fetch_or(1 << bit, Ordering::SeqCst);
        } else {
            BUTTON_STATES.fetch_and(!(1 << bit), Ordering::SeqCst);
        }
    }
}

/// Poll keyboard for button states (non-blocking)
pub fn poll_keyboard() {
    // In hosted mode, the keyboard events come through minifb
    // and are injected via the keyboard service.
    // We use get_keys_blocking in a non-blocking way by checking
    // if there are pending keys.
    //
    // For now, we rely on the main loop to call this periodically
    // and the keyboard events are handled by the GAM.
    //
    // A more complete implementation would register as a raw listener.
}

fn get_lvgl_cid() -> Option<xous::CID> {
    let cid = LVGL_CID.load(Ordering::Acquire);
    if cid != 0 {
        Some(xous::CID::from(cid))
    } else {
        None
    }
}

/// Helper: Copy C string to buffer, return length
fn copy_cstr_to_buffer(text_ptr: *const c_char) -> usize {
    if text_ptr.is_null() {
        return 0;
    }

    unsafe {
        // Safely read the C string with a maximum length limit
        let mut len = 0usize;
        let mut ptr = text_ptr;
        while len < TEXT_BUF_SIZE - 1 {
            let c = *ptr;
            if c == 0 {
                break;
            }
            TEXT_BUFFER[len] = c as u8;
            len += 1;
            ptr = ptr.add(1);
        }
        TEXT_BUFFER[len] = 0; // Null terminate
        len + 1 // Include null terminator in length
    }
}

// ============================================================================
// Clipin Core APIs
// ============================================================================

/// Native function: Log debug message
#[no_mangle]
pub extern "C" fn native_clipin_log(msg_ptr: *const c_char) {
    if msg_ptr.is_null() {
        info!("WASM: (null message)");
        return;
    }
    unsafe {
        // Safety: Check if pointer looks valid before dereferencing
        let msg = core::ffi::CStr::from_ptr(msg_ptr);
        if let Ok(s) = msg.to_str() {
            info!("WASM: {}", s);
        } else {
            info!("WASM: (invalid UTF-8)");
        }
    }
}

/// Native function: Get system uptime in milliseconds
#[no_mangle]
pub extern "C" fn native_clipin_get_uptime() -> u32 {
    unsafe {
        if let Some(ref tt) = TICKTIMER {
            tt.elapsed_ms() as u32
        } else {
            0
        }
    }
}

/// Native function: Read button state
/// Button IDs for Precursor device:
///   0 = Up
///   1 = Down
///   2 = Left
///   3 = Right
///   4 = Center/Select (Home key in simulator)
/// Returns 1 if pressed, 0 if released
#[no_mangle]
pub extern "C" fn native_clipin_button_read(button_id: i32) -> i32 {
    if button_id < 0 || button_id > 4 {
        return 0;
    }

    let states = BUTTON_STATES.load(Ordering::SeqCst);
    if (states >> button_id) & 1 != 0 {
        1
    } else {
        0
    }
}

// ============================================================================
// LVGL Display APIs
// ============================================================================

/// Native function: Get LCD canvas handle (always returns 1)
#[no_mangle]
pub extern "C" fn native_lvgl_get_lcd() -> u32 {
    info!("native_lvgl_get_lcd called");
    1 // Screen is always handle 1
}

/// Native function: Get screen handle (alias for get_lcd)
#[no_mangle]
pub extern "C" fn native_lvgl_get_screen() -> u32 {
    native_lvgl_get_lcd()
}

/// Native function: Create label with text
#[no_mangle]
pub extern "C" fn native_lvgl_create_label(parent_handle: u32, text_ptr: *const c_char) -> u32 {
    info!("native_lvgl_create_label(parent={})", parent_handle);

    let text_len = copy_cstr_to_buffer(text_ptr);

    match get_lvgl_cid() {
        Some(cid) => {
            // First create the label
            let response = xous::send_message(
                cid,
                xous::Message::new_blocking_scalar(2, parent_handle as usize, 0, 0, 0)
            );

            match response {
                Ok(xous::Result::Scalar1(handle)) => {
                    let handle = handle as u32;
                    // Then set the text if provided
                    if text_len > 0 && handle != 0 {
                        send_text_to_lvgl(cid, handle, text_len);
                    }
                    handle
                }
                _ => {
                    log::warn!("Failed to create label");
                    0
                }
            }
        }
        None => {
            log::warn!("Not connected to LVGL runtime");
            0
        }
    }
}

/// Native function: Create button with text
#[no_mangle]
pub extern "C" fn native_lvgl_create_button(parent_handle: u32, text_ptr: *const c_char) -> u32 {
    info!("native_lvgl_create_button(parent={})", parent_handle);

    let text_len = copy_cstr_to_buffer(text_ptr);

    match get_lvgl_cid() {
        Some(cid) => {
            // First create the button
            let response = xous::send_message(
                cid,
                xous::Message::new_blocking_scalar(3, parent_handle as usize, 0, 0, 0)
            );

            match response {
                Ok(xous::Result::Scalar1(handle)) => {
                    let handle = handle as u32;
                    // Then set the text if provided
                    if text_len > 0 && handle != 0 {
                        send_text_to_lvgl(cid, handle, text_len);
                    }
                    handle
                }
                _ => {
                    log::warn!("Failed to create button");
                    0
                }
            }
        }
        None => {
            log::warn!("Not connected to LVGL runtime");
            0
        }
    }
}

/// Helper: Send text buffer to LVGL runtime
fn send_text_to_lvgl(cid: xous::CID, handle: u32, text_len: usize) {
    #[allow(static_mut_refs)]
    unsafe {
        // Use memory message to send text
        let buf = xous::MemoryRange::new(
            TEXT_BUFFER.as_ptr() as usize,
            TEXT_BUF_SIZE
        ).expect("Failed to create memory range");

        // Send with handle in offset field, text_len in valid field
        let _ = xous::send_message(
            cid,
            xous::Message::new_lend(
                4, // SetText opcode
                buf,
                xous::MemoryAddress::new(handle as usize),
                xous::MemorySize::new(text_len)
            )
        );
    }
}

/// Native function: Set widget text
#[no_mangle]
pub extern "C" fn native_lvgl_set_text(handle: u32, text_ptr: *const c_char) {
    info!("native_lvgl_set_text(handle={})", handle);

    if text_ptr.is_null() {
        return;
    }

    let text_len = copy_cstr_to_buffer(text_ptr);

    if let Some(cid) = get_lvgl_cid() {
        send_text_to_lvgl(cid, handle, text_len);
    } else {
        log::warn!("Not connected to LVGL runtime");
    }
}

/// Native function: Set widget position
#[no_mangle]
pub extern "C" fn native_lvgl_set_pos(handle: u32, x: i32, y: i32) {
    info!("native_lvgl_set_pos(handle={}, x={}, y={})", handle, x, y);

    if let Some(cid) = get_lvgl_cid() {
        let _ = xous::send_message(
            cid,
            xous::Message::new_blocking_scalar(
                13, // SetPos opcode
                handle as usize,
                x as usize,
                y as usize,
                0
            )
        );
    }
}

/// Native function: Set widget size
#[no_mangle]
pub extern "C" fn native_lvgl_set_size(handle: u32, width: i32, height: i32) {
    info!("native_lvgl_set_size(handle={}, w={}, h={})", handle, width, height);

    if let Some(cid) = get_lvgl_cid() {
        let _ = xous::send_message(
            cid,
            xous::Message::new_blocking_scalar(
                6, // SetSize opcode
                handle as usize,
                width as usize,
                height as usize,
                0
            )
        );
    }
}

/// Native function: Align widget
#[no_mangle]
pub extern "C" fn native_lvgl_align(handle: u32, align: i32, x_ofs: i32, y_ofs: i32) {
    info!("native_lvgl_align(handle={}, align={}, x={}, y={})", handle, align, x_ofs, y_ofs);

    if let Some(cid) = get_lvgl_cid() {
        let _ = xous::send_message(
            cid,
            xous::Message::new_blocking_scalar(
                5, // AlignObject opcode
                handle as usize,
                align as usize,
                x_ofs as usize,
                y_ofs as usize
            )
        );
    }
}

/// Native function: Delete widget
#[no_mangle]
pub extern "C" fn native_lvgl_delete(handle: u32) {
    info!("native_lvgl_delete(handle={})", handle);

    if let Some(cid) = get_lvgl_cid() {
        let _ = xous::send_message(
            cid,
            xous::Message::new_blocking_scalar(
                10, // DeleteObject opcode
                handle as usize,
                0, 0, 0
            )
        );
    }
}

/// Native function: Set text color (RGB)
#[no_mangle]
pub extern "C" fn native_lvgl_set_style_text_color(handle: u32, r: i32, g: i32, b: i32) {
    info!("native_lvgl_set_style_text_color(handle={}, r={}, g={}, b={})", handle, r, g, b);

    if let Some(cid) = get_lvgl_cid() {
        // Pack RGB into two args: arg2 = r | (g << 8), arg3 = b
        let rg = (r as usize) | ((g as usize) << 8);
        let _ = xous::send_message(
            cid,
            xous::Message::new_blocking_scalar(
                11, // SetStyleTextColor opcode
                handle as usize,
                rg,
                b as usize,
                0
            )
        );
    }
}

/// Native function: Set background color (RGB)
#[no_mangle]
pub extern "C" fn native_lvgl_set_style_bg_color(handle: u32, r: i32, g: i32, b: i32) {
    info!("native_lvgl_set_style_bg_color(handle={}, r={}, g={}, b={})", handle, r, g, b);

    if let Some(cid) = get_lvgl_cid() {
        // Pack RGB into two args: arg2 = r | (g << 8), arg3 = b
        let rg = (r as usize) | ((g as usize) << 8);
        let _ = xous::send_message(
            cid,
            xous::Message::new_blocking_scalar(
                12, // SetStyleBgColor opcode
                handle as usize,
                rg,
                b as usize,
                0
            )
        );
    }
}

// ============================================================================
// Legacy/Alias APIs (for backward compatibility)
// ============================================================================

/// Native function: Print debug message (alias for clipin_log)
#[no_mangle]
pub extern "C" fn native_print(msg_ptr: *const c_char) {
    native_clipin_log(msg_ptr);
}
