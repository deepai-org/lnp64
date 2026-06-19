#include <lnp64/intrinsics.h>

typedef unsigned long size_t;

enum {
  LNP64_OBJECT_KIND_QUEUE = 2,
  LNP64_OBJECT_PROFILE_PIPE = 1,
};

long write(long fd, const void *buf, size_t len);

static lnp64_cap_t parent_to_child_read;
static lnp64_cap_t parent_to_child_write;
static lnp64_cap_t child_to_parent_read;
static lnp64_cap_t child_to_parent_write;

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

static int pull_word(lnp64_cap_t read_cap, lnp64_word_t *value) {
  for (int i = 0; i < 100000; i = i + 1) {
    if (__lnp_pull(read_cap, (lnp64_word_t)value, sizeof(*value)) ==
        sizeof(*value))
      return 0;
  }
  return -1;
}

static int child_main(lnp64_word_t arg) {
  (void)arg;
  lnp64_word_t value = 0;
  if (pull_word(parent_to_child_read, &value) != 0)
    __lnp_exit(1);
  value = value + 1;
  if (__lnp_push(child_to_parent_write, (lnp64_word_t)&value, sizeof(value)) !=
      sizeof(value))
    __lnp_exit(2);
  __lnp_exit(0);
  return 0;
}

int main(void) {
  lnp64_word_t value = 0;
  if (queue_create(&parent_to_child_read, &parent_to_child_write) != 0)
    return 1;
  if (queue_create(&child_to_parent_read, &child_to_parent_write) != 0)
    return 2;

  lnp64_word_t child = __lnp_spawn_entry((lnp64_word_t)child_main, 0);
  if (child == (lnp64_word_t)-1)
    return 3;
  if (__lnp_push(parent_to_child_write, (lnp64_word_t)&value, sizeof(value)) !=
      sizeof(value))
    return 4;
  if (pull_word(child_to_parent_read, &value) != 0)
    return 5;
  if (__lnp_thread_join(child, 0) != 0)
    return 6;

  if (value == 1) {
    write(1, "ping pong ok\n", 13);
    return 0;
  }
  write(2, "ping pong failed\n", 17);
  return 7;
}
