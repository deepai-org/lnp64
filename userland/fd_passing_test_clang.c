#include <unistd.h>

#include "lnp64_intrinsics.h"

enum {
  LNP64_OBJECT_KIND_QUEUE = 2,
  LNP64_OBJECT_PROFILE_PIPE = 1,
  LNP64_ESTALE = 116,
};

static int queue_create(lnp64_cap_t *read_cap, lnp64_cap_t *write_cap) {
  lnp64_word_t record[9];
  record[0] = LNP64_OBJECT_CTL_CREATE;
  record[1] = LNP64_OBJECT_KIND_QUEUE;
  record[2] = LNP64_OBJECT_PROFILE_PIPE;
  record[3] = 0;
  record[4] = 0;
  record[5] = 0;
  record[6] = 0;
  record[7] = 0;
  record[8] = 0;
  if (__lnp_object_ctl((lnp64_word_t)record) != 0)
    return -1;
  *read_cap = record[3];
  *write_cap = record[4];
  return 0;
}

static int lnp64_errno_load(void) {
  int value;
  __asm__ volatile("errno_get %0" : "=r"(value) : : "memory");
  return value;
}

int main(void) {
  lnp64_cap_t data_read;
  lnp64_cap_t data_write;
  lnp64_cap_t transfer_read;
  lnp64_cap_t transfer_write;
  lnp64_cap_t duplicate;
  lnp64_cap_t received;
  char value = 'f';

  if (queue_create(&data_read, &data_write) != 0)
    return 1;
  if (queue_create(&transfer_read, &transfer_write) != 0)
    return 2;

  duplicate = __lnp_cap_dup(data_read, 0, 0);
  if (duplicate == (lnp64_cap_t)-1)
    return 3;
  if (__lnp_cap_send(transfer_write, duplicate, 0) != 1)
    return 4;
  received = __lnp_cap_recv(transfer_read, 0);
  if (received == (lnp64_cap_t)-1)
    return 5;

  if (__lnp_push(data_write, (lnp64_word_t)&value, 1) != 1)
    return 6;
  value = 0;
  if (__lnp_pull(received, (lnp64_word_t)&value, 1) != 1)
    return 7;
  if (value != 'f')
    return 8;

  if (__lnp_cap_revoke(data_read, 0) < 3)
    return 9;
  value = 's';
  if (__lnp_push(data_write, (lnp64_word_t)&value, 1) != 1)
    return 10;
  value = 0;
  if (__lnp_pull(received, (lnp64_word_t)&value, 1) != (lnp64_word_t)-1)
    return 11;
  if (lnp64_errno_load() != LNP64_ESTALE)
    return 12;

  write(1, "fd_passing_test ok\n", 19);
  return 0;
}
