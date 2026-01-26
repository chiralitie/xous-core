#[derive(Debug, num_derive::FromPrimitive, num_derive::ToPrimitive)]
pub(crate) enum Opcode {
    /// Load a WASM module from memory
    LoadModule = 0,
    /// Execute a function in the loaded module
    Execute = 1,
    /// Unload the current module
    UnloadModule = 2,
    /// Quit the server
    Quit = 3,
    /// Keyboard event from keyboard service
    KeyboardEvent = 4,
}

pub const SERVER_NAME_WAMR: &str = "_WAMR Runtime_";
