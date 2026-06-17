#include "include/cwalk.h"
#include <stdio.h>
#include <string.h>

static int expect_path(const char *got, const char *want) {
    return strcmp(got, want) == 0;
}

int main(void) {
    char buffer[128];
    char small[6];
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

    written = cwk_path_join("/alpha", "beta/gamma", small, sizeof(small));
    if (written != 17) return 21;
    if (small[sizeof(small) - 1] != 0) return 22;
    if (strncmp(small, "/alph", sizeof(small) - 1) != 0) return 23;

    written = cwk_path_normalize("", buffer, sizeof(buffer));
    if (written != 0) return 24;
    if (!expect_path(buffer, "")) return 25;

    cwk_path_get_basename("/", &base, &length);
    if (length != 0) return 26;
    if (base != 0) return 27;
    if (cwk_path_get_first_segment("////", &segment)) return 28;

    written = cwk_path_normalize("///alpha////beta//", buffer, sizeof(buffer));
    if (written != 11) return 29;
    if (!expect_path(buffer, "/alpha/beta")) return 30;

    if (cwk_path_guess_style("C:\\temp\\file.txt") != CWK_STYLE_WINDOWS) return 31;
    cwk_path_set_style(CWK_STYLE_WINDOWS);
    if (!cwk_path_is_absolute("C:\\temp\\file.txt")) return 32;
    written = cwk_path_normalize("C:\\temp\\..\\out\\file.txt", buffer, sizeof(buffer));
    if (written != 15) return 33;
    if (!expect_path(buffer, "C:\\out\\file.txt")) return 34;

    written = cwk_path_normalize("C:/temp\\..//out\\file.txt", buffer, sizeof(buffer));
    if (written != 15) return 35;
    if (!expect_path(buffer, "C:\\out\\file.txt")) return 36;

    written = cwk_path_normalize("C:folder\\file.txt", buffer, sizeof(buffer));
    if (written != 17) return 37;
    if (!expect_path(buffer, "C:folder\\file.txt")) return 38;
    if (cwk_path_is_absolute("C:folder\\file.txt")) return 39;
    if (!cwk_path_is_relative("C:folder\\file.txt")) return 40;
    cwk_path_get_root("C:folder\\file.txt", &length);
    if (length != 2) return 41;

    written = cwk_path_normalize("\\\\server\\share\\dir\\..\\file.txt", buffer, sizeof(buffer));
    if (written != 23) return 42;
    if (!expect_path(buffer, "\\\\server\\share\\file.txt")) return 43;
    cwk_path_get_root("\\\\server\\share\\dir\\file.txt", &length);
    if (length != 15) return 44;

    written = cwk_path_get_relative("C:\\base\\root", "C:\\base\\target\\file", buffer, sizeof(buffer));
    if (written != 14) return 45;
    if (!expect_path(buffer, "..\\target\\file")) return 46;

    written = cwk_path_get_absolute("C:\\base\\root", "..\\target", buffer, sizeof(buffer));
    if (written != 14) return 47;
    if (!expect_path(buffer, "C:\\base\\target")) return 48;

    strcpy(buffer, "C:\\base\\old.txt");
    written = cwk_path_change_extension(buffer, ".log", buffer, sizeof(buffer));
    if (written != 15) return 49;
    if (!expect_path(buffer, "C:\\base\\old.log")) return 50;

    strcpy(buffer, "C:\\base\\old.txt");
    written = cwk_path_change_basename(buffer, "new.bin", buffer, sizeof(buffer));
    if (written != 15) return 51;
    if (!expect_path(buffer, "C:\\base\\new.bin")) return 52;

    strcpy(buffer, "C:\\base\\old.txt");
    if (!cwk_path_get_first_segment(buffer, &segment)) return 53;
    written = cwk_path_change_segment(&segment, "drive", buffer, sizeof(buffer));
    if (written != 16) return 54;
    if (!expect_path(buffer, "C:\\drive\\old.txt")) return 55;

    cwk_path_set_style(CWK_STYLE_UNIX);
    if (cwk_path_has_extension("archive")) return 56;
    if (!cwk_path_has_extension(".profile")) return 57;
    if (!cwk_path_get_extension(".profile", &base, &length)) return 58;
    if (length != 8) return 59;
    if (strncmp(base, ".profile", length) != 0) return 60;
    if (!cwk_path_get_extension("dir/file.", &base, &length)) return 61;
    if (length != 1) return 62;
    if (strncmp(base, ".", length) != 0) return 63;
    if (!cwk_path_get_extension("dir/.hidden.txt", &base, &length)) return 64;
    if (length != 4) return 65;
    if (strncmp(base, ".txt", length) != 0) return 66;

    written = cwk_path_change_extension("archive", "gz", buffer, sizeof(buffer));
    if (written != 10) return 67;
    if (!expect_path(buffer, "archive.gz")) return 68;
    written = cwk_path_change_extension("/", "root", buffer, sizeof(buffer));
    if (written != 6) return 69;
    if (!expect_path(buffer, "/.root")) return 70;
    written = cwk_path_change_extension("dir/file.", ".txt", buffer, sizeof(buffer));
    if (written != 12) return 71;
    if (!expect_path(buffer, "dir/file.txt")) return 72;

    written = cwk_path_normalize("../../alpha/../beta", buffer, sizeof(buffer));
    if (written != 10) return 73;
    if (!expect_path(buffer, "../../beta")) return 74;
    written = cwk_path_join("alpha", "../../beta", buffer, sizeof(buffer));
    if (written != 7) return 75;
    if (!expect_path(buffer, "../beta")) return 76;

    written = cwk_path_join("/alpha", "beta/gamma", small, 4);
    if (written != 17) return 77;
    if (small[0] != '/') return 78;
    if (small[1] != 'a') return 79;
    if (small[2] != 'l') return 80;
    if (small[3] != 0) return 81;
    small[0] = 'x';
    written = cwk_path_join("/alpha", "beta/gamma", small, 1);
    if (written != 17) return 82;
    if (small[0] != 0) return 83;

    strcpy(buffer, "/root/dir/file");
    if (!cwk_path_get_first_segment(buffer, &segment)) return 84;
    written = cwk_path_change_segment(&segment, "/trimmed/", buffer, sizeof(buffer));
    if (written != 17) return 85;
    if (!expect_path(buffer, "/trimmed/dir/file")) return 86;

    cwk_path_set_style(CWK_STYLE_WINDOWS);
    cwk_path_get_root("\\\\server\\share", &length);
    if (length != 14) return 87;
    if (cwk_path_is_absolute("\\\\server\\share")) return 88;
    if (!cwk_path_is_relative("\\\\server\\share")) return 89;
    cwk_path_get_root("\\\\server\\share\\", &length);
    if (length != 15) return 90;
    if (!cwk_path_is_absolute("\\\\server\\share\\")) return 91;
    cwk_path_get_root("\\\\server", &length);
    if (length != 8) return 92;
    if (cwk_path_is_absolute("\\\\server")) return 93;

    puts("cwalk ok");
    return 0;
}
