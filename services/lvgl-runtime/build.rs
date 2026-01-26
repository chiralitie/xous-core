use std::env;
use std::path::PathBuf;
use std::fs;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let lvgl_dir = PathBuf::from("../lvgl-lib");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=lv_conf.h");
    println!("cargo:rerun-if-changed=../lvgl-lib/");

    // Check if we're building for Xous (RISC-V)
    let target = env::var("TARGET").unwrap_or_default();
    let is_xous = target.contains("xous") || target.contains("riscv32");

    let mut build = cc::Build::new();

    if is_xous {
        // Use riscv64-unknown-elf-gcc with 32-bit flags
        build.compiler("riscv64-unknown-elf-gcc");
        build.archiver("riscv64-unknown-elf-ar");

        // Include stubs for bare-metal
        build.include("stubs");

        // Add C library stubs
        build.file("stubs/xous_stubs.c");
    } else {
        // Use native compiler for hosted builds
        println!("cargo:warning=LVGL: Building for hosted Linux target");
    }

    // Include paths
    build.include(".");  // for lv_conf.h
    build.include(&lvgl_dir);
    build.include(lvgl_dir.join("src"));

    // Compiler flags
    build.flag("-fno-exceptions");
    build.flag("-ffunction-sections");
    build.flag("-fdata-sections");
    build.flag("-Wall");
    build.flag("-Wno-unused-function");
    build.flag("-Wno-unused-variable");
    build.flag("-Wno-unused-but-set-variable");

    // LVGL configuration
    build.define("LV_CONF_INCLUDE_SIMPLE", None);

    // Find all .c files in lvgl/src recursively, excluding examples and demos
    let src_dir = lvgl_dir.join("src");
    collect_c_files(&mut build, &src_dir);

    // Also add lv_init.c from root
    build.file(lvgl_dir.join("src/lv_init.c"));

    // Compile
    build.compile("lvgl");

    // Link
    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=lvgl");
}

fn collect_c_files(build: &mut cc::Build, dir: &PathBuf) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Skip example/demo/test directories
                let dir_name = path.file_name().unwrap().to_str().unwrap();
                if !dir_name.contains("examples") && !dir_name.contains("demos") && !dir_name.contains("tests") {
                    collect_c_files(build, &path);
                }
            } else if path.extension().and_then(|s| s.to_str()) == Some("c") {
                // Skip files we don't need
                let file_name = path.file_name().unwrap().to_str().unwrap();
                if !file_name.contains("example") && !file_name.contains("demo") && !file_name.contains("test") {
                    build.file(&path);
                }
            }
        }
    }
}
