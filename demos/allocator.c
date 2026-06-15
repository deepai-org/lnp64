int main() {
    int ptr;
    ptr = alloc(32);
    if (ptr != 0) {
        write(1, "alloc ok\n", 9);
        free(ptr);
        return 0;
    }
    write(2, "alloc failed\n", 13);
    return 1;
}
