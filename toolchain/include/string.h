#ifndef LNP64_STRING_H
#define LNP64_STRING_H

#include <stddef.h>

void *memchr(const void *s, int c, size_t n);
int memcmp(const void *lhs, const void *rhs, size_t n);
void *memcpy(void *dst, const void *src, size_t n);
void *memmem(const void *haystack, size_t haystack_len, const void *needle,
             size_t needle_len);
void *memmove(void *dst, const void *src, size_t n);
void *memset(void *dst, int c, size_t n);
char *strchr(const char *s, int c);
int strcmp(const char *lhs, const char *rhs);
int strcoll(const char *lhs, const char *rhs);
char *strcat(char *dst, const char *src);
char *strcpy(char *dst, const char *src);
char *strerror(int errnum);
size_t strcspn(const char *s, const char *reject);
size_t strlcat(char *dst, const char *src, size_t size);
size_t strlcpy(char *dst, const char *src, size_t size);
size_t strlen(const char *s);
char *strpbrk(const char *s, const char *accept);
int strncmp(const char *lhs, const char *rhs, size_t n);
char *strncat(char *dst, const char *src, size_t n);
char *strncpy(char *dst, const char *src, size_t n);
char *strrchr(const char *s, int c);
size_t strspn(const char *s, const char *accept);
char *strstr(const char *haystack, const char *needle);
char *strtok(char *str, const char *delim);
int strcasecmp(const char *a, const char *b);
int strncasecmp(const char *a, const char *b, size_t n);
size_t strnlen(const char *s, size_t maxlen);
char *strndup(const char *s, size_t n);
char *strdup(const char *s);

#endif
