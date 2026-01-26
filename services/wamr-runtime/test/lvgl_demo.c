/**
 * LVGL Demo Application for WebAssembly
 * Creates a simple UI with a label and button
 */

#include "lvgl.h"

// Entry point for the WASM module
__attribute__((export_name("main")))
int main(void) {
    print("WASM: Starting LVGL demo");

    // Get the screen
    unsigned int screen = lvgl_get_screen();
    print("WASM: Got screen handle");

    // Create a label
    unsigned int label = lvgl_create_label(screen);
    if (label == 0) {
        print("WASM: Failed to create label");
        return -1;
    }
    print("WASM: Created label");

    // Set label text
    lvgl_set_text(label, "Hello from WASM!");
    print("WASM: Set label text");

    // Align label to center
    lvgl_align(label, LV_ALIGN_CENTER, 0, -30);
    print("WASM: Aligned label");

    // Create a button
    unsigned int button = lvgl_create_button(screen);
    if (button == 0) {
        print("WASM: Failed to create button");
        return -1;
    }
    print("WASM: Created button");

    // Set button size
    lvgl_set_size(button, 100, 40);
    print("WASM: Set button size");

    // Align button below label
    lvgl_align(button, LV_ALIGN_CENTER, 0, 20);
    print("WASM: Aligned button");

    // Create label for button text
    unsigned int btn_label = lvgl_create_label(button);
    if (btn_label != 0) {
        lvgl_set_text(btn_label, "Click");
        lvgl_align(btn_label, LV_ALIGN_CENTER, 0, 0);
        print("WASM: Created button label");
    }

    print("WASM: LVGL demo complete!");
    return 0;
}
