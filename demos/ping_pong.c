int parent_pid;

int child_actor() {
    int value;
    int i;
    i = 0;
    while (i < 100000) {
        value = msg_recv();
        msg_send(parent_pid, value + 1, 0);
        i = i + 1;
    }
    exit(0);
}

int main() {
    int child;
    int value;
    int i;
    parent_pid = pid();
    child = fork();
    if (child == 0) {
        child_actor();
        return 0;
    }
    i = 0;
    value = 0;
    while (i < 100000) {
        msg_send(child, value, 0);
        value = msg_recv();
        i = i + 1;
    }
    if (value == 100000) {
        write(1, "ping pong ok\n", 13);
        return 0;
    }
    write(2, "ping pong failed\n", 17);
    return 1;
}
