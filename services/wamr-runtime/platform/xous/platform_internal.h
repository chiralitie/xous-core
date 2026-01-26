/*
 * Copyright (C) 2026 Xous Project.  All rights reserved.
 * SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception
 */

#ifndef _PLATFORM_INTERNAL_H
#define _PLATFORM_INTERNAL_H

/* Use compiler's stdint.h */
#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdarg.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Standard macros */
#ifndef NULL
#define NULL                ((void *)0)
#endif

/* Xous doesn't have standard errno.h, define minimal needed values */
#ifndef ERANGE
#define ERANGE 34
#endif

#ifndef EINVAL
#define EINVAL 22
#endif

#ifndef EOVERFLOW
#define EOVERFLOW 75
#endif

#ifndef ENOSYS
#define ENOSYS 38
#endif

#ifndef ENOTSUP
#define ENOTSUP 95
#endif

/* Type definitions required by WAMR */
typedef uint32_t korp_tid;
typedef uint32_t korp_mutex;
typedef uint32_t korp_rwlock;
typedef uint32_t korp_sem;
typedef uint32_t korp_cond;

/* File handle type - Xous doesn't use file descriptors in the same way */
typedef int os_file_handle;
typedef int os_raw_file_handle;
typedef void* os_dir_stream;
typedef unsigned int os_nfds_t;

/* Poll file handle struct for compatibility */
typedef struct os_poll_file_handle {
    os_file_handle handle;
    short events;
    short revents;
} os_poll_file_handle;

static inline os_file_handle
os_get_invalid_handle(void)
{
    return -1;
}

/* Memory functions - will be linked from newlib */
void *memset(void *s, int c, size_t n);
void *memcpy(void *dest, const void *src, size_t n);
void *memmove(void *dest, const void *src, size_t n);
int memcmp(const void *s1, const void *s2, size_t n);
size_t strlen(const char *s);
char *strcpy(char *dest, const char *src);
char *strncpy(char *dest, const char *src, size_t n);
int strcmp(const char *s1, const char *s2);
int strncmp(const char *s1, const char *s2, size_t n);
char *strstr(const char *haystack, const char *needle);
char *strchr(const char *s, int c);

/* Memory allocation */
void *malloc(size_t size);
void *realloc(void *ptr, size_t size);
void *calloc(size_t nmemb, size_t size);
void free(void *ptr);
int posix_memalign(void **memptr, size_t alignment, size_t size);

/* Printf functions */
int printf(const char *format, ...);
int vprintf(const char *format, va_list ap);
int snprintf(char *str, size_t size, const char *format, ...);
int vsnprintf(char *str, size_t size, const char *format, va_list ap);

#ifdef __cplusplus
}
#endif

#endif /* end of _PLATFORM_INTERNAL_H */
