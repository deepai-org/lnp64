#ifndef LNP64_SEARCH_H
#define LNP64_SEARCH_H

#include <stddef.h>

void insque(void *elem, void *pred);
void *lfind(const void *key, const void *base, size_t *nelp, size_t width,
            int (*compar)(const void *, const void *));
void *lsearch(const void *key, void *base, size_t *nelp, size_t width,
              int (*compar)(const void *, const void *));
void remque(void *elem);

#endif
