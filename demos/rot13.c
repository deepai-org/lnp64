void *alloc(unsigned long size);
void free(void *ptr);
long write(long fd, const void *buf, unsigned long len);

int main() {
    char *start;
    char *p;
    int c;
    start = alloc(10);
    start[0] = 'e';
    start[1] = 'b';
    start[2] = 'g';
    start[3] = '1';
    start[4] = '3';
    start[5] = ' ';
    start[6] = 'b';
    start[7] = 'x';
    start[8] = '\n';
    start[9] = 0;
    p = start;
    while (*p != 0) {
        c = *p;
        if (c >= 97) {
            if (c <= 122) {
                c = c + 13;
                if (c > 122) {
                    c = c - 26;
                }
                *p = c;
            }
        }
        p = p + 1;
    }
    write(1, start, 9);
    free(start);
    return 0;
}
