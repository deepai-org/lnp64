int gate_service() {
    ret_cap(0x4e425344, 0);
    return 0;
}

int main() {
    int domain;
    int result;
    domain = domain_create(5000000, 2, 16, 63);
    if (domain == -1) return 1;
    call_gate(12, domain, gate_service);
    result = call_cap(12, 0, 0);
    if (result != 0x4e425344) return 2;
    if (domain_destroy(domain) != 0) return 3;
    write(1, "gate_trace_test ok\n", 19);
    return 0;
}
