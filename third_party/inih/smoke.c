#include "ini.h"

int seen;

int handler(void *user, const char *section, const char *name, const char *value) {
    if (strcmp(section, "server") == 0 && strcmp(name, "host") == 0 && strcmp(value, "localhost") == 0) {
        seen = seen + 1;
    }
    if (strcmp(section, "server") == 0 && strcmp(name, "port") == 0 && strcmp(value, "41066") == 0) {
        seen = seen + 2;
    }
    if (strcmp(section, "feature") == 0 && strcmp(name, "enabled") == 0 && strcmp(value, "yes") == 0) {
        seen = seen + 4;
    }
    return 1;
}

int main() {
    int rc;
    seen = 0;
    rc = ini_parse("third_party/inih/smoke.ini", handler, 0);
    if (rc != 0) {
        return 1;
    }
    if (seen != 7) {
        return 2;
    }
    seen = 0;
    rc = ini_parse_string("[server]\nhost=localhost\nport=41066\n[feature]\nenabled=yes\n", handler, 0);
    if (rc != 0) {
        return 3;
    }
    if (seen != 7) {
        return 4;
    }
    write(1, "inih ok\n", 8);
    return 0;
}
