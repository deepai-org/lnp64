int main() {
    int fd;
    int ptr;
    int old;
    fd = open(3, "demos/sqlite_lite.db", 1);
    ptr = mmap(3, 64, 3);
    old = cmpxchg(ptr, 0, 1);
    fence();
    if (old == 0) {
        write(1, "sqlite lite ok\n", 15);
        return 0;
    }
    write(2, "sqlite lite failed\n", 19);
    return 1;
}
