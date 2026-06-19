long openat(int dirfd, const char *path, int flags);
void *alloc(unsigned long size);
long read(long fd, void *buf, unsigned long len);
long write(long fd, const void *buf, unsigned long len);
void free(void *ptr);

int main() {
    long fd;
    void *buf;
    long n;
    fd = openat(-100, "demos/cat_input.txt", 0);
    if (fd == -1) {
        return 1;
    }
    buf = alloc(128);
    n = read(fd, buf, 128);
    if (n < 0) {
        free(buf);
        return 2;
    }
    write(1, buf, n);
    free(buf);
    return 0;
}
