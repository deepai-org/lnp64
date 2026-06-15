int handler() {
    int buf;
    int filebuf;
    int n;
    buf = alloc(256);
    read(3, buf, 256);
    open(4, "demos/index.html", 0);
    filebuf = alloc(64);
    n = read(4, filebuf, 64);
    write(3, "HTTP/1.1 200 OK\r\nContent-Length: 16\r\n\r\n", 39);
    write(3, filebuf, n);
    exit(0);
}

int main() {
    open(3, "tcp-listen:127.0.0.1:41066", 0);
    write(1, "httpd ready\n", 12);
    wait_on_fd(3);
    spawn(handler);
    sleep(10);
    return 0;
}
