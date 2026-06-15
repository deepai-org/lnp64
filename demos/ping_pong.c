int parent_pid;

int child_actor() {
    int value;
    value = msg_recv();
    msg_send(parent_pid, value + 1, 0);
    exit(0);
}

int main() {
    int child;
    int value;
    parent_pid = pid();
    child = fork();
    if (child == 0) {
        child_actor();
        return 0;
    }
    msg_send(child, 41, 0);
    value = msg_recv();
    if (value == 42) {
        write(1, "ping pong ok\n", 13);
        return 0;
    }
    write(2, "ping pong failed\n", 17);
    return 1;
}
