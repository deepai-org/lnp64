int main() {
    int nodes;
    int i;
    int node;
    nodes = alloc(8);
    store(nodes, 0);
    i = 0;
    while (i < 64) {
        node = alloc(32);
        store(node, i);
        store(nodes, load(nodes) + 1);
        free(node);
        i = i + 1;
    }
    if (load(nodes) == 64) {
        write(1, "json parser ok\n", 15);
        free(nodes);
        return 0;
    }
    write(2, "json parser failed\n", 19);
    return 1;
}
