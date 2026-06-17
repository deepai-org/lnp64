#include "strnatcmp.h"

int main(void) {
    if (!(strnatcmp("rfc822.txt", "rfc2086.txt") < 0)) return 1;
    if (!(strnatcmp("a10", "a2") > 0)) return 2;
    if (strnatcmp("x2-y08", "x2-y7") >= 0) return 3;
    if (strnatcasecmp("File9", "file10") >= 0) return 4;
    puts("natsort ok");
    return 0;
}
