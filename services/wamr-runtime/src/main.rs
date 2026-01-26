#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]

mod api;
mod wamr_ffi;
mod native_funcs;

use api::*;

use num_traits::FromPrimitive;
use log::info;

// WAMR Runtime implementation
mod wamr {
    use super::wamr_ffi::*;
    use log::{info, error};
    use core::ptr;

    const ERROR_BUF_SIZE: usize = 256;

    pub struct WamrRuntime {
        initialized: bool,
        module: *mut wasm_module_t,
        module_inst: *mut wasm_module_inst_t,
        exec_env: *mut wasm_exec_env_t,
    }

    impl WamrRuntime {
        pub fn new() -> Self {
            info!("Initializing WAMR runtime");

            let mut runtime = WamrRuntime {
                initialized: false,
                module: ptr::null_mut(),
                module_inst: ptr::null_mut(),
                exec_env: ptr::null_mut(),
            };

            // Register native LVGL functions
            use super::native_funcs::*;

            let native_symbols = [
                // ============================================================
                // Clipin Core APIs
                // ============================================================
                NativeSymbol {
                    symbol: b"clipin_log\0".as_ptr() as *const core::ffi::c_char,
                    func_ptr: native_clipin_log as *const core::ffi::c_void,
                    signature: b"($)\0".as_ptr() as *const core::ffi::c_char,
                    attachment: ptr::null(),
                },
                NativeSymbol {
                    symbol: b"clipin_get_uptime\0".as_ptr() as *const core::ffi::c_char,
                    func_ptr: native_clipin_get_uptime as *const core::ffi::c_void,
                    signature: b"()i\0".as_ptr() as *const core::ffi::c_char,
                    attachment: ptr::null(),
                },
                NativeSymbol {
                    symbol: b"clipin_button_read\0".as_ptr() as *const core::ffi::c_char,
                    func_ptr: native_clipin_button_read as *const core::ffi::c_void,
                    signature: b"(i)i\0".as_ptr() as *const core::ffi::c_char,
                    attachment: ptr::null(),
                },
                // ============================================================
                // LVGL Display APIs
                // ============================================================
                NativeSymbol {
                    symbol: b"lvgl_get_lcd\0".as_ptr() as *const core::ffi::c_char,
                    func_ptr: native_lvgl_get_lcd as *const core::ffi::c_void,
                    signature: b"()i\0".as_ptr() as *const core::ffi::c_char,
                    attachment: ptr::null(),
                },
                NativeSymbol {
                    symbol: b"lvgl_get_screen\0".as_ptr() as *const core::ffi::c_char,
                    func_ptr: native_lvgl_get_screen as *const core::ffi::c_void,
                    signature: b"()i\0".as_ptr() as *const core::ffi::c_char,
                    attachment: ptr::null(),
                },
                NativeSymbol {
                    symbol: b"lvgl_create_label\0".as_ptr() as *const core::ffi::c_char,
                    func_ptr: native_lvgl_create_label as *const core::ffi::c_void,
                    signature: b"(i$)i\0".as_ptr() as *const core::ffi::c_char,
                    attachment: ptr::null(),
                },
                NativeSymbol {
                    symbol: b"lvgl_create_button\0".as_ptr() as *const core::ffi::c_char,
                    func_ptr: native_lvgl_create_button as *const core::ffi::c_void,
                    signature: b"(i$)i\0".as_ptr() as *const core::ffi::c_char,
                    attachment: ptr::null(),
                },
                NativeSymbol {
                    symbol: b"lvgl_set_text\0".as_ptr() as *const core::ffi::c_char,
                    func_ptr: native_lvgl_set_text as *const core::ffi::c_void,
                    signature: b"(i$)\0".as_ptr() as *const core::ffi::c_char,
                    attachment: ptr::null(),
                },
                NativeSymbol {
                    symbol: b"lvgl_set_pos\0".as_ptr() as *const core::ffi::c_char,
                    func_ptr: native_lvgl_set_pos as *const core::ffi::c_void,
                    signature: b"(iii)\0".as_ptr() as *const core::ffi::c_char,
                    attachment: ptr::null(),
                },
                NativeSymbol {
                    symbol: b"lvgl_set_size\0".as_ptr() as *const core::ffi::c_char,
                    func_ptr: native_lvgl_set_size as *const core::ffi::c_void,
                    signature: b"(iii)\0".as_ptr() as *const core::ffi::c_char,
                    attachment: ptr::null(),
                },
                NativeSymbol {
                    symbol: b"lvgl_align\0".as_ptr() as *const core::ffi::c_char,
                    func_ptr: native_lvgl_align as *const core::ffi::c_void,
                    signature: b"(iiii)\0".as_ptr() as *const core::ffi::c_char,
                    attachment: ptr::null(),
                },
                NativeSymbol {
                    symbol: b"lvgl_delete\0".as_ptr() as *const core::ffi::c_char,
                    func_ptr: native_lvgl_delete as *const core::ffi::c_void,
                    signature: b"(i)\0".as_ptr() as *const core::ffi::c_char,
                    attachment: ptr::null(),
                },
                NativeSymbol {
                    symbol: b"lvgl_set_style_text_color\0".as_ptr() as *const core::ffi::c_char,
                    func_ptr: native_lvgl_set_style_text_color as *const core::ffi::c_void,
                    signature: b"(iiii)\0".as_ptr() as *const core::ffi::c_char,
                    attachment: ptr::null(),
                },
                NativeSymbol {
                    symbol: b"lvgl_set_style_bg_color\0".as_ptr() as *const core::ffi::c_char,
                    func_ptr: native_lvgl_set_style_bg_color as *const core::ffi::c_void,
                    signature: b"(iiii)\0".as_ptr() as *const core::ffi::c_char,
                    attachment: ptr::null(),
                },
                // ============================================================
                // Legacy/Alias APIs
                // ============================================================
                NativeSymbol {
                    symbol: b"print\0".as_ptr() as *const core::ffi::c_char,
                    func_ptr: native_print as *const core::ffi::c_void,
                    signature: b"($)\0".as_ptr() as *const core::ffi::c_char,
                    attachment: ptr::null(),
                },
            ];

            // Initialize WAMR with native symbols
            let init_args = RuntimeInitArgs::new_with_native_symbols(
                native_symbols.as_ptr(),
                native_symbols.len() as u32,
            );
            let result = unsafe { wasm_runtime_full_init(&init_args) };

            if result {
                runtime.initialized = true;
                info!("WAMR runtime initialized with {} native functions", native_symbols.len());
            } else {
                error!("Failed to initialize WAMR runtime");
            }

            runtime
        }

        pub fn load_module(&mut self, data: &[u8]) -> Result<(), &'static str> {
            if !self.initialized {
                return Err("WAMR runtime not initialized");
            }

            // Unload existing module if any
            if !self.module.is_null() {
                self.unload_module();
            }

            info!("Loading WASM module, size: {} bytes", data.len());

            let mut error_buf = [0u8; ERROR_BUF_SIZE];

            let module = unsafe {
                wasm_runtime_load(
                    data.as_ptr(),
                    data.len() as u32,
                    error_buf.as_mut_ptr() as *mut core::ffi::c_char,
                    ERROR_BUF_SIZE as u32,
                )
            };

            if module.is_null() {
                let error_str = core::str::from_utf8(&error_buf)
                    .unwrap_or("Unknown error")
                    .trim_end_matches('\0');
                error!("Failed to load WASM module: {}", error_str);
                return Err("Failed to load WASM module");
            }

            self.module = module;
            info!("WASM module loaded successfully");

            // Instantiate module
            let stack_size = 16 * 1024; // 16KB stack
            let heap_size = 32 * 1024;  // 32KB heap

            let module_inst = unsafe {
                wasm_runtime_instantiate(
                    module,
                    stack_size,
                    heap_size,
                    error_buf.as_mut_ptr() as *mut core::ffi::c_char,
                    ERROR_BUF_SIZE as u32,
                )
            };

            if module_inst.is_null() {
                let error_str = core::str::from_utf8(&error_buf)
                    .unwrap_or("Unknown error")
                    .trim_end_matches('\0');
                error!("Failed to instantiate WASM module: {}", error_str);
                unsafe { wasm_runtime_unload(module); }
                self.module = ptr::null_mut();
                return Err("Failed to instantiate WASM module");
            }

            self.module_inst = module_inst;
            info!("WASM module instantiated successfully");

            // Create execution environment
            let exec_env = unsafe {
                wasm_runtime_create_exec_env(module_inst, stack_size)
            };

            if exec_env.is_null() {
                error!("Failed to create execution environment");
                unsafe {
                    wasm_runtime_deinstantiate(module_inst);
                    wasm_runtime_unload(module);
                }
                self.module_inst = ptr::null_mut();
                self.module = ptr::null_mut();
                return Err("Failed to create execution environment");
            }

            self.exec_env = exec_env;
            info!("WASM execution environment created");

            Ok(())
        }

        pub fn execute(&mut self, func_name: &str) -> Result<i32, &'static str> {
            if self.module_inst.is_null() || self.exec_env.is_null() {
                return Err("No module loaded");
            }

            info!("Executing WASM function: {}", func_name);

            // Convert function name to C string
            let mut name_buf = [0u8; 64];
            let name_bytes = func_name.as_bytes();
            if name_bytes.len() >= name_buf.len() {
                return Err("Function name too long");
            }
            name_buf[..name_bytes.len()].copy_from_slice(name_bytes);

            // Lookup function
            let func = unsafe {
                wasm_runtime_lookup_function(
                    self.module_inst,
                    name_buf.as_ptr() as *const core::ffi::c_char,
                    ptr::null(),
                )
            };

            if func.is_null() {
                error!("Function '{}' not found in module", func_name);
                return Err("Function not found");
            }

            // Call function (no arguments for now)
            let mut argv: [u32; 8] = [0; 8];
            let result = unsafe {
                wasm_runtime_call_wasm(
                    self.exec_env,
                    func,
                    0,
                    argv.as_mut_ptr(),
                )
            };

            if !result {
                let exception = unsafe {
                    let exc_ptr = wasm_runtime_get_exception(self.module_inst);
                    if !exc_ptr.is_null() {
                        core::ffi::CStr::from_ptr(exc_ptr).to_str().ok()
                    } else {
                        None
                    }
                };

                if let Some(exc_msg) = exception {
                    error!("WASM execution failed: {}", exc_msg);
                } else {
                    error!("WASM execution failed: unknown error");
                }
                return Err("WASM execution failed");
            }

            // Return value is in argv[0]
            Ok(argv[0] as i32)
        }

        /// Check if a function exists in the loaded module
        pub fn has_function(&self, func_name: &str) -> bool {
            if self.module_inst.is_null() {
                return false;
            }

            // Convert function name to C string
            let mut name_buf = [0u8; 64];
            let name_bytes = func_name.as_bytes();
            if name_bytes.len() >= name_buf.len() {
                return false;
            }
            name_buf[..name_bytes.len()].copy_from_slice(name_bytes);

            let func = unsafe {
                wasm_runtime_lookup_function(
                    self.module_inst,
                    name_buf.as_ptr() as *const core::ffi::c_char,
                    ptr::null(),
                )
            };

            !func.is_null()
        }

        pub fn unload_module(&mut self) {
            info!("Unloading WASM module");

            if !self.exec_env.is_null() {
                unsafe { wasm_runtime_destroy_exec_env(self.exec_env); }
                self.exec_env = ptr::null_mut();
            }

            if !self.module_inst.is_null() {
                unsafe { wasm_runtime_deinstantiate(self.module_inst); }
                self.module_inst = ptr::null_mut();
            }

            if !self.module.is_null() {
                unsafe { wasm_runtime_unload(self.module); }
                self.module = ptr::null_mut();
            }

            info!("WASM module unloaded");
        }
    }

    impl Drop for WamrRuntime {
        fn drop(&mut self) {
            self.unload_module();
            if self.initialized {
                unsafe { wasm_runtime_destroy(); }
                info!("WAMR runtime destroyed");
            }
        }
    }
}

fn main() -> ! {
    use crate::wamr::WamrRuntime;

    log_server::init_wait().unwrap();
    log::set_max_level(log::LevelFilter::Info);
    info!("WAMR Runtime starting, PID: {}", xous::process::id());

    let xns = xous_names::XousNames::new().unwrap();
    let wamr_sid = xns
        .register_name(api::SERVER_NAME_WAMR, None)
        .expect("can't register WAMR server");
    info!("WAMR server registered with NS: {:?}", wamr_sid);

    let mut runtime = WamrRuntime::new();

    // Connect to LVGL runtime for native function calls
    native_funcs::init_lvgl_connection();

    // Register as keyboard listener for button input
    match keyboard::Keyboard::new(&xns) {
        Ok(kbd) => {
            kbd.register_listener(api::SERVER_NAME_WAMR, Opcode::KeyboardEvent as usize);
            info!("Registered as keyboard listener");
        }
        Err(e) => {
            log::warn!("Failed to register keyboard listener: {:?}", e);
        }
    }

    // Get ticktimer for periodic app_tick calls
    let tt = ticktimer_server::Ticktimer::new().unwrap();

    let mut app_running = false;
    let mut has_app_tick = false;

    // WASM execution in hosted mode crashes due to memory mapping differences
    // between WAMR's WASM memory and the host process address space.
    // Disable for now until this is fixed.
    #[cfg(target_os = "xous")]
    {
        // Load and execute test WASM module
        info!("Loading test WASM module...");
        const WASM_BINARY: &[u8] = include_bytes!("../test/hello_world.wasm");

        match runtime.load_module(WASM_BINARY) {
            Ok(()) => {
                info!("WASM module loaded successfully");

                // Check if app_tick is exported
                has_app_tick = runtime.has_function("app_tick");
                info!("app_tick exported: {}", has_app_tick);

                // Execute app_main (the initialization function)
                match runtime.execute("app_main") {
                    Ok(result) => {
                        info!("app_main executed successfully, returned: {}", result);
                        if result >= 0 {
                            app_running = true;
                        } else {
                            log::warn!("app_main returned error code: {}", result);
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to execute app_main: {}", e);
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to load WASM module: {}", e);
            }
        }
    }

    #[cfg(not(target_os = "xous"))]
    {
        info!("WAMR runtime ready (WASM execution disabled in hosted mode - needs memory mapping fix)");
        let _ = (&mut app_running, &mut has_app_tick); // suppress unused warnings
    }

    info!("WAMR runtime initialized, ready to accept requests");

    let mut last_tick = tt.elapsed_ms();
    const TICK_INTERVAL_MS: u64 = 16; // ~60Hz

    loop {
        // Call app_tick periodically if the app is running and exports it
        if app_running && has_app_tick {
            let now = tt.elapsed_ms();
            if now - last_tick >= TICK_INTERVAL_MS {
                last_tick = now;

                match runtime.execute("app_tick") {
                    Ok(result) => {
                        if result < 0 {
                            info!("app_tick returned {}, stopping app", result);
                            app_running = false;
                        }
                    }
                    Err(e) => {
                        log::error!("app_tick failed: {}, stopping app", e);
                        app_running = false;
                    }
                }
            }
        }

        // Check for IPC messages (non-blocking with short timeout)
        let msg = xous::try_receive_message(wamr_sid);
        match msg {
            Ok(Some(envelope)) => {
                match FromPrimitive::from_usize(envelope.body.id()) {
                    Some(Opcode::LoadModule) => {
                        info!("Received LoadModule request");
                        // TODO: Implement module loading from message
                    }
                    Some(Opcode::Execute) => {
                        info!("Received Execute request");
                        // TODO: Implement function execution
                    }
                    Some(Opcode::UnloadModule) => {
                        info!("Received UnloadModule request");
                        runtime.unload_module();
                        app_running = false;
                    }
                    Some(Opcode::Quit) => {
                        info!("Quit received, shutting down WAMR runtime");
                        break;
                    }
                    Some(Opcode::KeyboardEvent) => {
                        // Handle keyboard event - extract character from scalar message
                        if let xous::Message::Scalar(scalar) = envelope.body {
                            let c = char::from_u32(scalar.arg1 as u32).unwrap_or('\0');
                            if c != '\0' {
                                // Key press event
                                native_funcs::update_button_from_char(c, true);
                                // Auto-release after a short time (simulated key press)
                                // In a real implementation, we'd track key up events separately
                            }
                        }
                    }
                    None => {
                        log::error!("Unknown opcode: {:?}", envelope.body.id());
                    }
                }
            }
            Ok(None) => {
                // No message, sleep briefly to avoid busy-waiting
                if app_running && has_app_tick {
                    tt.sleep_ms(1).ok();
                } else {
                    // If no app_tick, use blocking receive
                    let msg = xous::receive_message(wamr_sid).unwrap();
                    match FromPrimitive::from_usize(msg.body.id()) {
                        Some(Opcode::LoadModule) => {
                            info!("Received LoadModule request");
                        }
                        Some(Opcode::Execute) => {
                            info!("Received Execute request");
                        }
                        Some(Opcode::UnloadModule) => {
                            info!("Received UnloadModule request");
                            runtime.unload_module();
                        }
                        Some(Opcode::Quit) => {
                            info!("Quit received, shutting down WAMR runtime");
                            break;
                        }
                        Some(Opcode::KeyboardEvent) => {
                            // Handle keyboard event
                            if let xous::Message::Scalar(scalar) = msg.body {
                                let c = char::from_u32(scalar.arg1 as u32).unwrap_or('\0');
                                if c != '\0' {
                                    native_funcs::update_button_from_char(c, true);
                                }
                            }
                        }
                        None => {
                            log::error!("Unknown opcode: {:?}", msg.body.id());
                        }
                    }
                }
            }
            Err(_) => {
                // Error receiving message, continue
            }
        }
    }

    info!("Cleaning up WAMR runtime");
    xns.unregister_server(wamr_sid).unwrap();
    xous::destroy_server(wamr_sid).unwrap();
    info!("WAMR runtime shutdown complete");
    xous::terminate_process(0)
}
