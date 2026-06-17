int image_base;

int join_path(int dst, int root, int suffix) {
    strcpy(dst, root);
    strcat(dst, suffix);
    return dst;
}

int record_at(int index) {
    return image_base + 64 + index * 64;
}

int clear_range(int ptr, int len) {
    int i;
    i = 0;
    while (i < len) {
        storeb(ptr + i, 0);
        i = i + 1;
    }
    return 0;
}

int copy_z(int dst, int src) {
    int i;
    int c;
    i = 0;
    c = loadb(src + i);
    while (c != 0) {
        storeb(dst + i, c);
        i = i + 1;
        c = loadb(src + i);
    }
    storeb(dst + i, 0);
    return i;
}

int str_eq(int a, int b) {
    int i;
    int ca;
    int cb;
    i = 0;
    ca = loadb(a + i);
    cb = loadb(b + i);
    while (ca == cb) {
        if (ca == 0) return 1;
        i = i + 1;
        ca = loadb(a + i);
        cb = loadb(b + i);
    }
    return 0;
}

int mount_image() {
    if (loadb(image_base) != 'L') return 1;
    if (loadb(image_base + 1) != 'N') return 2;
    if (loadb(image_base + 2) != 'P') return 3;
    if (loadb(image_base + 3) != 'F') return 4;
    if (loadb(image_base + 4) != 'S') return 5;
    if (loadb(image_base + 5) != '2') return 6;
    return 0;
}

int find_record(int path) {
    int i;
    int rec;
    i = 0;
    while (i < 6) {
        rec = record_at(i);
        if (loadb(rec) == '1') {
            if (str_eq(rec + 4, path)) return rec;
        }
        i = i + 1;
    }
    return 0;
}

int allocate_record() {
    int i;
    int rec;
    i = 0;
    while (i < 6) {
        rec = record_at(i);
        if (loadb(rec) == 0) return rec;
        if (loadb(rec) == '0') return rec;
        i = i + 1;
    }
    return 0;
}

int set_record(int rec, int kind, int path, int data, int mode) {
    clear_range(rec, 64);
    storeb(rec, '1');
    storeb(rec + 1, kind);
    storeb(rec + 2, '1');
    storeb(rec + 3, '1');
    copy_z(rec + 4, path);
    storeb(rec + 36, mode);
    if (data != 0) {
        copy_z(rec + 40, data);
    }
    return 0;
}

int bump_generation(int rec) {
    int gen;
    gen = loadb(rec + 3);
    if (gen < '9') {
        storeb(rec + 3, gen + 1);
    }
    return 0;
}

int create_file(int dir, int name, int data) {
    int parent;
    int path;
    int rec;
    parent = find_record(dir);
    if (parent == 0) return -1;
    if (loadb(parent + 1) != 'd') return -2;
    path = alloc(64);
    copy_z(path, dir);
    strcat(path, "/");
    strcat(path, name);
    if (find_record(path) != 0) return -3;
    rec = allocate_record();
    if (rec == 0) return -4;
    set_record(rec, 'f', path, data, 'r');
    return rec;
}

int rename_path(int old_path, int new_path) {
    int rec;
    rec = find_record(old_path);
    if (rec == 0) return -1;
    if (find_record(new_path) != 0) return -2;
    clear_range(rec + 4, 32);
    copy_z(rec + 4, new_path);
    bump_generation(rec);
    return 0;
}

int link_path(int src_path, int dst_path) {
    int src;
    int rec;
    src = find_record(src_path);
    if (src == 0) return -1;
    if (loadb(src + 1) != 'f') return -2;
    if (find_record(dst_path) != 0) return -3;
    rec = allocate_record();
    if (rec == 0) return -4;
    set_record(rec, 'f', dst_path, src + 40, loadb(src + 36));
    storeb(src + 2, '2');
    storeb(rec + 2, '2');
    return rec;
}

int unlink_path(int path) {
    int rec;
    rec = find_record(path);
    if (rec == 0) return -1;
    storeb(rec, '0');
    bump_generation(rec);
    return 0;
}

int set_metadata(int path, int mode) {
    int rec;
    rec = find_record(path);
    if (rec == 0) return -1;
    storeb(rec + 36, mode);
    bump_generation(rec);
    return 0;
}

int flush_image(int fd) {
    storeb(image_base + 7, loadb(image_base + 7) + 1);
    if (pwrite(fd, image_base, 512, 0) != 512) return -1;
    return 0;
}

int verify_persisted_link(int image_path) {
    int fd;
    int buf;
    fd = open(10, image_path, 0);
    if (fd == -1) return 1;
    if (lseek(10, 384, 0) != 384) return 2;
    buf = alloc(64);
    if (read(10, buf, 64) != 64) return 3;
    close(10);
    if (loadb(buf) != '1') return 4;
    if (loadb(buf + 1) != 'f') return 5;
    if (loadb(buf + 4) != '/') return 6;
    if (loadb(buf + 5) != 't') return 7;
    if (loadb(buf + 6) != 'm') return 8;
    if (loadb(buf + 7) != 'p') return 9;
    if (loadb(buf + 8) != '/') return 10;
    if (loadb(buf + 9) != 'l') return 11;
    if (loadb(buf + 36) != 'x') return 12;
    if (loadb(buf + 40) != 'h') return 13;
    if (loadb(buf + 45) != 10) return 14;
    return 0;
}

int main(int argc, char **argv) {
    int root;
    int path;
    int fd;
    int rc;

    if (argc > 1) {
        root = argv[1];
    } else {
        root = "/tmp/lnp64-netbsd-personality-root";
    }

    path = alloc(256);
    join_path(path, root, "/etc/netbsd_personality.fs");
    fd = open(10, path, O_CREAT);
    if (fd == -1) return 1;

    image_base = mmap(10, 512, 3);
    if (image_base == -1) return 2;
    rc = mount_image();
    if (rc != 0) return 10 + rc;
    if (find_record("/etc/motd") == 0) return 20;
    if (create_file("/tmp", "a", "hello\n") == 0) return 21;
    if (find_record("/tmp/a") == 0) return 22;
    if (rename_path("/tmp/a", "/tmp/b") != 0) return 23;
    if (find_record("/tmp/a") != 0) return 24;
    if (find_record("/tmp/b") == 0) return 25;
    if (link_path("/tmp/b", "/tmp/link-b") == 0) return 26;
    if (unlink_path("/tmp/b") != 0) return 27;
    if (find_record("/tmp/b") != 0) return 28;
    if (find_record("/tmp/link-b") == 0) return 29;
    if (set_metadata("/tmp/link-b", 'x') != 0) return 30;
    if (flush_image(10) != 0) return 31;
    close(10);
    rc = verify_persisted_link(path);
    if (rc != 0) return 40 + rc;

    write(1, "fs_service_test ok\n", 19);
    return 0;
}
