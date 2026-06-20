#ifndef LNP64_ENDIAN_H
#define LNP64_ENDIAN_H

#define __LITTLE_ENDIAN 1234
#define __BIG_ENDIAN    4321
#define __PDP_ENDIAN    3412

#define LITTLE_ENDIAN __LITTLE_ENDIAN
#define BIG_ENDIAN    __BIG_ENDIAN
#define PDP_ENDIAN    __PDP_ENDIAN

/* LNP64 is little-endian */
#define __BYTE_ORDER __LITTLE_ENDIAN
#define BYTE_ORDER    LITTLE_ENDIAN

#endif
