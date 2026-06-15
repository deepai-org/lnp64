int total;

int worker() {
    int buf;
    int n;
    int old;
    int done;
    buf = alloc(64);
    n = read(4, buf, 64);
    done = 0;
    while (done == 0) {
        old = load(total);
        if (cmpxchg(total, old, old + n) == old) {
            done = 1;
        }
    }
    free(buf);
    exit(0);
}

int main() {
    int i;
    open(4, "demos/hash_input.txt", 0);
    total = alloc(8);
    store(total, 0);
    i = 0;
    while (i < 4) {
        spawn(worker);
        i = i + 1;
    }
    sleep(100);
    if (load(total) == 256) {
        write(1, "parallel hash ok\n", 17);
        return 0;
    }
    write(2, "parallel hash failed\n", 21);
    return 1;
}
