int total;

int worker() {
    int old;
    int done;
    done = 0;
    while (done == 0) {
        old = load(total);
        if (cmpxchg(total, old, old + 1) == old) {
            done = 1;
        }
    }
    exit(0);
}

int main() {
    int i;
    total = alloc(8);
    store(total, 0);
    i = 0;
    while (i < 4) {
        spawn(worker);
        i = i + 1;
    }
    sleep(100);
    if (load(total) == 4) {
        write(1, "parallel hash ok\n", 17);
        return 0;
    }
    write(2, "parallel hash failed\n", 21);
    return 1;
}
