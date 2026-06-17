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

int cat_file(int root, int suffix) {
    int path;
    int buf;
    int n;

    path = alloc(256);
    buf = alloc(512);
    join_path(path, root, suffix);
    if (open(3, path, 0) == -1) {
        write(1, "missing ", 8);
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
    free(path);
    free(buf);
    return 0;
}

int command(int root, int name, int arg) {
    write(1, "$ ", 2);
    write_cstr(1, name);
    if (arg != 0) {
        write(1, " ", 1);
        write_cstr(1, arg);
    }
    write(1, "\n", 1);

    if (strcmp(name, "pwd") == 0) {
        write(1, "/\n", 2);
        return 0;
    }
    if (strcmp(name, "ls") == 0) {
        return cat_file(root, "/etc/files");
    }
    if (strcmp(name, "cat") == 0) {
        return cat_file(root, arg);
    }
    if (strcmp(name, "devs") == 0) {
        return cat_file(root, "/dev/devices");
    }
    write(1, "unknown command\n", 16);
    return 1;
}

int main(int argc, char **argv) {
    int root;
    if (argc > 1) {
        root = argv[1];
    } else {
        root = "/tmp/lnp64-userland-root";
    }

    write(1, "lnpsh: scripted console\n", 24);
    command(root, "pwd", 0);
    command(root, "ls", 0);
    command(root, "cat", "/etc/motd");
    command(root, "devs", 0);
    write(1, "lnpsh: halt\n", 12);
    return 0;
}
