int db_ptr;

int writer() {
    cmpxchg(db_ptr, 0, 1);
    fence();
    exit(0);
}

int main() {
    int fd;
    int old;
    fd = open(3, "demos/sqlite_lite.db", 1);
    db_ptr = mmap(3, 64, 3);
    spawn(writer);
    sleep(10);
    old = cmpxchg(db_ptr, 1, 2);
    fence();
    if (old == 1) {
        write(1, "sqlite lite ok\n", 15);
        return 0;
    }
    write(2, "sqlite lite failed\n", 19);
    return 1;
}
