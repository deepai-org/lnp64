#ifndef LNP64_STDIO_H
#define LNP64_STDIO_H

typedef struct __lnp64_file FILE;

int fclose(FILE *stream);
int printf(const char *format, ...);
int puts(const char *s);
int putchar(int ch);
char *fgets(char *str, int count, FILE *stream);
FILE *fopen(const char *path, const char *mode);

#endif
