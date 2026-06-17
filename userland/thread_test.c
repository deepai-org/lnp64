int thread_value;

int worker(int value) {
    thread_value = value + 1;
    return 0;
}

int main() {
    int thread;
    thread_value = 0;
    if (pthread_create(&thread, 0, worker, 41) != 0) return 1;
    if (pthread_join(thread, 0) != 0) return 2;
    if (thread_value != 42) return 3;
    write(1, "thread_test ok\n", 15);
    return 0;
}
