int handler() {
    int buf;
    buf = alloc(256);
    read(3, buf, 256);
    write(3, "HTTP/1.1 200 OK\r\nContent-Length: 15\r\n\r\nhello from http", 54);
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
