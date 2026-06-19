#include <unistd.h>

#include "lnp64_intrinsics.h"

typedef unsigned long nfds_t;

struct pollfd {
  int fd;
  short events;
  short revents;
};

int poll(struct pollfd *fds, nfds_t nfds, int timeout);

enum {
  LNP64_OBJECT_KIND_QUEUE = 2,
  LNP64_OBJECT_KIND_MEMORY_OBJECT = 3,
  LNP64_OBJECT_KIND_CLASSIFIER = 7,
  LNP64_OBJECT_PROFILE_PIPE = 1,
  LNP64_OBJECT_PROFILE_CLASSIFIER_TABLE = 1,
  LNP64_RIGHT_READ = 1UL << 0,
  LNP64_RIGHT_WRITE = 1UL << 1,
  LNP64_RIGHT_STAT = 1UL << 3,
  LNP64_RIGHT_CALL = 1UL << 5,
  LNP64_POLLIN = 0x01,
  LNP64_CLASSIFIER_RULE_SIZE_WORDS = 8,
  LNP64_OBJECT_OP_CLASSIFY = 9,
  LNP64_OBJECT_OP_CLASSIFIER_QUERY = 10,
  LNP64_CLASSIFY_PROFILE_IPC = 2,
  LNP64_CLASSIFY_RULE_EXACT = 1,
  LNP64_CLASSIFY_FIELD_SERVICE_ID = 1,
  LNP64_CLASSIFY_FIELD_INLINE0 = 8,
  LNP64_CLASSIFY_ACTION_ROUTE = 4,
  LNP64_CLASSIFY_ACTION_DROP = 3,
  LNP64_CLASSIFY_ACTION_NEEDS_SOFTWARE = 5,
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

static lnp64_cap_t memory_object_create(lnp64_word_t size) {
  lnp64_word_t record[9];
  record[0] = LNP64_OBJECT_CTL_CREATE;
  record[1] = LNP64_OBJECT_KIND_MEMORY_OBJECT;
  record[2] = 0;
  record[3] = 0;
  record[4] = 0;
  record[5] = size;
  record[6] = 0;
  record[7] = 0;
  record[8] = 0;
  return __lnp_object_ctl((lnp64_word_t)record);
}

static lnp64_word_t classifier_ctl(lnp64_word_t op, lnp64_cap_t classifier,
                                   lnp64_word_t arg0, lnp64_word_t arg1) {
  lnp64_word_t record[9];
  record[0] = op;
  record[1] = classifier;
  record[2] = arg0;
  record[3] = arg1;
  record[4] = 0;
  record[5] = 0;
  record[6] = 0;
  record[7] = 0;
  record[8] = 0;
  return __lnp_object_ctl((lnp64_word_t)record);
}

static void set_rule(lnp64_word_t *rules, int slot, lnp64_word_t kind,
                     lnp64_word_t field, lnp64_word_t value,
                     lnp64_word_t mask_or_end, lnp64_word_t action,
                     lnp64_word_t action_arg, lnp64_word_t hash_mod) {
  lnp64_word_t *rule = rules + slot * LNP64_CLASSIFIER_RULE_SIZE_WORDS;
  rule[0] = kind;
  rule[1] = field;
  rule[2] = value;
  rule[3] = mask_or_end;
  rule[4] = action;
  rule[5] = action_arg;
  rule[6] = hash_mod;
  rule[7] = 0;
}

static lnp64_cap_t classifier_create(lnp64_word_t *rules,
                                     lnp64_word_t rule_count,
                                     lnp64_word_t *allowed,
                                     lnp64_word_t allowed_count) {
  lnp64_word_t record[9];
  record[0] = LNP64_OBJECT_CTL_CREATE;
  record[1] = LNP64_OBJECT_KIND_CLASSIFIER;
  record[2] = LNP64_OBJECT_PROFILE_CLASSIFIER_TABLE;
  record[3] = 0;
  record[4] = 0;
  record[5] = (lnp64_word_t)rules;
  record[6] = rule_count;
  record[7] = (lnp64_word_t)allowed;
  record[8] = allowed_count;
  return __lnp_object_ctl((lnp64_word_t)record);
}

int main(void) {
  lnp64_cap_t read_cap;
  lnp64_cap_t write_cap;
  lnp64_cap_t writer_cap;
  lnp64_cap_t source;
  lnp64_cap_t source_cap;
  lnp64_cap_t classifier;
  lnp64_cap_t classifier_cap;
  lnp64_word_t allowed[1];
  lnp64_word_t rules[2 * LNP64_CLASSIFIER_RULE_SIZE_WORDS];
  lnp64_word_t envelope[9];
  lnp64_word_t result[8];
  lnp64_word_t counters[5];
  char payload[3] = {'i', 'p', 'c'};
  char buf[3] = {0, 0, 0};
  struct pollfd pfd;

  if (queue_create(&read_cap, &write_cap) != 0)
    return 1;
  writer_cap = __lnp_cap_dup(write_cap, LNP64_RIGHT_WRITE, 0);
  if (writer_cap == (lnp64_cap_t)-1)
    return 2;
  source = memory_object_create(16);
  if (source == (lnp64_cap_t)-1)
    return 3;
  source_cap = __lnp_cap_dup(source, LNP64_RIGHT_READ, 0);
  if (source_cap == (lnp64_cap_t)-1)
    return 4;

  allowed[0] = writer_cap;
  set_rule(rules, 0, LNP64_CLASSIFY_RULE_EXACT,
           LNP64_CLASSIFY_FIELD_SERVICE_ID, 42, 0,
           LNP64_CLASSIFY_ACTION_ROUTE, writer_cap, 0);
  set_rule(rules, 1, LNP64_CLASSIFY_RULE_EXACT, LNP64_CLASSIFY_FIELD_INLINE0,
           7, 0, LNP64_CLASSIFY_ACTION_DROP, 0, 0);

  classifier = classifier_create(rules, 2, allowed, 1);
  if (classifier == (lnp64_cap_t)-1)
    return 5;
  classifier_cap =
      __lnp_cap_dup(classifier, LNP64_RIGHT_CALL | LNP64_RIGHT_STAT, 0);
  if (classifier_cap == (lnp64_cap_t)-1)
    return 6;

  envelope[0] = LNP64_CLASSIFY_PROFILE_IPC;
  envelope[1] = source_cap;
  envelope[2] = 2;
  envelope[3] = 0;
  envelope[4] = (lnp64_word_t)payload;
  envelope[5] = sizeof(payload);
  envelope[6] = 42;
  envelope[7] = 0;
  envelope[8] = 0;
  if (classifier_ctl(LNP64_OBJECT_OP_CLASSIFY, classifier_cap,
                     (lnp64_word_t)envelope,
                     (lnp64_word_t)result) != LNP64_CLASSIFY_ACTION_ROUTE)
    return 7;
  if (result[0] != LNP64_CLASSIFY_ACTION_ROUTE)
    return 8;
  if (result[2] != writer_cap)
    return 9;

  pfd.fd = (int)read_cap;
  pfd.events = LNP64_POLLIN;
  pfd.revents = 0;
  if (poll(&pfd, 1, 0) != 1)
    return 10;
  if (pfd.revents != LNP64_POLLIN)
    return 11;
  if (__lnp_pull(read_cap, (lnp64_word_t)buf, sizeof(buf)) != sizeof(buf))
    return 12;
  if (buf[0] != 'i')
    return 13;
  if (buf[1] != 'p')
    return 14;
  if (buf[2] != 'c')
    return 15;

  envelope[4] = 0;
  envelope[5] = 0;
  envelope[6] = 7;
  if (classifier_ctl(LNP64_OBJECT_OP_CLASSIFY, classifier_cap,
                     (lnp64_word_t)envelope,
                     (lnp64_word_t)result) != LNP64_CLASSIFY_ACTION_DROP)
    return 16;
  if (result[0] != LNP64_CLASSIFY_ACTION_DROP)
    return 17;

  envelope[6] = 99;
  if (classifier_ctl(LNP64_OBJECT_OP_CLASSIFY, classifier_cap,
                     (lnp64_word_t)envelope,
                     (lnp64_word_t)result) !=
      LNP64_CLASSIFY_ACTION_NEEDS_SOFTWARE)
    return 18;
  if (result[0] != LNP64_CLASSIFY_ACTION_NEEDS_SOFTWARE)
    return 19;

  if (classifier_ctl(LNP64_OBJECT_OP_CLASSIFIER_QUERY, classifier_cap,
                     (lnp64_word_t)counters, 0) != 40)
    return 20;
  if (counters[0] != 2)
    return 21;
  if (counters[1] != 1)
    return 22;
  if (counters[2] != 1)
    return 23;
  if (counters[3] != 0)
    return 24;
  if (counters[4] != 1)
    return 25;

  write(1, "classifier_test ok\n", 19);
  return 0;
}
