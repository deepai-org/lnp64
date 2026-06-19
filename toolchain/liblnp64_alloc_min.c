#include <lnp64/intrinsics.h>

#include <stdlib.h>
#include <string.h>

void *alloc(size_t size) {
  return __lnp_alloc(size);
}

void *malloc(size_t size) {
  return __lnp_alloc(size);
}

void free(void *ptr) {
  __lnp_free(ptr);
}

void *calloc(size_t count, size_t size) {
  size_t total = count * size;
  void *ptr = malloc(total);
  if (!ptr)
    return 0;
  return memset(ptr, 0, total);
}

void *realloc(void *ptr, size_t size) {
  if (!ptr)
    return malloc(size);
  if (size == 0) {
    free(ptr);
    return 0;
  }

  void *new_ptr = malloc(size);
  if (!new_ptr)
    return 0;

  size_t old_size = __lnp_alloc_size(ptr);
  size_t copy_size = old_size < size ? old_size : size;
  memcpy(new_ptr, ptr, copy_size);
  free(ptr);
  return new_ptr;
}
