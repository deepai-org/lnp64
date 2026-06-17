int join_path(int dst, int root, int suffix) {
    strcpy(dst, root);
    strcat(dst, suffix);
    return dst;
}

int write_cstr(int fd, int text) {
    int n;
    n = strlen(text);
    write(fd, text, n);
    return n;
}

int main(int argc, char **argv) {
    int root;
    int suffix;
    int path;
    int buf;
    int n;

    if (argc > 1) {
        root = argv[1];
    } else {
        root = "/tmp/lnp64-userland-root";
    }
    if (argc > 2) {
        suffix = argv[2];
    } else {
        suffix = "/etc/motd";
    }

    path = alloc(256);
    buf = alloc(512);
    join_path(path, root, suffix);
    if (open(3, path, 0) == -1) {
        write(1, "ucat: missing ", 14);
        write_cstr(1, suffix);
        write(1, "\n", 1);
        return 1;
    }
    n = read(3, buf, 511);
    while (n > 0) {
        write(1, buf, n);
        n = read(3, buf, 511);
    }
    close(3);
    return 0;
}
