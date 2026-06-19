#include <dirent.h>
#include <fcntl.h>
#include <sys/stat.h>
#include <unistd.h>

static int read_first_byte(int fd, char expected) {
  char buf[2];
  if (read(fd, buf, 1) != 1)
    return -1;
  return buf[0] == expected ? 0 : -1;
}

static int dirfd_from_dir(DIR *dir) {
  return (int)(long)dir;
}

int main(void) {
  int fd;
  int dirfd;
  DIR *dir;
  char buf[256];
  char cwd[256];
  struct stat st;

  if (getcwd(cwd, sizeof(cwd)) != cwd)
    return 1;
  fd = openat(AT_FDCWD, "/etc/passwd", O_RDONLY);
  if (fd != -1)
    return 2;

  fd = openat(AT_FDCWD, "/etc/motd", O_RDONLY);
  if (fd == -1)
    return 3;
  if (read_first_byte(fd, 'w') != 0)
    return 4;
  close(fd);

  dir = opendir("/etc");
  if (dir == 0)
    return 27;
  dirfd = dirfd_from_dir(dir);
  fd = openat(dirfd, "motd", O_RDONLY);
  if (fd == -1)
    return 28;
  if (read_first_byte(fd, 'w') != 0)
    return 29;
  close(fd);
  if (fstatat(dirfd, "motd", &st, 0) != 0)
    return 32;
  if (st.st_ino == 0)
    return 33;
  if (fstatat(dirfd, "../../etc/passwd", &st, 0) != -1)
    return 34;
  fd = openat(dirfd, "../../etc/passwd", O_RDONLY);
  if (fd != -1)
    return 31;
  closedir(dir);

  if (chdir("/tmp") != 0)
    return 6;
  if (getcwd(cwd, sizeof(cwd)) != cwd)
    return 7;
  fd = openat(AT_FDCWD, "../etc/motd", O_RDONLY);
  if (fd == -1)
    return 8;
  if (read_first_byte(fd, 'w') != 0)
    return 9;
  close(fd);

  fd = openat(AT_FDCWD, "../../etc/passwd", O_RDONLY);
  if (fd != -1)
    return 11;

  unlink("host_passwd_link");
  unlink("host_tmp_link");
  unlink("ns_probe");
  unlink("ns_probe_link");
  unlink("ns_probe_hard");
  unlink("rename_probe");
  unlink("rename_probe_done");
  unlink("unlink_probe");
  rmdir("ns_probe_dir");

  if (symlink("/etc/passwd", "host_passwd_link") != 0)
    return 12;
  fd = openat(AT_FDCWD, "host_passwd_link", O_RDONLY);
  if (fd != -1)
    return 13;
  if (readlink("host_passwd_link", buf, sizeof(buf)) != 11)
    return 14;
  if (buf[0] != '/')
    return 15;
  if (buf[5] != 'p')
    return 16;

  if (symlink("/tmp", "host_tmp_link") != 0)
    return 17;
  fd = openat(AT_FDCWD, "host_tmp_link/lnp64_ns_escape_probe",
              O_CREAT | O_TRUNC | O_RDWR);
  if (fd != -1)
    return 18;
  if (chdir("host_tmp_link") == 0)
    return 19;

  fd = openat(AT_FDCWD, "ns_probe", O_CREAT | O_TRUNC | O_RDWR);
  if (fd == -1)
    return 20;
  if (write(fd, "ok", 2) != 2)
    return 21;
  close(fd);

  dir = opendir("/tmp");
  if (dir == 0)
    return 35;
  dirfd = dirfd_from_dir(dir);
  if (fchmodat(dirfd, "ns_probe", 0600, 0) != 0)
    return 36;
  if (fstatat(dirfd, "ns_probe", &st, 0) != 0)
    return 37;
  if ((st.st_mode & 0777) != 0600)
    return 38;
  if (fchmodat(dirfd, "../../etc/motd", 0600, 0) != -1)
    return 39;
  if (fchownat(dirfd, "ns_probe", (uid_t)-1, (gid_t)-1, 0) != 0)
    return 52;
  if (fchownat(dirfd, "../../etc/motd", (uid_t)-1, (gid_t)-1, 0) != -1)
    return 53;
  if (utimensat(dirfd, "ns_probe", 0, 0) != 0)
    return 54;
  if (utimensat(dirfd, "../../etc/motd", 0, 0) != -1)
    return 55;
  if (faccessat(dirfd, "ns_probe", F_OK, 0) != 0)
    return 63;
  if (faccessat(dirfd, "../../etc/motd", F_OK, 0) != -1)
    return 64;
  if (mkdirat(dirfd, "ns_probe_dir", 0755) != 0)
    return 56;
  if (fstatat(dirfd, "ns_probe_dir", &st, 0) != 0)
    return 57;
  if (unlinkat(dirfd, "ns_probe_dir", 0) != 0)
    return 58;
  if (mkdirat(dirfd, "../../etc/ns_probe_dir", 0755) != -1)
    return 59;
  if (symlinkat("ns_probe", dirfd, "ns_probe_link") != 0)
    return 44;
  if (readlinkat(dirfd, "ns_probe_link", buf, sizeof(buf)) != 8)
    return 60;
  if (buf[0] != 'n')
    return 61;
  if (readlinkat(dirfd, "../../etc/ns_probe_link", buf, sizeof(buf)) != -1)
    return 62;
  if (fstatat(dirfd, "ns_probe_link", &st, 0) != 0)
    return 45;
  if (unlinkat(dirfd, "ns_probe_link", 0) != 0)
    return 46;
  if (symlinkat("ns_probe", dirfd, "../../etc/ns_probe_link") != -1)
    return 47;
  if (linkat(dirfd, "ns_probe", dirfd, "ns_probe_hard", 0) != 0)
    return 48;
  if (fstatat(dirfd, "ns_probe_hard", &st, 0) != 0)
    return 49;
  if (unlinkat(dirfd, "ns_probe_hard", 0) != 0)
    return 50;
  if (linkat(dirfd, "ns_probe", dirfd, "../../etc/ns_probe_hard", 0) != -1)
    return 51;

  fd = openat(dirfd, "rename_probe", O_CREAT | O_TRUNC | O_RDWR);
  if (fd == -1)
    return 65;
  close(fd);
  if (renameat(dirfd, "rename_probe", dirfd, "rename_probe_done") != 0)
    return 66;
  if (fstatat(dirfd, "rename_probe_done", &st, 0) != 0)
    return 67;
  if (renameat(dirfd, "rename_probe_done", dirfd, "../../etc/rename_probe") !=
      -1)
    return 68;
  if (renameat(dirfd, "../../etc/motd", dirfd, "rename_escape") != -1)
    return 69;
  if (unlinkat(dirfd, "rename_probe_done", 0) != 0)
    return 70;

  fd = openat(dirfd, "unlink_probe", O_CREAT | O_TRUNC | O_RDWR);
  if (fd == -1)
    return 40;
  close(fd);
  if (unlinkat(dirfd, "unlink_probe", 0) != 0)
    return 41;
  if (fstatat(dirfd, "unlink_probe", &st, 0) != -1)
    return 42;
  if (unlinkat(dirfd, "../../etc/motd", 0) != -1)
    return 43;
  closedir(dir);

  if (chdir("/") != 0)
    return 22;
  fd = openat(AT_FDCWD, "tmp/ns_probe", O_RDONLY);
  if (fd == -1)
    return 23;
  if (read(fd, buf, 2) != 2)
    return 24;
  if (buf[0] != 'o')
    return 25;
  if (buf[1] != 'k')
    return 26;
  close(fd);

  write(1, "namespace_test ok\n", 18);
  return 0;
}
