int fib_recursive(int n) {
    int a;
    int b;
    if (n < 2) {
        return n;
    }
    a = fib_recursive(n - 1);
    b = fib_recursive(n - 2);
    return a + b;
}

int main() {
    int i;
    int a;
    int b;
    int next;
    int rec;
    i = 0;
    a = 0;
    b = 1;
    while (i < 10) {
        next = a + b;
        a = b;
        b = next;
        i = i + 1;
    }
    rec = fib_recursive(10);
    if (a == 55) {
        if (rec == 55) {
            write(1, "fibonacci ok\n", 13);
            return 0;
        }
    }
    write(2, "fibonacci failed\n", 17);
    return 1;
}
