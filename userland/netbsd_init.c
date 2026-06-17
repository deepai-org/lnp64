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

    if (argc > 1) {
        root = argv[1];
    } else {
        root = "/tmp/lnp64-netbsd-personality-root";
    }

    write(1, "lnp64-netbsd-personality: supervisor boot\n", 42);
    write(1, "lnp64-netbsd-personality: root ", 31);
    write_cstr(1, root);
    write(1, "\n", 1);

    path = alloc(256);
    join_path(path, root, "/bin/netbsd_sh.s");
    execl(path, "netbsd_sh", root, 0);

    write(2, "netbsd init: exec failed\n", 25);
    return 1;
}
