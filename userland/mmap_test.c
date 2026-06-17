int main() {
    int p;
    p = mmap(0, 4096, 3);
    if (p == -1) return 1;
    *p = 42;
    if (*p != 42) return 2;
    if (mprotect(p, 4096, 1) != 0) return 3;
    if (*p != 42) return 4;
    if (munmap(p, 4096) != 0) return 5;
    write(1, "mmap_test ok\n", 13);
    return 0;
}
