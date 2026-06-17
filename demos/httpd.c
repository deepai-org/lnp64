int handler(int client) {
    int buf;
    int filebuf;
    int n;
    buf = alloc(256);
    recv(client, buf, 256, 0);
    open(10, "demos/index.html", 0);
    filebuf = alloc(64);
    n = read(10, filebuf, 64);
    send(client, "HTTP/1.1 200 OK\r\nContent-Length: 16\r\n\r\n", 39, MSG_NOSIGNAL);
    send(client, filebuf, n, MSG_NOSIGNAL);
    return 0;
}

int main() {
    struct pollfd p[1];
    int server;
    int client;
    server = socket(AF_INET, SOCK_STREAM, 0);
    bind(server, "127.0.0.1:41066", 0);
    listen(server, 1);
    write(1, "httpd ready\n", 12);
    p[0].fd = server;
    p[0].events = POLLIN;
    while (poll(p, 1, 0) == 0) {
        yield_cpu();
    }
    client = accept(server, 0, 0);
    handler(client);
    return 0;
}
