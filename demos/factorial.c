int main() {
    int n;
    int acc;
    n = 5;
    acc = 1;
    while (n > 1) {
        acc = acc * n;
        n = n - 1;
    }
    if (acc == 120) {
        write(1, "factorial ok\n", 13);
        return 0;
    } else {
        write(2, "factorial failed\n", 17);
        return 1;
    }
}
