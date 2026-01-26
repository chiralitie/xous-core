//! FFI bindings to WAMR C API

#![allow(non_camel_case_types)]
#![allow(dead_code)]

use core::ffi::{c_char, c_void};

// Opaque types
#[repr(C)]
pub struct wasm_module_t {
    _private: [u8; 0],
}

#[repr(C)]
pub struct wasm_module_inst_t {
    _private: [u8; 0],
}

#[repr(C)]
pub struct wasm_exec_env_t {
    _private: [u8; 0],
}

#[repr(C)]
pub struct wasm_function_inst_t {
    _private: [u8; 0],
}

// Runtime initialization struct
#[repr(C)]
pub struct RuntimeInitArgs {
    pub mem_alloc_type: u32,
    pub mem_alloc_option: MemAllocOption,
    pub native_module_name: *const c_char,
    pub n_native_symbols: u32,
    pub native_symbols: *const NativeSymbol,
    pub max_thread_num: u32,
}

#[repr(C)]
pub union MemAllocOption {
    pub pool: MemAllocTypePool,
    pub allocator: MemAllocTypeAllocator,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct MemAllocTypePool {
    pub heap_buf: *mut u8,
    pub heap_size: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct MemAllocTypeAllocator {
    pub malloc_func: *const c_void,
    pub realloc_func: *const c_void,
    pub free_func: *const c_void,
}

#[repr(C)]
pub struct NativeSymbol {
    pub symbol: *const c_char,
    pub func_ptr: *const c_void,
    pub signature: *const c_char,
    pub attachment: *const c_void,
}

// Memory allocator types
pub const ALLOC_WITH_POOL: u32 = 0;
pub const ALLOC_WITH_ALLOCATOR: u32 = 1;
pub const ALLOC_WITH_SYSTEM_ALLOCATOR: u32 = 2;

// External C functions from WAMR
extern "C" {
    pub fn wasm_runtime_init() -> bool;
    pub fn wasm_runtime_full_init(init_args: *const RuntimeInitArgs) -> bool;
    pub fn wasm_runtime_destroy();

    pub fn wasm_runtime_load(
        buf: *const u8,
        size: u32,
        error_buf: *mut c_char,
        error_buf_size: u32,
    ) -> *mut wasm_module_t;

    pub fn wasm_runtime_unload(module: *mut wasm_module_t);

    pub fn wasm_runtime_instantiate(
        module: *const wasm_module_t,
        stack_size: u32,
        heap_size: u32,
        error_buf: *mut c_char,
        error_buf_size: u32,
    ) -> *mut wasm_module_inst_t;

    pub fn wasm_runtime_deinstantiate(module_inst: *mut wasm_module_inst_t);

    pub fn wasm_runtime_create_exec_env(
        module_inst: *mut wasm_module_inst_t,
        stack_size: u32,
    ) -> *mut wasm_exec_env_t;

    pub fn wasm_runtime_destroy_exec_env(exec_env: *mut wasm_exec_env_t);

    pub fn wasm_runtime_lookup_function(
        module_inst: *const wasm_module_inst_t,
        name: *const c_char,
        signature: *const c_char,
    ) -> *mut wasm_function_inst_t;

    pub fn wasm_runtime_call_wasm(
        exec_env: *mut wasm_exec_env_t,
        func: *const wasm_function_inst_t,
        argc: u32,
        argv: *mut u32,
    ) -> bool;

    pub fn wasm_runtime_get_exception(module_inst: *mut wasm_module_inst_t) -> *const c_char;
}

// Safe wrappers
impl RuntimeInitArgs {
    pub fn new_with_system_allocator() -> Self {
        RuntimeInitArgs {
            mem_alloc_type: ALLOC_WITH_SYSTEM_ALLOCATOR,
            mem_alloc_option: MemAllocOption {
                allocator: MemAllocTypeAllocator {
                    malloc_func: core::ptr::null(),
                    realloc_func: core::ptr::null(),
                    free_func: core::ptr::null(),
                },
            },
            native_module_name: core::ptr::null(),
            n_native_symbols: 0,
            native_symbols: core::ptr::null(),
            max_thread_num: 1,
        }
    }

    pub fn new_with_native_symbols(native_symbols: *const NativeSymbol, count: u32) -> Self {
        RuntimeInitArgs {
            mem_alloc_type: ALLOC_WITH_SYSTEM_ALLOCATOR,
            mem_alloc_option: MemAllocOption {
                allocator: MemAllocTypeAllocator {
                    malloc_func: core::ptr::null(),
                    realloc_func: core::ptr::null(),
                    free_func: core::ptr::null(),
                },
            },
            native_module_name: b"env\0".as_ptr() as *const c_char,
            n_native_symbols: count,
            native_symbols,
            max_thread_num: 1,
        }
    }
}
