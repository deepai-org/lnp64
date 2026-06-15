int slot;

int consumer() {
    futex_wait(slot, 0);
    cmpxchg(slot, 1, 2);
    futex_wake(slot, 1);
    exit(0);
}

int main() {
    slot = alloc(8);
    store(slot, 0);
    spawn(consumer);
    yield_cpu();
    store(slot, 1);
    futex_wake(slot, 1);
    sleep(8);
    if (load(slot) == 2) {
        write(1, "producer consumer ok\n", 21);
        return 0;
    }
    write(2, "producer consumer failed\n", 25);
    return 1;
}
