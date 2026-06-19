static int lnp64_errno_slot;
static int lnp64_errno_initialized;

static int lnp64_errno_get(void) {
  unsigned long value;
  __asm__ volatile("errno_get %0" : "=r"(value) : : "memory");
  return (int)value;
}

static void lnp64_errno_set(int value) {
  __asm__ volatile("errno_set %0"
                   :
                   : "r"((unsigned long)(unsigned int)value)
                   : "memory");
}

int *__errno_location(void) {
  if (!lnp64_errno_initialized) {
    lnp64_errno_slot = lnp64_errno_get();
    lnp64_errno_initialized = 1;
  }
  return &lnp64_errno_slot;
}

int lnp64_errno_store(int value) {
  lnp64_errno_slot = value;
  lnp64_errno_initialized = 1;
  lnp64_errno_set(value);
  return value;
}

const char *strerror(int value) {
  if (value == 0)
    return "Success";
  if (value == 9)
    return "Bad file descriptor";
  if (value == 12)
    return "Out of memory";
  if (value == 14)
    return "Bad address";
  if (value == 22)
    return "Invalid argument";
  return "Unknown error";
}
