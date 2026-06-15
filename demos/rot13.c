int main() {
    int start;
    int p;
    int c;
    start = "ebg13 bx\n";
    p = start;
    while (loadb(p) != 0) {
        c = loadb(p);
        if (c >= 97) {
            if (c <= 122) {
                c = c + 13;
                if (c > 122) {
                    c = c - 26;
                }
                storeb(p, c);
            }
        }
        p = p + 1;
    }
    write(1, start, 9);
    return 0;
}
