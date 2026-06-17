#include "include/cwalk.h"
#include <stdio.h>
#include <string.h>

static int expect_path(const char *got, const char *want) {
    return strcmp(got, want) == 0;
}

int main(void) {
    char buffer[128];
    const char *base;
    const char *paths[4];
    size_t length;
    size_t written;
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

    written = cwk_path_normalize("/var//log/../tmp/./cache/", buffer, sizeof(buffer));
    if (written != 14) return 10;
    if (!expect_path(buffer, "/var/tmp/cache")) return 11;

    written = cwk_path_join("/var/log", "../tmp/app.log", buffer, sizeof(buffer));
    if (written != 16) return 12;
    if (!expect_path(buffer, "/var/tmp/app.log")) return 13;

    paths[0] = "/srv";
    paths[1] = "./www";
    paths[2] = "assets/../index.html";
    paths[3] = 0;
    written = cwk_path_join_multiple(paths, buffer, sizeof(buffer));
    if (written != 19) return 14;
    if (!expect_path(buffer, "/srv/www/index.html")) return 15;

    written = cwk_path_get_absolute("/srv/www/root", "../etc/config", buffer, sizeof(buffer));
    if (written != 19) return 16;
    if (!expect_path(buffer, "/srv/www/etc/config")) return 17;

    written = cwk_path_get_relative("/srv/www/root", "/srv/www/assets/site.css", buffer, sizeof(buffer));
    if (written != 18) return 18;
    if (!expect_path(buffer, "../assets/site.css")) return 19;

    written = cwk_path_get_intersection("/srv/www/root/file", "/srv/www/root/assets/app.css");
    if (written != 13) return 20;

    if (cwk_path_guess_style("C:\\temp\\file.txt") != CWK_STYLE_WINDOWS) return 21;
    cwk_path_set_style(CWK_STYLE_WINDOWS);
    if (!cwk_path_is_absolute("C:\\temp\\file.txt")) return 22;
    written = cwk_path_normalize("C:\\temp\\..\\out\\file.txt", buffer, sizeof(buffer));
    if (written != 15) return 23;
    if (!expect_path(buffer, "C:\\out\\file.txt")) return 24;

    puts("cwalk ok");
    return 0;
}
