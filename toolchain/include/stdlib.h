#ifndef LNP64_STDLIB_H
#define LNP64_STDLIB_H

#include <stddef.h>

#define EXIT_FAILURE 1
#define EXIT_SUCCESS 0

void abort(void);
void exit(int status);
void free(void *ptr);
void *malloc(size_t size);
void *calloc(size_t count, size_t size);
void *realloc(void *ptr, size_t size);
void qsort(void *base, size_t count, size_t size,
           int (*compar)(const void *, const void *));
long long llabs(long long value);
double strtod(const char *nptr, char **endptr);
long strtol(const char *nptr, char **endptr, int base);
unsigned long strtoul(const char *nptr, char **endptr, int base);

#endif
