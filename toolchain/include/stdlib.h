#ifndef LNP64_STDLIB_H
#define LNP64_STDLIB_H

#include <stddef.h>

#define EXIT_FAILURE 1
#define EXIT_SUCCESS 0

void abort(void);
int clearenv(void);
void exit(int status);
void free(void *ptr);
char *getenv(const char *name);
void *malloc(size_t size);
void *calloc(size_t count, size_t size);
int putenv(char *string);
long random(void);
void *realloc(void *ptr, size_t size);
int setenv(const char *name, const char *value, int overwrite);
int mkstemp(char *template);
char *initstate(unsigned int seed, char *state, size_t size);
void srandom(unsigned int seed);
char *setstate(char *state);
int unsetenv(const char *name);
int atoi(const char *nptr);
long atol(const char *nptr);
void qsort(void *base, size_t count, size_t size,
           int (*compar)(const void *, const void *));
int abs(int value);
long labs(long value);
long long llabs(long long value);
double strtod(const char *nptr, char **endptr);
long strtol(const char *nptr, char **endptr, int base);
unsigned long strtoul(const char *nptr, char **endptr, int base);
long long strtoll(const char *nptr, char **endptr, int base);
unsigned long long strtoull(const char *nptr, char **endptr, int base);

#endif
