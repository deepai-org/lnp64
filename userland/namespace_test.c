int main() {
    int fd;
    int dir;
    int buf;
    int cwd;
    int st;

    buf = alloc(16);
    cwd = alloc(256);
    st = alloc(128);

    if (getcwd(cwd, 256) != cwd) return 1;
    fd = openat(AT_FDCWD, "/etc/passwd", 0);
    if (fd != -1) return 2;

    fd = openat(AT_FDCWD, "/etc/motd", 0);
    if (fd == -1) return 3;
    if (read(fd, buf, 1) != 1) return 4;
    if (loadb(buf) != 'w') return 5;
    close(fd);
    dir = opendir("/etc");
    if (dir == 0) return 27;
    fd = openat(dir, "motd", 0);
    if (fd == -1) return 28;
    if (read(fd, buf, 1) != 1) return 29;
    if (loadb(buf) != 'w') return 30;
    close(fd);
    if (fstatat(dir, "motd", st, 0) != 0) return 32;
    if (load(st + 8) == 0) return 33;
    if (fstatat(dir, "../../etc/passwd", st, 0) != -1) return 34;
    fd = openat(dir, "../../etc/passwd", 0);
    if (fd != -1) return 31;
    closedir(dir);

    if (chdir("/tmp") != 0) return 6;
    if (getcwd(cwd, 256) != cwd) return 7;
    fd = openat(AT_FDCWD, "../etc/motd", 0);
    if (fd == -1) return 8;
    if (read(fd, buf, 1) != 1) return 9;
    if (loadb(buf) != 'w') return 10;
    close(fd);

    fd = openat(AT_FDCWD, "../../etc/passwd", 0);
    if (fd != -1) return 11;

    if (symlink("/etc/passwd", "host_passwd_link") != 0) return 12;
    fd = openat(AT_FDCWD, "host_passwd_link", 0);
    if (fd != -1) return 13;
    if (readlink("host_passwd_link", buf, 16) != 11) return 14;
    if (loadb(buf) != '/') return 15;
    if (loadb(buf + 5) != 'p') return 16;

    if (symlink("/tmp", "host_tmp_link") != 0) return 17;
    fd = openat(AT_FDCWD, "host_tmp_link/lnp64_ns_escape_probe", O_CREAT | O_TRUNC);
    if (fd != -1) return 18;
    if (chdir("host_tmp_link") == 0) return 19;

    fd = openat(AT_FDCWD, "ns_probe", O_CREAT | O_TRUNC);
    if (fd == -1) return 20;
    if (write(fd, "ok", 2) != 2) return 21;
    close(fd);
    dir = opendir("/tmp");
    if (dir == 0) return 35;
    if (fchmodat(dir, "ns_probe", 384, 0) != 0) return 36;
    if (fstatat(dir, "ns_probe", st, 0) != 0) return 37;
    if ((load(st) & 511) != 384) return 38;
    if (fchmodat(dir, "../../etc/motd", 384, 0) != -1) return 39;
    fd = openat(dir, "unlink_probe", O_CREAT | O_TRUNC);
    if (fd == -1) return 40;
    close(fd);
    if (unlinkat(dir, "unlink_probe", 0) != 0) return 41;
    if (fstatat(dir, "unlink_probe", st, 0) != -1) return 42;
    if (unlinkat(dir, "../../etc/motd", 0) != -1) return 43;
    closedir(dir);

    if (chdir("/") != 0) return 22;
    fd = openat(AT_FDCWD, "tmp/ns_probe", 0);
    if (fd == -1) return 23;
    if (read(fd, buf, 2) != 2) return 24;
    if (loadb(buf) != 'o') return 25;
    if (loadb(buf + 1) != 'k') return 26;
    close(fd);

    write(1, "namespace_test ok\n", 18);
    return 0;
}
