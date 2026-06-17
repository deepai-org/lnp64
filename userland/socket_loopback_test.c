int main() {
    struct pollfd p[1];
    int server;
    int client;
    int accepted;
    int addr;
    int addrlen;
    int buf;

    server = socket(AF_INET, SOCK_STREAM, 0);
    if (server == -1) return 1;
    if (bind(server, "127.0.0.1:0", 0) != 0) return 2;
    if (listen(server, 1) != 0) return 3;
    addr = alloc(64);
    addrlen = alloc(8);
    store(addrlen, 64);
    if (getsockname(server, addr, addrlen) != 0) return 4;
    client = socket(AF_INET, SOCK_STREAM, 0);
    if (client == -1) return 5;
    if (connect(client, addr, load(addrlen)) != 0) return 6;
    p[0].fd = server;
    p[0].events = POLLIN;
    if (poll(p, 1, 0) != 1) return 7;
    accepted = accept(server, 0, 0);
    if (accepted == -1) return 8;
    if (send(client, "n", 1, MSG_NOSIGNAL) != 1) return 9;
    p[0].fd = accepted;
    p[0].events = POLLIN;
    if (poll(p, 1, 0) != 1) return 10;
    buf = alloc(1);
    if (recv(accepted, buf, 1, 0) != 1) return 11;
    if (loadb(buf) != 'n') return 12;

    write(1, "socket_loopback_test ok\n", 24);
    return 0;
}
