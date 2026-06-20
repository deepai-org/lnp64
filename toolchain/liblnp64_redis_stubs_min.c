/*
 * Stubs for Redis features not supported on LNP64:
 * TLS, Sentinel. These return safe no-ops/errors
 * so redis-server can start and handle basic commands.
 */
#include <stddef.h>
#include <stdlib.h>
#include <math.h>
#include <errno.h>

/* ---- TLS stubs ---- */
void *connCreateTLS(void)          { return NULL; }
void *connCreateAcceptedTLS(void *el, int fd, int reqs) {
  (void)el; (void)fd; (void)reqs; return NULL;
}
char *connTLSGetPeerCert(void *conn) { (void)conn; return NULL; }
void  tlsProcessPendingData(void)  {}
int   tlsHasPendingData(void)      { return 0; }
int   tlsConfigure(void *ctx)      { (void)ctx; return 0; }
int   tlsInit(void)                { return 0; }

/* ---- Sentinel stubs ---- */
void sentinelCommand(void *c)       { (void)c; }
void sentinelPublishCommand(void *c){ (void)c; }
void sentinelRoleCommand(void *c)   { (void)c; }
void sentinelInfoCommand(void *c)   { (void)c; }
void sentinelTimer(void *el, long long id, void *data) {
  (void)el; (void)id; (void)data;
}
int  queueSentinelConfig(void *cfg, int argc, int linenum, char **err) {
  (void)cfg; (void)argc; (void)linenum; (void)err; return 1;
}
int  rewriteConfigSentinelOption(void *state) { (void)state; return 0; }
void initSentinelConfig(void)      {}
void initSentinel(void)            {}
void loadSentinelConfigFromQueue(void) {}
int  sentinelCheckConfigFile(void) { return 0; }
void sentinelIsRunning(void)       {}

/* ---- Math stubs ---- */
long long llroundl(long double x) { return (long long)(x + (x >= 0 ? 0.5L : -0.5L)); }
long long llabs(long long x)      { return x < 0 ? -x : x; }
long double ceill(long double x)  { return (long double)ceil((double)x); }
void tzset(void)                   {}

/* ---- ioctl stub ---- */
#include <stdarg.h>
int ioctl(int fd, int request, ...) {
  (void)fd; (void)request; return -1;
}

/* ---- Process stubs ---- */
int setsid(void) { return (int)1; }

/* ---- pthread stubs ---- */
int pthread_setcanceltype(int type, int *oldtype) {
  (void)type;
  if (oldtype) *oldtype = 0;
  return 0;
}

