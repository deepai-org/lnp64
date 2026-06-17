int create_domain_under(int parent, int profile, int memory, int pids, int fdrs, int caps) {
    if (parent == 0) {
        return domain_create(memory, pids, fdrs, caps);
    }
    return domain_create_child(parent, profile, memory, pids, fdrs, caps);
}

int main() {
    int info;
    int original;
    int parent;
    int jail;
    int container;
    int too_large;
    int original_depth;

    info = alloc(208);
    if (domain_query(0, info) != 200) return 1;
    original = load(info + 8);
    original_depth = load(info + 128);

    parent = create_domain_under(0, 40, 95000000, 8, 64, 63);
    if (parent == -1) return 2;
    jail = create_domain_under(parent, 41, 90000000, 6, 48, 31);
    if (jail == -1) return 3;
    container = create_domain_under(jail, 42, 85000000, 4, 32, 15);
    if (container == -1) return 4;

    if (domain_query(container, info) != 200) return 5;
    if (load(info + 8) != container) return 6;
    if (load(info + 120) != jail) return 7;
    if (load(info + 128) != original_depth + 3) return 8;

    if (domain_query(jail, info) != 200) return 9;
    if (load(info + 120) != parent) return 10;
    if (load(info + 128) != original_depth + 2) return 11;
    if (load(info + 136) != 1) return 12;

    too_large = create_domain_under(container, 43, 90000000, 2, 16, 15);
    if (too_large != -1) return 13;

    if (domain_attach_self(container) != 0) return 14;
    if (domain_query(0, info) != 200) return 15;
    if (load(info + 8) != container) return 16;
    if (load(info + 96) != 1) return 17;
    if (domain_detach_self() != jail) return 18;
    if (domain_query(container, info) != 200) return 19;
    if (load(info + 96) != 0) return 20;
    if (domain_detach_self() != parent) return 21;
    if (domain_detach_self() != original) return 22;

    if (domain_freeze(jail) != 0) return 23;
    if (domain_query(container, info) != 200) return 24;
    if (load(info + 112) != 1) return 25;
    if (domain_resume(jail) != 0) return 26;
    if (domain_query(container, info) != 200) return 27;
    if (load(info + 112) != 0) return 28;

    if (domain_destroy(container) != 0) return 29;
    if (domain_query(container, info) != -1) return 30;
    if (domain_query(jail, info) != 200) return 31;
    if (load(info + 136) != 0) return 32;
    if (domain_destroy(jail) != 0) return 33;
    if (domain_query(parent, info) != 200) return 34;
    if (load(info + 136) != 0) return 35;
    if (domain_destroy(parent) != 0) return 36;

    write(1, "domain_nested_test ok\n", 22);
    return 0;
}
