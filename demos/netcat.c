int main() {
    struct pollfd p[1];
    int server;
    int client;
    int buf;
    int n;
    server = socket(AF_INET, SOCK_STREAM, 0);
    bind(server, "127.0.0.1:41065", 0);
    listen(server, 1);
    write(1, "netcat ready\n", 13);
    buf = alloc(128);
    p[0].fd = server;
    p[0].events = POLLIN;
    while (poll(p, 1, 0) == 0) {
        yield_cpu();
    }
    client = accept(server, 0, 0);
    n = recv(client, buf, 128, 0);
    write(1, buf, n);
    send(client, buf, n, MSG_NOSIGNAL);
    return 0;
}
