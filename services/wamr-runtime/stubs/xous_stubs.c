/* Stub implementations for bare-metal WAMR on Xous */

#include <stddef.h>
#include <stdarg.h>
#include <stdint.h>

/* Errno */
int errno = 0;

/* Memory allocation stubs - use simple bump allocator for now */
static unsigned char heap_buf[256 * 1024]; /* 256KB heap */
static size_t heap_offset = 0;

void *malloc(size_t size) {
    if (size == 0) return NULL;

    /* Align to 8 bytes */
    size = (size + 7) & ~7;

    if (heap_offset + size > sizeof(heap_buf)) {
        return NULL; /* Out of memory */
    }

    void *ptr = &heap_buf[heap_offset];
    heap_offset += size;
    return ptr;
}

void free(void *ptr) {
    /* Simple bump allocator doesn't support free */
    (void)ptr;
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
    /* Simple implementation: allocate new, copy old, ignore free */
    if (ptr == NULL) {
        return malloc(size);
    }

    if (size == 0) {
        free(ptr);
        return NULL;
    }

    void *new_ptr = malloc(size);
    if (new_ptr && ptr) {
        /* Copy old data - we don't know the old size, so this is unsafe */
        /* For now, just return the new allocation */
    }
    return new_ptr;
}

int posix_memalign(void **memptr, size_t alignment, size_t size) {
    /* Align heap offset to requested alignment */
    heap_offset = (heap_offset + alignment - 1) & ~(alignment - 1);
    *memptr = malloc(size);
    return (*memptr == NULL) ? 12 : 0; /* 12 = ENOMEM */
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
        if (*p1 != *p2) {
            return *p1 - *p2;
        }
        p1++;
        p2++;
    }
    return 0;
}

/* Quick sort - simple implementation */
static void swap(void *a, void *b, size_t size) {
    unsigned char *pa = a;
    unsigned char *pb = b;
    while (size--) {
        unsigned char tmp = *pa;
        *pa++ = *pb;
        *pb++ = tmp;
    }
}

void qsort(void *base, size_t nmemb, size_t size,
           int (*compar)(const void *, const void *)) {
    /* Simple bubble sort for small arrays */
    if (nmemb <= 1) return;

    unsigned char *arr = base;
    for (size_t i = 0; i < nmemb - 1; i++) {
        for (size_t j = 0; j < nmemb - i - 1; j++) {
            void *p1 = arr + j * size;
            void *p2 = arr + (j + 1) * size;
            if (compar(p1, p2) > 0) {
                swap(p1, p2, size);
            }
        }
    }
}

/* Printf family - minimal implementation */
int vsnprintf(char *str, size_t size, const char *format, va_list ap) {
    /* Very minimal implementation - just copy format string */
    size_t i = 0;
    (void)ap; /* Ignore args for now */

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
    /* Could send to Xous log here */
    return 0;
}

int printf(const char *format, ...) {
    va_list ap;
    va_start(ap, format);
    int ret = vprintf(format, ap);
    va_end(ap);
    return ret;
}

/* Assert - abort is provided by Xous */
extern void abort(void);

void __assert_fail(const char *assertion, const char *file,
                  unsigned int line, const char *function) {
    /* Could send to Xous log */
    (void)assertion;
    (void)file;
    (void)line;
    (void)function;
    abort();
}

/* Math functions - use compiler builtins */
double fabs(double x) { return __builtin_fabs(x); }
float fabsf(float x) { return __builtin_fabsf(x); }
double floor(double x) { return __builtin_floor(x); }
float floorf(float x) { return __builtin_floorf(x); }
double ceil(double x) { return __builtin_ceil(x); }
float ceilf(float x) { return __builtin_ceilf(x); }
double sqrt(double x) { return __builtin_sqrt(x); }
float sqrtf(float x) { return __builtin_sqrtf(x); }
double trunc(double x) { return __builtin_trunc(x); }
float truncf(float x) { return __builtin_truncf(x); }
double round(double x) { return __builtin_round(x); }
float roundf(float x) { return __builtin_roundf(x); }
double rint(double x) { return __builtin_rint(x); }
float rintf(float x) { return __builtin_rintf(x); }
double fmin(double x, double y) { return __builtin_fmin(x, y); }
float fminf(float x, float y) { return __builtin_fminf(x, y); }
double fmax(double x, double y) { return __builtin_fmax(x, y); }
float fmaxf(float x, float y) { return __builtin_fmaxf(x, y); }
double copysign(double x, double y) { return __builtin_copysign(x, y); }
float copysignf(float x, float y) { return __builtin_copysignf(x, y); }
/* signbit for float and double */
int __signbitf(float x) { return __builtin_signbitf(x); }
int __signbitd(double x) { return __builtin_signbit(x); }

/* signbit function - called directly by WAMR */
int signbit(double x) { return __builtin_signbit(x); }

/* isnan function - used by WAMR floating point operations */
int isnan(double x) { return __builtin_isnan(x); }
int isnanf(float x) { return __builtin_isnanf(x); }

/* isinf function */
int isinf(double x) { return __builtin_isinf(x); }
int isinff(float x) { return __builtin_isinff(x); }

/* Binary search */
void *bsearch(const void *key, const void *base, size_t nmemb, size_t size,
              int (*compar)(const void *, const void *)) {
    const unsigned char *arr = base;

    while (nmemb > 0) {
        size_t mid = nmemb / 2;
        const void *midp = arr + mid * size;
        int cmp = compar(key, midp);

        if (cmp == 0) {
            return (void *)midp;
        } else if (cmp < 0) {
            nmemb = mid;
        } else {
            arr = (const unsigned char *)midp + size;
            nmemb = nmemb - mid - 1;
        }
    }

    return NULL;
}

/* WAMR platform functions */
int os_getpagesize(void) {
    return 4096; /* Standard page size */
}

/* invokeNative - generic function call, args passed as array of uint32 */
/* For interpreter-only mode, this is typically called when native functions are invoked */
typedef uint32_t (*native_func_void)(void);
typedef uint32_t (*native_func_1)(uint32_t);
typedef uint32_t (*native_func_2)(uint32_t, uint32_t);
typedef uint32_t (*native_func_3)(uint32_t, uint32_t, uint32_t);
typedef uint32_t (*native_func_4)(uint32_t, uint32_t, uint32_t, uint32_t);

void invokeNative(void (*func)(void), uint32_t *args, uint32_t sz, void *return_val) {
    /* Simple implementation for up to 4 args */
    uint32_t result = 0;
    uint32_t argc = sz / sizeof(uint32_t);

    switch (argc) {
        case 0:
            result = ((native_func_void)func)();
            break;
        case 1:
            result = ((native_func_1)func)(args[0]);
            break;
        case 2:
            result = ((native_func_2)func)(args[0], args[1]);
            break;
        case 3:
            result = ((native_func_3)func)(args[0], args[1], args[2]);
            break;
        case 4:
            result = ((native_func_4)func)(args[0], args[1], args[2], args[3]);
            break;
        default:
            /* Unsupported number of args */
            break;
    }

    if (return_val) {
        *(uint32_t*)return_val = result;
    }
}

void wasm_trap_delete(void *trap) {
    /* Stub for C-API trap handling */
    (void)trap;
}
