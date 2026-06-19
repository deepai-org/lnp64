typedef unsigned long size_t;

void *memcpy(void *dst, const void *src, size_t len);

struct lnp64_qelem {
  struct lnp64_qelem *q_forw;
  struct lnp64_qelem *q_back;
};

void *lfind(const void *key, const void *base, size_t *nelp, size_t width,
            int (*compar)(const void *, const void *)) {
  const char *cursor = (const char *)base;
  for (size_t i = 0; i < *nelp; i = i + 1) {
    const void *entry = cursor + i * width;
    if (compar(key, entry) == 0)
      return (void *)entry;
  }
  return 0;
}

void *lsearch(const void *key, void *base, size_t *nelp, size_t width,
              int (*compar)(const void *, const void *)) {
  char *cursor = (char *)base;
  void *found = lfind(key, base, nelp, width, compar);
  if (found)
    return found;
  found = cursor + (*nelp * width);
  memcpy(found, key, width);
  *nelp = *nelp + 1;
  return found;
}

void insque(void *elem, void *pred) {
  struct lnp64_qelem *entry = (struct lnp64_qelem *)elem;
  struct lnp64_qelem *prev = (struct lnp64_qelem *)pred;
  if (!prev) {
    entry->q_forw = 0;
    entry->q_back = 0;
    return;
  }
  entry->q_forw = prev->q_forw;
  entry->q_back = prev;
  if (prev->q_forw)
    prev->q_forw->q_back = entry;
  prev->q_forw = entry;
}

void remque(void *elem) {
  struct lnp64_qelem *entry = (struct lnp64_qelem *)elem;
  if (entry->q_back)
    entry->q_back->q_forw = entry->q_forw;
  if (entry->q_forw)
    entry->q_forw->q_back = entry->q_back;
}
