#include <lnp64/intrinsics.h>

#include <stddef.h>

enum {
  LNP64_EINVAL = 22,
  LNP64_ENOMEM = 12,
  LNP64_ENV_KEY_PAGE_SIZE = 2,
  LNP64_ENV_KEY_HWCAP0 = 5,
  LNP64_AT_PAGESZ = 6,
  LNP64_AT_HWCAP = 16,
};

extern int lnp64_errno_store(int value);

static char *lnp64_environ_slots[17];
static char lnp64_env_storage[16][128];
char **environ = lnp64_environ_slots;

static size_t lnp64_strlen(const char *s) {
  size_t n = 0;
  while (s[n] != 0)
    n = n + 1;
  return n;
}

static int lnp64_env_name_matches(const char *entry, const char *name,
                                  size_t name_len) {
  size_t i = 0;
  while (i < name_len) {
    if (entry[i] != name[i])
      return 0;
    i = i + 1;
  }
  return entry[name_len] == '=';
}

static int lnp64_env_valid_name(const char *name) {
  if (!name || name[0] == 0)
    return 0;
  for (const char *p = name; *p != 0; p = p + 1) {
    if (*p == '=')
      return 0;
  }
  return 1;
}

static size_t lnp64_env_count(void) {
  size_t count = 0;
  while (count < 16 && lnp64_environ_slots[count] != 0)
    count = count + 1;
  return count;
}

static int lnp64_env_find(const char *name, size_t name_len) {
  for (size_t i = 0; i < 16 && lnp64_environ_slots[i] != 0; i = i + 1) {
    if (lnp64_env_name_matches(lnp64_environ_slots[i], name, name_len))
      return (int)i;
  }
  return -1;
}

static int lnp64_env_store(size_t index, const char *name, const char *value) {
  size_t name_len = lnp64_strlen(name);
  size_t value_len = lnp64_strlen(value);
  if (name_len + 1 + value_len + 1 > sizeof lnp64_env_storage[index]) {
    lnp64_errno_store(LNP64_ENOMEM);
    return -1;
  }

  size_t out = 0;
  for (size_t i = 0; i < name_len; i = i + 1)
    lnp64_env_storage[index][out++] = name[i];
  lnp64_env_storage[index][out++] = '=';
  for (size_t i = 0; i < value_len; i = i + 1)
    lnp64_env_storage[index][out++] = value[i];
  lnp64_env_storage[index][out] = 0;
  lnp64_environ_slots[index] = lnp64_env_storage[index];
  return 0;
}

static lnp64_word_t lnp64_env_get(lnp64_word_t key, lnp64_word_t index,
                                  lnp64_word_t len) {
  lnp64_word_t result;
  __asm__ volatile("env_get %0, %1, %2, %3"
                   : "=r"(result)
                   : "r"(key), "r"(index), "r"(len)
                   : "memory");
  return result;
}

unsigned long getauxval(unsigned long type) {
  if (type == LNP64_AT_PAGESZ)
    return lnp64_env_get(LNP64_ENV_KEY_PAGE_SIZE, 0, 0);
  if (type == LNP64_AT_HWCAP)
    return lnp64_env_get(LNP64_ENV_KEY_HWCAP0, 0, 0);
  return 0;
}

char *getenv(const char *name) {
  if (!lnp64_env_valid_name(name))
    return 0;
  size_t name_len = lnp64_strlen(name);
  int index = lnp64_env_find(name, name_len);
  if (index < 0)
    return 0;
  return lnp64_environ_slots[index] + name_len + 1;
}

int clearenv(void) {
  for (size_t i = 0; i < 17; i = i + 1)
    lnp64_environ_slots[i] = 0;
  return 0;
}

int putenv(char *string) {
  if (!string)
    return lnp64_errno_store(LNP64_EINVAL), -1;
  size_t name_len = 0;
  while (string[name_len] != 0 && string[name_len] != '=')
    name_len = name_len + 1;
  if (name_len == 0 || string[name_len] != '=')
    return lnp64_errno_store(LNP64_EINVAL), -1;

  int index = lnp64_env_find(string, name_len);
  if (index < 0) {
    size_t count = lnp64_env_count();
    if (count >= 16)
      return lnp64_errno_store(LNP64_ENOMEM), -1;
    index = (int)count;
    lnp64_environ_slots[index + 1] = 0;
  }
  lnp64_environ_slots[index] = string;
  return 0;
}

int setenv(const char *name, const char *value, int overwrite) {
  if (!lnp64_env_valid_name(name))
    return lnp64_errno_store(LNP64_EINVAL), -1;
  if (!value)
    value = "";

  size_t name_len = lnp64_strlen(name);
  int index = lnp64_env_find(name, name_len);
  if (index >= 0) {
    if (!overwrite)
      return 0;
    return lnp64_env_store((size_t)index, name, value);
  }

  size_t count = lnp64_env_count();
  if (count >= 16)
    return lnp64_errno_store(LNP64_ENOMEM), -1;
  if (lnp64_env_store(count, name, value) != 0)
    return -1;
  lnp64_environ_slots[count + 1] = 0;
  return 0;
}

int unsetenv(const char *name) {
  if (!lnp64_env_valid_name(name))
    return lnp64_errno_store(LNP64_EINVAL), -1;
  size_t name_len = lnp64_strlen(name);
  size_t out = 0;
  for (size_t in = 0; in < 16 && lnp64_environ_slots[in] != 0; in = in + 1) {
    if (!lnp64_env_name_matches(lnp64_environ_slots[in], name, name_len)) {
      lnp64_environ_slots[out] = lnp64_environ_slots[in];
      out = out + 1;
    }
  }
  lnp64_environ_slots[out] = 0;
  return 0;
}
