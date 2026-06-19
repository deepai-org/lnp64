#ifndef LNP64_STDIO_H
#define LNP64_STDIO_H

#include <stdarg.h>

typedef struct __lnp64_file FILE;

#define EOF (-1)
#define BUFSIZ 1024

extern FILE *stdin;
extern FILE *stdout;
extern FILE *stderr;

int fclose(FILE *stream);
int fflush(FILE *stream);
int fprintf(FILE *stream, const char *format, ...);
int printf(const char *format, ...);
int puts(const char *s);
int putchar(int ch);
int getchar(void);
int rename(const char *oldpath, const char *newpath);
int fputc(int ch, FILE *stream);
int fputs(const char *s, FILE *stream);
int fgetc(FILE *stream);
int getc(FILE *stream);
int ungetc(int ch, FILE *stream);
int ferror(FILE *stream);
int feof(FILE *stream);
void clearerr(FILE *stream);
int fileno(FILE *stream);
int fseek(FILE *stream, long offset, int whence);
long ftell(FILE *stream);
int fseeko(FILE *stream, long offset, int whence);
long ftello(FILE *stream);
int fscanf(FILE *stream, const char *format, ...);
int vfprintf(FILE *stream, const char *format, va_list ap);
int sprintf(char *str, const char *format, ...);
int vsnprintf(char *str, unsigned long size, const char *format, va_list ap);
int snprintf(char *str, unsigned long size, const char *format, ...);
long getline(char **lineptr, unsigned long *n, FILE *stream);
unsigned long fread(void *ptr, unsigned long size, unsigned long count,
                    FILE *stream);
unsigned long fwrite(const void *ptr, unsigned long size, unsigned long count,
                     FILE *stream);
char *fgets(char *str, int count, FILE *stream);
FILE *fmemopen(void *buf, unsigned long size, const char *mode);
FILE *fdopen(int fd, const char *mode);
FILE *fopen(const char *path, const char *mode);
FILE *tmpfile(void);

#endif
