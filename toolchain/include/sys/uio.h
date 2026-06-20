#ifndef LNP64_SYS_UIO_H
#define LNP64_SYS_UIO_H

#include <stddef.h>

struct iovec {
  void *iov_base;
  size_t iov_len;
};

long readv(int fd, const struct iovec *iov, int iovcnt);
long writev(int fd, const struct iovec *iov, int iovcnt);

#endif
