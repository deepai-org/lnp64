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
    int path;
    int fd;
    int buf;
    int n;

    if (argc > 1) {
        root = argv[1];
    } else {
        root = "/tmp/lnp64-userland-root";
    }

    write(1, "lnp64 init: boot\n", 17);
    write(1, "lnp64 init: root ", 17);
    write_cstr(1, root);
    write(1, "\n", 1);

    path = alloc(256);
    buf = alloc(256);

    join_path(path, root, "/etc/motd");
    fd = open(3, path, 0);
    if (fd != -1) {
        n = read(3, buf, 255);
        if (n > 0) {
            write(1, buf, n);
        }
        close(3);
    }

    join_path(path, root, "/bin/lnpsh.s");
    execl(path, "lnpsh", root, 0);

    write(2, "lnp64 init: exec failed\n", 24);
    return 1;
}
