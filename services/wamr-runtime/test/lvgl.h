/**
 * LVGL bindings for WebAssembly
 * These functions are imported from the host (Xous)
 */

#ifndef LVGL_H
#define LVGL_H

#ifdef __cplusplus
extern "C" {
#endif

// Alignment constants
#define LV_ALIGN_CENTER 0
#define LV_ALIGN_TOP_LEFT 1
#define LV_ALIGN_TOP_MID 2
#define LV_ALIGN_TOP_RIGHT 3
#define LV_ALIGN_BOTTOM_LEFT 4
#define LV_ALIGN_BOTTOM_MID 5
#define LV_ALIGN_BOTTOM_RIGHT 6

// Import declarations (these are provided by the host)
__attribute__((import_module("env"))) __attribute__((import_name("lvgl_get_screen")))
unsigned int lvgl_get_screen(void);

__attribute__((import_module("env"))) __attribute__((import_name("lvgl_create_label")))
unsigned int lvgl_create_label(unsigned int parent);

__attribute__((import_module("env"))) __attribute__((import_name("lvgl_set_text")))
int lvgl_set_text(unsigned int handle, const char* text);

__attribute__((import_module("env"))) __attribute__((import_name("lvgl_align")))
int lvgl_align(unsigned int handle, int align, int x_ofs, int y_ofs);

__attribute__((import_module("env"))) __attribute__((import_name("lvgl_create_button")))
unsigned int lvgl_create_button(unsigned int parent);

__attribute__((import_module("env"))) __attribute__((import_name("lvgl_set_size")))
int lvgl_set_size(unsigned int handle, int width, int height);

__attribute__((import_module("env"))) __attribute__((import_name("print")))
void print(const char* msg);

#ifdef __cplusplus
}
#endif

#endif /* LVGL_H */
