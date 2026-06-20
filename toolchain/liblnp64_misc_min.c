#include <sys/file.h>
#include <glob.h>
#include <dlfcn.h>
#include <errno.h>
#include <string.h>
#include <stdlib.h>
#include <sys/resource.h>

int flock(int fd, int operation) { (void)fd; (void)operation; return 0; }

int glob(const char *pattern, int flags,
         int (*errfunc)(const char *epath, int eerrno), glob_t *pglob) {
  (void)pattern; (void)flags; (void)errfunc;
  if (pglob) { pglob->gl_pathc = 0; pglob->gl_pathv = 0; pglob->gl_offs = 0; }
  return GLOB_NOMATCH;
}
void globfree(glob_t *pglob) {
  if (pglob) { pglob->gl_pathc = 0; pglob->gl_pathv = 0; }
}

void *dlopen(const char *filename, int flags) {
  (void)filename; (void)flags; return 0;
}
void *dlsym(void *handle, const char *symbol) {
  (void)handle; (void)symbol; return 0;
}
int dlclose(void *handle) { (void)handle; return 0; }
char *dlerror(void) { return "dlopen not supported"; }
int dladdr(const void *addr, Dl_info *info) {
  (void)addr;
  if (info) { info->dli_fname=0; info->dli_fbase=0; info->dli_sname=0; info->dli_saddr=0; }
  return 0;
}

#include <sys/utsname.h>
#include <string.h>
int uname(struct utsname *buf) {
  if (!buf) return -1;
  strncpy(buf->sysname,  "LNP64",    sizeof(buf->sysname)-1);
  strncpy(buf->nodename, "lnp64",    sizeof(buf->nodename)-1);
  strncpy(buf->release,  "1.0",      sizeof(buf->release)-1);
  strncpy(buf->version,  "#1",       sizeof(buf->version)-1);
  strncpy(buf->machine,  "lnp64",    sizeof(buf->machine)-1);
  return 0;
}

int getpeername(int fd, void *addr, unsigned long *len) {
  (void)fd; (void)addr; (void)len; return -1;
}
long sendto(int fd, const void *buf, size_t len, int flags,
            const void *dest, unsigned long addrlen) {
  (void)flags; (void)dest; (void)addrlen;
  extern long write(int,const void*,size_t);
  return write(fd, buf, len);
}
long recvfrom(int fd, void *buf, size_t len, int flags,
              void *src, unsigned long *addrlen) {
  (void)flags; (void)src; (void)addrlen;
  extern long read(int,void*,size_t);
  return read(fd, buf, len);
}
int shutdown(int fd, int how) { (void)fd; (void)how; return 0; }
