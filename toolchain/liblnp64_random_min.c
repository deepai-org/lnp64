#include <stddef.h>

static unsigned char lnp64_random_default_state[8];
static char *lnp64_random_current = (char *)lnp64_random_default_state;
static int lnp64_random_default_ready;

static unsigned long long lnp64_random_load(char *state) {
  unsigned long long value = 0;
  for (size_t i = 0; i < 8; i = i + 1)
    value |= (unsigned long long)(unsigned char)state[i] << (i * 8);
  return value;
}

static void lnp64_random_store(char *state, unsigned long long value) {
  for (size_t i = 0; i < 8; i = i + 1)
    state[i] = (char)(unsigned char)(value >> (i * 8));
}

static unsigned long long lnp64_random_seed(unsigned int seed) {
  return seed ? (unsigned long long)seed : 1;
}

static void lnp64_random_ensure_default(void) {
  if (lnp64_random_default_ready)
    return;
  lnp64_random_store((char *)lnp64_random_default_state, 1);
  lnp64_random_default_ready = 1;
}

void srandom(unsigned int seed) {
  lnp64_random_ensure_default();
  lnp64_random_store(lnp64_random_current, lnp64_random_seed(seed));
}

long random(void) {
  lnp64_random_ensure_default();
  unsigned long long state = lnp64_random_load(lnp64_random_current);
  state = state * 6364136223846793005ULL + 1442695040888963407ULL;
  lnp64_random_store(lnp64_random_current, state);
  return (long)((state >> 33) & 0x7fffffffULL);
}

char *initstate(unsigned int seed, char *state, size_t size) {
  if (!state || size < 8)
    return 0;
  lnp64_random_ensure_default();
  char *previous = lnp64_random_current;
  lnp64_random_store(state, lnp64_random_seed(seed));
  lnp64_random_current = state;
  return previous;
}

char *setstate(char *state) {
  if (!state)
    return 0;
  lnp64_random_ensure_default();
  char *previous = lnp64_random_current;
  lnp64_random_current = state;
  return previous;
}
