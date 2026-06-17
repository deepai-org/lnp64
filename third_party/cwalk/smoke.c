#include "include/cwalk.h"
#include <stdio.h>
#include <string.h>

int main(void) {
    char buffer[128];
    const char *base;
    size_t length;
    struct cwk_segment segment;

    cwk_path_set_style(CWK_STYLE_UNIX);
    if (!cwk_path_is_absolute("/tmp/archive.tar.gz")) return 1;
    if (!cwk_path_is_relative("tmp/archive.tar.gz")) return 2;

    cwk_path_get_basename("/tmp/archive.tar.gz", &base, &length);
    if (length != 14) return 3;
    if (strncmp(base, "archive.tar.gz", length) != 0) return 4;

    cwk_path_get_dirname("/tmp/archive.tar.gz", &length);
    if (length != 5) return 5;

    if (!cwk_path_get_first_segment("/tmp/archive.tar.gz", &segment)) return 6;
    if (segment.size != 3) return 7;
    if (strncmp(segment.begin, "tmp", segment.size) != 0) return 8;

    cwk_path_change_extension("/tmp/archive.tar.gz", ".zip", buffer, sizeof(buffer));
    if (strcmp(buffer, "/tmp/archive.tar.zip") != 0) return 9;

    puts("cwalk ok");
    return 0;
}
