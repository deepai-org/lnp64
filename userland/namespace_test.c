int main() {
    int fd;
    int buf;
    int cwd;

    buf = alloc(16);
    cwd = alloc(256);

    if (getcwd(cwd, 256) != cwd) return 1;
    fd = openat(AT_FDCWD, "/etc/passwd", 0);
    if (fd != -1) return 2;

    fd = openat(AT_FDCWD, "/etc/motd", 0);
    if (fd == -1) return 3;
    if (read(fd, buf, 1) != 1) return 4;
    if (loadb(buf) != 'w') return 5;
    close(fd);

    if (chdir("/tmp") != 0) return 6;
    if (getcwd(cwd, 256) != cwd) return 7;
    fd = openat(AT_FDCWD, "../etc/motd", 0);
    if (fd == -1) return 8;
    if (read(fd, buf, 1) != 1) return 9;
    if (loadb(buf) != 'w') return 10;
    close(fd);

    fd = openat(AT_FDCWD, "ns_probe", O_CREAT | O_TRUNC);
    if (fd == -1) return 11;
    if (write(fd, "ok", 2) != 2) return 12;
    close(fd);

    if (chdir("/") != 0) return 13;
    fd = openat(AT_FDCWD, "tmp/ns_probe", 0);
    if (fd == -1) return 14;
    if (read(fd, buf, 2) != 2) return 15;
    if (loadb(buf) != 'o') return 16;
    if (loadb(buf + 1) != 'k') return 17;
    close(fd);

    write(1, "namespace_test ok\n", 18);
    return 0;
}
