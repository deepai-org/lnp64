#ifndef LNP64_STRING_H
#define LNP64_STRING_H

#include <stddef.h>

void *memchr(const void *s, int c, size_t n);
int memcmp(const void *lhs, const void *rhs, size_t n);
void *memcpy(void *dst, const void *src, size_t n);
void *memmove(void *dst, const void *src, size_t n);
void *memset(void *dst, int c, size_t n);
char *strchr(const char *s, int c);
int strcmp(const char *lhs, const char *rhs);
char *strcpy(char *dst, const char *src);
size_t strlen(const char *s);
int strncmp(const char *lhs, const char *rhs, size_t n);
char *strncpy(char *dst, const char *src, size_t n);

#endif
