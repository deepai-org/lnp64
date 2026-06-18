int classifier_ctl(int op, int a, int b, int c) {
    int ctl;

    ctl = alloc(72);
    store(ctl, op);
    store(ctl + 8, a);
    store(ctl + 16, b);
    store(ctl + 24, c);
    return __lnp_object_ctl(ctl);
}

int main() {
    int fds[2];
    int pollfd[3];
    int allowed;
    int rules;
    int create;
    int classifier;
    int classifier_cap;
    int source;
    int source_cap;
    int writer_cap;
    int envelope;
    int result;
    int counters;
    int payload;
    int buf;

    if (pipe(fds) != 0) return 1;
    writer_cap = cap_dup(fds[1], 0, 2, 0);
    if (writer_cap == -1) return 2;
    source = memory_object_create(16);
    if (source == -1) return 3;
    source_cap = cap_dup(source, 0, 1, 0);
    if (source_cap == -1) return 4;

    allowed = alloc(8);
    rules = alloc(128);
    payload = alloc(3);
    envelope = alloc(72);
    result = alloc(64);
    counters = alloc(40);
    buf = alloc(3);

    store(allowed, writer_cap);
    storeb(payload, 'i');
    storeb(payload + 1, 'p');
    storeb(payload + 2, 'c');
    store(rules, 1);
    store(rules + 8, 1);
    store(rules + 16, 42);
    store(rules + 24, 0);
    store(rules + 32, 4);
    store(rules + 40, writer_cap);
    store(rules + 48, 0);
    store(rules + 64, 1);
    store(rules + 72, 1);
    store(rules + 80, 7);
    store(rules + 88, 0);
    store(rules + 96, 3);
    store(rules + 104, 0);
    store(rules + 112, 0);

    create = alloc(72);
    store(create, 1);
    store(create + 8, 7);
    store(create + 16, 1);
    store(create + 24, 0);
    store(create + 32, 0);
    store(create + 40, rules);
    store(create + 48, 2);
    store(create + 56, allowed);
    store(create + 64, 1);
    classifier = __lnp_object_ctl(create);
    if (classifier == -1) return 5;
    classifier_cap = cap_dup(classifier, 0, 40, 0);
    if (classifier_cap == -1) return 6;

    store(envelope, 2);
    store(envelope + 8, source_cap);
    store(envelope + 16, 2);
    store(envelope + 24, 0);
    store(envelope + 32, payload);
    store(envelope + 40, 3);
    store(envelope + 48, 42);
    store(envelope + 56, 0);
    store(envelope + 64, 0);
    if (classifier_ctl(9, classifier_cap, envelope, result) != 4) return 7;
    if (load(result) != 4) return 8;
    if (load(result + 16) != writer_cap) return 9;
    pollfd[0] = fds[0];
    pollfd[1] = POLLIN;
    pollfd[2] = 0;
    if (poll(pollfd, 1, 0) != 1) return 10;
    if (pollfd[2] != POLLIN) return 11;
    if (read(fds[0], buf, 3) != 3) return 12;
    if (loadb(buf) != 'i') return 13;
    if (loadb(buf + 1) != 'p') return 14;
    if (loadb(buf + 2) != 'c') return 15;

    store(envelope + 32, 0);
    store(envelope + 40, 0);
    store(envelope + 48, 7);
    if (classifier_ctl(9, classifier_cap, envelope, result) != 3) return 16;
    if (load(result) != 3) return 17;

    store(envelope + 48, 99);
    if (classifier_ctl(9, classifier_cap, envelope, result) != 5) return 18;
    if (load(result) != 5) return 19;

    if (classifier_ctl(10, classifier_cap, counters, 0) != 40) return 20;
    if (load(counters) != 2) return 21;
    if (load(counters + 8) != 1) return 22;
    if (load(counters + 16) != 1) return 23;
    if (load(counters + 24) != 0) return 24;
    if (load(counters + 32) != 1) return 25;

    write(1, "classifier_test ok\n", 19);
    return 0;
}
