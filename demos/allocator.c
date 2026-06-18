int main() {
    int ptr;
    int second;
    ptr = alloc(32);
    second = alloc(16);
    if (ptr != 0 && second != 0 && ptr != second) {
        write(1, "alloc ok\n", 9);
        free(second);
        free(ptr);
        return 0;
    }
    write(2, "alloc failed\n", 13);
    return 1;
}
