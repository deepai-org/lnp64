#include "zlib.h"

int main(void) {
    const Bytef data[] = "hello zlib";
    uLong adler = adler32(0L, Z_NULL, 0);
    uLong crc = crc32(0L, Z_NULL, 0);
    adler = adler32(adler, data, 10);
    crc = crc32(crc, data, 10);
    if (adler != 0x159503e6UL) {
        return 1;
    }
    if (crc != 0x96b34bd1UL) {
        return 2;
    }
    puts("zlib checksum ok");
    return 0;
}
