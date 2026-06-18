int main() {
    int domain;
    int info;
    int ptr;
    int memory_limit;
    int pid_limit;
    int fdr_limit;

    info = alloc(208);
    if (domain_query(0, info) != 200) return 1;
    memory_limit = 100000000;
    pid_limit = 16;
    fdr_limit = 128;
    domain = domain_create(memory_limit, pid_limit, fdr_limit, 63);
    if (domain == -1) return 2;

    if (domain_attach_self(domain) != 0) return 3;
    ptr = alloc(200000000);
    if (ptr != -1) return 4;
    if (domain_detach_self() == 0) return 5;
    if (domain_query(domain, info) != 200) return 6;
    if (load(info + 96) != 0) return 7;
    if (load(info + 88) != 0) return 8;
    if (load(info + 104) != 0) return 9;
    if (domain_destroy(domain) != 0) return 10;
    if (domain_query(domain, info) != -1) return 11;

    write(1, "domain_budget_test ok\n", 22);
    return 0;
}
