void *alloc(unsigned long size);
void free(void *ptr);
long write(long fd, const void *buf, unsigned long len);

int main() {
    long *nodes;
    long i;
    long *node;
    nodes = alloc(8);
    *nodes = 0;
    i = 0;
    while (i < 64) {
        node = alloc(32);
        node[0] = i;
        *nodes = *nodes + 1;
        free(node);
        i = i + 1;
    }
    if (*nodes == 64) {
        write(1, "json parser ok\n", 15);
        free(nodes);
        return 0;
    }
    write(2, "json parser failed\n", 19);
    return 1;
}
