#ifndef LNP64_DLFCN_H
#define LNP64_DLFCN_H

#define RTLD_LAZY   1
#define RTLD_NOW    2
#define RTLD_GLOBAL 0x100
#define RTLD_LOCAL  0

typedef struct {
  const char *dli_fname;
  void       *dli_fbase;
  const char *dli_sname;
  void       *dli_saddr;
} Dl_info;

int dladdr(const void *addr, Dl_info *info);
void *dlopen(const char *filename, int flags);
void *dlsym(void *handle, const char *symbol);
int   dlclose(void *handle);
char *dlerror(void);

#endif
