/*
 * Copyright (C) 2026 Xous Project.  All rights reserved.
 * SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception
 */

#include "platform_api_vmcore.h"
#include "platform_api_extension.h"

int
bh_platform_init(void)
{
    return 0;
}

void
bh_platform_destroy(void)
{
}

void *
os_malloc(unsigned size)
{
    return malloc(size);
}

void *
os_realloc(void *ptr, unsigned size)
{
    return realloc(ptr, size);
}

void
os_free(void *ptr)
{
    free(ptr);
}

int
os_printf(const char *format, ...)
{
    int ret = 0;
    va_list args;
    va_start(args, format);
    ret = vprintf(format, args);
    va_end(args);
    return ret;
}

int
os_vprintf(const char *format, va_list ap)
{
    return vprintf(format, ap);
}

uint64
os_time_get_boot_us(void)
{
    /* TODO: Implement using Xous ticktimer */
    return 0;
}

uint64
os_time_thread_cputime_us(void)
{
    /* TODO: Implement using Xous ticktimer */
    return 0;
}

korp_tid
os_self_thread(void)
{
    /* Return current process ID */
    return 0;
}

uint8 *
os_thread_get_stack_boundary(void)
{
    /* Return NULL for now - TODO: implement stack boundary detection */
    return NULL;
}

void
os_thread_jit_write_protect_np(bool enabled)
{
    /* Not needed for interpreter-only mode */
    (void)enabled;
}

int
os_mutex_init(korp_mutex *mutex)
{
    /* TODO: Implement with Xous primitives if needed */
    *mutex = 0;
    return 0;
}

int
os_mutex_destroy(korp_mutex *mutex)
{
    /* TODO: Implement with Xous primitives if needed */
    (void)mutex;
    return 0;
}

int
os_mutex_lock(korp_mutex *mutex)
{
    /* TODO: Implement with Xous primitives if needed */
    (void)mutex;
    return 0;
}

int
os_mutex_unlock(korp_mutex *mutex)
{
    /* TODO: Implement with Xous primitives if needed */
    (void)mutex;
    return 0;
}

void *
os_mmap(void *hint, size_t size, int prot, int flags, os_file_handle file)
{
    void *p;
    (void)hint;
    (void)prot;
    (void)flags;
    (void)file;

    /* Use aligned allocation for WASM memory */
    if (posix_memalign(&p, 32, size)) {
        return NULL;
    }

    /* Zero the memory as required by os_mmap */
    memset(p, 0, size);
    return p;
}

void
os_munmap(void *addr, size_t size)
{
    (void)size;
    free(addr);
}

int
os_mprotect(void *addr, size_t size, int prot)
{
    /* Xous doesn't have mprotect - return success */
    (void)addr;
    (void)size;
    (void)prot;
    return 0;
}

void *
os_mremap(void *old_addr, size_t old_size, size_t new_size)
{
    /* Use slow path */
    return os_mremap_slow(old_addr, old_size, new_size);
}

void
os_dcache_flush(void)
{
    /* TODO: Implement if needed for RISC-V cache coherency */
}

void
os_icache_flush(void *start, size_t len)
{
    /* TODO: Implement if needed for RISC-V cache coherency */
    (void)start;
    (void)len;
}

int
os_dumps_proc_mem_info(char *out, unsigned int size)
{
    (void)out;
    (void)size;
    return -1;
}
