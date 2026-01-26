/* Minimal inttypes.h stub for bare-metal LVGL build */
#ifndef _INTTYPES_H
#define _INTTYPES_H

#include <stdint.h>

/* printf format macros */
#define PRId8   "d"
#define PRId16  "d"
#define PRId32  "ld"
#define PRId64  "lld"

#define PRIi8   "i"
#define PRIi16  "i"
#define PRIi32  "li"
#define PRIi64  "lli"

#define PRIu8   "u"
#define PRIu16  "u"
#define PRIu32  "lu"
#define PRIu64  "llu"

#define PRIx8   "x"
#define PRIx16  "x"
#define PRIx32  "lx"
#define PRIx64  "llx"

#define PRIX8   "X"
#define PRIX16  "X"
#define PRIX32  "lX"
#define PRIX64  "llX"

#define SCNd8   "hhd"
#define SCNd16  "hd"
#define SCNd32  "ld"
#define SCNd64  "lld"

#define SCNu8   "hhu"
#define SCNu16  "hu"
#define SCNu32  "lu"
#define SCNu64  "llu"

#endif /* _INTTYPES_H */
