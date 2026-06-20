#ifndef LNP64_STDINT_H
#define LNP64_STDINT_H

typedef signed char int8_t;
typedef unsigned char uint8_t;
typedef short int16_t;
typedef unsigned short uint16_t;
typedef int int32_t;
typedef unsigned int uint32_t;
typedef long int64_t;
typedef unsigned long uint64_t;

/* exact-width aliases for pointer-sized types */
typedef long intptr_t;
typedef unsigned long uintptr_t;
typedef long intmax_t;
typedef unsigned long uintmax_t;

/* least-width types — LNP64 has no narrower native types, so all are 64-bit */
typedef signed char    int_least8_t;
typedef unsigned char  uint_least8_t;
typedef short          int_least16_t;
typedef unsigned short uint_least16_t;
typedef int            int_least32_t;
typedef unsigned int   uint_least32_t;
typedef long           int_least64_t;
typedef unsigned long  uint_least64_t;

/* fast types */
typedef long           int_fast8_t;
typedef unsigned long  uint_fast8_t;
typedef long           int_fast16_t;
typedef unsigned long  uint_fast16_t;
typedef long           int_fast32_t;
typedef unsigned long  uint_fast32_t;
typedef long           int_fast64_t;
typedef unsigned long  uint_fast64_t;

#define INT8_MIN (-128)
#define INT8_MAX 127
#define UINT8_MAX 255U
#define INT16_MIN (-32768)
#define INT16_MAX 32767
#define UINT16_MAX 65535U
#define INT32_MIN (-2147483647 - 1)
#define INT32_MAX 2147483647
#define UINT32_MAX 4294967295U
#define INT64_MIN (-9223372036854775807L - 1L)
#define INT64_MAX 9223372036854775807L
#define UINT64_MAX 18446744073709551615UL

#define INTPTR_MIN INT64_MIN
#define INTPTR_MAX INT64_MAX
#define UINTPTR_MAX UINT64_MAX
#define INTMAX_MIN INT64_MIN
#define INTMAX_MAX INT64_MAX
#define UINTMAX_MAX UINT64_MAX
#define SIZE_MAX UINT64_MAX

/* C99 integer constant macros */
#define INT8_C(v)   ((int8_t)(v))
#define UINT8_C(v)  ((uint8_t)(v))
#define INT16_C(v)  ((int16_t)(v))
#define UINT16_C(v) ((uint16_t)(v))
#define INT32_C(v)  ((int32_t)(v))
#define UINT32_C(v) ((uint32_t)(v ## U))
#define INT64_C(v)  ((int64_t)(v ## L))
#define UINT64_C(v) ((uint64_t)(v ## UL))
#define INTMAX_C(v)  INT64_C(v)
#define UINTMAX_C(v) UINT64_C(v)

#endif
