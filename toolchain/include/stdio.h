#ifndef LNP64_STDIO_H
#define LNP64_STDIO_H

#include <stdarg.h>
#include <stddef.h>
#include <sys/types.h>

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
int fseeko(FILE *stream, off_t offset, int whence);
off_t ftello(FILE *stream);
int fscanf(FILE *stream, const char *format, ...);
int vfprintf(FILE *stream, const char *format, va_list ap);
int sprintf(char *str, const char *format, ...);
int vsnprintf(char *str, size_t size, const char *format, va_list ap);
int snprintf(char *str, size_t size, const char *format, ...);
ssize_t getline(char **lineptr, size_t *n, FILE *stream);
size_t fread(void *ptr, size_t size, size_t count, FILE *stream);
size_t fwrite(const void *ptr, size_t size, size_t count, FILE *stream);
char *fgets(char *str, int count, FILE *stream);
FILE *fmemopen(void *buf, size_t size, const char *mode);
FILE *fdopen(int fd, const char *mode);
FILE *fopen(const char *path, const char *mode);
FILE *tmpfile(void);

#endif
