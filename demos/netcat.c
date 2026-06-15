int main() {
    int buf;
    int n;
    open(3, "tcp-listen:127.0.0.1:41065", 0);
    write(1, "netcat ready\n", 13);
    buf = alloc(128);
    wait_on_fd(3);
    n = read(3, buf, 128);
    write(1, buf, n);
    write(3, buf, n);
    return 0;
}
