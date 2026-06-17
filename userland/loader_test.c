int nul_terminate_line(int buf, int start, int limit) {
    int i;
    i = start;
    while (i < limit) {
        if (loadb(buf + i) == 10) {
            storeb(buf + i, 0);
            return 0;
        }
        i = i + 1;
    }
    return -1;
}

int check_magic(int buf) {
    if (loadb(buf) != 'L') return 1;
    if (loadb(buf + 1) != 'N') return 2;
    if (loadb(buf + 2) != 'P') return 3;
    if (loadb(buf + 3) != '6') return 4;
    if (loadb(buf + 4) != '4') return 5;
    if (loadb(buf + 5) != 'E') return 6;
    if (loadb(buf + 6) != 'X') return 7;
    if (loadb(buf + 7) != 'E') return 8;
    if (loadb(buf + 8) != 'C') return 9;
    if (loadb(buf + 9) != '1') return 10;
    if (loadb(buf + 10) != 10) return 11;
    return 0;
}

int main() {
    int fd;
    int buf;
    int n;
    int child;
    int status;
    int path;

    buf = alloc(128);
    fd = openat(AT_FDCWD, "/etc/loader_target.execplan", 0);
    if (fd == -1) return 1;
    n = read(fd, buf, 128);
    close(fd);
    if (n < 12) return 2;
    if (check_magic(buf) != 0) return 3;
    if (nul_terminate_line(buf, 11, n) != 0) return 4;

    path = buf + 11;
    child = fork();
    if (child == 0) {
        execl(path, "loader_target", 0);
        _exit(127);
    }
    if (wait(&status) != 0) return 5;
    if (!WIFEXITED(status)) return 6;
    if (WEXITSTATUS(status) != 0) return 7;

    write(1, "loader_test ok\n", 15);
    return 0;
}
