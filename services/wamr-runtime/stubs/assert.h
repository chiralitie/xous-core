/* Minimal assert.h stub for bare-metal WAMR build */
#ifndef _ASSERT_H
#define _ASSERT_H

#ifdef NDEBUG
#define assert(x) ((void)0)
#else
void __assert_fail(const char *assertion, const char *file, unsigned int line, const char *function);
#define assert(x) ((x) ? (void)0 : __assert_fail(#x, __FILE__, __LINE__, __func__))
#endif

#endif /* _ASSERT_H */
