#include <syslog.h>
#include <stdarg.h>
#include <stdio.h>

void openlog(const char *ident, int option, int facility) {
  (void)ident; (void)option; (void)facility;
}

void closelog(void) {}

void syslog(int priority, const char *fmt, ...) {
  (void)priority;
  va_list ap;
  va_start(ap, fmt);
  vfprintf(stderr, fmt, ap);
  va_end(ap);
  fputc('\n', stderr);
}
