#include <fcntl.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

static unsigned char *image_base;

static unsigned char *record_at(int index) {
  return image_base + 64 + index * 64;
}

static void clear_range(unsigned char *ptr, int len) {
  for (int i = 0; i < len; i = i + 1)
    ptr[i] = 0;
}

static int copy_z(unsigned char *dst, const char *src) {
  int i = 0;
  while (src[i] != 0) {
    dst[i] = (unsigned char)src[i];
    i = i + 1;
  }
  dst[i] = 0;
  return i;
}

static int mount_image(void) {
  if (image_base[0] != 'L')
    return 1;
  if (image_base[1] != 'N')
    return 2;
  if (image_base[2] != 'P')
    return 3;
  if (image_base[3] != 'F')
    return 4;
  if (image_base[4] != 'S')
    return 5;
  if (image_base[5] != '2')
    return 6;
  return 0;
}

static unsigned char *find_record(const char *path) {
  for (int i = 0; i < 6; i = i + 1) {
    unsigned char *rec = record_at(i);
    if (rec[0] == '1' && strcmp((const char *)(rec + 4), path) == 0)
      return rec;
  }
  return 0;
}

static unsigned char *allocate_record(void) {
  for (int i = 0; i < 6; i = i + 1) {
    unsigned char *rec = record_at(i);
    if (rec[0] == 0 || rec[0] == '0')
      return rec;
  }
  return 0;
}

static void set_record(unsigned char *rec, int kind, const char *path,
                       const char *data, int mode) {
  clear_range(rec, 64);
  rec[0] = '1';
  rec[1] = (unsigned char)kind;
  rec[2] = '1';
  rec[3] = '1';
  copy_z(rec + 4, path);
  rec[36] = (unsigned char)mode;
  if (data != 0)
    copy_z(rec + 40, data);
}

static void bump_generation(unsigned char *rec) {
  if (rec[3] < '9')
    rec[3] = (unsigned char)(rec[3] + 1);
}

static int live_generation_matches(unsigned char *rec, int generation) {
  if (rec == 0)
    return 0;
  if (rec[0] != '1')
    return 0;
  return rec[3] == generation;
}

static void build_child_path(char *dst, const char *dir, const char *name) {
  size_t d = strlen(dir);
  size_t n = strlen(name);
  memcpy(dst, dir, d);
  dst[d] = '/';
  memcpy(dst + d + 1, name, n + 1);
}

static unsigned char *create_file(const char *dir, const char *name,
                                  const char *data) {
  unsigned char *parent = find_record(dir);
  if (parent == 0 || parent[1] != 'd')
    return 0;

  char *path = malloc(64);
  if (path == 0)
    return 0;
  build_child_path(path, dir, name);
  if (find_record(path) != 0) {
    free(path);
    return 0;
  }

  unsigned char *rec = allocate_record();
  if (rec == 0) {
    free(path);
    return 0;
  }
  set_record(rec, 'f', path, data, 'r');
  free(path);
  return rec;
}

static int rename_path(const char *old_path, const char *new_path) {
  unsigned char *rec = find_record(old_path);
  if (rec == 0 || find_record(new_path) != 0)
    return -1;
  clear_range(rec + 4, 32);
  copy_z(rec + 4, new_path);
  bump_generation(rec);
  return 0;
}

static unsigned char *link_path(const char *src_path, const char *dst_path) {
  unsigned char *src = find_record(src_path);
  if (src == 0 || src[1] != 'f' || find_record(dst_path) != 0)
    return 0;

  unsigned char *rec = allocate_record();
  if (rec == 0)
    return 0;
  set_record(rec, 'f', dst_path, (const char *)(src + 40), src[36]);
  src[2] = '2';
  rec[2] = '2';
  return rec;
}

static int unlink_path(const char *path) {
  unsigned char *rec = find_record(path);
  if (rec == 0)
    return -1;
  rec[0] = '0';
  bump_generation(rec);
  return 0;
}

static int set_metadata(const char *path, int mode) {
  unsigned char *rec = find_record(path);
  if (rec == 0)
    return -1;
  rec[36] = (unsigned char)mode;
  bump_generation(rec);
  return 0;
}

static int read_full_image(int fd, unsigned char *buf) {
  if (lseek(fd, 0, SEEK_SET) != 0)
    return -1;
  int done = 0;
  while (done < 512) {
    int got = read(fd, buf + done, 512 - done);
    if (got <= 0)
      return -1;
    done = done + got;
  }
  return 0;
}

static int flush_image(const char *image_path) {
  image_base[7] = (unsigned char)(image_base[7] + 1);
  int fd = open(image_path, O_RDWR);
  if (fd == -1)
    return -1;
  if (lseek(fd, 0, SEEK_SET) != 0)
    return close(fd), -1;
  int done = 0;
  while (done < 512) {
    int sent = write(fd, image_base + done, 512 - done);
    if (sent <= 0)
      return close(fd), -1;
    done = done + sent;
  }
  return close(fd);
}

static int verify_persisted_link(const char *image_path) {
  int fd = open(image_path, O_RDONLY);
  if (fd == -1)
    return 1;
  if (lseek(fd, 384, SEEK_SET) != 384)
    return close(fd), 2;

  unsigned char *buf = malloc(64);
  if (buf == 0)
    return close(fd), 3;
  if (read(fd, buf, 64) != 64)
    return free(buf), close(fd), 4;
  close(fd);

  if (buf[0] != '1') {
    free(buf);
    return 5;
  }
  if (buf[1] != 'f') {
    free(buf);
    return 6;
  }
  if (buf[4] != '/') {
    free(buf);
    return 7;
  }
  if (buf[5] != 't') {
    free(buf);
    return 8;
  }
  if (buf[6] != 'm') {
    free(buf);
    return 9;
  }
  if (buf[7] != 'p') {
    free(buf);
    return 10;
  }
  if (buf[8] != '/') {
    free(buf);
    return 11;
  }
  if (buf[9] != 'l') {
    free(buf);
    return 12;
  }
  if (buf[36] != 'x') {
    free(buf);
    return 13;
  }
  if (buf[40] != 'h') {
    free(buf);
    return 14;
  }
  if (buf[45] != '\n') {
    free(buf);
    return 15;
  }
  free(buf);
  return 0;
}

static int verify_recovered_image(const char *image_path) {
  int fd = open(image_path, O_RDONLY);
  if (fd == -1)
    return 1;

  unsigned char *saved_base = image_base;
  image_base = malloc(512);
  if (image_base == 0) {
    image_base = saved_base;
    return 2;
  }
  if (read_full_image(fd, image_base) != 0) {
    close(fd);
    free(image_base);
    image_base = saved_base;
    return 3;
  }

  int rc = mount_image();
  if (rc != 0) {
    close(fd);
    free(image_base);
    image_base = saved_base;
    return 10 + rc;
  }
  if (image_base[7] == '0') {
    close(fd);
    free(image_base);
    image_base = saved_base;
    return 20;
  }
  if (find_record("/tmp/b") != 0) {
    close(fd);
    free(image_base);
    image_base = saved_base;
    return 21;
  }

  unsigned char *rec = find_record("/tmp/link-b");
  if (rec == 0) {
    close(fd);
    free(image_base);
    image_base = saved_base;
    return 22;
  }
  if (rec[1] != 'f') {
    close(fd);
    free(image_base);
    image_base = saved_base;
    return 23;
  }
  if (rec[36] != 'x') {
    close(fd);
    free(image_base);
    image_base = saved_base;
    return 24;
  }
  if (rec[40] != 'h') {
    close(fd);
    free(image_base);
    image_base = saved_base;
    return 25;
  }
  if (rec[45] != '\n') {
    close(fd);
    free(image_base);
    image_base = saved_base;
    return 26;
  }
  close(fd);
  free(image_base);
  image_base = saved_base;
  return 0;
}

int main(void) {
  const char *path = "/etc/netbsd_personality.fs";
  int fd = open(path, O_RDONLY);
  if (fd == -1)
    return 1;

  image_base = malloc(512);
  if (image_base == 0)
    return 2;
  if (read_full_image(fd, image_base) != 0)
    return 3;

  int rc = mount_image();
  if (rc != 0)
    return 10 + rc;
  if (find_record("/etc/motd") == 0)
    return 20;

  unsigned char *rec = create_file("/tmp", "a", "hello\n");
  if (rec == 0)
    return 21;
  int gen = rec[3];
  if (live_generation_matches(rec, gen) == 0)
    return 22;
  if (find_record("/tmp/a") == 0)
    return 23;
  if (rename_path("/tmp/a", "/tmp/b") != 0)
    return 24;
  if (live_generation_matches(rec, gen) != 0)
    return 25;
  gen = rec[3];
  if (live_generation_matches(rec, gen) == 0)
    return 26;
  if (find_record("/tmp/a") != 0)
    return 27;
  if (find_record("/tmp/b") == 0)
    return 28;

  unsigned char *link_rec = link_path("/tmp/b", "/tmp/link-b");
  if (link_rec == 0)
    return 29;
  int link_gen = link_rec[3];
  if (unlink_path("/tmp/b") != 0)
    return 30;
  if (live_generation_matches(rec, gen) != 0)
    return 31;
  if (find_record("/tmp/b") != 0)
    return 32;
  if (find_record("/tmp/link-b") == 0)
    return 33;
  if (set_metadata("/tmp/link-b", 'x') != 0)
    return 34;
  if (live_generation_matches(link_rec, link_gen) != 0)
    return 35;
  link_gen = link_rec[3];
  if (live_generation_matches(link_rec, link_gen) == 0)
    return 36;
  close(fd);
  if (flush_image(path) != 0)
    return 37;

  rc = verify_persisted_link(path);
  if (rc != 0)
    return 40 + rc;
  rc = verify_recovered_image(path);
  if (rc != 0)
    return 70 + rc;

  write(1, "fs_service_test ok\n", 19);
  return 0;
}
