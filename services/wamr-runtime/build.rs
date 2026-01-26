use std::env;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let wamr_dir = PathBuf::from("wamr");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=wamr/");

    // Check if we're building for Xous (RISC-V)
    let target = env::var("TARGET").unwrap_or_default();
    let is_xous = target.contains("xous") || target.contains("riscv32");

    let mut build = cc::Build::new();

    // WAMR core paths
    let iwasm_common = wamr_dir.join("core/iwasm/common");
    let iwasm_interp = wamr_dir.join("core/iwasm/interpreter");
    let iwasm_include = wamr_dir.join("core/iwasm/include");
    let shared_platform = wamr_dir.join("core/shared/platform");
    let shared_utils = wamr_dir.join("core/shared/utils");
    let shared_mem = wamr_dir.join("core/shared/mem-alloc");

    if is_xous {
        // Use riscv64-unknown-elf-gcc with 32-bit flags for Xous
        build.compiler("riscv64-unknown-elf-gcc");
        build.archiver("riscv64-unknown-elf-ar");

        // Include paths - stubs first so they override system headers
        build.include("stubs");
        build.include(shared_platform.join("xous"));

        // Platform-specific defines for Xous
        build.define("BH_PLATFORM_XOUS", "1");
        build.define("BUILD_TARGET_RISCV32_ILP32", "1");

        // Platform source for Xous
        build.file(shared_platform.join("xous/xous_platform.c"));

        // C library stubs for bare-metal
        build.file("stubs/xous_stubs.c");
    } else {
        // Use native compiler for hosted builds (Linux)
        println!("cargo:warning=WAMR: Building for hosted Linux target");

        // Include paths for Linux
        build.include(shared_platform.join("linux"));
        build.include(shared_platform.join("common/posix"));

        // Platform-specific defines for Linux
        build.define("BH_PLATFORM_LINUX", "1");
        if cfg!(target_pointer_width = "64") {
            build.define("BUILD_TARGET_X86_64", "1");
        } else {
            build.define("BUILD_TARGET_X86_32", "1");
        }

        // Platform source for Linux
        build.file(shared_platform.join("linux/platform_init.c"));
        build.file(shared_platform.join("common/posix/posix_thread.c"));
        build.file(shared_platform.join("common/posix/posix_time.c"));
        build.file(shared_platform.join("common/posix/posix_malloc.c"));
        build.file(shared_platform.join("common/posix/posix_memmap.c"));

        // Link pthread for Linux
        println!("cargo:rustc-link-lib=pthread");
    }

    // Common include paths
    build.include(wamr_dir.join("core"));
    build.include(&iwasm_common);
    build.include(&iwasm_interp);
    build.include(&iwasm_include);
    build.include(shared_platform.join("include"));
    build.include(&shared_utils);
    build.include(&shared_mem);
    build.include(shared_mem.join("ems"));

    // Compiler flags
    build.flag("-fno-exceptions");
    build.flag("-ffunction-sections");
    build.flag("-fdata-sections");
    build.flag("-Wall");
    build.flag("-Wno-unused-parameter");
    build.flag("-Wno-unused-variable");

    // WAMR configuration defines - interpreter only, minimal config
    build.define("WASM_ENABLE_INTERP", "1");
    build.define("WASM_ENABLE_AOT", "0");
    build.define("WASM_ENABLE_JIT", "0");
    build.define("WASM_ENABLE_FAST_INTERP", "0");
    build.define("WASM_ENABLE_MINI_LOADER", "1");
    build.define("WASM_ENABLE_LIBC_BUILTIN", "0");
    build.define("WASM_ENABLE_LIBC_WASI", "0");
    build.define("WASM_ENABLE_MULTI_MODULE", "0");
    build.define("WASM_ENABLE_THREAD_MGR", "0");
    build.define("WASM_ENABLE_MEMORY_PROFILING", "0");
    build.define("WASM_ENABLE_MEMORY_TRACING", "0");
    build.define("WASM_ENABLE_DUMP_CALL_STACK", "0");
    build.define("WASM_ENABLE_PERF_PROFILING", "0");
    build.define("WASM_ENABLE_SPEC_TEST", "0");
    build.define("WASM_ENABLE_BULK_MEMORY", "0");
    build.define("WASM_ENABLE_REF_TYPES", "0");
    build.define("WASM_ENABLE_SIMD", "0");
    build.define("WASM_ENABLE_TAIL_CALL", "0");
    build.define("WASM_ENABLE_SHARED_MEMORY", "0");
    build.define("WASM_ENABLE_CUSTOM_NAME_SECTION", "0");
    build.define("WASM_ENABLE_LOAD_CUSTOM_SECTION", "0");
    build.define("WASM_ENABLE_GLOBAL_HEAP_POOL", "0");
    build.define("WASM_ENABLE_WAMR_COMPILER", "0");
    build.define("WASM_ENABLE_EXCE_HANDLING", "0");
    build.define("WASM_ENABLE_GC", "0");
    build.define("WASM_ENABLE_STRINGREF", "0");
    build.define("WASM_ENABLE_WASI", "0");
    build.define("WASM_DISABLE_HW_BOUND_CHECK", "1");
    build.define("BH_MALLOC", "wasm_runtime_malloc");
    build.define("BH_FREE", "wasm_runtime_free");

    // Shared utils
    build.file(shared_utils.join("bh_assert.c"));
    build.file(shared_utils.join("bh_common.c"));
    build.file(shared_utils.join("bh_hashmap.c"));
    build.file(shared_utils.join("bh_leb128.c"));
    build.file(shared_utils.join("bh_list.c"));
    build.file(shared_utils.join("bh_log.c"));
    build.file(shared_utils.join("bh_queue.c"));
    build.file(shared_utils.join("bh_vector.c"));
    build.file(shared_utils.join("runtime_timer.c"));

    // Memory allocator (EMS)
    build.file(shared_mem.join("mem_alloc.c"));
    build.file(shared_mem.join("ems/ems_kfc.c"));
    build.file(shared_mem.join("ems/ems_hmu.c"));
    build.file(shared_mem.join("ems/ems_alloc.c"));
    build.file(shared_mem.join("ems/ems_gc.c"));

    // IWASM common - removed wasm_c_api.c and wasm_application.c (not needed for basic interpreter)
    build.file(iwasm_common.join("wasm_runtime_common.c"));
    build.file(iwasm_common.join("wasm_native.c"));
    build.file(iwasm_common.join("wasm_exec_env.c"));
    build.file(iwasm_common.join("wasm_memory.c"));
    build.file(iwasm_common.join("wasm_loader_common.c"));
    build.file(iwasm_common.join("wasm_c_api.c"));

    // Architecture-specific invokeNative (generic C implementation)
    build.file(iwasm_common.join("arch/invokeNative_general.c"));

    // Common memory helpers (os_mremap fallback)
    build.file(shared_platform.join("common/memory/mremap.c"));

    // IWASM interpreter
    build.file(iwasm_interp.join("wasm_runtime.c"));
    build.file(iwasm_interp.join("wasm_mini_loader.c"));
    build.file(iwasm_interp.join("wasm_interp_classic.c"));

    // Compile
    build.compile("wamr");

    // Link
    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=wamr");
}
