int main() {
    if (pid() == 1) {
        write(1, "pcr ok\n", 7);
        return 0;
    }
    write(2, "pcr failed\n", 11);
    return 1;
}
