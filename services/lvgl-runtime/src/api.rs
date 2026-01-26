//! LVGL Runtime API

#[derive(Debug, num_derive::FromPrimitive, num_derive::ToPrimitive)]
pub(crate) enum Opcode {
    /// Initialize LVGL display
    Init = 0,
    /// Get screen handle (always returns 1)
    GetScreen = 1,
    /// Create a label widget
    CreateLabel = 2,
    /// Create a button widget
    CreateButton = 3,
    /// Set object text (memory message: offset=handle, valid=text_len)
    SetText = 4,
    /// Align object
    AlignObject = 5,
    /// Set object size
    SetSize = 6,
    /// Handle input event
    InputEvent = 7,
    /// Force screen refresh
    Refresh = 8,
    /// Quit the runtime
    Quit = 9,
    /// Delete object and free handle
    DeleteObject = 10,
    /// Set text color (RGB packed in args)
    SetStyleTextColor = 11,
    /// Set background color (RGB packed in args)
    SetStyleBgColor = 12,
    /// Set object position
    SetPos = 13,
}

pub const SERVER_NAME_LVGL: &str = "_LVGL Runtime_";

// Message structures for scalar messages (reserved for future use)
#[allow(dead_code)]
#[derive(Debug)]
pub struct CreateWidgetMsg {
    pub parent_handle: u32,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct SetSizeMsg {
    pub handle: u32,
    pub width: i32,
    pub height: i32,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct AlignMsg {
    pub handle: u32,
    pub align: i32,
    pub x_ofs: i32,
    pub y_ofs: i32,
}
