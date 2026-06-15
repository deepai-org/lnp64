int main() {
    int fd;
    int buf;
    int n;
    fd = open(3, "demos/cat_input.txt", 0);
    buf = alloc(128);
    n = read(3, buf, 128);
    write(1, buf, n);
    free(buf);
    return 0;
}
