#include "zlib.h"

int main(void) {
    const Bytef data[] = "hello zlib";
    uLong adler = adler32(0L, Z_NULL, 0);
    adler = adler32(adler, data, 10);
    if (adler != 0x159503e6UL) {
        return 1;
    }
    puts("zlib adler32 ok");
    return 0;
}
