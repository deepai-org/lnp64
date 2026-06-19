#ifndef LNP64_STDIO_H
#define LNP64_STDIO_H

typedef struct __lnp64_file FILE;

#define EOF (-1)

extern FILE *stdin;
extern FILE *stdout;
extern FILE *stderr;

int fclose(FILE *stream);
int fflush(FILE *stream);
int fprintf(FILE *stream, const char *format, ...);
int printf(const char *format, ...);
int puts(const char *s);
int putchar(int ch);
int fputc(int ch, FILE *stream);
int fputs(const char *s, FILE *stream);
int ferror(FILE *stream);
int fileno(FILE *stream);
int vfprintf(FILE *stream, const char *format, __builtin_va_list ap);
int vsnprintf(char *str, unsigned long size, const char *format,
              __builtin_va_list ap);
int snprintf(char *str, unsigned long size, const char *format, ...);
unsigned long fread(void *ptr, unsigned long size, unsigned long count,
                    FILE *stream);
unsigned long fwrite(const void *ptr, unsigned long size, unsigned long count,
                     FILE *stream);
char *fgets(char *str, int count, FILE *stream);
FILE *fopen(const char *path, const char *mode);

#endif
