/* Stub implementations for bare-metal LVGL on Xous */

#include <stddef.h>
#include <stdarg.h>
#include <stdint.h>

/* GCC builtins for bit operations */
int __ffssi2(int a) {
    if (a == 0) return 0;
    int i;
    for (i = 1; (a & 1) == 0; i++) {
        a >>= 1;
    }
    return i;
}

/* Errno */
int errno = 0;

/* Memory allocation - use small heap for LVGL */
static unsigned char lvgl_heap[64 * 1024]; /* 64KB heap for LVGL */
static size_t heap_offset = 0;

void *malloc(size_t size) {
    if (size == 0) return NULL;
    size = (size + 7) & ~7;  /* Align to 8 bytes */
    if (heap_offset + size > sizeof(lvgl_heap)) {
        return NULL;
    }
    void *ptr = &lvgl_heap[heap_offset];
    heap_offset += size;
    return ptr;
}

void free(void *ptr) {
    (void)ptr;  /* Simple bump allocator doesn't support free */
}

void *calloc(size_t nmemb, size_t size) {
    size_t total = nmemb * size;
    void *ptr = malloc(total);
    if (ptr) {
        for (size_t i = 0; i < total; i++) {
            ((unsigned char*)ptr)[i] = 0;
        }
    }
    return ptr;
}

void *realloc(void *ptr, size_t size) {
    if (ptr == NULL) return malloc(size);
    if (size == 0) {
        free(ptr);
        return NULL;
    }
    void *new_ptr = malloc(size);
    return new_ptr;
}

/* String functions */
int strcmp(const char *s1, const char *s2) {
    while (*s1 && (*s1 == *s2)) {
        s1++;
        s2++;
    }
    return *(unsigned char *)s1 - *(unsigned char *)s2;
}

size_t strlen(const char *s) {
    size_t len = 0;
    while (*s++) len++;
    return len;
}

void *memset(void *s, int c, size_t n) {
    unsigned char *p = s;
    while (n--) *p++ = (unsigned char)c;
    return s;
}

void *memcpy(void *dest, const void *src, size_t n) {
    unsigned char *d = dest;
    const unsigned char *s = src;
    while (n--) *d++ = *s++;
    return dest;
}

int memcmp(const void *s1, const void *s2, size_t n) {
    const unsigned char *p1 = s1;
    const unsigned char *p2 = s2;
    while (n--) {
        if (*p1 != *p2) return *p1 - *p2;
        p1++;
        p2++;
    }
    return 0;
}

/* Printf family - minimal implementation */
int vsnprintf(char *str, size_t size, const char *format, va_list ap) {
    size_t i = 0;
    (void)ap;
    if (size == 0) return 0;
    while (*format && i < size - 1) {
        str[i++] = *format++;
    }
    str[i] = '\0';
    return i;
}

int snprintf(char *str, size_t size, const char *format, ...) {
    va_list ap;
    va_start(ap, format);
    int ret = vsnprintf(str, size, format, ap);
    va_end(ap);
    return ret;
}

int vprintf(const char *format, va_list ap) {
    char buf[256];
    vsnprintf(buf, sizeof(buf), format, ap);
    return 0;
}

int printf(const char *format, ...) {
    va_list ap;
    va_start(ap, format);
    int ret = vprintf(format, ap);
    va_end(ap);
    return ret;
}
