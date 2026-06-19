void *alloc(unsigned long size);

int main(void) {
    char *buf = alloc(16);
    buf[0] = 101;
    buf[8] = 10;
    return buf[0] + buf[8] * 1000;
}
